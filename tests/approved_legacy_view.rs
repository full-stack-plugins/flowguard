use flowguard::{cli, legacy::LEGACY_REVISION};
#[test]
fn cli_separates_declaration_effective_fingerprint_state_and_eligibility() {
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!("../fixtures/legacy/differential.json")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("docs/project")).unwrap();
    for case in fixture["cases"].as_array().unwrap().iter().take(8) {
        std::fs::write(dir.path().join("docs/project/02-architecture.md"),case["text"].as_str().unwrap()).unwrap();
        let output = cli::run(["legacy", "status", "--root", dir.path().to_str().unwrap(), "--feature", "sample", "--stage", "02-architecture", "--revision", LEGACY_REVISION].map(String::from).to_vec());
        assert_eq!(output.code, 0, "{}: {}",case["name"], output.stderr);
        let view: serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(view["legacy_effective"],case["legacy"]["status"]);
        assert_eq!(view["new_eligibility"],"not_evaluated");
        if case["name"]=="body-invalidated" { assert_eq!(view["declared"], "accepted"); }
    }
}
