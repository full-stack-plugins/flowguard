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

#[test]
fn published_release_routes_four_phases_and_rechecks_every_feature() {
    use flowguard::{
        local_migration::{LocalStageRoute, RouteMode},
        run_store::MemoryRunStore,
    };
    let (_dir, binding) = binding();
    let graph = release_graph();
    let mut authority = Controller::default();
    let store = MemoryRunStore::default();
    let mut delta = None;
    let mut make = |id: &str| {
        produce_with_pending(
            &binding,
            &graph,
            StageIntent::freeze(&graph, id, StageMode::Accept, &BTreeMap::new()).unwrap(),
            false,
            &mut authority,
            |pending| {
                store.advance(pending, 0).unwrap();
                store.reserve(pending, id, 1).unwrap();
                if id == "a09" {
                    delta = Some(withdrawal(pending, &graph.nodes()[id]));
                }
            },
        )
    };
    let (a, arun) = make("a09");
    let (b, brun) = make("b09");
    for run in [&arun, &brun] {
        store.append(run).unwrap();
        store.publish(&run.envelope().run_id, 1).unwrap();
    }
    approve(&mut authority, &a, "a");
    approve(&mut authority, &b, "b");
    let release = ReleasePlan::freeze(
        &graph,
        "release",
        &binding,
        &BTreeMap::from([("a".into(), &a), ("b".into(), &b)]),
    )
    .unwrap();
    let mut ra = LocalStageRoute::new_published(
        &a,
        StageState::Accepted,
        flowguard::digest(b"legacy a"),
        &store,
    )
    .unwrap();
    let mut rb = LocalStageRoute::new_published(
        &b,
        StageState::Inherited,
        flowguard::digest(b"legacy b"),
        &store,
    )
    .unwrap();
    ra.submit().unwrap();
    rb.submit().unwrap();
    let mut phases = Vec::new();
    for mode in [RouteMode::Advisory, RouteMode::Shadow] {
        ra.set_mode(mode).unwrap();
        rb.set_mode(mode).unwrap();
        assert!(!ra.qualify(&arun, "a", vec![], &authority, NOW).unwrap());
        assert!(!rb.qualify(&brun, "b", vec![], &authority, NOW).unwrap());
        assert!(ra.consume(&authority, NOW).is_err());
        assert!(rb.consume(&authority, NOW).is_err());
        assert!(
            release
                .consume_routed_published(
                    &binding,
                    &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
                    &store,
                    &authority,
                    NOW
                )
                .is_err()
        );
        phases.push(
            serde_json::json!({"phase": mode, "stage_eligible": false, "release_eligible": false}),
        );
    }
    ra.set_mode(RouteMode::OptIn).unwrap();
    rb.set_mode(RouteMode::OptIn).unwrap();
    assert!(ra.qualify(&arun, "a", vec![], &authority, NOW).unwrap());
    assert!(rb.qualify(&brun, "b", vec![], &authority, NOW).unwrap());
    release
        .consume_routed_published(
            &binding,
            &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
            &store,
            &authority,
            NOW,
        )
        .unwrap();
    phases.push(serde_json::json!({"phase": "opt_in", "stage_eligible": ra.consume(&authority, NOW).is_ok(), "release_eligible": true}));
    assert_eq!(ra.consume(&authority, NOW).unwrap(), StageState::Accepted);
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra)]),
                &store,
                &authority,
                NOW
            )
            .is_err()
    );
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra), ("b".into(), &ra)]),
                &store,
                &authority,
                NOW
            )
            .is_err()
    );
    let other_store = MemoryRunStore::default();
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
                &other_store,
                &authority,
                NOW
            )
            .is_err()
    );
    let mut detached =
        LocalStageRoute::new(&a, StageState::Accepted, flowguard::digest(b"legacy a")).unwrap();
    detached.submit().unwrap();
    detached.set_mode(RouteMode::OptIn).unwrap();
    detached
        .qualify(&arun, "a", vec![], &authority, NOW)
        .unwrap();
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &detached), ("b".into(), &rb)]),
                &store,
                &authority,
                NOW
            )
            .is_err()
    );
    let history = serde_json::to_value(ra.history()).unwrap();
    ra.rollback().unwrap();
    assert!(ra.consume(&authority, NOW).is_err());
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
                &store,
                &authority,
                NOW
            )
            .is_err(),
        "release must honor feature route rollback"
    );
    assert_eq!(
        &serde_json::to_value(ra.history())
            .unwrap()
            .as_array()
            .unwrap()[..history.as_array().unwrap().len()],
        history.as_array().unwrap()
    );
    phases.push(serde_json::json!({"phase": "rollback", "stage_eligible": false, "release_eligible": false}));
    ra.set_mode(RouteMode::OptIn).unwrap();
    authority.approvals.get_mut("b").unwrap().validity.revoked = true;
    assert!(ra.consume(&authority, NOW).is_ok());
    assert!(rb.consume(&authority, NOW).is_err());
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
                &store,
                &authority,
                NOW
            )
            .is_err()
    );
    authority.approvals.get_mut("b").unwrap().validity.revoked = false;
    let original = serde_json::to_vec(&[arun.envelope(), brun.envelope()]).unwrap();
    delta.unwrap().apply(&store).unwrap();
    assert!(ra.consume(&authority, NOW).is_err());
    assert_eq!(rb.consume(&authority, NOW).unwrap(), StageState::Accepted);
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
                &store,
                &authority,
                NOW
            )
            .is_err()
    );
    assert_eq!(
        original,
        serde_json::to_vec(&[arun.envelope(), brun.envelope()]).unwrap()
    );
    eprintln!(
        "FLOWGUARD_PUBLISHED_ROLLOUT {}",
        serde_json::json!({
            "schema": "flowguard.local-rollout-evidence/v1", "authority": "explicit local fixture",
            "feature_plan_digests": [a.digest(), b.digest()],
            "envelope_digests": [flowguard::digest(&serde_json::to_vec(arun.envelope()).unwrap()), flowguard::digest(&serde_json::to_vec(brun.envelope()).unwrap())],
            "phases": phases, "revocation_refused": true, "publication_withdrawal_refused": true,
            "history_prefix_preserved": true, "envelopes_preserved": true,
            "history": ra.history(), "real_migration": false, "host_enforced": false
        })
    );
}

