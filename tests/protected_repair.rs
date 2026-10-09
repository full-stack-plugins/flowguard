mod qualification_support;
use flowguard::{
    action_policy::{Action, ActionPolicy, ProtectedRepairPlan, RepairContext, RepairRequest},
    obligations::*,
    stage_qualification::*,
};
use qualification_support::*;
use std::collections::{BTreeMap, BTreeSet};
fn missing_testguard(
    b: &flowguard::context::ValidatedBinding,
    g: &flowguard::dependencies::StageGraph,
    c: &mut Controller,
) -> (
    ProtectedStagePlan,
    flowguard::gate::GateRun,
    FrozenObligations,
) {
    use flowguard::gate::*;
    use guardengine::integration::eligibility::{ProducerRecord, Validity};
    let u = upstream(b, guardengine::Enforcement::Advise, false);
    let f = freeze(
        g,
        vec![
            EvidenceObligation {
                stage_id: "first".into(),
                guard: "specguard".into(),
                coverage: BTreeSet::from(["A".into()]),
                rules_digest: flowguard::digest(&u.contract),
                analyzer_version: "1".into(),
            },
            EvidenceObligation {
                stage_id: "second".into(),
                guard: "testguard".into(),
                coverage: BTreeSet::from(["A".into()]),
                rules_digest: flowguard::digest(&u.contract),
                analyzer_version: "1".into(),
            },
        ],
        &b.domain_digest(),
        &flowguard::digest(b"baseline"),
    )
    .unwrap();
    let intent = StageIntent::freeze(g, "first", StageMode::Accept, &BTreeMap::new()).unwrap();
    let policies = f
        .obligations()
        .iter()
        .map(|o| {
            let mut p = policy(&u);
            p.action = intent.action().into();
            p.producer.guard = o.guard.clone();
            (obligation_scope(o), p)
        })
        .collect();
    let pending = prepare_gate(
        b,
        &f,
        policies,
        GateRequest {
            run_id: "repair-missing-testguard".into(),
            action: intent.action().into(),
            started_at: TIME.into(),
        },
    )
    .unwrap();
    let stage = ProtectedStagePlan::freeze(
        intent,
        b,
        &f,
        &pending,
        BTreeSet::from(["fixture-producer".into()]),
        BTreeSet::from(["stage-reviewer".into()]),
    )
    .unwrap();
    let digest = flowguard::digest(&serde_json::to_vec(&u.envelope).unwrap());
    c.issuers.insert(
        digest.clone(),
        ProducerRecord {
            principal: "fixture-producer".into(),
            producer: u.envelope.producer.clone(),
            envelope_digest: digest,
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        },
    );
    let scope = obligation_scope(
        f.obligations()
            .iter()
            .find(|o| o.guard == "specguard")
            .unwrap(),
    );
    let run = pending
        .evaluate(
            &[SpecialistEvidence {
                scope: &scope,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            c,
            NOW,
            TIME,
        )
        .unwrap();
    (stage, run, f)
}
#[test]
fn blocked_delivery_can_authorize_exact_test_repair_without_shrinking_obligations() {
    let (_dir, b) = binding();
    let g = graph();
    let mut controller = Controller::default();
    let (stage, run, frozen) = missing_testguard(&b, &g, &mut controller);
    assert_eq!(run.envelope().decision, Some(guardengine::Decision::Block));
    assert!(frozen.obligations().iter().any(|o| o.guard == "testguard"));
    let original = serde_json::to_vec(&frozen).unwrap();
    let policy = ActionPolicy {
        required_stages: g.nodes().keys().cloned().collect(),
        allow_test_repair: true,
    };
    let context = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    let paths = BTreeSet::from(["tests/required_case.rs".into()]);
    let plan = ProtectedRepairPlan::freeze(&context, &paths, NOW).unwrap();
    let p = plan.approval_policy();
    controller.approvals.insert(
        "repair".into(),
        guardengine::integration::eligibility::ApprovalRecord {
            principal: "stage-reviewer".into(),
            purpose: "workflow.write-tests".into(),
            action: p.action.clone(),
            binding: p.binding.clone(),
            contract_digest: p.contract_digest.clone(),
            validity: guardengine::integration::eligibility::Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 20,
                revoked: false,
            },
        },
    );
    let request = RepairRequest {
        action: Action::WriteTests,
        paths: &paths,
        approval_reference: "repair",
        now: NOW,
    };
    let receipt = plan.authorize(&context, &request, &controller).unwrap();
    assert_eq!(receipt.paths(), &paths);
    assert!(
        receipt
            .refresh(&plan, &context, &controller, NOW + 1)
            .is_ok()
    );
    let changed_paths = BTreeSet::from(["tests/different_case.rs".into()]);
    let changed_plan = ProtectedRepairPlan::freeze(&context, &changed_paths, NOW).unwrap();
    assert_ne!(changed_plan.digest(), plan.digest());
    assert!(
        receipt
            .refresh(&changed_plan, &context, &controller, NOW + 1)
            .is_err()
    );
    let mut weaker = policy.clone();
    weaker.required_stages.remove("second");
    let weak = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &weaker,
    };
    assert!(ProtectedRepairPlan::freeze(&weak, &paths, NOW).is_err());
    assert!(plan.authorize(&weak, &request, &controller).is_err());
    assert_eq!(serde_json::to_vec(&frozen).unwrap(), original);
    assert_eq!(run.envelope().decision, Some(guardengine::Decision::Block));
    controller
        .approvals
        .get_mut("repair")
        .unwrap()
        .validity
        .revoked = true;
    assert!(
        receipt
            .refresh(&plan, &context, &controller, NOW + 2)
            .is_err()
    );
}

