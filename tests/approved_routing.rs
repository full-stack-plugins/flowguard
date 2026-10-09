mod qualification_support;
use flowguard::{
    local_migration::{LocalStageRoute, RouteMode},
    stage_qualification::*,
    stage_transition::StageState,
};
use qualification_support::*;
use std::collections::BTreeMap;
#[test]
fn shadow_opt_in_rollback_preserve_history_and_fresh_revocation() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let intent = StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap();
    let (p, r) = produce(&b, &g, intent, false, &mut c);
    approve(&mut c, &p, "approval");
    let mut route = LocalStageRoute::new(
        &p,
        StageState::Accepted,
        flowguard::digest(b"legacy accepted"),
    )
    .unwrap();
    assert_eq!(route.state(), StageState::Invalidated);
    assert!(route.qualify(&r, "approval", vec![], &c, NOW).is_err());
    route.submit().unwrap();
    assert!(route.qualify(&r, "approval", vec![], &c, NOW).unwrap() == false);
    route.set_mode(RouteMode::Shadow).unwrap();
    assert!(route.qualify(&r, "approval", vec![], &c, NOW).unwrap() == false);
    route.set_mode(RouteMode::OptIn).unwrap();
    assert!(route.qualify(&r, "approval", vec![], &c, NOW).unwrap());
    assert_eq!(route.state(), StageState::Accepted);
    assert_eq!(route.consume(&c, NOW).unwrap(), StageState::Accepted);
    route.rollback().unwrap();
    assert!(route.consume(&c, NOW).is_err());
    route.set_mode(RouteMode::OptIn).unwrap();
    assert!(route.submit().is_err());
    route.invalidate().unwrap();
    route.submit().unwrap();
    c.approvals.get_mut("approval").unwrap().validity.revoked = true;
    assert!(route.qualify(&r, "approval", vec![], &c, NOW).is_err());
    let before = serde_json::to_value(route.history()).unwrap();
    route.rollback().unwrap();
    assert_eq!(route.mode(), RouteMode::Advisory);
    assert!(route.qualify(&r, "approval", vec![], &c, NOW).is_err());
    let after = serde_json::to_value(route.history()).unwrap();
    assert_eq!(
        &after.as_array().unwrap()[..before.as_array().unwrap().len()],
        before.as_array().unwrap()
    );
    route.set_mode(RouteMode::OptIn).unwrap();
    assert!(route.qualify(&r, "approval", vec![], &c, NOW).is_err());
}
#[test]
fn edited_satisfied_stage_invalidates_before_resubmitting_new_plan() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (p, r) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &p, "old");
    let mut nodes: Vec<_> = g.nodes().values().cloned().collect();
    nodes
        .iter_mut()
        .find(|n| n.id == "first")
        .unwrap()
        .source_digest = flowguard::digest(b"edited full bytes");
    let changed = flowguard::dependencies::build_graph(
        nodes,
        flowguard::dependencies::GraphLimits::default(),
    )
    .unwrap();
    let (p2, r2) = produce(
        &b,
        &changed,
        StageIntent::freeze(&changed, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &p2, "new");
    let mut route =
        LocalStageRoute::new(&p, StageState::Accepted, flowguard::digest(b"old legacy")).unwrap();
    route.submit().unwrap();
    route.set_mode(RouteMode::OptIn).unwrap();
    route.qualify(&r, "old", vec![], &c, NOW).unwrap();
    route.edited(&p2).unwrap();
    assert_eq!(route.state(), StageState::Invalidated);
    assert!(route.consume(&c, NOW).is_err());
    assert!(route.qualify(&r2, "new", vec![], &c, NOW).is_err());
    route.submit().unwrap();
    assert!(route.qualify(&r2, "old", vec![], &c, NOW).is_err());
    assert!(route.qualify(&r2, "new", vec![], &c, NOW).unwrap());
}

