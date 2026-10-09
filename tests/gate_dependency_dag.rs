//! Same-candidate SG -> specialists -> FG technical gate; no execution dependency.
#[path = "support/approval_bridge.rs"]
mod approval_bridge;
#[path = "support/dependency_plan.rs"]
mod dependency_plan;
#[path = "support/readonly_controller.rs"]
mod readonly_controller;
use approval_bridge::{Captured, NOW, PROVIDERS, TIME, request};
use flowguard::{
    context::ValidatedBinding, dependencies::StageGraph, evidence::ScopedSources, gate::*,
    obligations::FrozenObligations,
};
use guardengine::integration::{GuardRunEnvelope, eligibility::*};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};
struct Fixture {
    native: approval_bridge::Fixture,
    binding: ValidatedBinding,
    frozen: FrozenObligations,
    sources: ScopedSources,
    policies: BTreeMap<String, EligibilityPolicy>,
    scopes: BTreeMap<String, String>,
    graph: StageGraph,
}
impl Fixture {
    fn new() -> Self {
        let native = approval_bridge::Fixture::new();
        let plan =
            dependency_plan::ReadPlan::freeze(&native.binding, &native.frozen, &native.policies);
        assert_eq!(
            plan.graph.order().first().map(String::as_str),
            Some("specguard")
        );
        assert_eq!(plan.graph.nodes().len(), 6);
        Self {
            binding: native.binding.clone(),
            native,
            frozen: plan.frozen,
            sources: plan.sources,
            policies: plan.policies,
            scopes: plan.scopes,
            graph: plan.graph,
        }
    }
    fn pending(&self) -> PendingGate {
        prepare_scoped_gate(
            &self.binding,
            &self.frozen,
            &self.sources,
            self.policies.clone(),
            request(),
        )
        .unwrap()
    }
}
struct FixtureAuthority {
    native: RefCell<approval_bridge::FixtureAuthority>,
    revoked: Cell<bool>,
    unavailable: Cell<bool>,
}
impl FixtureAuthority {
    fn new() -> Self {
        Self {
            native: RefCell::new(approval_bridge::FixtureAuthority::new()),
            revoked: Cell::new(false),
            unavailable: Cell::new(false),
        }
    }
}
impl AuthorityProvider for FixtureAuthority {
    fn verify_producer(
        &self,
        e: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        let mut record = self.native.borrow().verify_producer(e, d)?;
        record.validity.revoked |= self.revoked.get();
        Ok(record)
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        let mut record = self.native.borrow().verify_approval(r)?;
        record.validity.revoked |= self.revoked.get();
        Ok(record)
    }
}
fn read_dependencies(fixture: &Fixture) -> (Vec<String>, Vec<Captured>) {
    let expected: std::collections::BTreeSet<_> = PROVIDERS.iter().copied().collect();
    assert_eq!(
        fixture
            .scopes
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    let mut completed = std::collections::BTreeSet::new();
    let mut names = Vec::new();
    let mut captures = Vec::new();
    for name in fixture.graph.order() {
        if name == "gate" {
            break;
        }
        let node = &fixture.graph.nodes()[name];
        assert!(node.dependencies.iter().all(|d| completed.contains(d)));
        captures.push(Captured::new(name)); // actual immutable artifact reads
        names.push(name.clone());
        completed.insert(name.clone());
    }
    assert_eq!(names.len(), 5);
    assert_eq!(names[0], "specguard");
    (names, captures)
}
#[test]
fn readonly_controller_refreshes_real_retained_producers_before_preview() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.graph.order().last().map(String::as_str),
        Some("gate")
    );
    let (read_order, captures) = read_dependencies(&fixture);
    let evidence: Vec<_> = captures
        .iter()
        .enumerate()
        .map(|(i, c)| c.evidence(&fixture.scopes[&read_order[i]]))
        .collect();
    let authority = FixtureAuthority::new();
    let pending = fixture.pending();
    let controller = readonly_controller::ReadOnlyController::freeze(
        fixture.binding.binding(),
        &pending,
        &authority,
    );
    let run = controller.run(pending, &evidence, NOW, TIME);
    assert_eq!(run.envelope().decision, Some(guardengine::Decision::Allow));
    let original = serde_json::to_vec(run.envelope()).unwrap();
    let original_report = run.artifacts().unwrap().report.to_vec();
    let approvals_before = controller
        .calls()
        .iter()
        .filter(|c| **c == "verify-approval")
        .count();
    assert!(approvals_before > 0);
    assert!(controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
    assert!(
        controller
            .calls()
            .iter()
            .filter(|c| **c == "verify-approval")
            .count()
            > approvals_before
    );
    authority.revoked.set(true);
    assert!(!controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
    authority.revoked.set(false);
    authority.native.borrow_mut().baseline_auth.revoked = true;
    assert!(!controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
    authority.native.borrow_mut().baseline_auth.revoked = false;
    authority
        .native
        .borrow_mut()
        .approval
        .as_mut()
        .unwrap()
        .validity
        .revoked = true;
    assert!(!controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
    authority
        .native
        .borrow_mut()
        .approval
        .as_mut()
        .unwrap()
        .validity
        .revoked = false;
    authority.unavailable.set(true);
    assert!(!controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
    authority.unavailable.set(false);
    assert!(controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
    assert!(!controller.preview(&run, &"f".repeat(40), NOW));
    assert!(!controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW + 60));
    assert_eq!(original, serde_json::to_vec(run.envelope()).unwrap());
    assert_eq!(original_report, run.artifacts().unwrap().report);
    assert!(controller.calls().iter().all(|call| {
        [
            "read-specialists",
            "technical-gate",
            "read-current-candidate",
            "refresh-eligibility",
            "verify-producer",
            "verify-approval",
        ]
        .contains(call)
    }));
    assert_eq!(
        readonly_controller::PROFILE,
        "flowguard.test.readonly-controller/v1"
    );
}

fn git_read(fixture: &Fixture, args: &[&str]) -> Vec<u8> {
    let out = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(fixture.native.repo_root.clone())
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(out.stdout.len() < 1024 * 1024);
    out.stdout
}
fn repository_state(fixture: &Fixture) -> String {
    let root = fixture.native.repo_root.clone();
    let files: Vec<_> = git_read(fixture, &["ls-files", "-z"])
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let name = String::from_utf8(p.to_vec()).unwrap();
            let bytes = std::fs::read(root.join(&name)).unwrap();
            (name, bytes)
        })
        .collect();
    flowguard::digest(
        &serde_json::to_vec(&(
            git_read(
                fixture,
                &["for-each-ref", "--format=%(refname) %(objectname)"],
            ),
            git_read(fixture, &["rev-parse", "HEAD"]),
            std::fs::read(root.join(".git/index")).unwrap(),
            files,
            git_read(
                fixture,
                &["status", "--porcelain=v1", "--untracked-files=all"],
            ),
        ))
        .unwrap(),
    )
}
#[test]
fn repository_advance_is_external_and_old_gate_never_authorizes_the_new_candidate() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.graph.order().last().map(String::as_str),
        Some("gate")
    );
    let (read_order, captures) = read_dependencies(&fixture);
    let evidence: Vec<_> = captures
        .iter()
        .enumerate()
        .map(|(i, c)| c.evidence(&fixture.scopes[&read_order[i]]))
        .collect();
    let authority = FixtureAuthority::new();
    let pending = fixture.pending();
    let controller = readonly_controller::ReadOnlyController::freeze(
        fixture.binding.binding(),
        &pending,
        &authority,
    );
    let before = repository_state(&fixture);
    let run = controller.run(pending, &evidence, NOW, TIME);
    let head = String::from_utf8(git_read(&fixture, &["rev-parse", "HEAD"])).unwrap();
    assert!(controller.preview(&run, head.trim(), NOW));
    assert_eq!(before, repository_state(&fixture));
    let original = serde_json::to_vec(run.envelope()).unwrap();
    // External fixture setup simulates a trusted controller receiving an updated
    // candidate. This command is never reachable through ReadOnlyController.
    let status = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(fixture.native.repo_root.clone())
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "new candidate after technical gate",
        ])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .unwrap();
    assert!(status.success());
    let new_head = String::from_utf8(git_read(&fixture, &["rev-parse", "HEAD"])).unwrap();
    assert_ne!(head, new_head);
    let after_external_change = repository_state(&fixture);
    assert!(!controller.preview(&run, new_head.trim(), NOW));
    assert_eq!(after_external_change, repository_state(&fixture));
    assert_eq!(original, serde_json::to_vec(run.envelope()).unwrap());
    assert!(
        !controller
            .calls()
            .iter()
            .any(|c| ["grant", "refs", "release", "merge-executor"].contains(c))
    );
    println!(
        "{}",
        serde_json::json!({"profile":readonly_controller::PROFILE,"oldCandidate":head.trim(),"newCandidate":new_head.trim(),"beforeTechnical":before,"afterExternalAdvance":after_external_change,"oldResultRejected":true,"topologicalReads":read_order,"calls":controller.calls()})
    );
}

