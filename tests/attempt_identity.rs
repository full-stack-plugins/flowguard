mod common;
use common::*;
use flowguard::{gate::*, run_store::*};
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
fn idempotency_is_separate_from_retry_run_identity() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Review, false);
    let p = plan(&b, &u, "first");
    let store = MemoryRunStore::default();
    let generation = store.advance(&p, 0).unwrap();
    let first = store.reserve(&p, "request", generation).unwrap();
    let duplicate = store
        .reserve(&plan(&b, &u, "not-executed"), "request", generation)
        .unwrap();
    assert_eq!(first, duplicate);
    let retry = store
        .reserve(&plan(&b, &u, "retry"), "retry-key", generation)
        .unwrap();
    assert_ne!(retry.run_id, first.run_id);
    assert!(store.reserve(&p, "different-key", generation).is_err());
    let changed = upstream(&b, guardengine::Enforcement::Enforce, false);
    assert!(
        store
            .reserve(&plan(&b, &changed, "changed"), "request", generation)
            .is_err()
    );
}
#[test]
fn append_keeps_original_bytes_and_rejects_overwrite_or_unbound_envelope() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Review, false);
    let p = plan(&b, &u, "first");
    let store = MemoryRunStore::default();
    let g = store.advance(&p, 0).unwrap();
    store.reserve(&p, "req", g).unwrap();
    let run = p.cancel(TIME).unwrap();
    let bytes = serde_json::to_vec(run.envelope()).unwrap();
    store.append(&run).unwrap();
    store.append(&run).unwrap();
    let changed = plan(&b, &u, "first")
        .cancel("2026-10-09T00:00:01Z")
        .unwrap();
    assert!(store.append(&changed).is_err());
    assert_eq!(store.history(&plan(&b, &u, "query")).unwrap(), vec![bytes]);
    let unregistered = plan(&b, &u, "absent").cancel(TIME).unwrap();
    assert!(store.append(&unregistered).is_err());
    assert_eq!(store.history(&plan(&b, &u, "query")).unwrap().len(), 1);
}
#[test]
fn late_results_stay_history_and_cas_never_rolls_back() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let old = plan(&b, &u, "old");
    let store = MemoryRunStore::default();
    let g = store.advance(&old, 0).unwrap();
    store.reserve(&old, "old", g).unwrap();
    let newer_u = upstream(&b, guardengine::Enforcement::Enforce, false);
    let newer = plan(&b, &newer_u, "new");
    let g2 = store.advance(&newer, g).unwrap();
    store.reserve(&newer, "new", g2).unwrap();
    let finish = |pending: PendingGate, upstream: &Upstream| {
        let frozen = frozen(&b, upstream, false);
        let scope = obligation_scope(frozen.obligations().first().unwrap());
        pending
            .evaluate(
                &[SpecialistEvidence {
                    scope: &scope,
                    envelope: &upstream.envelope,
                    artifacts: upstream.artifacts(),
                }],
                &FixtureAuthority {
                    approval: None,
                    unavailable: false,
                },
                NOW,
                TIME,
            )
            .unwrap()
    };
    let newrun = finish(newer, &newer_u);
    assert_eq!(
        newrun.envelope().decision,
        Some(guardengine::Decision::Block)
    );
    let newbytes = serde_json::to_vec(newrun.envelope()).unwrap();
    store.append(&newrun).unwrap();
    store.publish("new", g2).unwrap();
    let oldrun = finish(old, &u);
    assert_eq!(
        oldrun.envelope().decision,
        Some(guardengine::Decision::Allow)
    );
    store.append(&oldrun).unwrap();
    assert_eq!(store.publish("old", g), Err(StoreError::Stale));
    assert_eq!(store.publish("old", g2), Err(StoreError::Stale));
    assert_eq!(
        store.current(&plan(&b, &newer_u, "q")).unwrap(),
        Some(newbytes)
    );
    assert_eq!(store.current(&plan(&b, &u, "q")).unwrap(), None);
    assert_eq!(store.history(&plan(&b, &u, "q")).unwrap().len(), 2);
}
#[test]
fn racing_generation_advances_have_one_winner() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Review, false);
    let store = std::sync::Arc::new(MemoryRunStore::default());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|i| {
            let p = plan(&b, &u, &format!("run{i}"));
            let store = store.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.advance(&p, 0)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results.contains(&Err(StoreError::Stale)));
}
#[test]
fn concurrent_publication_is_single_assignment_with_immutable_history() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Review, false);
    let store = std::sync::Arc::new(MemoryRunStore::default());
    let first = plan(&b, &u, "one");
    let generation = store.advance(&first, 0).unwrap();
    for id in ["one", "two"] {
        let p = plan(&b, &u, id);
        store.reserve(&p, id, generation).unwrap();
        let run = p.cancel(TIME).unwrap();
        store.append(&run).unwrap();
    }
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let joins: Vec<_> = ["one", "two"]
        .into_iter()
        .map(|id| {
            let store = store.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.publish(id, generation)
            })
        })
        .collect();
    let results: Vec<_> = joins.into_iter().map(|j| j.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results.contains(&Err(StoreError::Conflict)));
    assert_eq!(store.history(&first).unwrap().len(), 2);
    assert!(store.current(&first).unwrap().is_some());
    store.advance(&first, generation).unwrap();
    assert!(store.current(&first).unwrap().is_none());
    assert_eq!(store.history(&first).unwrap().len(), 2);
}
#[test]
fn changed_authority_policy_conflicts_even_when_required_scopes_match() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Review, false);
    let p = plan(&b, &u, "first");
    let store = MemoryRunStore::default();
    let g = store.advance(&p, 0).unwrap();
    store.reserve(&p, "key", g).unwrap();
    let f = frozen(&b, &u, false);
    let scope = obligation_scope(f.obligations().first().unwrap());
    let mut policy = policy(&u);
    policy.producer_principals.insert("other-controller".into());
    let other = prepare_gate(
        &b,
        &f,
        BTreeMap::from([(scope, policy)]),
        GateRequest {
            run_id: "other".into(),
            action: "commit".into(),
            started_at: TIME.into(),
        },
    )
    .unwrap();
    assert_eq!(p.required_scopes(), other.required_scopes());
    assert_eq!(store.reserve(&other, "key", g), Err(StoreError::Conflict));
}
#[test]
fn reused_worktree_path_cannot_share_a_different_requirement_pointer() {
    let (dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Review, false);
    let repo = gitguard::Repository::discover(dir.path(), "repo").unwrap();
    let oid = b.binding().candidate_oid.clone();
    let scope = gitguard::scope::TaskScope::advisory(
        "task-B",
        vec!["B".into()],
        vec![b"a".to_vec()],
        &"a".repeat(64),
        None,
    )
    .unwrap();
    let subject = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(oid.clone()))
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
    let other = flowguard::context::bind(
        &flowguard::context::InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec!["task-B".into()],
            worktree_id: "w".into(),
            requirement_ids: vec!["B".into()],
            candidate_oid: oid.clone(),
            base_oid: oid,
        },
        &repo,
        &candidate,
    )
    .unwrap();
    let ub = upstream(&other, guardengine::Enforcement::Review, false);
    let a = plan(&b, &u, "A-run");
    let original_query = plan(&b, &u, "A-query");
    let b = plan(&other, &ub, "B-run");
    let store = MemoryRunStore::default();
    let ga = store.advance(&a, 0).unwrap();
    let gb = store.advance(&b, 0).unwrap();
    store.reserve(&a, "same-key", ga).unwrap();
    store.reserve(&b, "same-key", gb).unwrap();
    let arun = a.cancel(TIME).unwrap();
    let bytes = serde_json::to_vec(arun.envelope()).unwrap();
    store.append(&arun).unwrap();
    store.publish("A-run", ga).unwrap();
    assert_eq!(store.current(&b).unwrap(), None);
    assert!(store.history(&b).unwrap().is_empty());
    let brun = b.cancel(TIME).unwrap();
    let bbytes = serde_json::to_vec(brun.envelope()).unwrap();
    store.append(&brun).unwrap();
    store.publish("B-run", gb).unwrap();
    assert_eq!(
        store.current(&plan(&other, &ub, "query")).unwrap(),
        Some(bbytes)
    );
    assert_eq!(store.current(&original_query).unwrap(), Some(bytes));
}
