use flowguard::dependencies::{GraphLimits, StageRecord, build_graph};
use std::collections::BTreeSet;
fn node(id: &str, stage: &str, deps: &[&str]) -> StageRecord {
    StageRecord {
        version: "flowguard.workflow/v1".into(),
        id: id.into(),
        stage: stage.into(),
        owner: "a".into(),
        source_digest: flowguard::digest(id.as_bytes()),
        dependencies: deps.iter().map(|x| x.to_string()).collect(),
    }
}
#[test]
fn deterministic_order_validates_identity_and_dependencies() {
    let a = node("a/01", "01-requirements", &[]);
    let b = node("a/03", "03-solution", &["a/01"]);
    assert_eq!(
        build_graph(vec![b.clone(), a.clone()], GraphLimits::default()).unwrap(),
        build_graph(vec![a.clone(), b.clone()], GraphLimits::default()).unwrap()
    );
    let graph = build_graph(vec![b.clone(), a.clone()], GraphLimits::default()).unwrap();
    assert_eq!(graph.order(), &["a/01", "a/03"]);
    assert!(build_graph(vec![b], GraphLimits::default()).is_err());
    assert!(build_graph(vec![a.clone(), a.clone()], GraphLimits::default()).is_err());
    let mut duplicate = a.clone();
    duplicate.id = "other".into();
    assert!(build_graph(vec![a.clone(), duplicate], GraphLimits::default()).is_err());
    let mut a_cycle = a.clone();
    a_cycle.dependencies = BTreeSet::from(["a/03".into()]);
    assert!(
        build_graph(
            vec![a_cycle, node("a/03", "03-solution", &["a/01"])],
            GraphLimits::default()
        )
        .is_err()
    );
    let mut invalid = a.clone();
    invalid.version = "future".into();
    assert!(build_graph(vec![invalid], GraphLimits::default()).is_err());
    assert!(
        build_graph(
            vec![a],
            GraphLimits {
                max_nodes: 0,
                max_edges: 10,
                max_depth: 10
            }
        )
        .is_err()
    );
}
#[test]
fn limits_depth_and_enforces_project_ownership() {
    assert!(
        build_graph(
            vec![
                node("a/01", "01-requirements", &[]),
                node("a/03", "03-solution", &["a/01"])
            ],
            GraphLimits {
                max_nodes: 10,
                max_edges: 10,
                max_depth: 1
            }
        )
        .is_err()
    );
    assert!(
        build_graph(
            vec![node("a/02", "02-architecture", &[])],
            GraphLimits::default()
        )
        .is_err()
    );
}
#[test]
fn strict_record_schema_rejects_unknown_fields() {
    let mut v = serde_json::to_value(node("a/01", "01-requirements", &[])).unwrap();
    v["accepted"] = true.into();
    assert!(serde_json::from_value::<StageRecord>(v).is_err());
}
#[test]
fn all_ten_native_stage_nodes_form_explicit_dag() {
    let deps: [&[&str]; 10] = [
        &[],
        &[],
        &["01-requirements", "02-architecture"],
        &["03-solution"],
        &["04-testcases"],
        &["05-hld"],
        &[],
        &["06-lld", "07-standards"],
        &["08-review"],
        &["07-standards", "09-docs"],
    ];
    let nodes = flowguard::stage::STAGES
        .iter()
        .zip(deps)
        .map(|((id, project), deps)| {
            let mut record = node(id, id, deps);
            if *project {
                record.owner = "project".into();
            }
            record
        })
        .collect();
    let graph = build_graph(nodes, GraphLimits::default()).unwrap();
    assert_eq!(graph.order().len(), 10);
    for (i, (stage, _)) in flowguard::stage::STAGES.iter().enumerate() {
        for dep in deps[i] {
            assert!(
                graph.order().iter().position(|s| s == dep).unwrap()
                    < graph.order().iter().position(|s| s == stage).unwrap()
            );
        }
    }
}
