mod common;
use common::*;
use flowguard::gate::*;
use guardengine::integration::{RunStatus, eligibility::*};
use guardengine::{Decision, Enforcement};
use std::collections::BTreeMap;
fn plan(b: &flowguard::context::ValidatedBinding, u: &Upstream, missing: bool) -> PendingGate {
    let f = frozen(b, u, missing);
    let key = obligation_scope(f.obligations().first().unwrap());
    prepare_gate(
        b,
        &f,
        BTreeMap::from([(key, policy(u))]),
        GateRequest {
            run_id: "flow-run".into(),
            action: "commit".into(),
            started_at: TIME.into(),
        },
    )
    .unwrap()
}
#[test]
fn actual_engine_gate_keeps_upstream_report_bytes_and_independent_review_decision() {
    let (_dir, b) = binding();
    let u = upstream(&b, Enforcement::Review, false);
    let report_before = u.report.clone();
    let envelope_before = serde_json::to_vec(&u.envelope).unwrap();
    let f = frozen(&b, &u, false);
    let key = obligation_scope(f.obligations().first().unwrap());
    let no_approval = FixtureAuthority {
        approval: None,
        unavailable: false,
    };
    let before = plan(&b, &u, false)
        .evaluate(
            &[SpecialistEvidence {
                scope: &key,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            &no_approval,
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(before.envelope().decision, Some(Decision::RequireApproval));
    assert_eq!(serde_json::to_vec(&u.envelope).unwrap(), envelope_before);
    // A producer emits a later independently authenticated envelope with a new approval
    // reference. The original upstream report bytes remain unchanged.
    let mut later = u.clone();
    later.envelope.run_id = "specialist-run-approved-observation".into();
    later.envelope.approval_refs = vec!["fixture:approval".into()];
    let authority = FixtureAuthority {
        approval: Some(ApprovalRecord {
            principal: "fixture-reviewer".into(),
            purpose: "review".into(),
            action: "commit".into(),
            binding: b.binding().clone(),
            contract_digest: flowguard::digest(&u.contract),
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        }),
        unavailable: false,
    };
    let after = plan(&b, &later, false)
        .evaluate(
            &[SpecialistEvidence {
                scope: &key,
                envelope: &later.envelope,
                artifacts: later.artifacts(),
            }],
            &authority,
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(after.envelope().decision, Some(Decision::Allow));
    assert_eq!(
        after.domain().unwrap().qualification,
        ActionQualification::Eligible
    );
    assert_eq!(u.report, report_before);
    assert_eq!(later.report, report_before);
    assert_eq!(serde_json::to_vec(&u.envelope).unwrap(), envelope_before);
    assert_eq!(
        serde_json::from_slice::<guardengine::GuardReport>(&u.report)
            .unwrap()
            .decision,
        Decision::RequireApproval
    );
    let own = guardengine::integration::verify_engine_artifacts(
        after.envelope(),
        after.artifacts().unwrap().contract,
        after.artifacts().unwrap().facts,
        after.artifacts().unwrap().report,
    )
    .unwrap();
    assert_eq!(own.decision, Decision::Allow);
    let own_policy = EligibilityPolicy {
        binding: b.binding().clone(),
        producer: after.envelope().producer.clone(),
        required_scopes: after.envelope().coverage.required_scopes.clone(),
        contract_digest: after
            .envelope()
            .artifacts
            .contract
            .as_ref()
            .unwrap()
            .digest
            .clone(),
        action: "commit".into(),
        producer_principals: std::collections::BTreeSet::from(["fixture-producer".into()]),
        approval_principals: BTreeMap::new(),
    };
    assert!(
        after
            .evaluate_eligibility(&own_policy, &authority, NOW)
            .unwrap()
            .eligible
    );
    let mut revoked_record = authority.approval.clone().unwrap();
    revoked_record.validity.revoked = true;
    let revoked_authority = FixtureAuthority {
        approval: Some(revoked_record),
        unavailable: false,
    };
    assert!(
        !after
            .evaluate_eligibility(&own_policy, &revoked_authority, NOW)
            .is_ok_and(|r| r.eligible)
    );
    let mut wrong_action = own_policy.clone();
    wrong_action.action = "release".into();
    assert!(
        !after
            .evaluate_eligibility(&wrong_action, &authority, NOW)
            .is_ok_and(|r| r.eligible)
    );
    assert!(
        !after
            .evaluate_eligibility(
                &own_policy,
                &FixtureAuthority {
                    approval: None,
                    unavailable: true
                },
                NOW
            )
            .is_ok_and(|r| r.eligible)
    );
}
#[test]
fn incomplete_missing_and_enforced_evidence_never_opens_gate() {
    let (_dir, b) = binding();
    for (enforcement, partial, missing_stage, absent) in [
        (Enforcement::Enforce, false, false, false),
        (Enforcement::Advise, true, false, false),
        (Enforcement::Advise, false, true, false),
        (Enforcement::Advise, false, false, true),
    ] {
        let mut u = upstream(&b, enforcement, partial);
        u.envelope.approval_refs = vec!["fixture:approval".into()];
        let authority = FixtureAuthority {
            approval: Some(ApprovalRecord {
                principal: "fixture-reviewer".into(),
                purpose: "review".into(),
                action: "commit".into(),
                binding: b.binding().clone(),
                contract_digest: flowguard::digest(&u.contract),
                validity: Validity {
                    issued_at: NOW - 1,
                    expires_at: NOW + 100,
                    revoked: false,
                },
            }),
            unavailable: false,
        };
        let f = frozen(&b, &u, missing_stage);
        let key = obligation_scope(f.obligations().first().unwrap());
        let evidence = if absent {
            vec![]
        } else {
            vec![SpecialistEvidence {
                scope: &key,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }]
        };
        let run = plan(&b, &u, missing_stage)
            .evaluate(&evidence, &authority, NOW, TIME)
            .unwrap();
        assert_eq!(run.envelope().decision, Some(Decision::Block));
        assert_eq!(
            run.domain().unwrap().qualification,
            ActionQualification::Blocked
        );
        assert!(run.envelope().coverage.required_scopes.contains(&key));
    }
}
#[test]
fn provider_failure_and_cancellation_use_real_bound_error_transport() {
    let (_dir, b) = binding();
    let u = upstream(&b, Enforcement::Advise, false);
    let f = frozen(&b, &u, false);
    let key = obligation_scope(f.obligations().first().unwrap());
    let run = plan(&b, &u, false)
        .evaluate(
            &[SpecialistEvidence {
                scope: &key,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            &FixtureAuthority {
                approval: None,
                unavailable: true,
            },
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(run.envelope().run_status, RunStatus::Error);
    assert_eq!(run.envelope().decision, None);
    assert!(run.domain().is_none());
    assert!(run.envelope().artifacts.report.is_none());
    let cancelled = plan(&b, &u, false).cancel(TIME).unwrap();
    assert_eq!(cancelled.envelope().run_status, RunStatus::Cancelled);
    assert_eq!(cancelled.envelope().decision, None);
}
#[test]
fn required_scope_identity_includes_the_entire_frozen_snapshot() {
    let (_dir, b) = binding();
    let u = upstream(&b, Enforcement::Advise, false);
    let first = frozen(&b, &u, false);
    let graph = flowguard::dependencies::build_graph(
        vec![flowguard::dependencies::StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "A/01".into(),
            stage: "01-requirements".into(),
            owner: "a".into(),
            source_digest: flowguard::digest(b"changed stage bytes"),
            dependencies: std::collections::BTreeSet::new(),
        }],
        flowguard::dependencies::GraphLimits::default(),
    )
    .unwrap();
    let second = flowguard::obligations::freeze(
        &graph,
        first.obligations().iter().cloned().collect(),
        &b.domain_digest(),
        &flowguard::digest(b"changed baseline"),
    )
    .unwrap();
    let key = obligation_scope(first.obligations().first().unwrap());
    let request = || GateRequest {
        run_id: "flow-run".into(),
        action: "commit".into(),
        started_at: TIME.into(),
    };
    let one = prepare_gate(
        &b,
        &first,
        BTreeMap::from([(key.clone(), policy(&u))]),
        request(),
    )
    .unwrap();
    let two = prepare_gate(&b, &second, BTreeMap::from([(key, policy(&u))]), request()).unwrap();
    assert_ne!(one.required_scopes(), two.required_scopes());
}
