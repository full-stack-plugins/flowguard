use flowguard::{cli, legacy::LEGACY_REVISION};
#[test]
fn cli_separates_declaration_effective_fingerprint_state_and_eligibility() {
    let fixture: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/legacy/differential.json")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("docs/project")).unwrap();
    for case in fixture["cases"].as_array().unwrap().iter().take(8) {
        std::fs::write(
            dir.path().join("docs/project/02-architecture.md"),
            case["text"].as_str().unwrap(),
        )
        .unwrap();
        let output = cli::run(
            [
                "legacy",
                "status",
                "--root",
                dir.path().to_str().unwrap(),
                "--feature",
                "sample",
                "--stage",
                "02-architecture",
                "--revision",
                LEGACY_REVISION,
            ]
            .map(String::from)
            .to_vec(),
        );
        assert_eq!(output.code, 0, "{}: {}", case["name"], output.stderr);
        let view: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(view["legacy_effective"], case["legacy"]["status"]);
        assert_eq!(view["new_eligibility"], "not_evaluated");
        if case["name"] == "body-invalidated" {
            assert_eq!(view["declared"], "accepted");
        }
    }
}
#[test]
fn excluded_metadata_and_evidence_change_full_bytes_but_not_legacy_effective_state() {
    let fixture: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/legacy/differential.json")).unwrap();
    let original = fixture["cases"][3]["text"].as_str().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("docs/project")).unwrap();
    let path = dir.path().join("docs/project/02-architecture.md");
    std::fs::write(&path, original).unwrap();
    let root = flowguard::input_limits::AllowedRoot::new(dir.path(), 1024 * 1024).unwrap();
    let before =
        flowguard::legacy::status(&root, "sample", "02-architecture", LEGACY_REVISION).unwrap();
    let changed = format!(
        "{original}\n<!-- FLOWGUARD_EVIDENCE_START -->\nany evidence\n<!-- FLOWGUARD_EVIDENCE_END -->\n"
    );
    std::fs::write(&path, changed).unwrap();
    let after =
        flowguard::legacy::status(&root, "sample", "02-architecture", LEGACY_REVISION).unwrap();
    assert_eq!(
        after.legacy_effective,
        flowguard::stage_transition::StageState::Accepted
    );
    assert_ne!(before.source_digest, after.source_digest);
    let changed = original.replace("| 阶段状态 | accepted |", "| 阶段状态 | skipped |");
    std::fs::write(&path, changed).unwrap();
    assert_eq!(
        flowguard::legacy::status(&root, "sample", "02-architecture", LEGACY_REVISION)
            .unwrap()
            .legacy_effective,
        flowguard::stage_transition::StageState::Skipped
    );
}