fn approve_repair(c: &mut Controller, p: &ProtectedRepairPlan) {
    let p = p.approval_policy();
    c.approvals.insert(
        "repair".into(),
        guardengine::integration::eligibility::ApprovalRecord {
            principal: "stage-reviewer".into(),
            purpose: "workflow.write-tests".into(),
            action: p.action.clone(),
            binding: p.binding.clone(),
            contract_digest: p.contract_digest.clone(),
            validity: guardengine::integration::eligibility::Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 20,
                revoked: false,
            },
        },
    );
}
#[test]
fn exact_action_paths_authority_and_clock_are_rechecked() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (stage, _, frozen) = missing_testguard(&b, &g, &mut c);
    let policy = ActionPolicy {
        required_stages: g.nodes().keys().cloned().collect(),
        allow_test_repair: true,
    };
    let context = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    let paths = BTreeSet::from(["tests/new_case.rs".into()]);
    let plan = ProtectedRepairPlan::freeze(&context, &paths, NOW).unwrap();
    approve_repair(&mut c, &plan);
    for action in [Action::Read, Action::Clarify, Action::Deliver, Action::Skip] {
        assert!(
            plan.authorize(
                &context,
                &RepairRequest {
                    action,
                    paths: &paths,
                    approval_reference: "repair",
                    now: NOW
                },
                &c
            )
            .is_err()
        );
    }
    let request = RepairRequest {
        action: Action::WriteTests,
        paths: &paths,
        approval_reference: "repair",
        now: NOW,
    };
    for change in 0..9 {
        let saved = c.approvals["repair"].clone();
        {
            let a = c.approvals.get_mut("repair").unwrap();
            match change {
                0 => a.purpose = "stage.accept".into(),
                1 => a.action = stage.approval_policy().action.clone(),
                2 => a.binding.task_id = "other".into(),
                3 => a.binding.base_oid = "f".repeat(40),
                4 => a.contract_digest = flowguard::digest(b"wrong"),
                5 => a.principal = "other".into(),
                6 => a.validity.expires_at = NOW,
                7 => a.validity.issued_at = NOW + 1,
                _ => a.validity.revoked = true,
            }
        }
        assert!(
            plan.authorize(&context, &request, &c).is_err(),
            "change {change}"
        );
        c.approvals.insert("repair".into(), saved);
    }
    let receipt = plan.authorize(&context, &request, &c).unwrap();
    assert!(receipt.refresh(&plan, &context, &c, NOW + 3).is_ok());
    assert!(receipt.refresh(&plan, &context, &c, NOW + 2).is_err());
    assert!(receipt.refresh(&plan, &context, &c, NOW + 20).is_err());
    let newer = ProtectedRepairPlan::freeze(&context, &paths, NOW + 1).unwrap();
    assert!(newer.authorize(&context, &request, &c).is_err());
    assert!(receipt.refresh(&newer, &context, &c, NOW + 4).is_err());
    c.approvals.clear();
    assert!(plan.authorize(&context, &request, &c).is_err());
    for invalid in [
        "src/main.rs",
        "tests/../main.rs",
        "tests/*",
        "tests/.git/config.rs",
        "/tests/a.rs",
        "tests/a.sh",
        "tests/a\\b.rs",
    ] {
        let paths = BTreeSet::from([invalid.into()]);
        assert!(ProtectedRepairPlan::freeze(&context, &paths, NOW).is_err());
    }
    let outside = BTreeSet::from(["tests/different.rs".into()]);
    assert!(
        plan.authorize(
            &context,
            &RepairRequest {
                action: Action::WriteTests,
                paths: &outside,
                approval_reference: "repair",
                now: NOW
            },
            &c
        )
        .is_err()
    );
}
#[test]
fn candidate_policy_weakening_and_admission_budgets_cannot_create_receipts() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (stage, _, frozen) = missing_testguard(&b, &g, &mut c);
    let mut policy = ActionPolicy {
        required_stages: g.nodes().keys().cloned().collect(),
        allow_test_repair: true,
    };
    let paths = BTreeSet::from(["tests/new_case.rs".into()]);
    let context = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    let plan = ProtectedRepairPlan::freeze(&context, &paths, NOW).unwrap();
    approve_repair(&mut c, &plan);
    let huge = BTreeSet::from(["x".repeat(17 * 1024 * 1024)]);
    assert!(matches!(
        ProtectedRepairPlan::freeze(&context, &huge, NOW),
        Err(flowguard::action_policy::RepairError::Budget)
    ));
    let many = (0..65).map(|i| format!("tests/t{i}.rs")).collect();
    assert!(ProtectedRepairPlan::freeze(&context, &many, NOW).is_err());
    policy.allow_test_repair = false;
    let disabled = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    assert!(
        plan.authorize(
            &disabled,
            &RepairRequest {
                action: Action::WriteTests,
                paths: &paths,
                approval_reference: "repair",
                now: NOW
            },
            &c
        )
        .is_err()
    );
    policy.required_stages.clear();
    let empty = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    assert!(ProtectedRepairPlan::freeze(&empty, &paths, NOW).is_err());
}

