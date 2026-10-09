mod qualification_support;
use flowguard::{
    context::ValidatedBinding, dependencies::*, stage_qualification::*,
    stage_transition::StageState,
};
use qualification_support::*;
use std::collections::{BTreeMap, BTreeSet};
#[test]
fn stage_qualification_freezes_before_run_and_refreshes_entire_dependency_chain() {
    let (_directory, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let first = StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap();
    let (p1, r1) = produce(&b, &g, first, false, &mut c);
    approve(&mut c, &p1, "first-approval");
    let q1 = p1
        .qualify(
            StageState::PendingAcceptance,
            &r1,
            "first-approval",
            vec![],
            &c,
            NOW,
        )
        .unwrap();
    let second = StageIntent::freeze(
        &g,
        "second",
        StageMode::Accept,
        &BTreeMap::from([("first".into(), &p1)]),
    )
    .unwrap();
    let (p2, r2) = produce(&b, &g, second, false, &mut c);
    approve(&mut c, &p2, "second-approval");
    let q2 = p2
        .qualify(
            StageState::PendingAcceptance,
            &r2,
            "second-approval",
            vec![&q1],
            &c,
            NOW,
        )
        .unwrap();
    assert_eq!(q2.consume(&p2, &c, NOW).unwrap(), StageState::Accepted);
    c.approvals
        .get_mut("first-approval")
        .unwrap()
        .validity
        .revoked = true;
    assert!(q2.consume(&p2, &c, NOW).is_err());
    c.approvals
        .get_mut("first-approval")
        .unwrap()
        .validity
        .revoked = false;
    c.revoked = true;
    assert!(q2.consume(&p2, &c, NOW).is_err());
}

#[test]
fn explicit_stage_approval_rejects_cross_purpose_action_binding_expiry_and_missing_records() {
    let (_dir, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let intent = StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap();
    let (plan, run) = produce(&b, &g, intent, false, &mut c);
    approve(&mut c, &plan, "approval");
    let original = c.approvals["approval"].clone();
    for case in 0..9 {
        let mut changed = original.clone();
        match case {
            0 => changed.purpose = "stage.skip".into(),
            1 => changed.action = "commit".into(),
            2 => changed.binding.candidate_oid = "a".repeat(40),
            3 => changed.contract_digest = flowguard::digest(b"foreign contract"),
            4 => changed.principal = "unauthorized".into(),
            5 => changed.validity.expires_at = NOW,
            6 => changed.validity.issued_at = NOW + 1,
            7 => changed.validity.revoked = true,
            _ => (),
        }
        c.approvals.insert("approval".into(), changed);
        if case == 8 {
            c.approvals.remove("approval");
        }
        assert!(
            plan.qualify(
                StageState::PendingAcceptance,
                &run,
                "approval",
                vec![],
                &c,
                NOW
            )
            .is_err(),
            "case {case}"
        );
    }
    c.approvals.insert("approval".into(), original);
    for state in [
        StageState::Pending,
        StageState::Invalidated,
        StageState::Accepted,
        StageState::Skipped,
        StageState::Inherited,
    ] {
        assert!(matches!(
            plan.qualify(state, &run, "approval", vec![], &c, NOW),
            Err(QualificationError::Transition)
        ));
    }
}

#[test]
fn exact_prerequisite_set_and_current_plan_reject_substitution_and_candidate_drift() {
    let (_dir, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (p1, r1) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &p1, "one");
    let q1 = p1
        .qualify(StageState::PendingAcceptance, &r1, "one", vec![], &c, NOW)
        .unwrap();
    let (p2, r2) = produce(
        &b,
        &g,
        StageIntent::freeze(
            &g,
            "second",
            StageMode::Accept,
            &BTreeMap::from([("first".into(), &p1)]),
        )
        .unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &p2, "two");
    assert!(
        p2.qualify(StageState::PendingAcceptance, &r2, "two", vec![], &c, NOW)
            .is_err()
    );
    assert!(
        p2.qualify(
            StageState::PendingAcceptance,
            &r2,
            "two",
            vec![&q1, &q1],
            &c,
            NOW
        )
        .is_err()
    );
    let q2 = p2
        .qualify(
            StageState::PendingAcceptance,
            &r2,
            "two",
            vec![&q1],
            &c,
            NOW,
        )
        .unwrap();
    assert!(q2.consume(&p1, &c, NOW).is_err());
    let (_otherdir, other) = distinct_binding();
    assert_ne!(b.binding().candidate_oid, other.binding().candidate_oid);
    let (foreign, foreignrun) = produce(
        &other,
        &g,
        StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &foreign, "foreign");
    let qforeign = foreign
        .qualify(
            StageState::PendingAcceptance,
            &foreignrun,
            "foreign",
            vec![],
            &c,
            NOW,
        )
        .unwrap();
    assert!(
        p2.qualify(
            StageState::PendingAcceptance,
            &r2,
            "two",
            vec![&qforeign],
            &c,
            NOW
        )
        .is_err()
    );
    assert!(
        p1.qualify(
            StageState::PendingAcceptance,
            &foreignrun,
            "one",
            vec![],
            &c,
            NOW
        )
        .is_err()
    );
    assert!(StageIntent::freeze(&g, "second", StageMode::Accept, &BTreeMap::new()).is_err());
    assert!(
        StageIntent::freeze(
            &g,
            "first",
            StageMode::Accept,
            &BTreeMap::from([("second".into(), &p2)])
        )
        .is_err()
    );
}

fn inheritance_graph() -> StageGraph {
    build_graph(
        vec![StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "architecture".into(),
            stage: "02-architecture".into(),
            owner: "project".into(),
            source_digest: flowguard::digest(b"architecture"),
            dependencies: BTreeSet::new(),
        }],
        GraphLimits::default(),
    )
    .unwrap()
}
fn baseline(b: &ValidatedBinding) -> flowguard::baseline::BaselineRef {
    flowguard::baseline::BaselineRef {
        version: "flowguard.workflow/v1".into(),
        repo_id: b.binding().repo_id.clone(),
        stage: "02-architecture".into(),
        revision: b.binding().candidate_oid.clone(),
        content_digest: flowguard::digest(b"immutable baseline"),
        policy_digest: flowguard::digest(b"baseline policy"),
        approval_ref: "baseline-approval".into(),
        requirements: BTreeSet::from(["A".into()]),
    }
}
#[test]
fn protected_skip_and_local_inheritance_require_complete_technical_evidence_and_current_approval() {
    let (_dir, b) = binding();
    let g = inheritance_graph();
    for partial in [false, true] {
        for mode in [
            StageMode::ProtectedSkip {
                policy_digest: flowguard::digest(b"skip policy"),
            },
            StageMode::LocalControllerInheritance {
                baseline: baseline(&b),
            },
        ] {
            let expected = if matches!(mode, StageMode::ProtectedSkip { .. }) {
                StageState::Skipped
            } else {
                StageState::Inherited
            };
            let mut c = Controller::default();
            let intent = StageIntent::freeze(&g, "architecture", mode, &BTreeMap::new()).unwrap();
            let (plan, run) = produce(&b, &g, intent, partial, &mut c);
            approve(&mut c, &plan, "approval");
            let result = plan.qualify(StageState::Pending, &run, "approval", vec![], &c, NOW);
            if partial {
                assert!(result.is_err());
            } else {
                let qualified = result.unwrap();
                assert_eq!(qualified.consume(&plan, &c, NOW).unwrap(), expected);
                c.approvals.get_mut("approval").unwrap().validity.revoked = true;
                assert!(qualified.consume(&plan, &c, NOW).is_err());
            }
        }
    }
}

#[test]
fn graph_and_profile_budgets_fail_before_qualification() {
    let g = graph();
    assert!(StageIntent::freeze(&g, "unknown", StageMode::Accept, &BTreeMap::new()).is_err());
    assert!(
        StageIntent::freeze(
            &g,
            "first",
            StageMode::ProtectedSkip {
                policy_digest: "true".into()
            },
            &BTreeMap::new()
        )
        .is_err()
    );
    let large = build_graph(
        (0..65)
            .map(|i| StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: format!("n{i}"),
                stage: "01-requirements".into(),
                owner: format!("f{i}"),
                source_digest: flowguard::digest(b"s"),
                dependencies: BTreeSet::new(),
            })
            .collect(),
        GraphLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        StageIntent::freeze(&large, "n0", StageMode::Accept, &BTreeMap::new()),
        Err(QualificationError::Budget)
    ));
    let cyclic = vec![StageRecord {
        version: "flowguard.workflow/v1".into(),
        id: "cycle".into(),
        stage: "01-requirements".into(),
        owner: "a".into(),
        source_digest: flowguard::digest(b"s"),
        dependencies: BTreeSet::from(["cycle".into()]),
    }];
    assert!(build_graph(cyclic, GraphLimits::default()).is_err());
}

