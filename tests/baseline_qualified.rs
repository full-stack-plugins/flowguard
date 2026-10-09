mod common;
use flowguard::{approvals::*, baseline::*, context::*};
use std::{collections::BTreeSet, process::Command};
struct Fixture {
    directory: tempfile::TempDir,
    repo: gitguard::Repository,
    parent: gitguard::candidate::CandidateSnapshot,
    oid: String,
}
fn git(root: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn candidate(
    repo: &gitguard::Repository,
    oid: &str,
    task: &str,
    requirements: Vec<String>,
) -> gitguard::candidate::CandidateSnapshot {
    let subject = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(oid.into()))
        .unwrap();
    let scope = gitguard::scope::TaskScope::advisory(
        task,
        requirements,
        vec![b"docs".to_vec()],
        &"a".repeat(64),
        None,
    )
    .unwrap();
    repo.prepare_candidate(
        &subject,
        &scope,
        &gitguard::candidate::CandidateRequest {
            worktree_id: task.into(),
            base_oid: oid.into(),
            merge_group_id: None,
            members: vec![],
        },
    )
    .unwrap()
}
fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    git(directory.path(), &["init", "-q"]);
    std::fs::create_dir_all(directory.path().join("docs/project")).unwrap();
    for stage in ["02-architecture", "07-standards"] {
        std::fs::write(
            directory.path().join(format!("docs/project/{stage}.md")),
            stage,
        )
        .unwrap();
    }
    git(directory.path(), &["add", "."]);
    git(directory.path(), &["commit", "-qm", "parent"]);
    let oid = git(directory.path(), &["rev-parse", "HEAD"]);
    let repo = gitguard::Repository::discover(directory.path(), "repo").unwrap();
    let parent = candidate(&repo, &oid, "parent", vec!["A".into(), "B".into()]);
    Fixture {
        directory,
        repo,
        parent,
        oid,
    }
}
fn child(f: &Fixture, id: &str) -> ValidatedBinding {
    let c = candidate(&f.repo, &f.oid, id, vec![id.into()]);
    bind(
        &InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec![id.into()],
            worktree_id: id.into(),
            requirement_ids: vec![id.into()],
            candidate_oid: f.oid.clone(),
            base_oid: f.oid.clone(),
        },
        &f.repo,
        &c,
    )
    .unwrap()
}
fn reference(f: &Fixture, stage: &str) -> BaselineRef {
    BaselineRef {
        version: "flowguard.workflow/v1".into(),
        repo_id: "repo".into(),
        stage: stage.into(),
        revision: f.oid.clone(),
        content_digest: flowguard::digest(stage.as_bytes()),
        policy_digest: flowguard::digest(b"controller baseline policy"),
        approval_ref: "baseline-approval".into(),
        requirements: BTreeSet::from(["A".into(), "B".into()]),
    }
}
struct Controller(Option<ApprovalRecord>);
impl ApprovalProvider for Controller {
    fn fetch(&self, _: &str) -> Result<Option<ApprovalRecord>, ProviderError> {
        Ok(self.0.clone())
    }
}
fn issuer(parent: &FrozenBaseline, r: &BaselineRef) -> Controller {
    Controller(Some(ApprovalRecord {
        reference: r.approval_ref.clone(),
        issuer: "fixture-controller".into(),
        role: "baseline-owner".into(),
        action: "baseline.inherit.local-controller".into(),
        repo_id: r.repo_id.clone(),
        requirements: r.requirements.clone(),
        target_digest: parent.digest().into(),
        policy_digest: r.policy_digest.clone(),
        baseline_revision: r.revision.clone(),
        issued_at: 1,
        expires_at: 100,
        revoked: false,
    }))
}
#[test]
fn actual_shared_02_07_parents_create_separate_child_edges_and_refresh_approval() {
    let f = fixture();
    let a = child(&f, "A");
    let b = child(&f, "B");
    for stage in ["02-architecture", "07-standards"] {
        let r = reference(&f, stage);
        let parent = FrozenBaseline::from_candidate(&f.repo, &f.parent, &r).unwrap();
        let mut controller = issuer(&parent, &r);
        let ea = parent
            .inherit(&a, "fixture-controller", "baseline-owner", &controller, 2)
            .unwrap();
        let eb = parent
            .inherit(&b, "fixture-controller", "baseline-owner", &controller, 2)
            .unwrap();
        ea.consume(&parent, &f.repo, &f.parent, &a, &controller, 2)
            .unwrap();
        eb.consume(&parent, &f.repo, &f.parent, &b, &controller, 2)
            .unwrap();
        assert!(
            ea.consume(&parent, &f.repo, &f.parent, &b, &controller, 2)
                .is_err()
        );
        controller.0.as_mut().unwrap().revoked = true;
        assert!(
            ea.consume(&parent, &f.repo, &f.parent, &a, &controller, 2)
                .is_err()
        );
    }
}