#[test]
fn real_candidate_advance_requires_a_new_repair_plan_and_approval() {
    use flowguard::context::{InvocationInput, bind};
    let (dir, old) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (stage, _, frozen) = missing_testguard(&old, &g, &mut c);
    let policy = ActionPolicy {
        required_stages: g.nodes().keys().cloned().collect(),
        allow_test_repair: true,
    };
    let old_context = RepairContext {
        stage_plan: &stage,
        binding: &old,
        frozen: &frozen,
        policy: &policy,
    };
    let paths = BTreeSet::from(["tests/new_case.rs".into()]);
    let plan = ProtectedRepairPlan::freeze(&old_context, &paths, NOW).unwrap();
    approve_repair(&mut c, &plan);
    let request = RepairRequest {
        action: Action::WriteTests,
        paths: &paths,
        approval_reference: "repair",
        now: NOW,
    };
    let receipt = plan.authorize(&old_context, &request, &c).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("/usr/bin/git")
            .arg("-C")
            .arg(dir.path())
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
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    std::fs::write(dir.path().join("a"), "changed candidate").unwrap();
    git(&["add", "a"]);
    git(&["commit", "-qm", "second candidate"]);
    let oid = git(&["rev-parse", "HEAD"]);
    assert_ne!(oid, old.binding().candidate_oid);
    let repo = gitguard::Repository::discover(dir.path(), "repo").unwrap();
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
                base_oid: old.binding().base_oid.clone(),
                merge_group_id: None,
                members: vec![],
            },
        )
        .unwrap();
    let new = bind(
        &InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec!["task".into()],
            worktree_id: "w".into(),
            requirement_ids: vec!["A".into()],
            candidate_oid: oid,
            base_oid: old.binding().base_oid.clone(),
        },
        &repo,
        &candidate,
    )
    .unwrap();
    let mixed = RepairContext {
        binding: &new,
        ..old_context
    };
    assert!(ProtectedRepairPlan::freeze(&mixed, &paths, NOW).is_err());
    assert!(plan.authorize(&mixed, &request, &c).is_err());
    assert!(receipt.refresh(&plan, &mixed, &c, NOW + 1).is_err());
    let (new_stage, _, new_frozen) = missing_testguard(&new, &g, &mut c);
    let new_context = RepairContext {
        stage_plan: &new_stage,
        binding: &new,
        frozen: &new_frozen,
        policy: &policy,
    };
    let new_plan = ProtectedRepairPlan::freeze(&new_context, &paths, NOW).unwrap();
    assert_ne!(new_plan.digest(), plan.digest());
    assert!(new_plan.authorize(&new_context, &request, &c).is_err());
    assert!(
        receipt
            .refresh(&new_plan, &new_context, &c, NOW + 1)
            .is_err()
    );
    approve_repair(&mut c, &new_plan);
    assert!(new_plan.authorize(&new_context, &request, &c).is_ok());
}