#[test]
fn baseline_mode_rejects_malformed_revision_before_action_serialization() {
    let (_dir, b) = binding();
    let g = inheritance_graph();
    let mut parent = baseline(&b);
    parent.revision = "a".repeat(4097);
    assert!(
        StageIntent::freeze(
            &g,
            "architecture",
            StageMode::LocalControllerInheritance { baseline: parent },
            &BTreeMap::new()
        )
        .is_err()
    );
}

fn distinct_binding() -> (tempfile::TempDir, ValidatedBinding) {
    use flowguard::context::{InvocationInput, bind};
    let (directory, _) = binding();
    let status = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(directory.path())
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "distinct-candidate",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(status.success());
    let oid = String::from_utf8(
        std::process::Command::new("/usr/bin/git")
            .arg("-C")
            .arg(directory.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let repo = gitguard::Repository::discover(directory.path(), "repo").unwrap();
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
    let binding = bind(
        &InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec!["task".into()],
            worktree_id: "w".into(),
            requirement_ids: vec!["A".into()],
            candidate_oid: oid.clone(),
            base_oid: oid,
        },
        &repo,
        &candidate,
    )
    .unwrap();
    (directory, binding)
}

#[test]
fn graph_changes_and_alternative_protected_dependency_modes_do_not_reuse_qualified_receipts() {
    let (_dir, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (original, run) = produce(
        &b,
        &g,
        StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &original, "original");
    let receipt = original
        .qualify(
            StageState::PendingAcceptance,
            &run,
            "original",
            vec![],
            &c,
            NOW,
        )
        .unwrap();
    let (alternate, alternate_run) = produce(
        &b,
        &g,
        StageIntent::freeze(
            &g,
            "first",
            StageMode::ProtectedSkip {
                policy_digest: flowguard::digest(b"different mode"),
            },
            &BTreeMap::new(),
        )
        .unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &alternate, "alternate");
    assert!(receipt.consume(&alternate, &c, NOW).is_err());
    let alternate_receipt = alternate
        .qualify(
            StageState::Pending,
            &alternate_run,
            "alternate",
            vec![],
            &c,
            NOW,
        )
        .unwrap();
    let (child, child_run) = produce(
        &b,
        &g,
        StageIntent::freeze(
            &g,
            "second",
            StageMode::Accept,
            &BTreeMap::from([("first".into(), &original)]),
        )
        .unwrap(),
        false,
        &mut c,
    );
    approve(&mut c, &child, "child");
    assert!(
        child
            .qualify(
                StageState::PendingAcceptance,
                &child_run,
                "child",
                vec![&alternate_receipt],
                &c,
                NOW
            )
            .is_err()
    );
    let mut nodes: Vec<_> = g.nodes().values().cloned().collect();
    nodes[0].source_digest = flowguard::digest(b"changed graph source");
    let changed = build_graph(nodes, GraphLimits::default()).unwrap();
    assert!(
        StageIntent::freeze(
            &changed,
            "second",
            StageMode::Accept,
            &BTreeMap::from([("first".into(), &original)])
        )
        .is_err()
    );
}
