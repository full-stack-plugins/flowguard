use flowguard::{
    baseline::{BaselineRef, resolve_baseline},
    dependencies::{GraphLimits, StageRecord, build_graph},
    obligations::FrozenObligations,
};
#[test]
fn domain_fixture_versions_fields_and_digest_are_checked() {
    let stage: StageRecord =
        serde_json::from_str(include_str!("../fixtures/schema/stage-record-valid.json")).unwrap();
    assert!(build_graph(vec![stage], GraphLimits::default()).is_ok());
    assert!(
        serde_json::from_str::<StageRecord>(include_str!(
            "../fixtures/schema/stage-record-unknown-field.json"
        ))
        .is_err()
    );
    let stage: StageRecord = serde_json::from_str(include_str!(
        "../fixtures/schema/stage-record-unknown-version.json"
    ))
    .unwrap();
    assert!(build_graph(vec![stage], GraphLimits::default()).is_err());
    let b: BaselineRef =
        serde_json::from_str(include_str!("../fixtures/schema/baseline-ref-valid.json")).unwrap();
    assert!(
        resolve_baseline(
            &b,
            "repo",
            "A",
            &b.revision,
            &b.content_digest,
            &b.policy_digest
        )
        .is_ok()
    );
    assert!(
        FrozenObligations::from_json(include_bytes!(
            "../fixtures/schema/frozen-obligations-valid.json"
        ))
        .is_ok()
    );
    assert!(
        FrozenObligations::from_json(include_bytes!(
            "../fixtures/schema/frozen-obligations-unknown-field.json"
        ))
        .is_err()
    );
    assert!(
        FrozenObligations::from_json(include_bytes!(
            "../fixtures/schema/frozen-obligations-unknown-version.json"
        ))
        .is_err()
    );
}