struct TrackingAllocator;
thread_local! {
    static ALLOCATION_MAX: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}
unsafe impl std::alloc::GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCATION_MAX.with(|m| {
            if let Some(previous) = m.get() {
                m.set(Some(previous.max(layout.size())));
            }
        });
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: std::alloc::Layout, size: usize) -> *mut u8 {
        ALLOCATION_MAX.with(|m| {
            if let Some(previous) = m.get() {
                m.set(Some(previous.max(size)));
            }
        });
        unsafe { std::alloc::System.realloc(ptr, old, size) }
    }
}
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn oversized_paths_and_references_reject_before_cloning_or_authority_lookup() {
    use guardengine::integration::{GuardRunEnvelope, eligibility::*};
    struct MustNotQuery;
    impl AuthorityProvider for MustNotQuery {
        fn verify_producer(
            &self,
            _: &GuardRunEnvelope,
            _: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            panic!("unexpected producer lookup")
        }
        fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
            panic!("budget must precede authority lookup")
        }
    }
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (stage, _, frozen) = missing_testguard(&b, &g, &mut c);
    let policy = ActionPolicy {
        required_stages: g.nodes().keys().cloned().collect(),
        allow_test_repair: true,
    };
    let context = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    let paths = BTreeSet::from(["tests/new_case.rs".into()]);
    let plan = ProtectedRepairPlan::freeze(&context, &paths, NOW).unwrap();
    let huge = BTreeSet::from(["x".repeat(17 * 1024 * 1024)]);
    ALLOCATION_MAX.with(|m| m.set(Some(0)));
    let result = ProtectedRepairPlan::freeze(&context, &huge, NOW);
    let largest = ALLOCATION_MAX.with(|m| m.replace(None).unwrap());
    assert!(matches!(
        result,
        Err(flowguard::action_policy::RepairError::Budget)
    ));
    assert!(largest < 4096, "large pre-admission allocation: {largest}");
    for reference in [
        "\"".repeat(17 * 1024 * 1024),
        String::new(),
        "bad\nref".into(),
    ] {
        ALLOCATION_MAX.with(|m| m.set(Some(0)));
        let result = plan.authorize(
            &context,
            &RepairRequest {
                action: Action::WriteTests,
                paths: &paths,
                approval_reference: &reference,
                now: NOW,
            },
            &MustNotQuery,
        );
        let largest = ALLOCATION_MAX.with(|m| m.replace(None).unwrap());
        assert!(matches!(
            result,
            Err(flowguard::action_policy::RepairError::Budget)
        ));
        assert!(largest < 4096, "large pre-admission allocation: {largest}");
    }
}

#[test]
fn legacy_assessment_rejects_empty_applicability_without_querying_authority() {
    use flowguard::{
        action_policy::{ActionAssessment, assess},
        approvals::*,
    };
    struct MustNotFetch;
    impl ApprovalProvider for MustNotFetch {
        fn fetch(&self, _: &str) -> Result<Option<ApprovalRecord>, ProviderError> {
            panic!("invalid applicability must reject before authority lookup")
        }
    }
    let p = ActionPolicy {
        required_stages: BTreeSet::new(),
        allow_test_repair: true,
    };
    let q = ApprovalRequest {
        reference: "repair".into(),
        issuer: "fixture".into(),
        role: "repairer".into(),
        action: "write-tests".into(),
        repo_id: "repo".into(),
        requirements: BTreeSet::from(["A".into()]),
        target_digest: flowguard::digest(b"candidate"),
        policy_digest: flowguard::digest(b"policy"),
        baseline_revision: "baseline".into(),
        now: NOW as u64,
    };
    assert!(matches!(
        assess(Action::WriteTests, &p, &MustNotFetch, &q),
        Err(ProviderError::InvalidRecord)
    ));
    for (action, expected) in [
        (Action::Read, ActionAssessment::ReadOnly),
        (Action::Clarify, ActionAssessment::ReadOnly),
        (Action::Deliver, ActionAssessment::DeliveryGateRequired),
        (Action::Skip, ActionAssessment::ProtectedSkipGateRequired),
    ] {
        assert_eq!(assess(action, &p, &MustNotFetch, &q).unwrap(), expected);
    }
}

