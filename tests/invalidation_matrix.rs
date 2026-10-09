mod common;
use common::*;
use flowguard::{dependencies::*, gate::*, invalidation::*, run_store::*};
use std::collections::{BTreeMap, BTreeSet};
fn graph(extra: bool) -> StageGraph {
    build_graph(
        vec![
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "a".into(),
                stage: "01-requirements".into(),
                owner: "feature-a".into(),
                source_digest: flowguard::digest(b"a"),
                dependencies: BTreeSet::new(),
            },
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "b".into(),
                stage: "03-solution".into(),
                owner: "feature-a".into(),
                source_digest: flowguard::digest(b"b"),
                dependencies: BTreeSet::from(["a".into()]),
            },
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "other".into(),
                stage: "01-requirements".into(),
                owner: "feature-other".into(),
                source_digest: flowguard::digest(b"other"),
                dependencies: BTreeSet::new(),
            },
            StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: "release".into(),
                stage: "10-release".into(),
                owner: "project".into(),
                source_digest: flowguard::digest(b"release"),
                dependencies: if extra {
                    BTreeSet::from(["b".into(), "other".into()])
                } else {
                    BTreeSet::from(["b".into()])
                },
            },
        ],
        GraphLimits::default(),
    )
    .unwrap()
}
fn plan(b: &flowguard::context::ValidatedBinding, u: &Upstream, id: &str) -> PendingGate {
    let f = frozen(b, u, false);
    let scope = obligation_scope(f.obligations().first().unwrap());
    let mut p = policy(u);
    p.action = id.into();
    prepare_gate(
        b,
        &f,
        BTreeMap::from([(scope, p)]),
        GateRequest {
            run_id: id.into(),
            action: id.into(),
            started_at: TIME.into(),
        },
    )
    .unwrap()
}
fn finish(b: &flowguard::context::ValidatedBinding, u: &Upstream, p: PendingGate) -> GateRun {
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
fn context() -> ContextDigests {
    ContextDigests {
        source: flowguard::digest(b"source"),
        rules: flowguard::digest(b"rules"),
        analyzer: flowguard::digest(b"analyzer"),
        config: flowguard::digest(b"config"),
        coverage: flowguard::digest(b"coverage"),
        baseline: flowguard::digest(b"baseline"),
        dependencies: flowguard::digest(b"dependencies"),
    }
}
fn snapshot(
    graph: &StageGraph,
    plans: &BTreeMap<String, PendingGate>,
    contexts: &BTreeMap<String, ContextDigests>,
    bad: Option<&str>,
) -> FrozenInputs {
    let inputs: Vec<_> = plans
        .iter()
        .map(|(id, p)| NodeInput {
            node_id: id,
            gate: p,
            generation: 1,
            context: &contexts[id],
            freshness: Freshness {
                observed_at: NOW,
                expires_at: NOW + 60,
                revoked: bad == Some(id.as_str()),
                artifacts_available: true,
            },
        })
        .collect();
    FrozenInputs::freeze(graph, &inputs, NOW).unwrap()
}
#[test]
fn changed_dependency_revokes_exact_downstream_and_preserves_old_bytes() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let store = MemoryRunStore::default();
    let mut before = BTreeMap::new();
    for (id, p) in &plans {
        store.advance(p, 0).unwrap();
        store.reserve(p, id, 1).unwrap();
        let run = finish(&b, &u, plan(&b, &u, id));
        assert_eq!(run.envelope().decision, Some(guardengine::Decision::Allow));
        store.append(&run).unwrap();
        store.publish(id, 1).unwrap();
        before.insert(id.clone(), store.history(p).unwrap());
    }
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    let mut changed = contexts.clone();
    changed.get_mut("a").unwrap().source = flowguard::digest(b"changed");
    let current = snapshot(&g, &plans, &changed, None);
    let delta = old.changes(&current, NOW).unwrap();
    assert_eq!(
        delta.affected(),
        &BTreeSet::from(["a".into(), "b".into(), "release".into()])
    );
    delta.apply(&store).unwrap();
    for (id, p) in &plans {
        assert_eq!(store.history(p).unwrap(), before[id]);
        assert_eq!(store.current(p).unwrap().is_none(), id != "other");
    }
    assert_eq!(store.publish("a", 1), Err(StoreError::Stale));
    assert!(delta.apply(&store).is_err());
}
#[test]
fn stale_member_rejects_the_entire_batch_and_unchanged_delta_is_noop() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let store = MemoryRunStore::default();
    for (id, p) in &plans {
        store.advance(p, 0).unwrap();
        store.reserve(p, id, 1).unwrap();
        store
            .append(&plan(&b, &u, id).cancel(TIME).unwrap())
            .unwrap();
        store.publish(id, 1).unwrap();
    }
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    old.changes(&snapshot(&g, &plans, &contexts, None), NOW)
        .unwrap()
        .apply(&store)
        .unwrap();
    assert!(store.current(&plans["a"]).unwrap().is_some());
    let delta = old
        .changes(&snapshot(&g, &plans, &contexts, Some("a")), NOW)
        .unwrap();
    store.advance(&plans["release"], 1).unwrap();
    assert!(delta.apply(&store).is_err());
    assert!(store.current(&plans["a"]).unwrap().is_some());
    assert!(store.current(&plans["b"]).unwrap().is_some());
}