#[test]
fn missing_dag_reader_cannot_be_replaced_by_executor_availability() {
    let fixture = Fixture::new();
    let (read_order, captures) = read_dependencies(&fixture);
    for omitted in 0..captures.len() {
        let evidence: Vec<_> = captures
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != omitted)
            .map(|(i, c)| c.evidence(&fixture.scopes[&read_order[i]]))
            .collect();
        let authority = FixtureAuthority::new();
        let pending = fixture.pending();
        let controller = readonly_controller::ReadOnlyController::freeze(
            fixture.binding.binding(),
            &pending,
            &authority,
        );
        let run = controller.run(pending, &evidence, NOW, TIME);
        assert_eq!(run.envelope().decision, Some(guardengine::Decision::Block));
        assert_eq!(
            run.envelope().coverage.status,
            guardengine::integration::CoverageStatus::Partial
        );
        assert!(!controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
        assert!(
            !controller
                .calls()
                .iter()
                .any(|c| ["grant", "refs", "release", "merge-executor"].contains(c))
        );
    }
}

#[test]
fn baseline_time_advance_closes_preview_even_when_review_approval_lives_longer() {
    let fixture = Fixture::new();
    let (read_order, captures) = read_dependencies(&fixture);
    let evidence: Vec<_> = captures
        .iter()
        .enumerate()
        .map(|(i, c)| c.evidence(&fixture.scopes[&read_order[i]]))
        .collect();
    let authority = FixtureAuthority::new();
    authority.native.borrow_mut().baseline_auth.expires_at = NOW + 1;
    authority
        .native
        .borrow_mut()
        .approval
        .as_mut()
        .unwrap()
        .validity
        .expires_at = i64::MAX;
    let pending = fixture.pending();
    let controller = readonly_controller::ReadOnlyController::freeze(
        fixture.binding.binding(),
        &pending,
        &authority,
    );
    let run = controller.run(pending, &evidence, NOW, TIME);
    assert_eq!(run.envelope().decision, Some(guardengine::Decision::Allow));
    let original = serde_json::to_vec(run.envelope()).unwrap();
    assert!(controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW));
    assert!(!controller.preview(&run, &fixture.binding.binding().candidate_oid, NOW + 2));
    assert_eq!(original, serde_json::to_vec(run.envelope()).unwrap());
}
