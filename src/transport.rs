//! Failure encoding only. This module does not supervise processes or grant execution.
use crate::cli::CliOutput;
use guardengine::integration::TransportDiagnostic;

/// A controller-observed failure after immutable binding. These classifications
/// are deliberately not deserializable and never claim successful execution.
#[derive(Clone, Copy, Debug)]
pub enum ObservedFailure {
    RuntimeCrash,
    Timeout,
    InvalidEvidence,
    Cancelled,
}

/// Unresolved input never obtains a fabricated candidate binding or envelope.
pub fn prebinding_error() -> CliOutput {
    let diagnostic = TransportDiagnostic {
        code: "flowguard.input_unresolved".into(),
        message: "invalid or unavailable command input".into(),
    };
    CliOutput {
        code: 4,
        stdout: Vec::new(),
        stderr: format!(
            "{}\n",
            serde_json::to_string(&diagnostic).expect("static diagnostic")
        ),
    }
}

/// Encode only the opaque result produced by the actual gate. `fixture` is a
/// presentation label, never an authorization input. Native CLI fields remain
/// unchanged; diagnostics cannot be interleaved with the JSON stdout record.
pub fn encode_gate(
    run: &crate::gate::GateRun,
    fixture: bool,
) -> Result<CliOutput, TransportDiagnostic> {
    use guardengine::integration::{EvidenceProfile, RunStatus, verify_engine_artifacts};
    let invalid = || TransportDiagnostic {
        code: "flowguard.output_invalid".into(),
        message: "gate result could not be encoded".into(),
    };
    run.envelope()
        .validate(EvidenceProfile::EngineBacked)
        .map_err(|_| invalid())?;
    let code = match (&run.envelope().run_status, &run.envelope().decision) {
        (RunStatus::Completed, Some(guardengine::Decision::Allow)) => 0,
        (RunStatus::Completed, Some(guardengine::Decision::Block)) => 2,
        (RunStatus::Completed, Some(guardengine::Decision::RequireApproval)) => 3,
        _ => 4,
    };
    let artifacts = run
        .artifacts()
        .map(|a| {
            verify_engine_artifacts(run.envelope(), a.contract, a.facts, a.report)
                .map_err(|_| invalid())?;
            Ok::<_, TransportDiagnostic>(serde_json::json!({
                "contract": std::str::from_utf8(a.contract).map_err(|_| invalid())?,
                "facts": std::str::from_utf8(a.facts).map_err(|_| invalid())?,
                "report": std::str::from_utf8(a.report).map_err(|_| invalid())?,
                "domain": serde_json::to_string(run.domain().ok_or_else(invalid)?).map_err(|_| invalid())?,
            }))
        })
        .transpose()?;
    let value = serde_json::json!({
        "apiVersion":"flowguard.cli/v1alpha1", "command":"gate check",
        "authority_profile":if fixture { "local_fixture" } else { "unavailable" },
        "execution_authorized":false, "envelope":run.envelope(), "artifacts":artifacts,
    });
    let mut stdout = serde_json::to_vec(&value).map_err(|_| invalid())?;
    stdout.push(b'\n');
    Ok(CliOutput {
        code,
        stdout,
        stderr: String::new(),
    })
}
