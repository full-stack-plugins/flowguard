use flowguard::specguard_adapter::{ExpectedExport, import_fixture};
use specguard::model::Identity;
use std::collections::BTreeSet;
fn expected() -> ExpectedExport {
    ExpectedExport {
        baseline_digest: "sha256:82b804c9a61cb334cdc1504decf4bdfde09dd35a7fd795627d79c48940369e91"
            .into(),
        source_digest: format!("sha256:{}", "1".repeat(64)),
        candidate_oid: "2".repeat(40),
        scope: BTreeSet::from([Identity {
            namespace: "demo".into(),
            id: "R1".into(),
        }]),
    }
}
#[test]
fn imports_real_sg_schema_as_explicit_fixture_not_authenticated_evidence() {
    let bytes = include_bytes!("../fixtures/providers/specguard-fixture-only.json");
    let imported = import_fixture(bytes, &expected()).unwrap();
    assert_eq!(imported.obligations().len(), 1);
    assert_eq!(imported.obligations()[0].requirement.id, "R1");
    assert_eq!(imported.obligations()[0].acceptance.id, "A1");
    assert!(
        !serde_json::to_string(imported.obligations())
            .unwrap()
            .contains("text\"")
    );
    for (field, value) in [
        ("apiVersion", serde_json::json!("specguard.domain/v999")),
        ("authenticationProfile", serde_json::json!("production")),
        ("scope", serde_json::json!([])),
        ("complete", serde_json::json!(false)),
        ("candidateOid", serde_json::json!("3".repeat(40))),
        (
            "baselineDigest",
            serde_json::json!(flowguard::digest(b"other")),
        ),
        ("obligations", serde_json::json!([])),
    ] {
        let mut json: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        json[field] = value;
        assert!(
            import_fixture(&serde_json::to_vec(&json).unwrap(), &expected()).is_err(),
            "{field}"
        );
    }
}