#[test]
fn actual_parent_rejects_latest_wrong_source_version_scope_and_self_reported_accepted() {
    let f = fixture();
    let original = reference(&f, "02-architecture");
    for case in 0..7 {
        let mut r = original.clone();
        match case {
            0 => r.revision = "latest".into(),
            1 => r.content_digest = flowguard::digest(b"accepted"),
            2 => r.version = "future".into(),
            3 => r.repo_id = "other".into(),
            4 => r.stage = "09-docs".into(),
            5 => {
                r.requirements.insert("C".into());
            }
            _ => r.approval_ref.clear(),
        }
        assert!(
            FrozenBaseline::from_candidate(&f.repo, &f.parent, &r).is_err(),
            "case {case}"
        );
    }
    let mut json = serde_json::to_value(original).unwrap();
    json["accepted"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BaselineRef>(json).is_err());
}

#[test]
fn parent_replacement_missing_approval_and_scope_shrink_do_not_rebind_existing_edges() {
    let f = fixture();
    let a = child(&f, "A");
    let r = reference(&f, "02-architecture");
    let parent = FrozenBaseline::from_candidate(&f.repo, &f.parent, &r).unwrap();
    let mut c = issuer(&parent, &r);
    let edge = parent
        .inherit(&a, "fixture-controller", "baseline-owner", &c, 2)
        .unwrap();
    let mut changed = r.clone();
    changed.policy_digest = flowguard::digest(b"new policy");
    let replacement = FrozenBaseline::from_candidate(&f.repo, &f.parent, &changed).unwrap();
    assert!(
        edge.consume(&replacement, &f.repo, &f.parent, &a, &c, 2)
            .is_err()
    );
    c.0.as_mut().unwrap().requirements = BTreeSet::from(["B".into()]);
    assert!(
        edge.consume(&parent, &f.repo, &f.parent, &a, &c, 2)
            .is_err()
    );
    c.0 = None;
    assert!(
        edge.consume(&parent, &f.repo, &f.parent, &a, &c, 2)
            .is_err()
    );
}

#[test]
fn new_parent_commit_requires_new_edge_and_missing_parent_objects_fail_closed() {
    let f = fixture();
    let a = child(&f, "A");
    let r = reference(&f, "02-architecture");
    let parent = FrozenBaseline::from_candidate(&f.repo, &f.parent, &r).unwrap();
    let c = issuer(&parent, &r);
    let edge = parent
        .inherit(&a, "fixture-controller", "baseline-owner", &c, 2)
        .unwrap();
    std::fs::write(
        f.directory.path().join("docs/project/02-architecture.md"),
        "changed parent bytes",
    )
    .unwrap();
    git(f.directory.path(), &["add", "."]);
    git(f.directory.path(), &["commit", "-qm", "parent-v2"]);
    let new_oid = git(f.directory.path(), &["rev-parse", "HEAD"]);
    assert_ne!(new_oid, f.oid);
    let current_repo = gitguard::Repository::discover(f.directory.path(), "repo").unwrap();
    let current_candidate = candidate(
        &current_repo,
        &new_oid,
        "parent",
        vec!["A".into(), "B".into()],
    );
    let mut new_ref = r.clone();
    new_ref.revision = new_oid;
    new_ref.content_digest = flowguard::digest(b"changed parent bytes");
    let new_parent =
        FrozenBaseline::from_candidate(&current_repo, &current_candidate, &new_ref).unwrap();
    assert_ne!(new_parent.digest(), parent.digest());
    assert!(
        edge.consume(&new_parent, &current_repo, &current_candidate, &a, &c, 2)
            .is_err()
    );
    // The immutable old parent remains independently readable; latest is not followed.
    edge.consume(&parent, &current_repo, &f.parent, &a, &c, 2)
        .unwrap();
    let unrelated = tempfile::tempdir().unwrap();
    git(unrelated.path(), &["init", "-q"]);
    std::fs::write(unrelated.path().join("different"), "unrelated object store").unwrap();
    git(unrelated.path(), &["add", "."]);
    git(unrelated.path(), &["commit", "-qm", "unrelated"]);
    let inaccessible = gitguard::Repository::discover(unrelated.path(), "repo").unwrap();
    assert!(
        edge.consume(&parent, &inaccessible, &f.parent, &a, &c, 2)
            .is_err()
    );
}