#[test]
fn refreshed_observation_time_alone_does_not_invalidate_unrelated_work() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    let inputs: Vec<_> = plans
        .iter()
        .map(|(id, p)| NodeInput {
            node_id: id,
            gate: p,
            generation: 1,
            context: &contexts[id],
            freshness: Freshness {
                observed_at: NOW + 1,
                expires_at: NOW + 60,
                revoked: false,
                artifacts_available: true,
            },
        })
        .collect();
    let new = FrozenInputs::freeze(&g, &inputs, NOW + 1).unwrap();
    assert!(old.changes(&new, NOW + 1).unwrap().affected().is_empty());
}

#[test]
fn all_protected_digest_dimensions_and_failed_freshness_propagate() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    let expected = BTreeSet::from(["a".into(), "b".into(), "release".into()]);
    for field in 0..7 {
        let mut changed = contexts.clone();
        let c = changed.get_mut("a").unwrap();
        let value = flowguard::digest(b"changed");
        match field {
            0 => c.source = value,
            1 => c.rules = value,
            2 => c.analyzer = value,
            3 => c.config = value,
            4 => c.coverage = value,
            5 => c.baseline = value,
            _ => c.dependencies = value,
        };
        assert_eq!(
            old.changes(&snapshot(&g, &plans, &changed, None), NOW)
                .unwrap()
                .affected(),
            &expected,
            "field {field}"
        );
    }
    for failure in ["expiry", "revoked", "missing"] {
        let inputs: Vec<_> = plans
            .iter()
            .map(|(id, p)| NodeInput {
                node_id: id,
                gate: p,
                generation: 1,
                context: &contexts[id],
                freshness: Freshness {
                    observed_at: NOW,
                    expires_at: if id == "a" && failure == "expiry" {
                        NOW + 1
                    } else {
                        NOW + 60
                    },
                    revoked: id == "a" && failure == "revoked",
                    artifacts_available: id != "a" || failure != "missing",
                },
            })
            .collect();
        let current = FrozenInputs::freeze(&g, &inputs, NOW).unwrap();
        assert_eq!(
            old.changes(&current, NOW + 1).unwrap().affected(),
            &expected,
            "{failure}"
        );
    }
}
#[test]
fn removed_edges_and_nodes_use_union_closure_not_only_new_graph() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    assert_eq!(
        old.changes(&snapshot(&graph(false), &plans, &contexts, None), NOW)
            .unwrap()
            .affected(),
        &BTreeSet::from(["release".into()])
    );
    let records = g
        .nodes()
        .values()
        .filter(|n| n.id != "a")
        .map(|n| {
            let mut n = n.clone();
            n.dependencies.remove("a");
            n
        })
        .collect();
    let smaller = build_graph(records, GraphLimits::default()).unwrap();
    let inputs: Vec<_> = plans
        .iter()
        .filter(|(id, _)| id.as_str() != "a")
        .map(|(id, p)| NodeInput {
            node_id: id,
            gate: p,
            generation: 1,
            context: &contexts[id],
            freshness: Freshness {
                observed_at: NOW,
                expires_at: NOW + 60,
                revoked: false,
                artifacts_available: true,
            },
        })
        .collect();
    let current = FrozenInputs::freeze(&smaller, &inputs, NOW).unwrap();
    assert_eq!(
        old.changes(&current, NOW).unwrap().affected(),
        &BTreeSet::from(["a".into(), "b".into(), "release".into()])
    );
}
#[test]
fn malformed_mapping_budgets_and_clock_fail_before_store_changes() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    assert!(old.changes(&old, NOW - 1).is_err());
    assert!(FrozenInputs::freeze(&g, &[], NOW).is_err());
    let inputs: Vec<_> = plans
        .keys()
        .map(|id| NodeInput {
            node_id: id,
            gate: &plans["a"],
            generation: 1,
            context: &contexts[id],
            freshness: Freshness {
                observed_at: NOW,
                expires_at: NOW + 60,
                revoked: false,
                artifacts_available: true,
            },
        })
        .collect();
    assert!(FrozenInputs::freeze(&g, &inputs, NOW).is_err());
    let mut records: Vec<_> = g.nodes().values().cloned().collect();
    records[0].owner = "x".repeat(1024 * 1024);
    let huge = build_graph(records, GraphLimits::default()).unwrap();
    assert!(FrozenInputs::freeze(&huge, &inputs, NOW).is_err());
}

