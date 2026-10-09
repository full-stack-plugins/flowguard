#![cfg(target_os = "linux")]
mod common;
use common::*;
use flowguard::{durable_run_store::DurableRunStore, gate::*};
use std::collections::BTreeMap;
fn plan(b: &flowguard::context::ValidatedBinding, u: &Upstream, id: &str) -> PendingGate {
    let f = frozen(b, u, false);
    let scope = obligation_scope(f.obligations().first().unwrap());
    prepare_gate(
        b,
        &f,
        BTreeMap::from([(scope, policy(u))]),
        GateRequest {
            run_id: id.into(),
            action: "commit".into(),
            started_at: TIME.into(),
        },
    )
    .unwrap()
}
#[test]
fn restart_preserves_registration_history_and_current_without_reset() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Review, false);
    let p = plan(&b, &u, "first");
    let directory = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let store = DurableRunStore::create(directory.path()).unwrap();
    let generation = store.advance(&p, 0).unwrap();
    let original = store.reserve(&p, "request", generation).unwrap();
    drop(store);
    let store = DurableRunStore::open(directory.path()).unwrap();
    assert_eq!(store.generation(&p).unwrap(), Some(generation));
    assert_eq!(store.reserve(&p, "request", generation).unwrap(), original);
    let run = p.cancel(TIME).unwrap();
    let bytes = serde_json::to_vec(run.envelope()).unwrap();
    store.append(&run).unwrap();
    store.publish("first", generation).unwrap();
    drop(store);
    let store = DurableRunStore::open(directory.path()).unwrap();
    let query = plan(&b, &u, "query");
    assert_eq!(store.current(&query).unwrap(), Some(bytes.clone()));
    assert_eq!(store.history(&query).unwrap(), vec![bytes]);
    assert!(DurableRunStore::create(directory.path()).is_err());
}
#[test]
fn absent_or_unsafe_storage_never_creates_an_empty_fallback() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    assert!(DurableRunStore::open(directory.path()).is_err());
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(DurableRunStore::create(directory.path()).is_err());
}
fn bind_head(
    path: &std::path::Path,
    task: &str,
    requirement: &str,
) -> flowguard::context::ValidatedBinding {
    let output = std::process::Command::new("git")
        .args(["-C", path.to_str().unwrap(), "rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let oid = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let repo = gitguard::Repository::discover(path, "repo").unwrap();
    let subject = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(oid.clone()))
        .unwrap();
    let scope = gitguard::scope::TaskScope::advisory(
        task,
        vec![requirement.into()],
        vec![b"a".to_vec()],
        &"a".repeat(64),
        None,
    )
    .unwrap();
    let candidate = repo
        .prepare_candidate(
            &subject,
            &scope,
            &gitguard::candidate::CandidateRequest {
                worktree_id: "w".into(),
                base_oid: oid.clone(),
                merge_group_id: None,
                members: vec![],
            },
        )
        .unwrap();
    flowguard::context::bind(
        &flowguard::context::InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec![task.into()],
            worktree_id: "w".into(),
            requirement_ids: vec![requirement.into()],
            candidate_oid: oid.clone(),
            base_oid: oid,
        },
        &repo,
        &candidate,
    )
    .unwrap()
}
fn finished(p: PendingGate, b: &flowguard::context::ValidatedBinding, u: &Upstream) -> GateRun {
    let f = frozen(b, u, false);
    let scope = obligation_scope(f.obligations().first().unwrap());
    p.evaluate(
        &[SpecialistEvidence {
            scope: &scope,
            envelope: &u.envelope,
            artifacts: u.artifacts(),
        }],
        &FixtureAuthority {
            approval: None,
            unavailable: false,
        },
        NOW,
        TIME,
    )
    .unwrap()
}
#[test]
fn real_head1_late_allow_cannot_replace_head2_block_or_requirement_b_after_reopen() {
    let (repo, b1) = binding();
    let u1 = upstream(&b1, guardengine::Enforcement::Advise, false);
    let old = plan(&b1, &u1, "A-head1");
    let directory = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let store = DurableRunStore::create(directory.path()).unwrap();
    let g1 = store.advance(&old, 0).unwrap();
    store.reserve(&old, "old-request", g1).unwrap();
    std::fs::write(repo.path().join("a"), "changed candidate\n").unwrap();
    for args in [
        vec!["add", "a"],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "new-head",
        ],
    ] {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo.path())
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
    let b2 = bind_head(repo.path(), "task", "A");
    assert_ne!(b1.binding().candidate_oid, b2.binding().candidate_oid);
    let bb = bind_head(repo.path(), "task-B", "B");
    let u2 = upstream(&b2, guardengine::Enforcement::Enforce, false);
    let ub = upstream(&bb, guardengine::Enforcement::Advise, false);
    let new = plan(&b2, &u2, "A-head2");
    let independent = plan(&bb, &ub, "B-head2");
    let g2 = store.advance(&new, g1).unwrap();
    let gb = store.advance(&independent, 0).unwrap();
    store.reserve(&new, "new-request", g2).unwrap();
    store.reserve(&independent, "b-request", gb).unwrap();
    let newrun = finished(new, &b2, &u2);
    let newbytes = serde_json::to_vec(newrun.envelope()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&newbytes).unwrap()["decision"],
        "BLOCK"
    );
    let brun = finished(independent, &bb, &ub);
    let bbytes = serde_json::to_vec(brun.envelope()).unwrap();
    store.append(&newrun).unwrap();
    store.publish("A-head2", g2).unwrap();
    store.append(&brun).unwrap();
    store.publish("B-head2", gb).unwrap();
    drop(store);
    let store = DurableRunStore::open(directory.path()).unwrap();
    let oldrun = finished(old, &b1, &u1);
    let oldbytes = serde_json::to_vec(oldrun.envelope()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&oldbytes).unwrap()["decision"],
        "ALLOW"
    );
    store.append(&oldrun).unwrap();
    assert!(store.publish("A-head1", g1).is_err());
    assert!(store.publish("A-head1", g2).is_err());
    drop(store);
    let store = DurableRunStore::open(directory.path()).unwrap();
    assert_eq!(
        store.current(&plan(&b2, &u2, "query-A")).unwrap(),
        Some(newbytes)
    );
    assert_eq!(
        store.current(&plan(&bb, &ub, "query-B")).unwrap(),
        Some(bbytes)
    );
    assert_eq!(store.current(&plan(&b1, &u1, "query-old")).unwrap(), None);
    assert_eq!(store.history(&plan(&b2, &u2, "query-A")).unwrap().len(), 2);
    assert_eq!(store.history(&plan(&bb, &ub, "query-B")).unwrap().len(), 1);
}
#[test]
fn symlink_hardlink_and_shared_file_permissions_reject() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for (name, case) in [
        ("run.lock", 0),
        ("run.lock", 1),
        ("state.json", 1),
        ("state.json", 2),
    ] {
        let dir = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        DurableRunStore::create(dir.path()).unwrap();
        let path = dir.path().join(name);
        match case {
            0 => {
                std::fs::remove_file(&path).unwrap();
                symlink("/dev/null", &path).unwrap();
            }
            1 => std::fs::hard_link(&path, dir.path().join("alias")).unwrap(),
            _ => std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap(),
        }
        assert!(DurableRunStore::open(dir.path()).is_err());
    }
}
