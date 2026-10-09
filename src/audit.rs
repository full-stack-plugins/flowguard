//! Bounded process-local retained artifacts and minimized tenant audit.
use crate::{
    gate::GateRun,
    run_store::MemoryRunStore,
    trust::{Access, TenantPolicy},
};
use guardengine::integration::eligibility::{
    AuditRecord, AuthorityProvider, EligibilityPolicy, EligibilityResult,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
#[derive(Clone, Copy)]
pub struct Retention {
    pub artifact_seconds: i64,
    pub audit_seconds: i64,
    pub max_entries: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError {
    Invalid,
    Budget,
    Access,
    ClockRollback,
    Missing,
    Expired,
    Conflict,
    Store,
    Evidence,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Retain,
    Consume,
    Remove,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Accepted,
    AccessDenied,
    ArtifactsUnavailable,
    EvidenceRejected,
}
#[derive(Clone, Serialize)]
pub struct Entry {
    pub sequence: u64,
    pub operation: Operation,
    pub outcome: Outcome,
    pub tenant_digest: String,
    pub caller_digest: Option<String>,
    pub run_digest: String,
    pub timestamp: i64,
    pub engine: Option<AuditRecord>,
}
struct Retained {
    run: Option<Arc<GateRun>>,
    expires_at: i64,
    bytes: usize,
}
#[derive(Default)]
struct State {
    runs: BTreeMap<String, Retained>,
    entries: Vec<Entry>,
    bytes: usize,
    last_time: i64,
}
pub struct LocalAuditStore<'a> {
    policy: TenantPolicy,
    retention: Retention,
    store: &'a MemoryRunStore,
    state: Mutex<State>,
}
const MAX_BYTES: usize = 64 * 1024 * 1024;
impl<'a> LocalAuditStore<'a> {
    pub fn new(
        policy: TenantPolicy,
        retention: Retention,
        store: &'a MemoryRunStore,
    ) -> Result<Self, AuditError> {
        if retention.artifact_seconds <= 0
            || retention.audit_seconds <= 0
            || retention.max_entries == 0
            || retention.max_entries > 1024
        {
            return Err(AuditError::Invalid);
        }
        Ok(Self {
            policy,
            retention,
            store,
            state: Mutex::new(State::default()),
        })
    }
    fn state(&self) -> Result<std::sync::MutexGuard<'_, State>, AuditError> {
        self.state.lock().map_err(|_| AuditError::Store)
    }
    fn authorize(&self, access: Access<'_>, read: bool) -> Result<String, AuditError> {
        {
            let mut state = self.state()?;
            if access.now < 0 || access.now < state.last_time {
                return Err(AuditError::ClockRollback);
            }
            state.last_time = access.now;
        }
        self.policy
            .authorize(access, read)
            .map_err(|_| AuditError::Access)
    }
    #[allow(clippy::too_many_arguments)]
    fn append(
        &self,
        state: &mut State,
        id: &str,
        caller: Option<String>,
        operation: Operation,
        outcome: Outcome,
        engine: Option<AuditRecord>,
        now: i64,
    ) -> Result<(), AuditError> {
        if state.entries.len() >= self.retention.max_entries {
            return Err(AuditError::Budget);
        }
        if now < state.last_time {
            return Err(AuditError::ClockRollback);
        }
        now.checked_add(self.retention.audit_seconds)
            .ok_or(AuditError::Invalid)?;
        state.entries.push(Entry {
            sequence: state.entries.len() as u64 + 1,
            operation,
            outcome,
            tenant_digest: crate::digest(self.policy.tenant.as_bytes()),
            caller_digest: caller,
            run_digest: crate::digest(id.as_bytes()),
            timestamp: now,
            engine,
        });
        Ok(())
    }
    pub fn retain(&self, run: GateRun, access: Access<'_>) -> Result<(), AuditError> {
        let caller = self.authorize(access, false)?;
        if run.envelope().binding.repo_id != self.policy.repo {
            return Err(AuditError::Access);
        }
        let bytes = run
            .retained_size(MAX_BYTES)
            .map_err(|_| AuditError::Budget)?;
        self.store
            .check_current_runs(&[&run])
            .map_err(|_| AuditError::Store)?;
        let expires_at = access
            .now
            .checked_add(self.retention.artifact_seconds)
            .ok_or(AuditError::Invalid)?;
        access
            .now
            .checked_add(self.retention.audit_seconds)
            .ok_or(AuditError::Invalid)?;
        let id = run.envelope().run_id.clone();
        let mut state = self.state()?;
        if state.runs.contains_key(&id) {
            return Err(AuditError::Conflict);
        }
        if state.runs.len() >= 1024 || state.bytes.checked_add(bytes).is_none_or(|n| n > MAX_BYTES)
        {
            return Err(AuditError::Budget);
        }
        self.append(
            &mut state,
            &id,
            Some(caller),
            Operation::Retain,
            Outcome::Accepted,
            None,
            access.now,
        )?;
        state.bytes += bytes;
        state.runs.insert(
            id,
            Retained {
                run: Some(Arc::new(run)),
                expires_at,
                bytes,
            },
        );
        Ok(())
    }
    fn retained(&self, id: &str, now: i64) -> Result<Arc<GateRun>, AuditError> {
        let mut state = self.state()?;
        if now < state.last_time {
            return Err(AuditError::ClockRollback);
        }
        let entry = state.runs.get_mut(id).ok_or(AuditError::Missing)?;
        if now >= entry.expires_at {
            let removed = if entry.run.take().is_some() {
                entry.bytes
            } else {
                0
            };
            state.bytes -= removed;
            return Err(AuditError::Expired);
        }
        entry.run.clone().ok_or(AuditError::Missing)
    }
    pub fn consume(
        &self,
        id: &str,
        policy: &EligibilityPolicy,
        provider: &dyn AuthorityProvider,
        access: Access<'_>,
    ) -> Result<EligibilityResult, AuditError> {
        if id.len() > 256 {
            return Err(AuditError::Budget);
        }
        let caller = match self.authorize(access, false) {
            Ok(c) => c,
            Err(e) => {
                if e == AuditError::Access {
                    // Denied callers must not learn capacity or run existence.
                    // Failure to audit an already refused access cannot grant it.
                    let _ = self.append(
                        &mut *self.state()?,
                        id,
                        None,
                        Operation::Consume,
                        Outcome::AccessDenied,
                        None,
                        access.now,
                    );
                }
                return Err(e);
            }
        };
        let mut engine = None;
        let result = (|| {
            let run = self.retained(id, access.now)?;
            let result = self
                .store
                .evaluate_eligibility(&run, policy, provider, access.now)
                .map_err(|_| AuditError::Evidence)?;
            engine = Some(result.audit.clone());
            if !result.eligible {
                return Err(AuditError::Evidence);
            }
            if self.authorize(access, false)? != caller {
                return Err(AuditError::Access);
            }
            self.store
                .check_current_runs(&[&run])
                .map_err(|_| AuditError::Store)?;
            let current = self.retained(id, access.now)?;
            if !Arc::ptr_eq(&run, &current) {
                return Err(AuditError::Conflict);
            }
            Ok(result)
        })();
        let outcome = match &result {
            Ok(_) => Outcome::Accepted,
            Err(AuditError::Access) => Outcome::AccessDenied,
            Err(AuditError::Missing | AuditError::Expired) => Outcome::ArtifactsUnavailable,
            Err(_) => Outcome::EvidenceRejected,
        };
        self.append(
            &mut *self.state()?,
            id,
            Some(caller),
            Operation::Consume,
            outcome,
            engine,
            access.now,
        )?;
        result
    }
    pub fn remove(&self, id: &str, access: Access<'_>) -> Result<(), AuditError> {
        if id.len() > 256 {
            return Err(AuditError::Budget);
        }
        let caller = self.authorize(access, false)?;
        let mut state = self.state()?;
        if !state.runs.get(id).is_some_and(|r| r.run.is_some()) {
            return Err(AuditError::Missing);
        }
        self.append(
            &mut state,
            id,
            Some(caller),
            Operation::Remove,
            Outcome::Accepted,
            None,
            access.now,
        )?;
        let entry = state.runs.get_mut(id).expect("checked under lock");
        entry.run = None;
        let bytes = entry.bytes;
        state.bytes -= bytes;
        Ok(())
    }
    pub fn records(&self, access: Access<'_>) -> Result<Vec<Entry>, AuditError> {
        self.authorize(access, true)?;
        let state = self.state()?;
        if access.now < state.last_time {
            return Err(AuditError::ClockRollback);
        }
        // Logical read expiry only: historical entries are never rewritten or deleted.
        Ok(state
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .timestamp
                    .checked_add(self.retention.audit_seconds)
                    .is_some_and(|end| access.now < end)
            })
            .cloned()
            .collect())
    }
}
