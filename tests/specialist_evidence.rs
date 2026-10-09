mod common;
use common::*;
use flowguard::gate::*;
use guardengine::{
    Enforcement,
    integration::{eligibility::*, *},
};
use std::collections::BTreeMap;
fn plan(b: &flowguard::context::ValidatedBinding, u: &Upstream) -> (PendingGate, String) {
    let f = frozen(b, u, false);
    let key = obligation_scope(f.obligations().first().unwrap());
    let gate = prepare_gate(
        b,
        &f,
        BTreeMap::from([(key.clone(), policy(u))]),
        GateRequest {
            run_id: "evidence-run".into(),
            action: "commit".into(),
            started_at: TIME.into(),
        },
    )
    .unwrap();
    (gate, key)
}
#[test]
fn wrong_binding_artifact_digest_and_execution_failure_are_bound_errors() {
    let (_dir, b) = binding();
    let u = upstream(&b, Enforcement::Advise, false);
    let authority = FixtureAuthority {
        approval: None,
        unavailable: false,
    };
    for case in 0..6 {
        let (pending, key) = plan(&b, &u);
        let required = pending.required_scopes().to_vec();
        let mut bad = u.clone();
        match case {
            0 => bad.envelope.binding.base_oid = "b".repeat(40),
            1 => bad.envelope.binding.merge_group_id = Some("wrong-group".into()),
            2 => bad.envelope.binding.requirement_ids = vec!["other".into()],
            3 => bad.facts.push(b' '),
            4 => bad.envelope.producer.analyzer_version = "future".into(),
            _ => {
                bad.envelope.run_status = RunStatus::Error;
                bad.envelope.decision = None;
                bad.envelope.artifacts.report = None;
                bad.envelope.diagnostics = vec![Diagnostic {
                    code: "fixture.crash".into(),
                    message: "fixture crash".into(),
                    retryable: false,
                    source: None,
                }];
            }
        }
        let run = pending
            .evaluate(
                &[SpecialistEvidence {
                    scope: &key,
                    envelope: &bad.envelope,
                    artifacts: bad.artifacts(),
                }],
                &authority,
                NOW,
                TIME,
            )
            .unwrap();
        assert_eq!(run.envelope().run_status, RunStatus::Error, "case {case}");
        assert_eq!(run.envelope().decision, None);
        assert_eq!(run.envelope().coverage.required_scopes, required);
        assert!(run.domain().is_none());
    }
}
struct RejectingAuthority;
impl AuthorityProvider for RejectingAuthority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        _: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
}
#[test]
fn unsigned_consistency_without_authenticated_fixture_producer_is_not_eligibility() {
    let (_dir, b) = binding();
    let u = upstream(&b, Enforcement::Advise, false);
    let (pending, key) = plan(&b, &u);
    assert!(verify_engine_artifacts(&u.envelope, &u.contract, &u.facts, &u.report).is_ok());
    let run = pending
        .evaluate(
            &[SpecialistEvidence {
                scope: &key,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            &RejectingAuthority,
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(run.envelope().decision, None);
    assert_eq!(run.envelope().run_status, RunStatus::Error);
}
#[test]
fn missing_or_weakened_protected_policy_fails_before_bound_output() {
    let (_dir, b) = binding();
    let u = upstream(&b, Enforcement::Advise, false);
    let f = frozen(&b, &u, false);
    let key = obligation_scope(f.obligations().first().unwrap());
    let request = || GateRequest {
        run_id: "prebinding-run".into(),
        action: "commit".into(),
        started_at: TIME.into(),
    };
    assert!(prepare_gate(&b, &f, BTreeMap::new(), request()).is_err());
    let mut weak = policy(&u);
    weak.required_scopes.clear();
    assert!(prepare_gate(&b, &f, BTreeMap::from([(key, weak)]), request()).is_err());
}
