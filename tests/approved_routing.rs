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
