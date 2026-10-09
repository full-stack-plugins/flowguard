mod common;
use common::*;
use flowguard::gate::*;
use std::collections::BTreeMap;
#[test]
fn domain_artifact_is_versioned_digest_bound_and_separate_from_engine_report() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let f = frozen(&b, &u, false);
    let key = obligation_scope(f.obligations().first().unwrap());
    let pending = prepare_gate(
        &b,
        &f,
        BTreeMap::from([(key.clone(), policy(&u))]),
        GateRequest {
            run_id: "schema-run".into(),
            action: "commit".into(),
            started_at: TIME.into(),
        },
    )
    .unwrap();
    let required = pending.required_scopes().to_vec();
    let run = pending
        .evaluate(
            &[SpecialistEvidence {
                scope: &key,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            &FixtureAuthority {
                approval: None,
                unavailable: false,
            },
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(run.envelope().coverage.required_scopes, required);
    assert_eq!(run.envelope().coverage.observed_scopes, required);
    let bytes = serde_json::to_vec(run.domain().unwrap()).unwrap();
    let domain = load_gate_decision(&bytes).unwrap();
    assert_eq!(domain.authority_profile, AuthorityProfile::Advisory);
    assert_eq!(domain.binding_digest, b.domain_digest());
    assert_eq!(domain.frozen_obligations_digest, f.digest());
    assert_eq!(
        run.envelope().artifacts.domain[0].digest,
        flowguard::digest(&bytes)
    );
    assert_eq!(
        domain.report,
        run.envelope().artifacts.report.clone().unwrap()
    );
    let mut empty_media = serde_json::to_value(&domain).unwrap();
    empty_media["report"]["mediaType"] = serde_json::json!("");
    assert!(load_gate_decision(&serde_json::to_vec(&empty_media).unwrap()).is_err());
    for (field, value) in [
        ("apiVersion", serde_json::json!("flowguard.gate/v999")),
        ("mappingVersion", serde_json::json!("unknown")),
        ("actorVerified", serde_json::json!(true)),
        ("qualification", serde_json::json!("blocked")),
        ("action", serde_json::json!("")),
        ("bindingDigest", serde_json::json!("changed")),
    ] {
        let mut v = serde_json::to_value(&domain).unwrap();
        v[field] = value;
        assert!(
            load_gate_decision(&serde_json::to_vec(&v).unwrap()).is_err(),
            "{field}"
        );
    }
}
#[test]
fn captured_fixture_recomputes_through_real_engine_and_preserves_specialist_verdict() {
    let e = guardengine::integration::load_envelope_json(
        include_bytes!("../fixtures/gate_mapping/envelope.json"),
        guardengine::integration::EvidenceProfile::EngineBacked,
    )
    .unwrap();
    let r = guardengine::integration::verify_engine_artifacts(
        &e,
        include_bytes!("../fixtures/gate_mapping/contract.json"),
        include_bytes!("../fixtures/gate_mapping/facts.json"),
        include_bytes!("../fixtures/gate_mapping/report.json"),
    )
    .unwrap();
    assert_eq!(r.decision, guardengine::Decision::Allow);
    let domain_bytes = include_bytes!("../fixtures/gate_mapping/gate-decision.json");
    let d = load_gate_decision(domain_bytes).unwrap();
    assert_eq!(
        flowguard::digest(domain_bytes),
        e.artifacts.domain[0].digest
    );
    assert_eq!(d.report, e.artifacts.report.unwrap());
    let upstream: guardengine::GuardReport = serde_json::from_slice(include_bytes!(
        "../fixtures/gate_mapping/upstream-report.json"
    ))
    .unwrap();
    assert_eq!(upstream.decision, guardengine::Decision::RequireApproval);
    assert!(d.upstream_reports.iter().any(|r| r.digest
        == flowguard::digest(include_bytes!(
            "../fixtures/gate_mapping/upstream-report.json"
        ))));
}
