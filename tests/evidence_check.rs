use guard_evidence::{hash_file, keygen, sign_receipt, snapshot_project};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture {
    root: PathBuf,
    project: PathBuf,
    key: PathBuf,
    private: PathBuf,
    policy: Value,
    payload: Value,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::var_os("TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                "flowguard-evidence-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("main.rs"), "fn main() {}\n").unwrap();
        let project = project.canonicalize().unwrap();
        let private = root.join("private.der");
        let key = root.join("public.hex");
        keygen(&private, &key).unwrap();
        let program = root.join("runner");
        fs::write(&program, "trusted runner").unwrap();
        let report = root.join("report.json");
        fs::write(&report, "real report fixture").unwrap();
        let plan = format!("sha256:{}", "a".repeat(64));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let policy = json!({"task_id":"demo","plan_digest":plan,"required_tests":["accepts"],"runner":"cargo","max_age_seconds":60});
        let payload = json!({"schema_version":"testguard.execution/v1","producer":"testguard","project":project,"snapshot":snapshot_project(&project).unwrap(),"task_id":"demo","plan_digest":plan,"runner":"cargo","executable":{"path":program,"sha256":hash_file(&program).unwrap()},"argv":["test"],"started_at_unix_ms":now-10,"finished_at_unix_ms":now-1,"exit_code":0,"status":"PASS","required_tests":["accepts"],"observed_tests":["accepts"],"artifacts":[{"path":report,"sha256":hash_file(&report).unwrap()}],"execution_verified":true});
        Self {
            root,
            project,
            key,
            private,
            policy,
            payload,
        }
    }
    fn check(&self, payload: &Value, signed: bool, key: &Path, mcp: bool) -> Value {
        let envelope = if signed {
            sign_receipt(payload, &self.private).unwrap()
        } else {
            payload.clone()
        };
        let receipt = self.root.join("receipt.json");
        fs::write(&receipt, envelope.to_string()).unwrap();
        let policy = self.root.join("policy.json");
        fs::write(&policy, self.policy.to_string()).unwrap();
        let args =
            json!({"project":self.project,"receipt":receipt,"policy":policy,"trusted_key":key});
        if mcp {
            let mut child = Command::new(env!("CARGO_BIN_EXE_flowguard"))
                .args(["serve", "--mcp"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let messages = [
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
                json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
                json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"flowguard_evidence_check","arguments":args}}),
            ];
            let mut input = child.stdin.take().unwrap();
            for message in messages {
                writeln!(input, "{message}").unwrap();
            }
            drop(input);
            let out = child.wait_with_output().unwrap();
            let reply: Value = String::from_utf8(out.stdout)
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str::<Value>(l).unwrap())
                .find(|v| v["id"] == 2)
                .unwrap();
            serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
        } else {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_flowguard"));
            cmd.args(["evidence", "check"]);
            for name in ["project", "receipt", "policy", "trusted_key"] {
                cmd.arg(format!("--{}", name.replace('_', "-")))
                    .arg(args[name].as_str().unwrap());
            }
            let out = cmd.output().unwrap();
            assert!(matches!(out.status.code(), Some(2 | 3)));
            serde_json::from_slice(&out.stdout).unwrap()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn signed_success_is_read_only_and_still_requires_approval_with_mcp_parity() {
    let f = Fixture::new();
    let before = snapshot_project(&f.project).unwrap();
    let result = f.check(&f.payload, true, &f.key, false);
    assert_eq!(result["technical_pass"], true, "{result}");
    assert_eq!(result["decision"], "requires_approval");
    assert_eq!(result["approval"], "required");
    assert_eq!(result, f.check(&f.payload, true, &f.key, true));
    assert_eq!(before, snapshot_project(&f.project).unwrap());
    assert!(!f.project.join(".flowguard").exists());
}
#[test]
fn signed_semantic_and_external_obligation_negative_cases_then_repair() {
    let f = Fixture::new();
    for (field, value) in [
        ("producer", json!("fake")),
        ("task_id", json!("other")),
        ("runner", json!("maven")),
        ("plan_digest", json!(format!("sha256:{}", "b".repeat(64)))),
        ("exit_code", json!(1)),
        ("status", json!("FAIL")),
        ("finished_at_unix_ms", json!(u64::MAX)),
        ("finished_at_unix_ms", json!(1)),
        ("required_tests", json!(["other"])),
        ("observed_tests", json!(["other"])),
    ] {
        let mut bad = f.payload.clone();
        bad[field] = value;
        let result = f.check(&bad, true, &f.key, false);
        assert_eq!(result["technical_pass"], false, "field {field}: {result}");
    }
    assert_eq!(
        f.check(&f.payload, false, &f.key, false)["technical_pass"],
        false
    );
    let wrong_private = f.root.join("wrong.der");
    let wrong_key = f.root.join("wrong.hex");
    keygen(&wrong_private, &wrong_key).unwrap();
    assert_eq!(
        f.check(&f.payload, true, &wrong_key, false)["technical_pass"],
        false
    );
    assert_eq!(
        f.check(&f.payload, true, &f.key, false)["technical_pass"],
        true
    );
}
#[test]
fn source_report_and_key_boundaries_reject_then_repair() {
    let f = Fixture::new();
    fs::write(f.project.join("main.rs"), "changed").unwrap();
    assert_eq!(
        f.check(&f.payload, true, &f.key, false)["technical_pass"],
        false
    );
    fs::write(f.project.join("main.rs"), "fn main() {}\n").unwrap();
    let report = Path::new(f.payload["artifacts"][0]["path"].as_str().unwrap());
    fs::remove_file(report).unwrap();
    assert_eq!(
        f.check(&f.payload, true, &f.key, false)["technical_pass"],
        false
    );
    fs::write(report, "real report fixture").unwrap();
    assert_eq!(
        f.check(&f.payload, true, &f.key, false)["technical_pass"],
        true
    );
    let inside = f.project.join("public.hex");
    fs::copy(&f.key, &inside).unwrap();
    assert_eq!(
        f.check(&f.payload, true, &inside, false)["technical_pass"],
        false
    );
}
#[test]
fn signature_tampering_is_rejected() {
    let f = Fixture::new();
    assert_eq!(
        f.check(&f.payload, true, &f.key, false)["technical_pass"],
        true
    );
    let receipt = f.root.join("receipt.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
    value["payload"]["task_id"] = json!("tampered");
    fs::write(&receipt, value.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_flowguard"))
        .args(["evidence", "check", "--project"])
        .arg(&f.project)
        .arg("--receipt")
        .arg(&receipt)
        .arg("--policy")
        .arg(f.root.join("policy.json"))
        .arg("--trusted-key")
        .arg(&f.key)
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["technical_pass"], false);
    assert!(result["error"].as_str().unwrap().contains("signature"));
}
#[test]
fn malformed_or_weakened_policy_is_rejected() {
    let mut f = Fixture::new();
    let original = f.policy.clone();
    for (field, value) in [
        ("task_id", json!("")),
        ("required_tests", json!([])),
        ("required_tests", json!(["accepts", "accepts"])),
        ("runner", json!("shell")),
        ("max_age_seconds", json!(0)),
        ("max_age_seconds", json!(604801)),
    ] {
        f.policy = original.clone();
        f.policy[field] = value;
        assert_eq!(
            f.check(&f.payload, true, &f.key, false)["technical_pass"],
            false,
            "{field}"
        );
    }
    f.policy = original;
    assert_eq!(
        f.check(&f.payload, true, &f.key, false)["technical_pass"],
        true
    );
}
