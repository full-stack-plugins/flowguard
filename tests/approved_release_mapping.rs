use flowguard::cli;
#[test]
fn release_mapping_cli_requires_exact_unambiguous_triples() {
    let d = tempfile::tempdir().unwrap();
    let row = serde_json::json!({"feature_id":"sample","requirement_id":"sample/REQ-1","immutable_09":flowguard::digest(b"immutable09")});
    let check = |mapping: serde_json::Value, expected: serde_json::Value| {
        std::fs::write(
            d.path().join("mapping.json"),
            serde_json::to_vec(&mapping).unwrap(),
        )
        .unwrap();
        std::fs::write(
            d.path().join("expected.json"),
            serde_json::to_vec(&expected).unwrap(),
        )
        .unwrap();
        cli::run(
            [
                "legacy",
                "release-map",
                "--root",
                d.path().to_str().unwrap(),
                "--mapping",
                "mapping.json",
                "--expected",
                "expected.json",
            ]
            .map(String::from)
            .to_vec(),
        )
    };
    let exact = serde_json::json!([row]);
    let valid = check(exact.clone(), exact.clone());
    assert_eq!(valid.code, 0, "{}", valid.stderr);
    let output: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(output["new_eligibility"], "not_evaluated");
    assert_eq!(output["real_legacy_migration"], false);
    assert_ne!(check(serde_json::json!([]), exact.clone()).code, 0);
    assert_ne!(check(serde_json::json!([row, row]), exact.clone()).code, 0);
    let mut wrong = row.clone();
    wrong["requirement_id"] = serde_json::json!("sample/REQ-2");
    assert_ne!(check(serde_json::json!([wrong]), exact).code, 0);
}
