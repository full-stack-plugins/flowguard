use flowguard::{
    dependencies::{GraphLimits, StageRecord, build_graph},
    obligations::*,
};
use std::collections::BTreeSet;
fn graph() -> flowguard::dependencies::StageGraph {
    build_graph(
        vec![StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "A/01".into(),
            stage: "01-requirements".into(),
            owner: "A".into(),
            source_digest: flowguard::digest(b"req"),
            dependencies: BTreeSet::new(),
        }],
        GraphLimits::default(),
    )
    .unwrap()
}
fn policy() -> Vec<EvidenceObligation> {
    vec![
        EvidenceObligation {
            stage_id: "A/01".into(),
            guard: "specguard".into(),
            coverage: BTreeSet::from(["A".into()]),
            rules_digest: flowguard::digest(b"rules"),
            analyzer_version: "1.0".into(),
        },
        EvidenceObligation {
            stage_id: "A/04".into(),
            guard: "testguard".into(),
            coverage: BTreeSet::from(["A".into()]),
            rules_digest: flowguard::digest(b"tests"),
            analyzer_version: "1.0".into(),
        },
    ]
}
#[test]
fn freezes_before_collection_and_cannot_shrink_for_missing_provider() {
    let g = graph();
    let p = policy();
    let f = freeze(
        &g,
        p.clone(),
        &flowguard::digest(b"exact context"),
        &flowguard::digest(b"baseline"),
    )
    .unwrap();
    assert_eq!(f.obligations().len(), 2);
    assert!(f.missing_stages().contains("A/04"));
    assert_eq!(
        f.missing_providers(&BTreeSet::from(["specguard".into(), "codeguard".into()])),
        BTreeSet::from(["testguard".into()])
    );
    let mut reversed = p;
    reversed.reverse();
    assert_eq!(
        f.digest(),
        freeze(
            &g,
            reversed,
            &flowguard::digest(b"exact context"),
            &flowguard::digest(b"baseline")
        )
        .unwrap()
        .digest()
    );
    assert_ne!(
        f.digest(),
        freeze(
            &g,
            policy(),
            &flowguard::digest(b"new candidate"),
            &flowguard::digest(b"baseline")
        )
        .unwrap()
        .digest()
    );
}
#[test]
fn rejects_empty_identity_and_detects_payload_tampering() {
    let g = graph();
    let mut p = policy();
    p[0].guard.clear();
    assert!(
        freeze(
            &g,
            p,
            &flowguard::digest(b"binding"),
            &flowguard::digest(b"baseline")
        )
        .is_err()
    );
    let f = freeze(
        &g,
        policy(),
        &flowguard::digest(b"binding"),
        &flowguard::digest(b"baseline"),
    )
    .unwrap();
    let mut json = serde_json::to_value(&f).unwrap();
    json["context_digest"] = flowguard::digest(b"tampered").into();
    assert!(FrozenObligations::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
    json["actorVerified"] = true.into();
    assert!(FrozenObligations::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
}
#[test]
fn rejects_duplicate_top_level_fields_before_canonicalization() {
    let good = include_str!("../fixtures/schema/frozen-obligations-valid.json");
    let duplicate = good.replacen(
        "\"version\":",
        "\"version\":\"flowguard.workflow/v1\",\"version\":",
        1,
    );
    assert!(FrozenObligations::from_json(duplicate.as_bytes()).is_err());
}