#[test]
fn exhausted_history_never_prevents_stopping_qualified_consumption() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (p, r) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &p, "approval");
    let mut nodes: Vec<_> = g.nodes().values().cloned().collect();
    nodes
        .iter_mut()
        .find(|n| n.id == "first")
        .unwrap()
        .source_digest = flowguard::digest(b"edit after audit exhausted");
    let changed = flowguard::dependencies::build_graph(
        nodes,
        flowguard::dependencies::GraphLimits::default(),
    )
    .unwrap();
    let (p2, _) = produce(
        &b,
        &changed,
        StageIntent::freeze(&changed, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    for operation in ["rollback", "invalidate", "edited", "shadow", "advisory"] {
        let mut route =
            LocalStageRoute::new(&p, StageState::Accepted, flowguard::digest(b"history")).unwrap();
        route.submit().unwrap();
        route.set_mode(RouteMode::OptIn).unwrap();
        route.qualify(&r, "approval", vec![], &c, NOW).unwrap();
        while route.history().len() < 1024 {
            route.set_mode(RouteMode::OptIn).unwrap();
        }
        let before = serde_json::to_vec(route.history()).unwrap();
        assert_eq!(route.consume(&c, NOW).unwrap(), StageState::Accepted);
        let result = match operation {
            "rollback" => route.rollback(),
            "invalidate" => route.invalidate(),
            "edited" => route.edited(&p2),
            "shadow" => route.set_mode(RouteMode::Shadow),
            _ => route.set_mode(RouteMode::Advisory),
        };
        assert_eq!(result, Err(QualificationError::Budget));
        assert!(
            route.consume(&c, NOW).is_err(),
            "{operation} must stop consumption despite audit exhaustion"
        );
        assert_eq!(serde_json::to_vec(route.history()).unwrap(), before);
        assert_eq!(
            route.set_mode(RouteMode::OptIn),
            Err(QualificationError::Budget)
        );
        assert!(route.consume(&c, NOW).is_err());
    }
}

#[test]
fn invalidated_import_and_repeated_edits_keep_history_and_require_latest_approval() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (p, _) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    let mut nodes: Vec<_> = g.nodes().values().cloned().collect();
    nodes
        .iter_mut()
        .find(|n| n.id == "first")
        .unwrap()
        .source_digest = flowguard::digest(b"first edit");
    let g2 = flowguard::dependencies::build_graph(
        nodes.clone(),
        flowguard::dependencies::GraphLimits::default(),
    )
    .unwrap();
    let (p2, _) = produce(
        &b,
        &g2,
        StageIntent::freeze(&g2, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &p2, "first-edit");
    nodes
        .iter_mut()
        .find(|n| n.id == "first")
        .unwrap()
        .source_digest = flowguard::digest(b"second edit");
    let g3 = flowguard::dependencies::build_graph(
        nodes,
        flowguard::dependencies::GraphLimits::default(),
    )
    .unwrap();
    let (p3, r3) = produce(
        &b,
        &g3,
        StageIntent::freeze(&g3, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &p3, "latest");
    for historical in [
        StageState::Accepted,
        StageState::Inherited,
        StageState::Skipped,
    ] {
        let mut route =
            LocalStageRoute::new(&p, historical, flowguard::digest(b"old history")).unwrap();
        let before = serde_json::to_value(route.history()).unwrap();
        route.edited(&p2).unwrap();
        route.edited(&p3).unwrap();
        assert_eq!(route.state(), StageState::Invalidated);
        let after = serde_json::to_value(route.history()).unwrap();
        assert_eq!(
            &after.as_array().unwrap()[..before.as_array().unwrap().len()],
            before.as_array().unwrap()
        );
        route.set_mode(RouteMode::OptIn).unwrap();
        assert!(route.qualify(&r3, "latest", vec![], &c, NOW).is_err());
        route.submit().unwrap();
        assert!(route.qualify(&r3, "first-edit", vec![], &c, NOW).is_err());
        assert!(route.qualify(&r3, "latest", vec![], &c, NOW).unwrap());
    }
}
