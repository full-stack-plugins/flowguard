mod qualification_support;
use flowguard::{
    dependencies::*, release::*, stage_qualification::*, stage_transition::StageState,
};
use qualification_support::*;
use std::collections::{BTreeMap, BTreeSet};
fn release_graph() -> StageGraph {
    build_graph(
        vec![
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "a09".into(),
                stage: "09-docs".into(),
                owner: "a".into(),
                source_digest: flowguard::digest(b"a09"),
                dependencies: BTreeSet::new(),
            },
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "b09".into(),
                stage: "09-docs".into(),
                owner: "b".into(),
                source_digest: flowguard::digest(b"b09"),
                dependencies: BTreeSet::new(),
            },
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "release".into(),
                stage: "10-release".into(),
                owner: "project".into(),
                source_digest: flowguard::digest(b"release"),
                dependencies: BTreeSet::from(["a09".into(), "b09".into()]),
            },
        ],
        GraphLimits::default(),
    )
    .unwrap()
}
#[test]
fn release_exact_09_set_consumes_typed_current_qualifications_and_refreshes_each() {
    let (_dir, b) = binding();
    let g = release_graph();
    let mut c = Controller::default();
    let (a, arun) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "a09", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    let (other, brun) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "b09", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    let release = ReleasePlan::freeze(
        &g,
        "release",
        &b,
        &BTreeMap::from([("a".into(), &a), ("b".into(), &other)]),
    )
    .unwrap();
    approve(&mut c, &a, "a");
    approve(&mut c, &other, "b");
    let qa = a
        .qualify(StageState::PendingAcceptance, &arun, "a", vec![], &c, NOW)
        .unwrap();
    let qb = other
        .qualify(StageState::PendingAcceptance, &brun, "b", vec![], &c, NOW)
        .unwrap();
    let observations = BTreeMap::from([("a".into(), (&qa, &a)), ("b".into(), (&qb, &other))]);
    release.consume(&b, &observations, &c, NOW).unwrap();
    for wrong in [
        BTreeMap::from([("a".into(), (&qa, &a))]),
        BTreeMap::from([("a".into(), (&qa, &a)), ("b".into(), (&qa, &a))]),
        BTreeMap::from([
            ("a".into(), (&qa, &a)),
            ("b".into(), (&qb, &other)),
            ("extra".into(), (&qa, &a)),
        ]),
    ] {
        assert!(release.consume(&b, &wrong, &c, NOW).is_err());
    }
    c.approvals.get_mut("b").unwrap().validity.revoked = true;
    assert!(release.consume(&b, &observations, &c, NOW).is_err());
}

#[test]
fn release_freeze_rejects_non_09_unknown_feature_and_changed_release_graph() {
    let (_dir, b) = binding();
    let g = release_graph();
    let mut c = Controller::default();
    let (a, _) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "a09", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    let (other, _) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "b09", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    assert!(ReleasePlan::freeze(&g, "release", &b, &BTreeMap::from([("a".into(), &a)])).is_err());
    assert!(
        ReleasePlan::freeze(
            &g,
            "release",
            &b,
            &BTreeMap::from([
                ("a".into(), &a),
                ("b".into(), &other),
                ("unknown".into(), &a)
            ])
        )
        .is_err()
    );
    assert!(
        ReleasePlan::freeze(
            &g,
            "a09",
            &b,
            &BTreeMap::from([("a".into(), &a), ("b".into(), &other)])
        )
        .is_err()
    );
    let mut records: Vec<_> = g.nodes().values().cloned().collect();
    records
        .iter_mut()
        .find(|n| n.id == "release")
        .unwrap()
        .dependencies
        .remove("b09");
    let changed = build_graph(records, GraphLimits::default()).unwrap();
    assert!(
        ReleasePlan::freeze(
            &changed,
            "release",
            &b,
            &BTreeMap::from([("a".into(), &a), ("b".into(), &other)])
        )
        .is_err()
    );
}

#[test]
fn release10_published_fanin_rejects_withdrawn_feature_and_preserves_the_other() {
    use flowguard::{invalidation::*, run_store::MemoryRunStore};
    let (_dir, b) = binding();
    let g = release_graph();
    let mut c = Controller::default();
    let store = MemoryRunStore::default();
    let single = build_graph(vec![g.nodes()["a09"].clone()], GraphLimits::default()).unwrap();
    let d = flowguard::digest(b"protected source inputs");
    let context = ContextDigests {
        source: d.clone(),
        rules: d.clone(),
        analyzer: d.clone(),
        config: d.clone(),
        coverage: d.clone(),
        baseline: d.clone(),
        dependencies: d,
    };
    let mut delta = None;
    let (a, arun) = produce_with_pending(
        &b,
        &g,
        StageIntent::freeze(&g, "a09", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
        |pending| {
            store.advance(pending, 0).unwrap();
            store.reserve(pending, "a09", 1).unwrap();
            let freeze = |revoked| {
                FrozenInputs::freeze(
                    &single,
                    &[NodeInput {
                        node_id: "a09",
                        gate: pending,
                        generation: 1,
                        context: &context,
                        freshness: Freshness {
                            observed_at: NOW,
                            expires_at: NOW + 60,
                            revoked,
                            artifacts_available: true,
                        },
                    }],
                    NOW,
                )
                .unwrap()
            };
            delta = Some(freeze(false).changes(&freeze(true), NOW).unwrap());
        },
    );
    let (other, brun) = produce_with_pending(
        &b,
        &g,
        StageIntent::freeze(&g, "b09", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
        |pending| {
            store.advance(pending, 0).unwrap();
            store.reserve(pending, "b09", 1).unwrap();
        },
    );
    for run in [&arun, &brun] {
        store.append(run).unwrap();
        store.publish(&run.envelope().run_id, 1).unwrap();
    }
    let release = ReleasePlan::freeze(
        &g,
        "release",
        &b,
        &BTreeMap::from([("a".into(), &a), ("b".into(), &other)]),
    )
    .unwrap();
    approve(&mut c, &a, "a");
    approve(&mut c, &other, "b");
    let qa = a
        .qualify(StageState::PendingAcceptance, &arun, "a", vec![], &c, NOW)
        .unwrap();
    let qb = other
        .qualify(StageState::PendingAcceptance, &brun, "b", vec![], &c, NOW)
        .unwrap();
    let features = BTreeMap::from([("a".into(), (&qa, &a)), ("b".into(), (&qb, &other))]);
    let original = serde_json::to_vec(&[arun.envelope(), brun.envelope()]).unwrap();
    release
        .consume_published(&b, &features, &store, &c, NOW)
        .unwrap();
    delta.unwrap().apply(&store).unwrap();
    assert!(
        release
            .consume_published(&b, &features, &store, &c, NOW)
            .is_err()
    );
    assert_eq!(
        qb.consume_published(&other, &store, &c, NOW).unwrap(),
        StageState::Accepted
    );
    assert_eq!(
        original,
        serde_json::to_vec(&[arun.envelope(), brun.envelope()]).unwrap()
    );
}
