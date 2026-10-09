use flowguard::{
    input_limits::AllowedRoot,
    legacy::{LEGACY_REVISION, observe},
    stage_transition::StageState,
};
#[test]
fn reads_declared_legacy_state_but_never_issues_migration_or_eligibility() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("docs/project")).unwrap();
    let text = "| 阶段 | 02-architecture |\n| 阶段状态 | accepted |\n";
    std::fs::write(dir.path().join("docs/project/02-architecture.md"), text).unwrap();
    let observation = observe(
        &AllowedRoot::new(dir.path(), 4096).unwrap(),
        "feature-a",
        "02-architecture",
        LEGACY_REVISION,
    )
    .unwrap();
    assert_eq!(observation.declared, StageState::Accepted);
    assert_eq!(
        observation.source_digest,
        flowguard::digest(text.as_bytes())
    );
    assert!(!observation.migration_supported());
}

#[test]
fn actual_pinned_legacy_vectors_have_explicit_unsupported_differences() {
    let vector: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/legacy/differential.json")).unwrap();
    assert_eq!(vector["revision"], LEGACY_REVISION);
    for case in vector["cases"].as_array().unwrap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("docs/project/02-architecture.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if let Some(text) = case["text"].as_str() {
            std::fs::write(&path, text).unwrap();
        }
        match case["fault"].as_str() {
            Some("directory") => std::fs::create_dir(&path).unwrap(),
            Some("invalid-utf8") => std::fs::write(&path, [255]).unwrap(),
            _ => (),
        }
        let root = AllowedRoot::new(dir.path(), 4096).unwrap();
        let result = observe(&root, "feature-a", "02-architecture", LEGACY_REVISION);
        let name = case["name"].as_str().unwrap();
        if [
            "unknown-state",
            "wrong-stage",
            "duplicate-state",
            "fenced-example",
            "missing",
            "directory",
            "invalid-utf8",
        ]
        .contains(&name)
        {
            assert!(result.is_err(), "{name}");
            continue;
        }
        let observation = result.unwrap();
        assert!(!observation.migration_supported());
        let declared = serde_json::to_value(observation.declared).unwrap();
        if name == "body-invalidated" {
            assert_eq!(declared, "accepted");
            assert_eq!(case["legacy"]["status"], "invalidated");
        } else {
            assert_eq!(declared, case["legacy"]["status"], "{name}");
        }
        assert!(observe(&root, "feature-a", "02-architecture", "unknown-sha").is_err());
    }
}

#[test]
fn fixed_transition_matrix_never_transfers_legacy_authority() {
    use flowguard::stage_transition::{TransitionCheck, transition};
    let vector: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/legacy/differential.json")).unwrap();
    let states = [
        "pending",
        "in_progress",
        "pending_acceptance",
        "accepted",
        "inherited",
        "skipped",
        "invalidated",
    ];
    let mut differences = 0;
    for from in states {
        for to in states {
            let old = vector["legal"][from]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(to));
            let new = transition(
                serde_json::from_value(serde_json::json!(from)).unwrap(),
                serde_json::from_value(serde_json::json!(to)).unwrap(),
            );
            if old && to == "accepted" {
                assert_ne!(new, TransitionCheck::DeclarationAllowed);
            }
            if old && to == "inherited" {
                assert!(matches!(
                    new,
                    TransitionCheck::RequiresExactBaselineAndApproval | TransitionCheck::Illegal
                ));
            }
            if old && to == "skipped" {
                assert!(matches!(
                    new,
                    TransitionCheck::RequiresProtectedSkipAndApproval | TransitionCheck::Illegal
                ));
            }
            if old != (new != TransitionCheck::Illegal) {
                differences += 1;
            }
        }
    }
    assert!(
        differences > 0,
        "legacy/new differences must remain visible"
    );
}

#[test]
fn boundaries_reject_ambiguous_or_faulty_sources_without_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("docs/project/02-architecture.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let valid = "| 阶段 | 02-architecture |\n| 阶段状态 | pending |\n";
    let text =
        format!("{valid}\n~~~~md\n| 阶段状态 | accepted |\n~~~\n| 阶段状态 | inherited |\n~~~~\n");
    std::fs::write(&path, &text).unwrap();
    let root = AllowedRoot::new(dir.path(), 4096).unwrap();
    assert_eq!(
        observe(&root, "feature-a", "02-architecture", LEGACY_REVISION)
            .unwrap()
            .declared,
        StageState::Pending
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    assert!(observe(&root, "../outside", "02-architecture", LEGACY_REVISION).is_err());
    assert!(observe(&root, "feature-a", "99-other", LEGACY_REVISION).is_err());
    assert!(
        observe(
            &AllowedRoot::new(dir.path(), 10).unwrap(),
            "feature-a",
            "02-architecture",
            LEGACY_REVISION
        )
        .is_err()
    );
    std::fs::write(&path, format!("{valid}| 批准依据 | a | b |\n")).unwrap();
    assert!(observe(&root, "feature-a", "02-architecture", LEGACY_REVISION).is_err());
    assert!(!dir.path().join(".flowguard").exists());
}
