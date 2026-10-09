use flowguard::host::{ReadOnlySession, VERSION};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;
fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    for (stage, project) in flowguard::stage::STAGES {
        let p = if project {
            format!("docs/project/{stage}.md")
        } else {
            format!("docs/features/a/{stage}.md")
        };
        let p = d.path().join(p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "native source").unwrap();
    }
    d
}
fn config(root: &std::path::Path) -> Vec<u8> {
    serde_json::to_vec(&json!({"version":VERSION,"tenant":"controller-only","jobs":{"discover":["discover","--root",root,"--feature","a"],"status":["stage","status","--root",root,"--feature","a"]}})).unwrap()
}
fn call(s: &mut ReadOnlySession, value: Value, cancel: bool) -> Value {
    s.call(
        &serde_json::to_vec(&value).unwrap(),
        &AtomicBool::new(cancel),
    )
}
#[test]
fn frozen_host_context_rejects_writes_permissions_paths_and_key_rebinding() {
    let d = fixture();
    let mut s = ReadOnlySession::freeze(&config(d.path())).unwrap();
    assert_eq!(
        call(&mut s, json!({"job":"discover","requestKey":"one"}), false)["exitCode"],
        0
    );
    for value in [
        json!({"job":"execute","requestKey":"bad"}),
        json!({"job":"discover","requestKey":"one","actorVerified":true}),
        json!({"job":"discover","requestKey":"bad","root":"/etc"}),
        json!({"job":"discover","requestKey":"bad","tenant":"other"}),
        json!({"job":"discover","requestKey":"bad","selfApprove":true}),
        json!({"job":"discover","requestKey":"bad","artifact":"../../other-tenant/evidence.json"}),
        json!({"job":"status","requestKey":"one"}),
        json!({"job":"discover","requestKey":"bad","shell":"touch /tmp/not-allowed"}),
    ] {
        assert_eq!(call(&mut s, value, false)["exitCode"], 4);
    }
    assert_eq!(
        call(&mut s, json!({"job":"discover","requestKey":"retry"}), true)["exitCode"],
        4
    );
    assert_eq!(
        call(
            &mut s,
            json!({"job":"discover","requestKey":"retry"}),
            false
        )["exitCode"],
        0
    );
    std::fs::remove_file(d.path().join("docs/project/02-architecture.md")).unwrap();
    assert_eq!(
        call(&mut s, json!({"job":"discover","requestKey":"one"}), false)["exitCode"],
        2
    );
    assert_eq!(
        s.call(
            br#"{"job":"discover","job":"status","requestKey":"dup"}"#,
            &AtomicBool::new(false)
        )["exitCode"],
        4
    );
    assert!(!d.path().join(".flowguard").exists());
}
#[test]
fn mcp_negotiates_version_ordinary_client_metadata_and_never_grants_claimed_capabilities() {
    let d = fixture();
    let session = ReadOnlySession::freeze(&config(d.path())).unwrap();
    let mut protocol = flowguard::mcp::Protocol::new(session);
    let cancel = AtomicBool::new(false);
    let mut send = |v: Value| protocol.handle(&serde_json::to_vec(&v).unwrap(), &cancel);
    assert!(
        send(json!({"jsonrpc":"2.0","id":0,"method":"tools/list"}))
            .unwrap()
            .get("error")
            .is_some()
    );
    let init=send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"future-version","capabilities":{"roots":{"listChanged":true},"sampling":{}},"clientInfo":{"name":"client","version":"1","title":"optional metadata"}}})).unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    assert!(send(json!({"jsonrpc":"2.0","method":"notifications/initialized"})).is_none());
    let tools = send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).unwrap();
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 1);
    assert_eq!(
        tools["result"]["tools"][0]["annotations"]["readOnlyHint"],
        true
    );
    for method in ["grant", "refs/write", "release", "execution/run"] {
        assert!(
            send(json!({"jsonrpc":"2.0","id":3,"method":method}))
                .unwrap()
                .get("error")
                .is_some()
        );
    }
    assert!(
        protocol
            .handle(
                br#"{"jsonrpc":"2.0","id":4,"id":5,"method":"ping"}"#,
                &cancel
            )
            .unwrap()
            .get("error")
            .is_some()
    );
    assert!(
        protocol
            .handle(&vec![b' '; 9000], &cancel)
            .unwrap()
            .get("error")
            .is_some()
    );
}
#[test]
fn actual_mcp_python_sdk_reads_same_library_and_rechecks_repeated_requests() {
    let d = fixture();
    let bytes = config(d.path());
    std::fs::write(d.path().join("controller.json"), &bytes).unwrap();
    let out = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/mcp_client.py"
        ))
        .arg(env!("CARGO_BIN_EXE_flowguard-mcp"))
        .arg(d.path())
        .arg(flowguard::digest(&bytes))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["client"], "mcp-python/1.29.0");
    assert_eq!(v["repeatedReadExit"], 2);
}

#[test]
fn bounded_frames_recover_and_controller_config_fails_closed() {
    let d = fixture();
    let mut value: Value = serde_json::from_slice(&config(d.path())).unwrap();
    value["jobs"]["execute"] = json!(["release", "--write"]);
    assert!(ReadOnlySession::freeze(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(ReadOnlySession::freeze(br#"{"version":"flowguard.host/v1","tenant":"x","jobs":{"read":["discover"],"read":["discover"]}}"#).is_err());
    let session = ReadOnlySession::freeze(&config(d.path())).unwrap();
    let mut input = vec![b'x'; 9000];
    input.extend_from_slice(b"\n{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n");
    let mut output = Vec::new();
    flowguard::mcp::serve(
        session,
        std::io::Cursor::new(input),
        &mut output,
        &AtomicBool::new(false),
    )
    .unwrap();
    let messages: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(messages.len(), 2);
    assert!(messages[0].get("error").is_some());
    assert_eq!(messages[1]["id"], 7);
    assert_eq!(messages[1]["result"], json!({}));
    let mut session = ReadOnlySession::freeze(&config(d.path())).unwrap();
    for i in 0..256 {
        assert_eq!(
            call(
                &mut session,
                json!({"job":"discover","requestKey":format!("key{i}")}),
                false
            )["exitCode"],
            0
        );
    }
    assert_eq!(
        call(
            &mut session,
            json!({"job":"discover","requestKey":"overflow"}),
            false
        )["exitCode"],
        4
    );
    assert_eq!(
        call(
            &mut session,
            json!({"job":"discover","requestKey":"key0"}),
            false
        )["exitCode"],
        0
    );
}

#[test]
fn launcher_requires_exact_profile_digest() {
    let d = fixture();
    std::fs::write(d.path().join("controller.json"), config(d.path())).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flowguard-mcp"))
        .args([
            "--local-readonly-v1",
            d.path().to_str().unwrap(),
            "controller.json",
            "incorrect",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
}