#[test]
fn independent_expired_refresh_cannot_reopen_earlier_clock() {
    let (_dir, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (stage, run, frozen) = missing_testguard(&b, &g, &mut c);
    assert_eq!(run.envelope().decision, Some(guardengine::Decision::Block));
    let policy = ActionPolicy {
        required_stages: g.nodes().keys().cloned().collect(),
        allow_test_repair: true,
    };
    let context = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    let paths = BTreeSet::from(["tests/new_case.rs".into()]);
    let plan = ProtectedRepairPlan::freeze(&context, &paths, NOW).unwrap();
    approve_repair(&mut c, &plan); // expires at NOW + 20
    let request = RepairRequest {
        action: Action::WriteTests,
        paths: &paths,
        approval_reference: "repair",
        now: NOW,
    };
    let receipt = plan.authorize(&context, &request, &c).unwrap();
    assert!(receipt.refresh(&plan, &context, &c, NOW + 1).is_ok());
    assert_eq!(
        receipt.refresh(&plan, &context, &c, NOW + 20),
        Err(flowguard::action_policy::RepairError::Approval)
    );
    assert_eq!(
        receipt.refresh(&plan, &context, &c, NOW + 2),
        Err(flowguard::action_policy::RepairError::Clock),
        "failed expiry check must advance observed time; approval cannot revive after rollback"
    );
}

#[test]
fn reentrant_authority_refresh_cannot_overwrite_a_newer_observed_time() {
    use guardengine::integration::{GuardRunEnvelope, eligibility::*};
    type Callback<'a> = Box<dyn Fn(&Reentrant<'a>) + 'a>;
    struct Reentrant<'a> {
        controller: &'a Controller,
        callback: std::cell::RefCell<Option<Callback<'a>>>,
    }
    impl AuthorityProvider for Reentrant<'_> {
        fn verify_producer(
            &self,
            _: &GuardRunEnvelope,
            _: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            Err(AuthorityError::Unavailable)
        }
        fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
            let callback = self.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback(self);
            }
            self.controller.verify_approval(reference)
        }
    }
    let (_dir, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let (stage, _, frozen) = missing_testguard(&b, &g, &mut c);
    let policy = ActionPolicy {
        required_stages: g.nodes().keys().cloned().collect(),
        allow_test_repair: true,
    };
    let context = RepairContext {
        stage_plan: &stage,
        binding: &b,
        frozen: &frozen,
        policy: &policy,
    };
    let paths = BTreeSet::from(["tests/new_case.rs".into()]);
    let plan = ProtectedRepairPlan::freeze(&context, &paths, NOW).unwrap();
    approve_repair(&mut c, &plan);
    let request = RepairRequest {
        action: Action::WriteTests,
        paths: &paths,
        approval_reference: "repair",
        now: NOW,
    };
    let receipt = plan.authorize(&context, &request, &c).unwrap();
    let provider = Reentrant {
        controller: &c,
        callback: std::cell::RefCell::new(Some(Box::new(|provider| {
            assert!(receipt.refresh(&plan, &context, provider, NOW + 3).is_ok());
        }))),
    };
    assert_eq!(
        receipt.refresh(&plan, &context, &provider, NOW + 2),
        Err(flowguard::action_policy::RepairError::Clock)
    );
    assert_eq!(
        receipt.refresh(&plan, &context, &c, NOW + 2),
        Err(flowguard::action_policy::RepairError::Clock)
    );
    assert!(receipt.refresh(&plan, &context, &c, NOW + 4).is_ok());
}