#[test]
fn actual_candidate_base_queue_worktree_and_requirement_changes_invalidate_full_work() {
    use flowguard::context::{InvocationInput, bind};
    let (dir, b) = binding();
    let original = b.binding().clone();
    let run = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(dir.path())
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "next",
        ])
        .output()
        .unwrap();
    assert!(run.status.success());
    let head = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(dir.path())
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(head.status.success());
    let next = String::from_utf8(head.stdout).unwrap().trim().to_owned();
    let repo = gitguard::Repository::discover(dir.path(), "repo").unwrap();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    for case in [
        "candidate",
        "base",
        "queue",
        "worktree",
        "requirements",
        "producer-policy",
    ] {
        let head = if case == "candidate" {
            next.clone()
        } else {
            original.candidate_oid.clone()
        };
        let base = if case == "base" {
            next.clone()
        } else {
            original.base_oid.clone()
        };
        let worktree = if case == "worktree" {
            "different"
        } else {
            &original.worktree_id
        };
        let req = if case == "requirements" {
            vec!["OTHER".into()]
        } else {
            original.requirement_ids.clone()
        };
        let scope = gitguard::scope::TaskScope::advisory(
            "task",
            req.clone(),
            vec![b"a".to_vec()],
            &"a".repeat(64),
            None,
        )
        .unwrap();
        let c = repo
            .prepare_candidate(
                &repo
                    .resolve_subject(gitguard::subject::SubjectRequest::Commit(head.clone()))
                    .unwrap(),
                &scope,
                &gitguard::candidate::CandidateRequest {
                    worktree_id: worktree.into(),
                    base_oid: base.clone(),
                    merge_group_id: if case == "queue" {
                        Some("group".into())
                    } else {
                        None
                    },
                    members: if case == "queue" {
                        vec![head.clone()]
                    } else {
                        vec![]
                    },
                },
            )
            .unwrap();
        let changed = bind(
            &InvocationInput {
                repo_candidates: vec!["repo".into()],
                task_candidates: vec!["task".into()],
                worktree_id: worktree.into(),
                requirement_ids: req,
                candidate_oid: head,
                base_oid: base,
            },
            &repo,
            &c,
        )
        .unwrap();
        let new_u = upstream(
            &changed,
            if case == "producer-policy" {
                guardengine::Enforcement::Enforce
            } else {
                guardengine::Enforcement::Advise
            },
            false,
        );
        let replacement = plan(&changed, &new_u, "a");
        let inputs: Vec<_> = plans
            .iter()
            .map(|(id, p)| NodeInput {
                node_id: id,
                gate: if id == "a" { &replacement } else { p },
                generation: 1,
                context: &contexts[id],
                freshness: Freshness {
                    observed_at: NOW,
                    expires_at: NOW + 60,
                    revoked: false,
                    artifacts_available: true,
                },
            })
            .collect();
        let current = FrozenInputs::freeze(&g, &inputs, NOW).unwrap();
        assert_eq!(
            old.changes(&current, NOW).unwrap().affected(),
            &BTreeSet::from(["a".into(), "b".into(), "release".into()]),
            "{case}"
        );
    }
}

