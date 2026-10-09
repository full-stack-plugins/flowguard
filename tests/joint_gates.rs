//! Opt-in actual local producers, not production controller authentication.
#[test]
#[ignore = "requires fixed fixture helper and native tool paths"]
fn actual_parallel_requirements_and_rebuilt_merge_candidates() {
    let output = tempfile::tempdir().unwrap();
    let capture = std::env::var_os("FLOWGUARD_JOINT_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| output.path().join("capture"));
    let result = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/providers/joint/capture.py"
        ))
        .arg(&capture)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["profile"], "flowguard.joint-local/v1");
    for name in ["A", "B", "M1", "M2"] {
        assert_eq!(report["cases"][name]["exit"], 0);
    }
    for name in ["A_to_B", "A_to_M1", "M1_to_M2", "A_expired", "B_revoked"] {
        assert_eq!(report["cases"][name]["exit"], 4);
    }
    assert_eq!(report["shared_project_invalidation"], true);
    assert_eq!(report["source_unchanged"], true);
    assert_eq!(
        report["invalidation"]["rule_and_analyzer_drift_withdraw_A_only"],
        true
    );
    assert_eq!(report["invalidation"]["old_M1_publication_refused"], true);
    assert_ne!(
        report["invalidation"]["scoped_approved_baselines"]["A"],
        report["invalidation"]["scoped_approved_baselines"]["B"]
    );
    assert_eq!(report["production_authenticated"], false);
    println!("retained capture: {}", capture.display());
}
