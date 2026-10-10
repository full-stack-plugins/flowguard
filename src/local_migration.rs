//! Narrow process-local stage routing. Callers supply existing protected plans and
//! authority ports. No persisted mode file, host controller or approval import.
use crate::{
    gate::GateRun,
    run_store::MemoryRunStore,
    stage_qualification::{ProtectedStagePlan, QualificationError, QualifiedStage},
    stage_transition::{StageDeclaration, StageState},
};
use guardengine::integration::eligibility::AuthorityProvider;
use serde::Serialize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteMode {
    Advisory,
    Shadow,
    OptIn,
}
#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub sequence: usize,
    pub event: String,
    pub state: StageState,
    pub mode: RouteMode,
    pub legacy_source_digest: String,
}
pub struct LocalStageRoute<'a> {
    receipt: Option<QualifiedStage<'a>>,
    store: Option<&'a MemoryRunStore>,
    plan: &'a ProtectedStagePlan,
    state: StageState,
    mode: RouteMode,
    history: Vec<HistoryEntry>,
    legacy_source_digest: String,
}
impl<'a> LocalStageRoute<'a> {
    pub fn new(
        plan: &'a ProtectedStagePlan,
        historical_state: StageState,
        legacy_source_digest: String,
    ) -> Result<Self, QualificationError> {
        if !crate::valid_digest(&legacy_source_digest) {
            return Err(QualificationError::Binding);
        }
        let mut route = Self {
            receipt: None,
            store: None,
            plan,
            state: historical_state,
            mode: RouteMode::Advisory,
            history: Vec::new(),
            legacy_source_digest,
        };
        route.append("legacy_history_only")?;
        route.state = if matches!(
            historical_state,
            StageState::Accepted
                | StageState::Inherited
                | StageState::Skipped
                | StageState::Invalidated
        ) {
            StageState::Invalidated
        } else {
            StageState::InProgress
        };
        route.append("new_workflow_unqualified")?;
        Ok(route)
    }
    /// Bind this route to the existing publication store. All qualification and
    /// consumption (including shadow observations) recheck the current run set.
    pub fn new_published(
        plan: &'a ProtectedStagePlan,
        historical_state: StageState,
        legacy_source_digest: String,
        store: &'a MemoryRunStore,
    ) -> Result<Self, QualificationError> {
        let mut route = Self::new(plan, historical_state, legacy_source_digest)?;
        route.store = Some(store);
        Ok(route)
    }

    pub fn state(&self) -> StageState {
        self.state
    }
    pub fn mode(&self) -> RouteMode {
        self.mode
    }
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }
    pub fn submit(&mut self) -> Result<(), QualificationError> {
        self.declare(StageState::PendingAcceptance, "submit")
    }
    pub fn invalidate(&mut self) -> Result<(), QualificationError> {
        self.declare(StageState::Invalidated, "invalidate")
    }
    pub fn set_mode(&mut self, mode: RouteMode) -> Result<(), QualificationError> {
        // Increasing authority needs an audit slot; stopping it must not.
        if mode == RouteMode::OptIn {
            self.capacity()?;
        }
        self.mode = mode;
        self.append("route_changed")
    }
    pub fn rollback(&mut self) -> Result<(), QualificationError> {
        // Fail closed even if recording the stop returns Budget.
        self.mode = RouteMode::Advisory;
        self.append("rollback_new_route_stopped")
    }
    pub fn qualify(
        &mut self,
        run: &'a GateRun,
        approval: &'a str,
        dependencies: Vec<&'a QualifiedStage<'a>>,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<bool, QualificationError> {
        self.capacity()?;
        let receipt =
            match self
                .plan
                .qualify(self.state, run, approval, dependencies, provider, now)
            {
                Ok(receipt) => receipt,
                Err(error) => {
                    self.append("qualification_refused")?;
                    return Err(error);
                }
            };
        // Preserve the original detached advisory/shadow call pattern.
        if self.store.is_none() && self.mode != RouteMode::OptIn {
            self.append("shadow_qualified_no_transfer")?;
            return Ok(false);
        }
        let state = match self.store {
            Some(store) => receipt.consume_published(self.plan, store, provider, now),
            None => receipt.consume(self.plan, provider, now),
        };
        let state = match state {
            Ok(state) => state,
            Err(error) => {
                self.append("qualification_refused")?;
                return Err(error);
            }
        };
        if self.mode != RouteMode::OptIn {
            self.append("shadow_qualified_no_transfer")?;
            return Ok(false);
        }
        self.state = state;
        self.append("qualified")?;
        self.receipt = Some(receipt);
        Ok(true)
    }
    pub fn edited(&mut self, plan: &'a ProtectedStagePlan) -> Result<(), QualificationError> {
        if plan.stage_id() != self.plan.stage_id() || plan.digest() == self.plan.digest() {
            return Err(QualificationError::Binding);
        }
        // Already-invalidated imports and subsequent edits stay unqualified;
        // replacing their plan must not require qualifying the obsolete plan.
        if self.state == StageState::Invalidated {
            self.append("edited_while_invalidated")?;
        } else {
            // Stop consumption even when the audit append runs out of space.
            self.invalidate()?;
        }
        self.plan = plan;
        self.receipt = None;
        Ok(())
    }
    pub fn consume(
        &self,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<StageState, QualificationError> {
        if self.mode != RouteMode::OptIn {
            return Err(QualificationError::Transition);
        }
        let receipt = self
            .receipt
            .as_ref()
            .ok_or(QualificationError::Transition)?;
        match self.store {
            Some(store) => receipt.consume_published(self.plan, store, provider, now),
            None => receipt.consume(self.plan, provider, now),
        }
    }
    pub(crate) fn published_release_entry(
        &self,
        store: &MemoryRunStore,
    ) -> Result<(&QualifiedStage<'a>, &ProtectedStagePlan), QualificationError> {
        if self.mode != RouteMode::OptIn {
            return Err(QualificationError::Transition);
        }
        if !self.store.is_some_and(|bound| std::ptr::eq(bound, store)) {
            return Err(QualificationError::Binding);
        }
        Ok((
            self.receipt
                .as_ref()
                .ok_or(QualificationError::Transition)?,
            self.plan,
        ))
    }

    fn capacity(&self) -> Result<(), QualificationError> {
        if self.history.len() >= 1024 {
            Err(QualificationError::Budget)
        } else {
            Ok(())
        }
    }
    fn append(&mut self, event: &str) -> Result<(), QualificationError> {
        self.capacity()?;
        self.history.push(HistoryEntry {
            sequence: self.history.len() + 1,
            event: event.into(),
            state: self.state,
            mode: self.mode,
            legacy_source_digest: self.legacy_source_digest.clone(),
        });
        Ok(())
    }
    fn declare(&mut self, state: StageState, event: &str) -> Result<(), QualificationError> {
        // Invalidating a satisfying state revokes the retained receipt before
        // attempting to append. Submission still requires available history.
        if state != StageState::Invalidated {
            self.capacity()?;
        }
        self.state = StageDeclaration::observed(self.state)
            .apply_declaration(state)
            .map_err(|_| QualificationError::Transition)?
            .state();
        self.receipt = None;
        self.append(event)
    }
}
