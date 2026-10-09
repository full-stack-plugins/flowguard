use flowguard::context::*;
use gitguard::{
    Repository, candidate::CandidateRequest, scope::TaskScope, subject::SubjectRequest,
};
use std::{path::Path, process::Command};
fn git(root: &Path, args: &[&str]) -> String {
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
#[test]
fn binds_actual_sha1_and_sha256_objects_and_rejects_ambiguity_and_drift() {
    for format in ["sha1", "sha256"] {
        let dir = tempfile::tempdir().unwrap();
        git(
            dir.path(),
            &["init", "-q", &format!("--object-format={format}")],
        );
        std::fs::write(dir.path().join("a"), "one").unwrap();
        git(dir.path(), &["add", "a"]);
        git(dir.path(), &["commit", "-qm", "fixture"]);
        let oid = git(dir.path(), &["rev-parse", "HEAD"]);
        let repo = Repository::discover(dir.path(), "repo").unwrap();
        let subject = repo
            .resolve_subject(SubjectRequest::Commit(oid.clone()))
            .unwrap();
        let scope = TaskScope::advisory(
            "task",
            vec!["A".into()],
            vec![b"a".to_vec()],
            &"a".repeat(64),
            None,
        )
        .unwrap();
        let snapshot = repo
            .prepare_candidate(
                &subject,
                &scope,
                &CandidateRequest {
                    worktree_id: "w".into(),
                    base_oid: oid.clone(),
                    merge_group_id: None,
                    members: vec![],
                },
            )
            .unwrap();
        let input = InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec!["task".into()],
            worktree_id: "w".into(),
            requirement_ids: vec!["A".into()],
            candidate_oid: oid.clone(),
            base_oid: oid.clone(),
        };
        let bound = bind(&input, &repo, &snapshot).unwrap();
        assert_eq!(bound.binding().candidate_oid, oid);
        assert_eq!(
            bound.binding().source_snapshot_digest,
            format!("sha256:{}", snapshot.source_snapshot_digest())
        );
        assert!(bound.is_advisory());
        for case in 0..5 {
            let mut bad = input.clone();
            match case {
                0 => bad.repo_candidates.push("other".into()),
                1 => bad.task_candidates.clear(),
                2 => bad.candidate_oid.clear(),
                3 => bad.requirement_ids = vec!["B".into()],
                _ => bad.base_oid = "0".repeat(oid.len()),
            };
            assert!(bind(&bad, &repo, &snapshot).is_err(), "{case}");
        }
        std::fs::write(dir.path().join("a"), "dirty").unwrap();
        assert!(bind(&input, &repo, &snapshot).is_err());
    }
}