#[test]
fn routed_release_rejects_freshly_qualified_changed_feature_plan() {
    use flowguard::{
        local_migration::{LocalStageRoute, RouteMode},
        run_store::MemoryRunStore,
    };
    let (_dir, binding) = binding();
    let graph = release_graph();
    let store = MemoryRunStore::default();
    let mut authority = Controller::default();
    let mut make = |graph: &StageGraph, id: &str, key: &str| {
        produce_with_pending(
            &binding,
            graph,
            StageIntent::freeze(graph, id, StageMode::Accept, &BTreeMap::new()).unwrap(),
            false,
            &mut authority,
            |pending| {
                store.advance(pending, 0).unwrap();
                store.reserve(pending, key, 1).unwrap();
            },
        )
    };
    let (a, arun) = make(&graph, "a09", "a");
    let (b, brun) = make(&graph, "b09", "b");
    let mut nodes: Vec<_> = graph.nodes().values().cloned().collect();
    nodes
        .iter_mut()
        .find(|n| n.id == "a09")
        .unwrap()
        .source_digest = flowguard::digest(b"changed docs09");
    let changed = build_graph(nodes, GraphLimits::default()).unwrap();
    let (new_a, new_run) = make(&changed, "a09", "changed");
    for run in [&arun, &brun, &new_run] {
        store.append(run).unwrap();
        store.publish(&run.envelope().run_id, 1).unwrap();
    }
    for (plan, key) in [(&a, "a"), (&b, "b"), (&new_a, "new")] {
        approve(&mut authority, plan, key);
    }
    let release = ReleasePlan::freeze(
        &graph,
        "release",
        &binding,
        &BTreeMap::from([("a".into(), &a), ("b".into(), &b)]),
    )
    .unwrap();
    let mut ra = LocalStageRoute::new_published(
        &a,
        StageState::Accepted,
        flowguard::digest(b"legacy a"),
        &store,
    )
    .unwrap();
    let mut rb = LocalStageRoute::new_published(
        &b,
        StageState::Accepted,
        flowguard::digest(b"legacy b"),
        &store,
    )
    .unwrap();
    for route in [&mut ra, &mut rb] {
        route.submit().unwrap();
        route.set_mode(RouteMode::OptIn).unwrap();
    }
    ra.qualify(&arun, "a", vec![], &authority, NOW).unwrap();
    rb.qualify(&brun, "b", vec![], &authority, NOW).unwrap();
    release
        .consume_routed_published(
            &binding,
            &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
            &store,
            &authority,
            NOW,
        )
        .unwrap();
    ra.edited(&new_a).unwrap();
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
                &store,
                &authority,
                NOW
            )
            .is_err()
    );
    ra.submit().unwrap();
    ra.qualify(&new_run, "new", vec![], &authority, NOW)
        .unwrap();
    assert_eq!(ra.consume(&authority, NOW).unwrap(), StageState::Accepted);
    assert_eq!(
        release.consume_routed_published(
            &binding,
            &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
            &store,
            &authority,
            NOW
        ),
        Err(ReleaseError::Binding)
    );
    while rb.history().len() < 1024 {
        rb.set_mode(RouteMode::OptIn).unwrap();
    }
    assert_eq!(rb.rollback(), Err(QualificationError::Budget));
    assert!(
        release
            .consume_routed_published(
                &binding,
                &BTreeMap::from([("a".into(), &ra), ("b".into(), &rb)]),
                &store,
                &authority,
                NOW
            )
            .is_err()
    );
}
