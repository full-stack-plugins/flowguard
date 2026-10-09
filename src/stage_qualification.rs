//! Protected local controller stage qualification, separate from declarations.
use crate::{
    baseline::BaselineRef,
    context::ValidatedBinding,
    dependencies::StageGraph,
    gate::{GateRun, PendingGate},
    obligations::FrozenObligations,
    stage_transition::StageState,
};
use guardengine::integration::{
    Producer,
    eligibility::{AuthorityProvider, EligibilityPolicy, validate_approval_record},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize)]
pub enum StageMode {
    Accept,
    ProtectedSkip { policy_digest: String },
    LocalControllerInheritance { baseline: BaselineRef },
}
impl StageMode {
    fn target(&self) -> StageState {
        match self {
            Self::Accept => StageState::Accepted,
            Self::ProtectedSkip { .. } => StageState::Skipped,
            Self::LocalControllerInheritance { .. } => StageState::Inherited,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationError {
    InvalidPlan,
    Budget,
    Transition,
    Binding,
    Prerequisites,
    Technical,
    Approval,
}
pub struct StageIntent {
    stage: String,
    graph_digest: String,
    mode: StageMode,
    dependencies: BTreeMap<String, String>,
    action: String,
}
pub struct ProtectedStagePlan {
    intent: StageIntent,
    digest: String,
    context: String,
    work: String,
    policy: EligibilityPolicy,
}
/// Qualification receipts cannot be forged from candidate JSON.
/// ```compile_fail
/// let _: flowguard::stage_qualification::QualifiedStage<'_> =
///     serde_json::from_str("{\"accepted\":true}").unwrap();
/// ```
pub struct QualifiedStage<'a> {
    plan: &'a ProtectedStagePlan,
    run: &'a GateRun,
    approval: &'a str,
    dependencies: Vec<&'a QualifiedStage<'a>>,
}
fn hash(value: &impl Serialize) -> String {
    crate::digest(&serde_json::to_vec(value).expect("closed stage profile"))
}
fn purpose(mode: &StageMode) -> &'static str {
    match mode {
        StageMode::Accept => "stage.accept",
        StageMode::ProtectedSkip { .. } => "stage.skip",
        StageMode::LocalControllerInheritance { .. } => "stage.inherit.local-controller",
    }
}
impl StageIntent {
    pub fn freeze(
        graph: &StageGraph,
        stage: &str,
        mode: StageMode,
        dependencies: &BTreeMap<String, &ProtectedStagePlan>,
    ) -> Result<Self, QualificationError> {
        use QualificationError::*;
        if graph.nodes().len() > 64 || dependencies.len() > 64 || stage.len() > 256 {
            return Err(Budget);
        }
        // Bound graph text before serializing/cloning; graph was already DAG-validated.
        if graph.nodes().values().any(|n| {
            n.id.len() > 256
                || n.owner.len() > 256
                || n.dependencies.len() > 64
                || n.dependencies.iter().any(|d| d.len() > 256)
        }) {
            return Err(Budget);
        }
        let node = graph.nodes().get(stage).ok_or(InvalidPlan)?;
        if !node.dependencies.iter().eq(dependencies.keys()) {
            return Err(Prerequisites);
        }
        let graph_digest = hash(graph);
        if dependencies
            .values()
            .any(|p| p.intent.graph_digest != graph_digest)
        {
            return Err(Prerequisites);
        }
        for (key, plan) in dependencies {
            if key != &plan.intent.stage {
                return Err(Prerequisites);
            }
        }
        match &mode {
            StageMode::Accept => (),
            StageMode::ProtectedSkip { policy_digest } if crate::valid_digest(policy_digest) => (),
            StageMode::LocalControllerInheritance { baseline } => {
                if baseline.version != "flowguard.workflow/v1"
                    || !matches!(baseline.revision.len(), 40 | 64)
                    || !baseline
                        .revision
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    || !crate::valid_digest(&baseline.policy_digest)
                    || baseline.requirements.is_empty()
                    || baseline.requirements.iter().any(|r| r.trim().is_empty())
                    || baseline.repo_id.trim().is_empty()
                    || baseline.approval_ref.trim().is_empty()
                    || baseline.stage != node.stage
                    || baseline.requirements.len() > 256
                    || baseline.requirements.iter().any(|r| r.len() > 256)
                    || baseline.repo_id.len() > 256
                    || baseline.approval_ref.len() > 256
                    || !crate::valid_digest(&baseline.content_digest)
                {
                    return Err(InvalidPlan);
                }
            }
            _ => return Err(InvalidPlan),
        }
        let dependencies: BTreeMap<_, _> = dependencies
            .iter()
            .map(|(id, p)| (id.clone(), p.digest.clone()))
            .collect();
        let action = format!(
            "{}:{}",
            purpose(&mode),
            hash(&(
                "flowguard.stage-intent/v1alpha1",
                &graph_digest,
                stage,
                &mode,
                &dependencies
            ))
        );
        Ok(Self {
            stage: stage.into(),
            graph_digest,
            mode,
            dependencies,
            action,
        })
    }
    pub fn action(&self) -> &str {
        &self.action
    }
}
impl ProtectedStagePlan {
    pub fn freeze(
        intent: StageIntent,
        binding: &ValidatedBinding,
        frozen: &FrozenObligations,
        pending: &PendingGate,
        producers: BTreeSet<String>,
        approvers: BTreeSet<String>,
    ) -> Result<Self, QualificationError> {
        use QualificationError::*;
        for principals in [&producers, &approvers] {
            if principals.is_empty()
                || principals.len() > 64
                || principals
                    .iter()
                    .any(|p| p.trim().is_empty() || p.len() > 256)
            {
                return Err(Budget);
            }
        }
        if intent.graph_digest != frozen.graph_digest()
            || !pending.stage_context_matches(binding, frozen, &intent.action)
        {
            return Err(Binding);
        }
        if let StageMode::LocalControllerInheritance { baseline } = &intent.mode {
            for requirement in &binding.binding().requirement_ids {
                crate::baseline::resolve_baseline(
                    baseline,
                    &binding.binding().repo_id,
                    requirement,
                    &baseline.revision,
                    &baseline.content_digest,
                    &baseline.policy_digest,
                )
                .map_err(|_| InvalidPlan)?;
            }
        }
        let work = pending.store_identity();
        let contract = crate::projection::project(
            &work.binding.source_snapshot_digest,
            &work.required,
            &work.required,
            &[],
        )
        .map_err(|_| InvalidPlan)?
        .contract;
        let policy = EligibilityPolicy {
            binding: work.binding,
            producer: Producer {
                guard: "flowguard".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                analyzer_id: "flowguard.stage-gates".into(),
                analyzer_version: env!("CARGO_PKG_VERSION").into(),
            },
            required_scopes: work.required,
            contract_digest: crate::digest(
                &serde_json::to_vec(&contract).map_err(|_| InvalidPlan)?,
            ),
            action: intent.action.clone(),
            producer_principals: producers,
            approval_principals: BTreeMap::from([(purpose(&intent.mode).into(), approvers)]),
        };
        let context = binding.domain_digest();
        let digest = hash(&(
            "flowguard.stage-plan/v1alpha1",
            &intent.action,
            &context,
            &work.digest,
            &policy,
        ));
        Ok(Self {
            intent,
            digest,
            context,
            work: work.digest,
            policy,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    /// Expected native policy, derived before execution; never filled from GateRun.
    pub fn approval_policy(&self) -> &EligibilityPolicy {
        &self.policy
    }
    pub fn qualify<'a>(
        &'a self,
        from: StageState,
        run: &'a GateRun,
        approval: &'a str,
        dependencies: Vec<&'a QualifiedStage<'a>>,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<QualifiedStage<'a>, QualificationError> {
        use crate::stage_transition::{TransitionCheck, transition};
        let expected = match self.intent.mode {
            StageMode::Accept => TransitionCheck::RequiresCurrentTechnicalEvidenceAndApproval,
            StageMode::ProtectedSkip { .. } => TransitionCheck::RequiresProtectedSkipAndApproval,
            StageMode::LocalControllerInheritance { .. } => {
                TransitionCheck::RequiresExactBaselineAndApproval
            }
        };
        if transition(from, self.intent.mode.target()) != expected {
            return Err(QualificationError::Transition);
        }
        if dependencies.len() > 64 || approval.is_empty() || approval.len() > 256 {
            return Err(QualificationError::Budget);
        }
        let receipt = QualifiedStage {
            plan: self,
            run,
            approval,
            dependencies,
        };
        receipt.consume(self, provider, now)?;
        Ok(receipt)
    }
}
impl QualifiedStage<'_> {
    /// Trusted time/current protected plan are supplied by the controller on every use.
    pub fn consume(
        &self,
        current: &ProtectedStagePlan,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<StageState, QualificationError> {
        if current.digest != self.plan.digest {
            return Err(QualificationError::Binding);
        }
        let mut count = 0;
        self.refresh(provider, now, 0, &mut count)?;
        Ok(self.plan.intent.mode.target())
    }
    fn refresh(
        &self,
        provider: &dyn AuthorityProvider,
        now: i64,
        depth: usize,
        count: &mut usize,
    ) -> Result<(), QualificationError> {
        use QualificationError::*;
        if depth >= 64 || *count >= 1024 || self.dependencies.len() > 64 {
            return Err(Budget);
        }
        *count += 1;
        let p = self.plan;
        if p.work != self.run.work_digest() {
            return Err(Binding);
        }
        if self.dependencies.len() != p.intent.dependencies.len() {
            return Err(Prerequisites);
        }
        let mut seen = BTreeSet::new();
        for dependency in &self.dependencies {
            let dp = dependency.plan;
            if !seen.insert(dp.intent.stage.as_str())
                || p.intent.dependencies.get(&dp.intent.stage) != Some(&dp.digest)
                || dp.context != p.context
                || dp.intent.graph_digest != p.intent.graph_digest
            {
                return Err(Prerequisites);
            }
            dependency.refresh(provider, now, depth + 1, count)?;
        }
        let outcome = self
            .run
            .evaluate_eligibility(&p.policy, provider, now)
            .map_err(|_| Technical)?;
        if !outcome.eligible {
            return Err(Technical);
        }
        let approval = provider
            .verify_approval(self.approval)
            .map_err(|_| Approval)?;
        validate_approval_record(&approval, &p.policy, purpose(&p.intent.mode), now)
            .map_err(|_| Approval)?;
        Ok(())
    }
}
