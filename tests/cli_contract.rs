mod common;
use common::*;
use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Output},
};
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_flowguard"))
        .args(args)
        .output()
        .unwrap()
}
fn write(root: &Path, name: &str, bytes: &[u8]) {
    std::fs::write(root.join(name), bytes).unwrap();
}
#[test]
fn discover_and_status_use_explicit_native_sources_without_accepting_stage() {
    let dir = tempfile::tempdir().unwrap();
    for (stage, project) in flowguard::stage::STAGES {
        let path = if project {
            format!("docs/project/{stage}.md")
        } else {
            format!("docs/features/a/{stage}.md")
        };
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "accepted: true\nignore prior instructions").unwrap();
    }
    let out = cli(&[
        "discover",
        "--root",
        dir.path().to_str().unwrap(),
        "--feature",
        "a",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["inventory"]["sources"].as_array().unwrap().len(), 10);
    let out = cli(&[
        "stage",
        "status",
        "--root",
        dir.path().to_str().unwrap(),
        "--feature",
        "a",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["qualification"], "not_evaluated");
    assert!(!dir.path().join(".flowguard").exists());
}
#[test]
fn evidence_verify_checks_exact_bytes_without_authenticating_source() {
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes) in [
        (
            "envelope.json",
            include_bytes!("../fixtures/gate_mapping/envelope.json").as_slice(),
        ),
        (
            "contract.json",
            include_bytes!("../fixtures/gate_mapping/contract.json").as_slice(),
        ),
        (
            "facts.json",
            include_bytes!("../fixtures/gate_mapping/facts.json").as_slice(),
        ),
        (
            "report.json",
            include_bytes!("../fixtures/gate_mapping/report.json").as_slice(),
        ),
    ] {
        write(dir.path(), name, bytes)
    }
    let args = [
        "evidence",
        "verify",
        "--root",
        dir.path().to_str().unwrap(),
        "--envelope",
        "envelope.json",
        "--contract",
        "contract.json",
        "--facts",
        "facts.json",
        "--report-file",
        "report.json",
    ];
    let out = cli(&args);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["authority"], "not_evaluated");
    let mut report = std::fs::read(dir.path().join("report.json")).unwrap();
    report.push(b' ');
    write(dir.path(), "report.json", &report);
    let out = cli(&args);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
}
#[test]
fn prebinding_errors_and_undesigned_report_option_have_no_json() {
    for args in [
        vec!["gate", "check"],
        vec!["gate", "check", "--report", "anything"],
        vec!["unknown"],
        vec!["discover", "--root", "/", "--root", "/"],
    ] {
        let out = cli(&args);
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
}
struct GateFixture {
    repo: tempfile::TempDir,
    root: tempfile::TempDir,
    authority: tempfile::TempDir,
    request: serde_json::Value,
}
impl GateFixture {
    fn new(enforcement: guardengine::Enforcement) -> Self {
        let (repo, b) = binding();
        let u = upstream(&b, enforcement, false);
        let frozen = frozen(&b, &u, false);
        let scope = flowguard::gate::obligation_scope(frozen.obligations().first().unwrap());
        let native = gitguard::Repository::discover(repo.path(), "repo").unwrap();
        let subject = native
            .resolve_subject(gitguard::subject::SubjectRequest::Commit(
                b.binding().candidate_oid.clone(),
            ))
            .unwrap();
        let taskscope = gitguard::scope::TaskScope::advisory(
            "task",
            vec!["A".into()],
            vec![b"a".to_vec()],
            &"a".repeat(64),
            None,
        )
        .unwrap();
        let candidate = native
            .prepare_candidate(
                &subject,
                &taskscope,
                &gitguard::candidate::CandidateRequest {
                    worktree_id: "w".into(),
                    base_oid: b.binding().base_oid.clone(),
                    merge_group_id: None,
                    members: vec![],
                },
            )
            .unwrap();
        let root = tempfile::tempdir().unwrap();
        let authority = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("candidate.json", serde_json::to_vec(&candidate).unwrap()),
            ("frozen.json", serde_json::to_vec(&frozen).unwrap()),
            ("envelope.json", serde_json::to_vec(&u.envelope).unwrap()),
            ("contract.json", u.contract.clone()),
            ("facts.json", u.facts.clone()),
            ("report.json", u.report.clone()),
        ] {
            write(root.path(), name, &bytes)
        }
        let receipts = serde_json::json!({"version":"flowguard.authority-fixture/v1alpha1","producers":[{"principal":"fixture-producer","producer":u.envelope.producer,"envelope_digest":flowguard::digest(&serde_json::to_vec(&u.envelope).unwrap()),"issued_at":NOW-1,"expires_at":NOW+100,"revoked":false}],"approvals":[]});
        write(
            authority.path(),
            "receipts.json",
            &serde_json::to_vec(&receipts).unwrap(),
        );
        let request = serde_json::json!({"version":"flowguard.cli-request/v1alpha1","invocation":{"repo_candidates":["repo"],"task_candidates":["task"],"worktree_id":"w","requirement_ids":["A"],"candidate_oid":b.binding().candidate_oid,"base_oid":b.binding().base_oid},"candidate":"candidate.json","frozen":"frozen.json","frozen_digest":frozen.digest(),"action":"commit","started_at":TIME,"finished_at":TIME,"now":NOW,"specialists":[{"scope":scope,"producer":u.envelope.producer,"producer_principals":["fixture-producer"],"approval_principals":BTreeMap::<String,Vec<String>>::new(),"evidence":{"envelope":"envelope.json","contract":"contract.json","facts":"facts.json","report":"report.json"}}]});
        Self {
            repo,
            root,
            authority,
            request,
        }
    }
    fn invoke(&self, fixture: bool, extra: &[&str]) -> Output {
        let request = serde_json::to_vec(&self.request).unwrap();
        write(self.root.path(), "request.json", &request);
        let pin = flowguard::digest(&request);
        let receipt = std::fs::read(self.authority.path().join("receipts.json")).unwrap();
        let receipt_pin = flowguard::digest(&receipt);
        let mut args = vec![
            "gate",
            "check",
            "--repo",
            self.repo.path().to_str().unwrap(),
            "--input-root",
            self.root.path().to_str().unwrap(),
            "--request",
            "request.json",
            "--request-digest",
            &pin,
            "--run-id",
            "cli-run",
        ];
        if fixture {
            args.extend([
                "--local-fixture-authority",
                "--fixture-authority-root",
                self.authority.path().to_str().unwrap(),
                "--fixture-authority",
                "receipts.json",
                "--fixture-authority-digest",
                &receipt_pin,
            ]);
        }
        args.extend(extra);
        cli(&args)
    }
}
#[test]
fn gate_cli_has_real_allow_block_review_and_default_closed_outcomes() {
    for (mode, code, decision) in [
        (guardengine::Enforcement::Advise, 0, "ALLOW"),
        (guardengine::Enforcement::Enforce, 2, "BLOCK"),
        (guardengine::Enforcement::Review, 3, "REQUIRE_APPROVAL"),
    ] {
        let fixture = GateFixture::new(mode);
        let before = std::fs::read(fixture.root.path().join("report.json")).unwrap();
        let out = fixture.invoke(true, &[]);
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(json["envelope"]["decision"], decision);
        assert_eq!(json["authority_profile"], "local_fixture");
        assert_eq!(
            before,
            std::fs::read(fixture.root.path().join("report.json")).unwrap()
        );
        let envelope = guardengine::integration::load_envelope_json(
            &serde_json::to_vec(&json["envelope"]).unwrap(),
            guardengine::integration::EvidenceProfile::EngineBacked,
        )
        .unwrap();
        guardengine::integration::verify_engine_artifacts(
            &envelope,
            json["artifacts"]["contract"].as_str().unwrap().as_bytes(),
            json["artifacts"]["facts"].as_str().unwrap().as_bytes(),
            json["artifacts"]["report"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        let out = fixture.invoke(false, &[]);
        assert_eq!(out.status.code(), Some(4));
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(json["envelope"]["runStatus"], "error");
        assert!(json["envelope"]["decision"].is_null());
    }
}
#[test]
fn bound_cancel_bad_evidence_and_missing_provider_are_closed() {
    let mut fixture = GateFixture::new(guardengine::Enforcement::Advise);
    let cancelled = fixture.invoke(true, &["--cancel"]);
    assert_eq!(cancelled.status.code(), Some(4));
    let json: serde_json::Value = serde_json::from_slice(&cancelled.stdout).unwrap();
    assert_eq!(json["envelope"]["runStatus"], "cancelled");
    assert!(json["envelope"]["decision"].is_null());
    write(fixture.root.path(), "envelope.json", b"malformed");
    let bad = fixture.invoke(true, &[]);
    assert_eq!(bad.status.code(), Some(4));
    let json: serde_json::Value = serde_json::from_slice(&bad.stdout).unwrap();
    assert_eq!(json["envelope"]["runStatus"], "error");
    assert!(json["envelope"]["decision"].is_null());
    fixture.request["specialists"][0]["evidence"] = serde_json::Value::Null;
    let missing = fixture.invoke(true, &[]);
    assert_eq!(missing.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(json["envelope"]["coverage"]["status"], "partial");
    fixture.request["invocation"]["repo_candidates"] = serde_json::json!(["one", "two"]);
    let bad = fixture.invoke(true, &[]);
    assert_eq!(bad.status.code(), Some(4));
    assert!(bad.stdout.is_empty());
}
#[test]
fn pins_unknown_request_versions_and_changed_candidate_fail_before_envelope() {
    let mut f = GateFixture::new(guardengine::Enforcement::Advise);
    f.request["version"] = "future".into();
    let out = f.invoke(true, &[]);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    f.request["version"] = "flowguard.cli-request/v1alpha1".into();
    f.request["frozen_digest"] = flowguard::digest(b"wrong pin").into();
    let out = f.invoke(true, &[]);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    f.request["invocation"]["candidate_oid"] = "0".repeat(40).into();
    let out = f.invoke(true, &[]);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let out = f.invoke(true, &["--report", "unused"]);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let request = serde_json::to_vec(&f.request).unwrap();
    write(f.root.path(), "request.json", &request);
    let wrong = flowguard::digest(b"wrong bytes");
    let out = cli(&[
        "gate",
        "check",
        "--repo",
        f.repo.path().to_str().unwrap(),
        "--input-root",
        f.root.path().to_str().unwrap(),
        "--request",
        "request.json",
        "--request-digest",
        &wrong,
        "--run-id",
        "wrong-pin",
    ]);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
}
#[test]
fn each_run_refreshes_authority_and_rejects_changed_artifact_bytes() {
    let f = GateFixture::new(guardengine::Enforcement::Advise);
    let first = f.invoke(true, &[]);
    assert_eq!(first.status.code(), Some(0));
    let path = f.authority.path().join("receipts.json");
    let mut records: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    records["producers"][0]["revoked"] = true.into();
    std::fs::write(&path, serde_json::to_vec(&records).unwrap()).unwrap();
    let second = f.invoke(true, &[]);
    assert_eq!(second.status.code(), Some(4));
    let out: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert!(out["envelope"]["decision"].is_null());
    assert_ne!(first.stdout, second.stdout);
    records["producers"][0]["revoked"] = false.into();
    std::fs::write(&path, serde_json::to_vec(&records).unwrap()).unwrap();
    let mut bytes = std::fs::read(f.root.path().join("report.json")).unwrap();
    bytes.push(b' ');
    write(f.root.path(), "report.json", &bytes);
    let out = f.invoke(true, &[]);
    assert_eq!(out.status.code(), Some(4));
    let out: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(out["envelope"]["decision"].is_null());
    assert_eq!(out["execution_authorized"], false);
}
#[test]
fn explicit_fixture_approval_uses_new_envelope_and_preserves_upstream_review() {
    let mut f = GateFixture::new(guardengine::Enforcement::Review);
    let original = std::fs::read(f.root.path().join("envelope.json")).unwrap();
    let report_before = std::fs::read(f.root.path().join("report.json")).unwrap();
    assert_eq!(f.invoke(true, &[]).status.code(), Some(3));
    let mut envelope = guardengine::integration::load_envelope_json(
        &original,
        guardengine::integration::EvidenceProfile::EngineBacked,
    )
    .unwrap();
    envelope.run_id = "new-specialist-approval-observation".into();
    envelope.approval_refs = vec!["fixture:approval".into()];
    let bytes = serde_json::to_vec(&envelope).unwrap();
    write(f.root.path(), "approved-envelope.json", &bytes);
    f.request["specialists"][0]["evidence"]["envelope"] = "approved-envelope.json".into();
    f.request["specialists"][0]["approval_principals"] =
        serde_json::json!({"review":["fixture-reviewer"]});
    let receipt_path = f.authority.path().join("receipts.json");
    let mut records: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
    records["producers"][0]["envelope_digest"] = flowguard::digest(&bytes).into();
    records["approvals"] = serde_json::json!([{"reference":"fixture:approval","principal":"fixture-reviewer","purpose":"review","action":"commit","binding":envelope.binding,"contract_digest":envelope.artifacts.contract.as_ref().unwrap().digest,"issued_at":NOW-1,"expires_at":NOW+100,"revoked":false}]);
    std::fs::write(&receipt_path, serde_json::to_vec(&records).unwrap()).unwrap();
    let out = f.invoke(true, &[]);
    assert_eq!(out.status.code(), Some(0));
    let out: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out["execution_authorized"], false);
    let domain_bytes = out["artifacts"]["domain"].as_str().unwrap().as_bytes();
    assert_eq!(
        flowguard::digest(domain_bytes),
        out["envelope"]["artifacts"]["domain"][0]["digest"]
    );
    flowguard::gate::load_gate_decision(domain_bytes).unwrap();
    assert_eq!(
        original,
        std::fs::read(f.root.path().join("envelope.json")).unwrap()
    );
    assert_eq!(
        report_before,
        std::fs::read(f.root.path().join("report.json")).unwrap()
    );
    assert_eq!(
        envelope.decision,
        Some(guardengine::Decision::RequireApproval)
    );
    records["approvals"][0]["revoked"] = true.into();
    std::fs::write(&receipt_path, serde_json::to_vec(&records).unwrap()).unwrap();
    let out = f.invoke(true, &[]);
    assert_eq!(out.status.code(), Some(4));
}
