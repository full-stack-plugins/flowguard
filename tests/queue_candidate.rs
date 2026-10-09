mod common;
use flowguard::merge_queue::*;
use gitguard::{
    Repository,
    candidate::{CandidateRequest, CandidateSnapshot},
    scope::TaskScope,
    subject::SubjectRequest,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::Path,
    process::Command,
};
const NOW: i64 = common::NOW;
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
struct Fixture {
    dir: tempfile::TempDir,
    repo: Repository,
    base: String,
    next_base: String,
    a: String,
    b: String,
    m1: String,
    m2: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q"]);
        for f in ["a", "b"] {
            std::fs::write(p.join(f), "original\n").unwrap();
        }
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "base"]);
        let base = git(p, &["rev-parse", "HEAD"]);
        std::fs::write(p.join("a"), "feature a\n").unwrap();
        git(p, &["commit", "-qam", "a"]);
        let a = git(p, &["rev-parse", "HEAD"]);
        git(p, &["checkout", "--detach", &base]);
        std::fs::write(p.join("b"), "feature b\n").unwrap();
        git(p, &["commit", "-qam", "b"]);
        let b = git(p, &["rev-parse", "HEAD"]);
        let tree = git(p, &["merge-tree", "--write-tree", &a, &b]);
        let m1 = git(p, &["commit-tree", &tree, "-p", &a, "-p", &b, "-m", "M1"]);
        git(p, &["checkout", "--detach", &base]);
        std::fs::write(p.join("c"), "advanced base\n").unwrap();
        git(p, &["add", "c"]);
        git(p, &["commit", "-qm", "base2"]);
        let next_base = git(p, &["rev-parse", "HEAD"]);
        let tree = git(p, &["merge-tree", "--write-tree", &next_base, &m1]);
        let m2 = git(
            p,
            &[
                "commit-tree",
                &tree,
                "-p",
                &next_base,
                "-p",
                &m1,
                "-m",
                "M2",
            ],
        );
        git(p, &["checkout", "--detach", &m1]);
        let repo = Repository::discover(p, "repo").unwrap();
        Self {
            dir,
            repo,
            base,
            next_base,
            a,
            b,
            m1,
            m2,
        }
    }
    fn candidate(
        &self,
        oid: &str,
        base: &str,
        group: &str,
        members: Vec<String>,
    ) -> CandidateSnapshot {
        let subject = self
            .repo
            .resolve_subject(SubjectRequest::Commit(oid.into()))
            .unwrap();
        let scope = TaskScope::advisory(
            "task",
            vec!["A".into(), "B".into()],
            vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
            &"a".repeat(64),
            None,
        )
        .unwrap();
        self.repo
            .prepare_candidate(
                &subject,
                &scope,
                &CandidateRequest {
                    worktree_id: "w".into(),
                    base_oid: base.into(),
                    merge_group_id: Some(group.into()),
                    members,
                },
            )
            .unwrap()
    }
    fn m1(&self) -> CandidateSnapshot {
        self.candidate(
            &self.m1,
            &self.base,
            "M1",
            vec![self.a.clone(), self.b.clone()],
        )
    }
    fn m2(&self) -> CandidateSnapshot {
        self.candidate(
            &self.m2,
            &self.next_base,
            "M2",
            vec![self.b.clone(), self.a.clone()],
        )
    }
    fn event(&self, reference: &str, sequence: u64, c: &CandidateSnapshot) -> ControllerRecord {
        ControllerRecord {
            version: "flowguard.queue-event/v1".into(),
            principal: "fixture-controller".into(),
            queue_id: "queue".into(),
            repo_id: "repo".into(),
            task_id: "task".into(),
            worktree_id: "w".into(),
            reference: reference.into(),
            sequence,
            issued_at: NOW - 1,
            expires_at: NOW + 60,
            revoked: false,
            candidate_oid: c.candidate_oid().into(),
            base_oid: c.base_oid().into(),
            merge_group_id: c.merge_group_id().unwrap().into(),
            members: c
                .members()
                .iter()
                .map(|oid| EventMember {
                    oid: oid.clone(),
                    requirement_ids: vec![if oid == &self.a { "A" } else { "B" }.into()],
                })
                .collect(),
        }
    }
    fn state(&self) -> String {
        flowguard::digest(
            &serde_json::to_vec(&(
                git(
                    self.dir.path(),
                    &["for-each-ref", "--format=%(refname) %(objectname)"],
                ),
                git(self.dir.path(), &["rev-parse", "HEAD"]),
                std::fs::read(self.dir.path().join(".git/index")).unwrap(),
                ["a", "b", "c"].map(|p| std::fs::read(self.dir.path().join(p)).ok()),
                git(
                    self.dir.path(),
                    &["status", "--porcelain=v1", "--untracked-files=all"],
                ),
            ))
            .unwrap(),
        )
    }
}
struct Clock(Cell<i64>);
impl QueueClock for Clock {
    fn now(&self) -> Result<i64, QueueError> {
        Ok(self.0.get())
    }
}
struct Issuer {
    record: RefCell<ControllerRecord>,
    unavailable: Cell<bool>,
    profile: ControllerProfile,
}
impl Issuer {
    fn new(record: ControllerRecord) -> Self {
        Self {
            record: RefCell::new(record),
            unavailable: Cell::new(false),
            profile: ControllerProfile::Fixture,
        }
    }
}
impl ControllerEventPort for Issuer {
    fn profile(&self) -> ControllerProfile {
        self.profile
    }
    fn current(&self, reference: &str) -> Result<ControllerRecord, QueueError> {
        let r = self.record.borrow();
        if self.unavailable.get() || reference != r.reference {
            return Err(QueueError::Authority);
        }
        Ok(r.clone())
    }
}
fn config() -> QueueConfig {
    QueueConfig {
        principal: "fixture-controller".into(),
        queue_id: "queue".into(),
        repo_id: "repo".into(),
        task_id: "task".into(),
        worktree_id: "w".into(),
        policy_digest: "a".repeat(64),
        allowed_paths: vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
    }
}
fn session() -> QueueSession {
    ProtectedQueue::freeze(config()).unwrap().session()
}
#[test]
fn real_queue_recomposition_rejects_old_candidate_members_and_old_evidence_without_writes() {
    let f = Fixture::new();
    let c1 = f.m1();
    let clock = Clock(Cell::new(NOW));
    let issuer = Issuer::new(f.event("event1", 1, &c1));
    let mut session = session();
    let before = f.state();
    let old = session
        .observe("event1", &f.repo, &c1, &issuer, &clock)
        .unwrap();
    assert_eq!(old.binding().binding().candidate_oid, f.m1);
    assert_eq!(before, f.state());
    // External test setup advances the checkout; the adapter cannot perform this write.
    git(f.dir.path(), &["checkout", "--detach", &f.m2]);
    let c2 = f.m2();
    let before = f.state();
    *issuer.record.borrow_mut() = f.event("event2", 2, &c2);
    assert!(
        session
            .revalidate(&old, &f.repo, &c1, &issuer, &clock)
            .is_err()
    );
    assert!(
        session
            .observe("event2", &f.repo, &c1, &issuer, &clock)
            .is_err()
    );
    let current = session
        .observe("event2", &f.repo, &c2, &issuer, &clock)
        .unwrap();
    assert_eq!(current.binding().binding().candidate_oid, f.m2);
    assert_ne!(
        old.binding().domain_digest(),
        current.binding().domain_digest()
    );
    assert_eq!(current.binding().binding().requirement_ids, vec!["A", "B"]);
    assert!(
        session
            .revalidate(&current, &f.repo, &c2, &issuer, &clock)
            .is_ok()
    );
    let wrong_order = f.candidate(&f.m2, &f.next_base, "M2", vec![f.a.clone(), f.b.clone()]);
    assert!(
        session
            .observe("event2", &f.repo, &wrong_order, &issuer, &clock)
            .is_err()
    );
    let head = f.candidate(&f.a, &f.next_base, "M2", vec![f.b.clone(), f.a.clone()]);
    assert!(
        session
            .observe("event2", &f.repo, &head, &issuer, &clock)
            .is_err()
    );
    use flowguard::gate::*;
    let fresh = common::upstream(current.binding(), guardengine::Enforcement::Advise, false);
    let frozen = common::frozen(current.binding(), &fresh, false);
    let scope = obligation_scope(frozen.obligations().first().unwrap());
    for prior in [old.binding(), current.binding()] {
        let u = common::upstream(prior, guardengine::Enforcement::Advise, false);
        let pending = prepare_gate(
            current.binding(),
            &frozen,
            BTreeMap::from([(scope.clone(), common::policy(&fresh))]),
            GateRequest {
                run_id: "queue-evidence".into(),
                action: "commit".into(),
                started_at: common::TIME.into(),
            },
        )
        .unwrap();
        let run = pending
            .evaluate(
                &[SpecialistEvidence {
                    scope: &scope,
                    envelope: &u.envelope,
                    artifacts: u.artifacts(),
                }],
                &common::FixtureAuthority {
                    approval: None,
                    unavailable: false,
                },
                NOW,
                common::TIME,
            )
            .unwrap();
        if prior.binding().candidate_oid == f.m2 {
            assert_eq!(run.envelope().decision, Some(guardengine::Decision::Allow));
        } else {
            assert_eq!(run.envelope().decision, None);
            assert_eq!(
                run.envelope().run_status,
                guardengine::integration::RunStatus::Error
            );
        }
    }
    assert_eq!(before, f.state());
}
#[test]
fn issuer_identity_replay_expiry_revoke_clock_and_failed_new_event_are_closed() {
    let f = Fixture::new();
    let c = f.m1();
    let clock = Clock(Cell::new(NOW));
    let original = f.event("event", 1, &c);
    let issuer = Issuer::new(original.clone());
    for case in 0..9 {
        let mut r = original.clone();
        match case {
            0 => r.principal = "forged".into(),
            1 => r.queue_id = "other".into(),
            2 => r.repo_id = "other".into(),
            3 => r.reference = "other".into(),
            4 => r.revoked = true,
            5 => r.expires_at = NOW,
            6 => r.issued_at = NOW + 1,
            7 => r.members[0].requirement_ids = vec!["wrong".into()],
            _ => r.sequence = 0,
        }
        *issuer.record.borrow_mut() = r;
        assert!(
            session()
                .observe("event", &f.repo, &c, &issuer, &clock)
                .is_err(),
            "{case}"
        );
    }
    *issuer.record.borrow_mut() = original.clone();
    let mut s = session();
    let old = s.observe("event", &f.repo, &c, &issuer, &clock).unwrap();
    let mut newer = original.clone();
    newer.sequence = 2;
    newer.expires_at = NOW;
    *issuer.record.borrow_mut() = newer;
    assert!(s.observe("event", &f.repo, &c, &issuer, &clock).is_err());
    *issuer.record.borrow_mut() = original.clone();
    assert!(s.revalidate(&old, &f.repo, &c, &issuer, &clock).is_err());
    let mut s = session();
    s.observe("event", &f.repo, &c, &issuer, &clock).unwrap();
    clock.0.set(NOW - 1);
    assert!(s.observe("event", &f.repo, &c, &issuer, &clock).is_err());
    clock.0.set(NOW + 2);
    issuer.unavailable.set(true);
    assert!(s.observe("event", &f.repo, &c, &issuer, &clock).is_err());
    issuer.unavailable.set(false);
    clock.0.set(NOW + 1);
    assert!(s.observe("event", &f.repo, &c, &issuer, &clock).is_err());
}
#[test]
fn queue_profile_and_input_budgets_reject_before_candidate_reads() {
    let mut c = config();
    c.principal.clear();
    assert!(ProtectedQueue::freeze(c).is_err());
    let mut c = config();
    c.allowed_paths = vec![b"../escape".to_vec()];
    assert!(ProtectedQueue::freeze(c).is_err());
    let f = Fixture::new();
    let candidate = f.m1();
    let clock = Clock(Cell::new(NOW));
    let mut issuer = Issuer::new(f.event("event", 1, &candidate));
    issuer.profile = ControllerProfile::Unsupported;
    assert!(
        session()
            .observe("event", &f.repo, &candidate, &issuer, &clock)
            .is_err()
    );
    issuer.profile = ControllerProfile::Fixture;
    issuer.record.borrow_mut().members[0].requirement_ids = vec!["x".repeat(1024 * 1024)];
    assert!(
        session()
            .observe("event", &f.repo, &candidate, &issuer, &clock)
            .is_err()
    );
}

