//! Real CLI process exercise, using unchanged native five-provider artifacts.
#[test]
fn actual_cli_scoped_sources_five_guard_matrix() {
    let directory = tempfile::tempdir().unwrap();
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/cli-five-guards/run.py"
        ))
        .args(["--flowguard", env!("CARGO_BIN_EXE_flowguard"), "--output"])
        .arg(directory.path().join("run"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rows.len(), 13);
    assert_eq!(rows[0]["decision"], "ALLOW");
    assert!(
        rows.iter()
            .any(|r| r["case"] == "expired" && r["exit"] == 4)
    );
    assert!(
        rows.iter()
            .any(|r| r["case"] == "candidate-drift" && r["prebinding"] == true)
    );
}