#[test]
fn approval_purpose_scope_candidate_digest_and_provider_outage_are_not_inheritance() {
    let f = fixture();
    let a = child(&f, "A");
    let r = reference(&f, "07-standards");
    let parent = FrozenBaseline::from_candidate(&f.repo, &f.parent, &r).unwrap();
    let mut c = issuer(&parent, &r);
    let original = c.0.clone();
    for case in 0..7 {
        c.0 = original.clone();
        let record = c.0.as_mut().unwrap();
        match case {
            0 => record.action = "accepted".into(),
            1 => record.target_digest = r.content_digest.clone(),
            2 => record.baseline_revision = "latest".into(),
            3 => record.policy_digest = flowguard::digest(b"other"),
            4 => record.expires_at = 2,
            5 => record.issuer = "other".into(),
            _ => record.role = "other".into(),
        }
        assert!(
            parent
                .inherit(&a, "fixture-controller", "baseline-owner", &c, 2)
                .is_err()
        );
    }
    struct Down;
    impl ApprovalProvider for Down {
        fn fetch(&self, _: &str) -> Result<Option<ApprovalRecord>, ProviderError> {
            Err(ProviderError::Unavailable)
        }
    }
    assert!(matches!(
        parent.inherit(&a, "fixture-controller", "baseline-owner", &Down, 2),
        Err(InheritanceError::Unavailable)
    ));
}

#[test]
fn review_oversized_parent_candidate_is_rejected_before_digest_expansion() {
    let f = fixture();
    let r = reference(&f, "02-architecture");
    let mut raw = serde_json::to_value(&f.parent).unwrap();
    raw["task_id"] = serde_json::json!("x".repeat(17 * 1024 * 1024));
    let candidate: gitguard::candidate::CandidateSnapshot = serde_json::from_value(raw).unwrap();
    assert!(
        FrozenBaseline::from_candidate(&f.repo, &candidate, &r).is_err(),
        "oversized full parent candidate was accepted and hashed without admission"
    );
}

#[test]
fn parent_candidate_admission_bounds_all_metadata_paths_and_fanout() {
    let f = fixture();
    let r = reference(&f, "02-architecture");
    let parent = FrozenBaseline::from_candidate(&f.repo, &f.parent, &r).unwrap();
    let a = child(&f, "A");
    let c = issuer(&parent, &r);
    let edge = parent
        .inherit(&a, "fixture-controller", "baseline-owner", &c, 2)
        .unwrap();
    for case in 0..9 {
        let mut raw = serde_json::to_value(&f.parent).unwrap();
        match case {
            0 => raw["task_id"] = "x".repeat(257).into(),
            1 => raw["worktree_id"] = "x".repeat(257).into(),
            2 => raw["merge_group_id"] = "x".repeat(257).into(),
            3 => raw["requirement_ids"] = serde_json::json!(["x".repeat(257)]),
            4 => raw["allowed_paths"] = serde_json::json!([vec![b'x'; 4097]]),
            5 => {
                raw["allowed_paths"] = serde_json::json!(
                    (0..257)
                        .map(|i| format!("docs/{i:04}").into_bytes())
                        .collect::<Vec<_>>()
                )
            }
            6 => raw["members"] = serde_json::json!(vec![f.oid.clone(); 65]),
            7 => {
                raw["allowed_paths"] = serde_json::json!(
                    (0..256)
                        .map(|i| format!("docs/{i:04}/{}", "x".repeat(256)).into_bytes())
                        .collect::<Vec<_>>()
                )
            }
            _ => {
                raw["requirement_ids"] =
                    serde_json::json!((0..4097).map(|i| format!("R{i:04}")).collect::<Vec<_>>())
            }
        }
        let candidate: gitguard::candidate::CandidateSnapshot =
            serde_json::from_value(raw).unwrap();
        assert_eq!(
            FrozenBaseline::from_candidate(&f.repo, &candidate, &r).err(),
            Some(InheritanceError::InvalidParent),
            "case {case}"
        );
        assert_eq!(
            edge.consume(&parent, &f.repo, &candidate, &a, &c, 2),
            Err(InheritanceError::InvalidParent),
            "case {case}"
        );
        let input = InvocationInput {
            repo_candidates: vec![candidate.repo_id().into()],
            task_candidates: vec![candidate.task_id().into()],
            worktree_id: candidate.worktree_id().into(),
            requirement_ids: candidate.requirement_ids().to_vec(),
            candidate_oid: candidate.candidate_oid().into(),
            base_oid: candidate.base_oid().into(),
        };
        assert!(
            bind(&input, &f.repo, &candidate).is_err(),
            "binding case {case}"
        );
    }
}