#[test]
fn per_node_observation_rollback_is_rejected_even_if_snapshot_time_advances() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let g = graph(true);
    let plans: BTreeMap<_, _> = g
        .nodes()
        .keys()
        .map(|id| (id.clone(), plan(&b, &u, id)))
        .collect();
    let contexts: BTreeMap<_, _> = plans.keys().map(|id| (id.clone(), context())).collect();
    let old = snapshot(&g, &plans, &contexts, None);
    let inputs: Vec<_> = plans
        .iter()
        .map(|(id, p)| NodeInput {
            node_id: id,
            gate: p,
            generation: 1,
            context: &contexts[id],
            freshness: Freshness {
                observed_at: if id == "a" { NOW - 1 } else { NOW },
                expires_at: NOW + 60,
                revoked: false,
                artifacts_available: true,
            },
        })
        .collect();
    let current = FrozenInputs::freeze(&g, &inputs, NOW + 1).unwrap();
    assert!(old.changes(&current, NOW + 1).is_err());
}

#[test]
fn competing_batches_have_one_atomic_winner() {
    let (_dir,b)=binding();let u=upstream(&b,guardengine::Enforcement::Advise,false);let g=graph(true);
    let plans:BTreeMap<_,_>=g.nodes().keys().map(|id|(id.clone(),plan(&b,&u,id))).collect();let contexts:BTreeMap<_,_>=plans.keys().map(|id|(id.clone(),context())).collect();
    let store=MemoryRunStore::default();for p in plans.values(){store.advance(p,0).unwrap();}
    let old=snapshot(&g,&plans,&contexts,None);let current=snapshot(&g,&plans,&contexts,Some("a"));let delta=old.changes(&current,NOW).unwrap();
    let barrier=std::sync::Barrier::new(2);
    let results=std::thread::scope(|scope|{
        let a=scope.spawn(||{barrier.wait();delta.apply(&store)});let b=scope.spawn(||{barrier.wait();delta.apply(&store)});
        [a.join().unwrap(),b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r|r.is_ok()).count(),1);
    for id in ["a","b","release"] {assert_eq!(store.advance(&plans[id],2).unwrap(),3);}
    assert_eq!(store.advance(&plans["other"],1).unwrap(),2);
}

struct AllocObserver;
thread_local! {static ALLOC_ON:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};static MAX_ALLOCATION:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
fn note_allocation(n:usize){let _=ALLOC_ON.try_with(|on|{if on.get(){let _=MAX_ALLOCATION.try_with(|m|m.set(m.get().max(n)));}});}
unsafe impl std::alloc::GlobalAlloc for AllocObserver {
    unsafe fn alloc(&self,l:std::alloc::Layout)->*mut u8{note_allocation(l.size());unsafe{std::alloc::GlobalAlloc::alloc(&std::alloc::System,l)}}
    unsafe fn dealloc(&self,p:*mut u8,l:std::alloc::Layout){unsafe{std::alloc::GlobalAlloc::dealloc(&std::alloc::System,p,l)}}
    unsafe fn realloc(&self,p:*mut u8,l:std::alloc::Layout,n:usize)->*mut u8{note_allocation(n);unsafe{std::alloc::GlobalAlloc::realloc(&std::alloc::System,p,l,n)}}
}
#[global_allocator]
static ALLOCATOR:AllocObserver=AllocObserver;
#[test]
fn oversized_prepared_policy_is_rejected_before_work_identity_copy_or_hash() {
    let (_dir,b)=binding();let u=upstream(&b,guardengine::Enforcement::Advise,false);let g=graph(true);
    let f=frozen(&b,&u,false);let scope=obligation_scope(f.obligations().first().unwrap());let mut p=policy(&u);p.action="a".into();p.producer_principals.insert("s".repeat(4*1024*1024));
    let oversized=prepare_gate(&b,&f,BTreeMap::from([(scope,p)]),GateRequest{run_id:"a".into(),action:"a".into(),started_at:TIME.into()}).unwrap();
    let plans:BTreeMap<_,_>=g.nodes().keys().map(|id|(id.clone(),plan(&b,&u,id))).collect();let c=context();
    let inputs:Vec<_>=plans.iter().map(|(id,p)|NodeInput{node_id:id,gate:if id=="a"{&oversized}else{p},generation:1,context:&c,freshness:Freshness{observed_at:NOW,expires_at:NOW+60,revoked:false,artifacts_available:true}}).collect();
    MAX_ALLOCATION.with(|m|m.set(0));ALLOC_ON.with(|on|on.set(true));let result=FrozenInputs::freeze(&g,&inputs,NOW);ALLOC_ON.with(|on|on.set(false));
    assert!(result.is_err());assert!(MAX_ALLOCATION.with(|m|m.get())<4096);
}
