#[test]
fn cli_prebinding_diagnostics_are_machine_readable_without_fabricated_context() {
    for args in [
        vec!["unknown-command"],
        vec![
            "gate",
            "check",
            "--repo",
            "private-candidate-secret",
            "--repo",
            "duplicate",
        ],
        vec!["gate", "check"],
    ] {
        let output = flowguard::cli::run(args.into_iter().map(String::from).collect());
        assert_eq!(output.code, 4);
        assert!(output.stdout.is_empty());
        let diagnostic: serde_json::Value = serde_json::from_str(&output.stderr)
            .expect("prebinding diagnostics must be a machine-readable object");
        assert_eq!(diagnostic.as_object().unwrap().len(), 2);
        assert_eq!(diagnostic["code"], "flowguard.input_unresolved");
        assert!(diagnostic.get("envelope").is_none());
        assert!(!output.stderr.contains("private-candidate-secret"));
    }
}

mod common;
use flowguard::{gate::*, transport::*};
use guardengine::integration::RunStatus;
use std::collections::BTreeMap;
fn pending() -> PendingGate {
    let (_root, binding) = common::binding();
    let up = common::upstream(&binding, guardengine::Enforcement::Advise, false);
    let frozen = common::frozen(&binding, &up, false);
    let key = obligation_scope(frozen.obligations().iter().next().unwrap());
    prepare_gate(
        &binding,
        &frozen,
        BTreeMap::from([(key, common::policy(&up))]),
        GateRequest {
            run_id: "transport-attempt".into(),
            action: "commit".into(),
            started_at: common::TIME.into(),
        },
    )
    .unwrap()
}
#[test]
fn bound_failure_kinds_preserve_exact_scope_and_never_manufacture_decisions() {
    for (failure, status, code) in [
        (
            ObservedFailure::RuntimeCrash,
            RunStatus::Error,
            "gate.runtime_failed",
        ),
        (ObservedFailure::Timeout, RunStatus::Error, "gate.timed_out"),
        (
            ObservedFailure::InvalidEvidence,
            RunStatus::Error,
            "gate.input_unavailable",
        ),
        (
            ObservedFailure::Cancelled,
            RunStatus::Cancelled,
            "gate.cancelled",
        ),
    ] {
        let plan = pending();
        let required = plan.required_scopes().to_vec();
        let result = plan.finish_failure(failure, common::TIME).unwrap();
        assert_eq!(result.envelope().run_status, status);
        assert_eq!(result.envelope().decision, None);
        assert_eq!(result.envelope().coverage.required_scopes, required);
        assert_eq!(result.envelope().diagnostics[0].code, code);
        assert!(result.artifacts().is_none());
        let output = encode_gate(&result, false).unwrap();
        assert_eq!(output.code, 4);
        assert!(output.stderr.is_empty());
        let wire: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(wire["envelope"]["decision"].is_null());
        assert!(wire["artifacts"].is_null());
        assert_eq!(wire["execution_authorized"], false);
    }
}
#[cfg(unix)]
#[test]
fn real_controller_process_failure_and_deadline_observations_have_bound_transport() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};
    // The test controller observes native process outcomes. Transport does not claim to supervise them.
    let crashed = Command::new("/bin/sh")
        .args(["-c", "kill -TERM $$"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(crashed.signal().is_some());
    let run = pending()
        .finish_failure(ObservedFailure::RuntimeCrash, common::TIME)
        .unwrap();
    assert_eq!(encode_gate(&run, false).unwrap().code, 4);
    let mut child = Command::new("/usr/bin/sleep")
        .arg("5")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(10);
    while std::time::Instant::now() < deadline {
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let run = pending()
        .finish_failure(ObservedFailure::Timeout, common::TIME)
        .unwrap();
    assert_eq!(run.envelope().diagnostics[0].code, "gate.timed_out");
    assert_eq!(run.envelope().decision, None);
}

#[test]
fn real_partial_and_completed_results_keep_native_json_and_exit_mapping() {
    use guardengine::{Decision, Enforcement};
    let (_root, binding) = common::binding();
    for (enforcement, partial, exit, decision) in [
        (Enforcement::Advise, false, 0, Decision::Allow),
        (Enforcement::Enforce, false, 2, Decision::Block),
        (Enforcement::Review, false, 3, Decision::RequireApproval),
        (Enforcement::Advise, true, 2, Decision::Block),
    ] {
        let up = common::upstream(&binding, enforcement, partial);
        let frozen = common::frozen(&binding, &up, false);
        let key = obligation_scope(frozen.obligations().iter().next().unwrap());
        let run = prepare_gate(
            &binding,
            &frozen,
            BTreeMap::from([(key.clone(), common::policy(&up))]),
            GateRequest {
                run_id: "transport-complete".into(),
                action: "commit".into(),
                started_at: common::TIME.into(),
            },
        )
        .unwrap()
        .evaluate(
            &[SpecialistEvidence {
                scope: &key,
                envelope: &up.envelope,
                artifacts: up.artifacts(),
            }],
            &common::FixtureAuthority {
                approval: None,
                unavailable: false,
            },
            common::NOW,
            common::TIME,
        )
        .unwrap();
        let output = encode_gate(&run, true).unwrap();
        assert_eq!(output.code, exit);
        assert!(output.stderr.is_empty());
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            json["envelope"],
            serde_json::to_value(run.envelope()).unwrap()
        );
        assert_eq!(run.envelope().decision, Some(decision));
        assert_eq!(json["authority_profile"], "local_fixture");
        assert_eq!(json["execution_authorized"], false);
        assert!(!json["artifacts"]["report"].as_str().unwrap().is_empty());
    }
}

#[cfg(unix)]
#[test]
fn actual_binary_rejects_non_utf8_arguments_with_same_static_diagnostic() {
    use std::os::unix::ffi::OsStringExt;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flowguard"))
        .arg(std::ffi::OsString::from_vec(vec![0xff, 0xfe]))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let diagnostic: guardengine::integration::TransportDiagnostic =
        serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic.code, "flowguard.input_unresolved");
}
