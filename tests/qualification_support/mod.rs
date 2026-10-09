#![allow(dead_code)]
#[path = "../common/mod.rs"]
mod common;
pub use common::*;
use flowguard::{
    context::ValidatedBinding, dependencies::*, gate::*, obligations::*, stage_qualification::*,
};
use guardengine::{
    Enforcement,
    integration::{eligibility::*, *},
};
use std::collections::{BTreeMap, BTreeSet};
pub fn graph() -> StageGraph {
    build_graph(
        vec![
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "first".into(),
                stage: "01-requirements".into(),
                owner: "a".into(),
                source_digest: flowguard::digest(b"first"),
                dependencies: BTreeSet::new(),
            },
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "second".into(),
                stage: "03-solution".into(),
                owner: "a".into(),
                source_digest: flowguard::digest(b"second"),
                dependencies: BTreeSet::from(["first".into()]),
            },
        ],
        GraphLimits::default(),
    )
    .unwrap()
}
#[derive(Default)]
pub struct Controller {
    pub issuers: BTreeMap<String, ProducerRecord>,
    pub approvals: BTreeMap<String, ApprovalRecord>,
    pub revoked: bool,
}
impl AuthorityProvider for Controller {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        let mut record = self
            .issuers
            .get(d)
            .cloned()
            .ok_or(AuthorityError::Untrusted)?;
        record.validity.revoked = self.revoked;
        Ok(record)
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.approvals
            .get(r)
            .cloned()
            .ok_or(AuthorityError::Unavailable)
    }
}
fn record(controller: &mut Controller, e: &GuardRunEnvelope) {
    let d = flowguard::digest(&serde_json::to_vec(e).unwrap());
    controller.issuers.insert(
        d.clone(),
        ProducerRecord {
            principal: "fixture-producer".into(),
            producer: e.producer.clone(),
            envelope_digest: d,
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        },
    );
}
pub fn produce(
    b: &ValidatedBinding,
    g: &StageGraph,
    intent: StageIntent,
    partial: bool,
    controller: &mut Controller,
) -> (ProtectedStagePlan, GateRun) {
    let u = upstream(b, Enforcement::Advise, partial);
    let frozen = freeze(
        g,
        g.nodes()
            .keys()
            .map(|stage| EvidenceObligation {
                stage_id: stage.clone(),
                guard: "specguard".into(),
                coverage: BTreeSet::from(["A".into()]),
                rules_digest: flowguard::digest(&u.contract),
                analyzer_version: "1".into(),
            })
            .collect(),
        &b.domain_digest(),
        &flowguard::digest(b"baseline"),
    )
    .unwrap();
    let mut p = policy(&u);
    p.action = intent.action().into();
    let pending = prepare_gate(
        b,
        &frozen,
        frozen
            .obligations()
            .iter()
            .map(|o| (obligation_scope(o), p.clone()))
            .collect(),
        GateRequest {
            run_id: format!("run:{}", intent.action()),
            action: intent.action().into(),
            started_at: TIME.into(),
        },
    )
    .unwrap();
    let plan = ProtectedStagePlan::freeze(
        intent,
        b,
        &frozen,
        &pending,
        BTreeSet::from(["fixture-producer".into()]),
        BTreeSet::from(["stage-reviewer".into()]),
    )
    .unwrap();
    record(controller, &u.envelope);
    let keys: Vec<_> = frozen.obligations().iter().map(obligation_scope).collect();
    let evidence: Vec<_> = keys
        .iter()
        .map(|key| SpecialistEvidence {
            scope: key,
            envelope: &u.envelope,
            artifacts: u.artifacts(),
        })
        .collect();
    let run = pending.evaluate(&evidence, controller, NOW, TIME).unwrap();
    record(controller, run.envelope());
    (plan, run)
}
pub fn approve(c: &mut Controller, plan: &ProtectedStagePlan, reference: &str) {
    let p = plan.approval_policy();
    c.approvals.insert(
        reference.into(),
        ApprovalRecord {
            principal: "stage-reviewer".into(),
            purpose: p.approval_principals.keys().next().unwrap().clone(),
            action: p.action.clone(),
            binding: p.binding.clone(),
            contract_digest: p.contract_digest.clone(),
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        },
    );
}
