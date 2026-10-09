mod common;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
struct Measured;
static BIGGEST: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Measured {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        BIGGEST.fetch_max(l.size(), Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        BIGGEST.fetch_max(n, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOC: Measured = Measured;

#[test]
fn oversized_candidate_is_rejected_without_encoding_allocation() {
    use flowguard::{baseline::*, context::*};
    use std::collections::BTreeSet;
    let (dir, b) = common::binding();
    let repo = gitguard::Repository::discover(dir.path(), "repo").unwrap();
    let oid = &b.binding().candidate_oid;
    let subject = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(oid.clone()))
        .unwrap();
    let scope = gitguard::scope::TaskScope::advisory(
        "task",
        vec!["A".into()],
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
    let reference = BaselineRef {
        version: "flowguard.workflow/v1".into(),
        repo_id: "repo".into(),
        stage: "02-architecture".into(),
        revision: oid.clone(),
        content_digest: flowguard::digest(b"source"),
        policy_digest: flowguard::digest(b"policy"),
        approval_ref: "approval".into(),
        requirements: BTreeSet::from(["A".into()]),
    };
    for field in ["task_id", "schema_version", "object_format"] {
        let mut raw = serde_json::to_value(&candidate).unwrap();
        raw[field] = "x".repeat(17 * 1024 * 1024).into();
        let oversized: gitguard::candidate::CandidateSnapshot =
            serde_json::from_value(raw).unwrap();
        let input = InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec![oversized.task_id().into()],
            worktree_id: "w".into(),
            requirement_ids: vec!["A".into()],
            candidate_oid: oid.clone(),
            base_oid: oid.clone(),
        };
        BIGGEST.store(0, Ordering::SeqCst);
        let result = FrozenBaseline::from_candidate(&repo, &oversized, &reference);
        let largest = BIGGEST.load(Ordering::SeqCst);
        assert_eq!(result.err(), Some(InheritanceError::InvalidParent));
        assert!(largest < 4096, "{field} allocated {largest}");
        BIGGEST.store(0, Ordering::SeqCst);
        let result = bind(&input, &repo, &oversized);
        let largest = BIGGEST.load(Ordering::SeqCst);
        assert_eq!(result.err(), Some(PreBindingDiagnostic::InputBudget));
        assert!(largest < 4096, "binding {field} allocated {largest}");
        eprintln!("rejected {field}; binding largest allocation={largest}");
    }
}
