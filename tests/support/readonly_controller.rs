//! Test-only trusted-process controller. No executor or Git writer interface.
use flowguard::gate::{GateRun, PendingGate, SpecialistEvidence};
use guardengine::integration::{GuardRunEnvelope, Producer, RunBinding, eligibility::*};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub const PROFILE: &str = "flowguard.test.readonly-controller/v1";
const PRINCIPAL: &str = "fixture-controller://readonly-dag";
const MAX_CALLS: usize = 256;

/// This fixture port returns only eligibility. It cannot merge, grant or release.
pub struct ReadOnlyController<'a> {
    policy: EligibilityPolicy,
    upstream: &'a dyn AuthorityProvider,
    issued: RefCell<Option<ProducerRecord>>,
    calls: RefCell<Vec<&'static str>>,
}
impl<'a> ReadOnlyController<'a> {
    /// Must be called before running the pending gate. Expected values derive from
    /// protected inputs and the native mapping, never a returned GateRun.
    pub fn freeze(
        binding: &RunBinding,
        pending: &PendingGate,
        upstream: &'a dyn AuthorityProvider,
    ) -> Self {
        let scopes = pending.required_scopes().to_vec();
        let contract =
            flowguard::projection::project(&binding.source_snapshot_digest, &scopes, &[], &[])
                .unwrap()
                .contract;
        let policy = EligibilityPolicy {
            binding: binding.clone(),
            producer: Producer {
                guard: "flowguard".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                analyzer_id: "flowguard.stage-gates".into(),
                analyzer_version: env!("CARGO_PKG_VERSION").into(),
            },
            required_scopes: scopes,
            contract_digest: flowguard::digest(&serde_json::to_vec(&contract).unwrap()),
            action: "commit".into(),
            producer_principals: BTreeSet::from([PRINCIPAL.into()]),
            approval_principals: BTreeMap::new(),
        };
        Self {
            policy,
            upstream,
            issued: RefCell::new(None),
            calls: RefCell::new(Vec::new()),
        }
    }
    fn record(&self, call: &'static str) -> Result<(), AuthorityError> {
        let mut calls = self.calls.borrow_mut();
        if calls.len() >= MAX_CALLS {
            return Err(AuthorityError::Unavailable);
        }
        calls.push(call);
        Ok(())
    }
    pub fn calls(&self) -> Vec<&'static str> {
        self.calls.borrow().clone()
    }
    /// Issuance is private to this owned evaluation. No arbitrary artifact/digest
    /// registration entrypoint exists, and technical output is never an executor.
    pub fn run(
        &self,
        pending: PendingGate,
        evidence: &[SpecialistEvidence<'_>],
        now: i64,
        finished: &str,
    ) -> GateRun {
        self.record("read-specialists").unwrap();
        let run = pending.evaluate(evidence, self, now, finished).unwrap();
        self.record("technical-gate").unwrap();
        let digest = flowguard::digest(&serde_json::to_vec(run.envelope()).unwrap());
        assert!(self.issued.borrow().is_none());
        *self.issued.borrow_mut() = Some(ProducerRecord {
            principal: PRINCIPAL.into(),
            producer: self.policy.producer.clone(),
            envelope_digest: digest,
            validity: Validity {
                issued_at: now,
                expires_at: now.checked_add(60).unwrap(),
                revoked: false,
            },
        });
        run
    }
    /// Controller caller supplies a fresh, protected current candidate observation;
    /// the integration harness obtains it from its real local repository.
    pub fn preview(&self, run: &GateRun, current_candidate: &str, now: i64) -> bool {
        if self.record("read-current-candidate").is_err() {
            return false;
        }
        if current_candidate != self.policy.binding.candidate_oid {
            return false;
        }
        if self.record("refresh-eligibility").is_err() {
            return false;
        }
        run.evaluate_eligibility(&self.policy, self, now)
            .is_ok_and(|result| result.eligible)
    }
}
impl AuthorityProvider for ReadOnlyController<'_> {
    fn verify_producer(
        &self,
        envelope: &GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.record("verify-producer")?;
        if envelope.producer == self.policy.producer {
            return self
                .issued
                .borrow()
                .as_ref()
                .filter(|r| r.envelope_digest == digest)
                .cloned()
                .ok_or(AuthorityError::Untrusted);
        }
        self.upstream.verify_producer(envelope, digest)
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.record("verify-approval")?;
        self.upstream.verify_approval(reference)
    }
}
