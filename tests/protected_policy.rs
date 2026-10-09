use flowguard::{
    dependencies::{GraphLimits, StageRecord, build_graph},
    input_limits::AllowedRoot,
    policy::load_and_freeze,
};
use std::{collections::BTreeSet, fs};
fn graph() -> flowguard::dependencies::StageGraph {
    build_graph(
        vec![StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "A/01".into(),
            stage: "01-requirements".into(),
            owner: "A".into(),
            source_digest: flowguard::digest(b"source"),
            dependencies: BTreeSet::new(),
        }],
        GraphLimits::default(),
    )
    .unwrap()
}
fn document() -> serde_json::Value {
    serde_json::json!({"version":"flowguard.policy/v1alpha1", "context_digest":flowguard::digest(b"context"), "baseline_digest":flowguard::digest(b"baseline"), "graph_digest":flowguard::digest(&serde_json::to_vec(&graph()).unwrap()), "obligations":[{"stage_id":"A/04","guard":"testguard","coverage":["A"],"rules_digest":flowguard::digest(b"rules"),"analyzer_version":"1"}]})
}
fn load(
    bytes: &[u8],
    pin: &str,
    context: &str,
    baseline: &str,
) -> Result<flowguard::policy::PolicySnapshot, &'static str> {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("policy.json"), bytes).unwrap();
    let root = AllowedRoot::new(dir.path(), 16 * 1024 * 1024).unwrap();
    load_and_freeze(&root, "policy.json", pin, context, baseline, &graph())
}
#[test]
fn controller_pin_freezes_exact_bytes_before_provider_collection() {
    let bytes = serde_json::to_vec(&document()).unwrap();
    let pin = flowguard::digest(&bytes);
    let loaded = load(
        &bytes,
        &pin,
        &flowguard::digest(b"context"),
        &flowguard::digest(b"baseline"),
    )
    .unwrap();
    assert_eq!(loaded.raw_digest(), pin);
    assert_eq!(loaded.frozen().obligations().len(), 1);
    assert!(loaded.frozen().missing_stages().contains("A/04"));
    assert_eq!(
        loaded.frozen().missing_providers(&BTreeSet::new()),
        BTreeSet::from(["testguard".into()])
    );
}
#[test]
fn pin_rejects_candidate_weakening_and_raw_byte_replacement() {
    let original = serde_json::to_vec(&document()).unwrap();
    let mut changed = document();
    changed["obligations"][0]["guard"] = "otherguard".into();
    let changed = serde_json::to_vec(&changed).unwrap();
    assert!(
        load(
            &changed,
            &flowguard::digest(&original),
            &flowguard::digest(b"context"),
            &flowguard::digest(b"baseline")
        )
        .is_err()
    );
    let mut whitespace = original.clone();
    whitespace.push(b' ');
    assert!(
        load(
            &whitespace,
            &flowguard::digest(&original),
            &flowguard::digest(b"context"),
            &flowguard::digest(b"baseline")
        )
        .is_err()
    );
}
#[test]
fn pinned_document_cannot_be_rebound_to_another_context_baseline_or_graph() {
    for field in ["context_digest", "baseline_digest", "graph_digest"] {
        let mut doc = document();
        doc[field] = flowguard::digest(b"different").into();
        let bytes = serde_json::to_vec(&doc).unwrap();
        assert!(
            load(
                &bytes,
                &flowguard::digest(&bytes),
                &flowguard::digest(b"context"),
                &flowguard::digest(b"baseline")
            )
            .is_err(),
            "{field}"
        );
    }
}
#[test]
fn closed_schema_rejects_version_fields_and_duplicate_obligations() {
    for case in 0..4 {
        let mut doc = document();
        match case {
            0 => doc["version"] = "flowguard.policy/v99".into(),
            1 => doc["authenticated"] = true.into(),
            2 => {
                let extra = doc["obligations"][0].clone();
                doc["obligations"].as_array_mut().unwrap().push(extra);
            }
            _ => {
                doc.as_object_mut().unwrap().remove("graph_digest");
            }
        }
        let bytes = serde_json::to_vec(&doc).unwrap();
        assert!(
            load(
                &bytes,
                &flowguard::digest(&bytes),
                &flowguard::digest(b"context"),
                &flowguard::digest(b"baseline")
            )
            .is_err(),
            "case {case}"
        );
    }
}
#[test]
fn policy_budget_cannot_be_expanded_by_authorized_root() {
    let mut bytes = serde_json::to_vec(&document()).unwrap();
    bytes.resize(1024 * 1024 + 1, b' ');
    assert!(
        load(
            &bytes,
            &flowguard::digest(&bytes),
            &flowguard::digest(b"context"),
            &flowguard::digest(b"baseline")
        )
        .is_err()
    );
}
#[test]
fn duplicate_fields_and_unauthorized_reads_fail_closed() {
    let text = serde_json::to_string(&document()).unwrap();
    let duplicate = text.replacen("{", "{\"version\":\"flowguard.policy/v1alpha1\",", 1);
    assert!(
        load(
            duplicate.as_bytes(),
            &flowguard::digest(duplicate.as_bytes()),
            &flowguard::digest(b"context"),
            &flowguard::digest(b"baseline")
        )
        .is_err()
    );
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let bytes = serde_json::to_vec(&document()).unwrap();
    fs::write(outside.path().join("policy.json"), &bytes).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("policy.json"),
        dir.path().join("escape.json"),
    )
    .unwrap();
    let root = AllowedRoot::new(dir.path(), 1024 * 1024).unwrap();
    for path in [
        "missing.json",
        "../policy.json",
        "escape.json",
        "https://example.invalid/policy.json",
    ] {
        assert!(
            load_and_freeze(
                &root,
                path,
                &flowguard::digest(&bytes),
                &flowguard::digest(b"context"),
                &flowguard::digest(b"baseline"),
                &graph()
            )
            .is_err()
        );
    }
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        1,
        "read-only loader creates no storage"
    );
}
