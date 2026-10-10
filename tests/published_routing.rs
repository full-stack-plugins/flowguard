mod qualification_support;
use flowguard::{
    local_migration::{LocalStageRoute, RouteMode},
    run_store::MemoryRunStore,
    stage_qualification::*,
    stage_transition::StageState,
};
use qualification_support::*;
use std::collections::BTreeMap;

#[test]
fn published_route_refuses_unpublished_evidence_in_every_mode() {
    let (_dir, binding) = binding();
    let graph = graph();
    let mut authority = Controller::default();
    let store = MemoryRunStore::default();
    let (plan, run) = produce_with_pending(
        &binding,
        &graph,
        StageIntent::freeze(&graph, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut authority,
        |pending| {
            store.advance(pending, 0).unwrap();
            store.reserve(pending, "first", 1).unwrap();
        },
    );
    approve(&mut authority, &plan, "approved");
    store.append(&run).unwrap();
    for mode in [RouteMode::Advisory, RouteMode::Shadow, RouteMode::OptIn] {
        let mut route = LocalStageRoute::new_published(
            &plan,
            StageState::Accepted,
            flowguard::digest(b"legacy"),
            &store,
        )
        .unwrap();
        route.submit().unwrap();
        route.set_mode(mode).unwrap();
        assert_eq!(
            route.qualify(&run, "approved", vec![], &authority, NOW),
            Err(QualificationError::Technical),
            "unpublished evidence must refuse in {mode:?}"
        );
        assert!(route.consume(&authority, NOW).is_err());
    }
}

#[test]
fn published_route_rechecks_transitive_publication_and_fresh_approval() {
    let (_dir, binding) = binding();
    let graph = graph();
    let mut authority = Controller::default();
    let store = MemoryRunStore::default();
    let mut delta = None;
    let (first, first_run) = produce_with_pending(
        &binding,
        &graph,
        StageIntent::freeze(&graph, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut authority,
        |pending| {
            store.advance(pending, 0).unwrap();
            store.reserve(pending, "first", 1).unwrap();
            delta = Some(withdrawal(pending, &graph.nodes()["first"]));
        },
    );
    let (second, second_run) = produce_with_pending(
        &binding,
        &graph,
        StageIntent::freeze(
            &graph,
            "second",
            StageMode::Accept,
            &BTreeMap::from([("first".into(), &first)]),
        )
        .unwrap(),
        false,
        &mut authority,
        |pending| {
            store.advance(pending, 0).unwrap();
            store.reserve(pending, "second", 1).unwrap();
        },
    );
    for run in [&first_run, &second_run] {
        store.append(run).unwrap();
        store.publish(&run.envelope().run_id, 1).unwrap();
    }
    approve(&mut authority, &first, "first");
    approve(&mut authority, &second, "second");
    let first_receipt = first
        .qualify(
            StageState::PendingAcceptance,
            &first_run,
            "first",
            vec![],
            &authority,
            NOW,
        )
        .unwrap();
    let mut route = LocalStageRoute::new_published(
        &second,
        StageState::Skipped,
        flowguard::digest(b"legacy skipped"),
        &store,
    )
    .unwrap();
    route.submit().unwrap();
    route.set_mode(RouteMode::OptIn).unwrap();
    route
        .qualify(&second_run, "second", vec![&first_receipt], &authority, NOW)
        .unwrap();
    assert_eq!(
        route.consume(&authority, NOW).unwrap(),
        StageState::Accepted
    );
    authority
        .approvals
        .get_mut("first")
        .unwrap()
        .validity
        .revoked = true;
    assert!(route.consume(&authority, NOW).is_err());
    authority
        .approvals
        .get_mut("first")
        .unwrap()
        .validity
        .revoked = false;
    assert!(route.consume(&authority, NOW).is_ok());
    delta.unwrap().apply(&store).unwrap();
    assert_eq!(
        route.consume(&authority, NOW),
        Err(QualificationError::Technical)
    );
    // Detached history remains readable, but no longer consumable through this route.
    assert!(first_receipt.consume(&first, &authority, NOW).is_ok());
    route.invalidate().unwrap();
    route.submit().unwrap();
    assert_eq!(
        route.qualify(&second_run, "second", vec![&first_receipt], &authority, NOW),
        Err(QualificationError::Technical)
    );
}

#[test]
fn published_route_changed_plan_needs_resubmission_publication_and_fresh_approval() {
    let (_dir, binding) = binding();
    let graph = graph();
    let mut authority = Controller::default();
    let store = MemoryRunStore::default();
    let (plan, run) = produce_with_pending(
        &binding,
        &graph,
        StageIntent::freeze(&graph, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut authority,
        |pending| {
            store.advance(pending, 0).unwrap();
            store.reserve(pending, "old", 1).unwrap();
        },
    );
    approve(&mut authority, &plan, "old");
    store.append(&run).unwrap();
    store.publish(&run.envelope().run_id, 1).unwrap();
    let mut nodes: Vec<_> = graph.nodes().values().cloned().collect();
    nodes
        .iter_mut()
        .find(|n| n.id == "first")
        .unwrap()
        .source_digest = flowguard::digest(b"changed stage bytes");
    let changed = flowguard::dependencies::build_graph(
        nodes,
        flowguard::dependencies::GraphLimits::default(),
    )
    .unwrap();
    let (new_plan, new_run) = produce_with_pending(
        &binding,
        &changed,
        StageIntent::freeze(&changed, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut authority,
        |pending| {
            store.advance(pending, 0).unwrap();
            store.reserve(pending, "new", 1).unwrap();
        },
    );
    approve(&mut authority, &new_plan, "new");
    let mut route = LocalStageRoute::new_published(
        &plan,
        StageState::Accepted,
        flowguard::digest(b"legacy"),
        &store,
    )
    .unwrap();
    route.submit().unwrap();
    route.set_mode(RouteMode::OptIn).unwrap();
    route.qualify(&run, "old", vec![], &authority, NOW).unwrap();
    let history = serde_json::to_value(route.history()).unwrap();
    route.edited(&new_plan).unwrap();
    assert!(route.consume(&authority, NOW).is_err());
    assert!(
        route
            .qualify(&new_run, "new", vec![], &authority, NOW)
            .is_err()
    );
    route.submit().unwrap();
    assert!(
        route
            .qualify(&new_run, "old", vec![], &authority, NOW)
            .is_err()
    );
    assert_eq!(
        route.qualify(&new_run, "new", vec![], &authority, NOW),
        Err(QualificationError::Technical)
    );
    store.append(&new_run).unwrap();
    store.publish(&new_run.envelope().run_id, 1).unwrap();
    assert!(
        route
            .qualify(&new_run, "new", vec![], &authority, NOW)
            .unwrap()
    );
    assert_eq!(
        &serde_json::to_value(route.history())
            .unwrap()
            .as_array()
            .unwrap()[..history.as_array().unwrap().len()],
        history.as_array().unwrap()
    );
    while route.history().len() < 1024 {
        route.set_mode(RouteMode::OptIn).unwrap();
    }
    let history = serde_json::to_vec(route.history()).unwrap();
    assert_eq!(route.rollback(), Err(QualificationError::Budget));
    assert!(route.consume(&authority, NOW).is_err());
    assert_eq!(history, serde_json::to_vec(route.history()).unwrap());
    assert_eq!(
        route.set_mode(RouteMode::OptIn),
        Err(QualificationError::Budget)
    );
}