#[test]
fn equivocated_sequence_and_failed_new_candidate_cannot_revive_an_old_event() {
    let f = Fixture::new();
    let candidate = f.m1();
    let clock = Clock(Cell::new(NOW));
    let original = f.event("event", 1, &candidate);
    let issuer = Issuer::new(original.clone());
    let mut s = session();
    let old = s
        .observe("event", &f.repo, &candidate, &issuer, &clock)
        .unwrap();
    issuer.record.borrow_mut().revoked = true;
    assert!(
        s.revalidate(&old, &f.repo, &candidate, &issuer, &clock)
            .is_err()
    );
    *issuer.record.borrow_mut() = original.clone();
    assert!(
        s.revalidate(&old, &f.repo, &candidate, &issuer, &clock)
            .is_err()
    );
    let mut s = session();
    s.observe("event", &f.repo, &candidate, &issuer, &clock)
        .unwrap();
    let mut new = original.clone();
    new.sequence = 2;
    new.candidate_oid = f.m2.clone();
    *issuer.record.borrow_mut() = new;
    assert!(
        s.observe("event", &f.repo, &candidate, &issuer, &clock)
            .is_err()
    );
    *issuer.record.borrow_mut() = original;
    assert!(
        s.revalidate(&old, &f.repo, &candidate, &issuer, &clock)
            .is_err()
    );
}
