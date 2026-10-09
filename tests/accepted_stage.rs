mod qualification_support;
use flowguard::{
    accepted_stage::AcceptedStageSnapshot, obligations::*, stage_qualification::*,
    stage_transition::StageState,
};
use qualification_support::*;
use std::collections::{BTreeMap, BTreeSet};
fn frozen(
    b: &flowguard::context::ValidatedBinding,
    g: &flowguard::dependencies::StageGraph,
) -> FrozenObligations {
    let u = upstream(b, guardengine::Enforcement::Advise, false);
    freeze(
        g,
        g.nodes()
            .keys()
            .map(|stage| EvidenceObligation {
                stage_id: stage.clone(),
                guard: "specguard".into(),
                coverage: BTreeSet::from(["A".into()]),
                rules_digest: flowguard::digest(&u.contract),
                analyzer_version: "1".into(),
            })
            .collect(),
        &b.domain_digest(),
        &flowguard::digest(b"baseline"),
    )
    .unwrap()
}
#[test]
fn real_qualified_export_is_canonical_audit_and_refreshes_approval() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let intent = StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap();
    let (p, r) = produce(&b, &g, intent, false, &mut c);
    approve(&mut c, &p, "approval");
    let q = p
        .qualify(
            StageState::PendingAcceptance,
            &r,
            "approval",
            vec![],
            &c,
            NOW,
        )
        .unwrap();
    let f = frozen(&b, &g);
    let snapshot = q.export_snapshot(&p, &g, &b, &f, &c, NOW).unwrap();
    let bytes = snapshot.to_json().unwrap();
    for _ in 0..10 {
        assert_eq!(
            q.export_snapshot(&p, &g, &b, &f, &c, NOW)
                .unwrap()
                .to_json()
                .unwrap(),
            bytes
        );
    }
    let loaded = AcceptedStageSnapshot::from_json(&bytes).unwrap();
    assert_eq!(loaded.to_json().unwrap(), bytes);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["payload"]["observed_state"], "accepted");
    let reordered = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        AcceptedStageSnapshot::from_json(&reordered)
            .unwrap()
            .to_json()
            .unwrap(),
        bytes
    );
    if std::env::var_os("FG_WRITE_STAGE_FIXTURE").is_some() {
        std::fs::write("fixtures/schema/accepted-stage-valid.json", &bytes).unwrap();
    }
    c.approvals.get_mut("approval").unwrap().validity.revoked = true;
    assert!(q.export_snapshot(&p, &g, &b, &f, &c, NOW).is_err());
    assert!(AcceptedStageSnapshot::from_json(&bytes).is_ok());
    for (field, new) in [
        ("api_version", serde_json::json!("bad")),
        ("profile", serde_json::json!("production")),
        ("digest", serde_json::json!(flowguard::digest(b"tamper"))),
        ("unknown", serde_json::json!(true)),
    ] {
        let mut v = value.clone();
        v[field] = new;
        assert!(AcceptedStageSnapshot::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    let mut v = value.clone();
    v["payload"]["stage"]["owner"] = serde_json::json!("");
    assert!(AcceptedStageSnapshot::from_json(&serde_json::to_vec(&v).unwrap()).is_err());
    let duplicate =
        String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("{", "{\"digest\":\"duplicate\",", 1);
    assert!(AcceptedStageSnapshot::from_json(duplicate.as_bytes()).is_err());
    assert!(AcceptedStageSnapshot::from_json(&vec![b' '; 1024 * 1024 + 1]).is_err());
}

#[test]
fn export_rejects_foreign_graph_context_obligations_and_skip() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let intent = StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap();
    let (p, r) = produce(&b, &g, intent, false, &mut c);
    approve(&mut c, &p, "approval");
    let q = p
        .qualify(
            StageState::PendingAcceptance,
            &r,
            "approval",
            vec![],
            &c,
            NOW,
        )
        .unwrap();
    let f = frozen(&b, &g);
    let wrong = freeze(
        &g,
        f.obligations().iter().cloned().collect(),
        &b.domain_digest(),
        &flowguard::digest(b"different baseline"),
    )
    .unwrap();
    assert!(q.export_snapshot(&p, &g, &b, &wrong, &c, NOW).is_err());
    let mut nodes: Vec<_> = g.nodes().values().cloned().collect();
    nodes[0].source_digest = flowguard::digest(b"changed source");
    let wrongg = flowguard::dependencies::build_graph(
        nodes,
        flowguard::dependencies::GraphLimits::default(),
    )
    .unwrap();
    assert!(q.export_snapshot(&p, &wrongg, &b, &f, &c, NOW).is_err());
    let repo = gitguard::Repository::discover(_d.path(), "repo").unwrap();
    let oid = b.binding().candidate_oid.clone();
    let subject = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(oid.clone()))
        .unwrap();
    let scope = gitguard::scope::TaskScope::advisory(
        "foreign-task",
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
    let other = flowguard::context::bind(
        &flowguard::context::InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec!["foreign-task".into()],
            worktree_id: "w".into(),
            requirement_ids: vec!["A".into()],
            candidate_oid: oid.clone(),
            base_oid: oid,
        },
        &repo,
        &candidate,
    )
    .unwrap();
    assert!(q.export_snapshot(&p, &g, &other, &f, &c, NOW).is_err());
    let skip = StageIntent::freeze(
        &g,
        "first",
        StageMode::ProtectedSkip {
            policy_digest: flowguard::digest(b"skip"),
        },
        &BTreeMap::new(),
    )
    .unwrap();
    let (sp, sr) = produce(&b, &g, skip, false, &mut c);
    approve(&mut c, &sp, "skip");
    let sq = sp
        .qualify(StageState::Pending, &sr, "skip", vec![], &c, NOW)
        .unwrap();
    assert!(sq.export_snapshot(&sp, &g, &b, &f, &c, NOW).is_err());
    assert!(q.export_snapshot(&sp, &g, &b, &f, &c, NOW).is_err());
}
#[test]
fn snapshot_export_refreshes_dependency_approval() {
    let (_d, b) = binding();
    let g = graph();
    let mut c = Controller::default();
    let first = StageIntent::freeze(&g, "first", StageMode::Accept, &BTreeMap::new()).unwrap();
    let (p, r) = produce(&b, &g, first, false, &mut c);
    approve(&mut c, &p, "first");
    let q = p
        .qualify(StageState::PendingAcceptance, &r, "first", vec![], &c, NOW)
        .unwrap();
    let second = StageIntent::freeze(
        &g,
        "second",
        StageMode::Accept,
        &BTreeMap::from([("first".into(), &p)]),
    )
    .unwrap();
    let (p2, r2) = produce(&b, &g, second, false, &mut c);
    approve(&mut c, &p2, "second");
    let q2 = p2
        .qualify(
            StageState::PendingAcceptance,
            &r2,
            "second",
            vec![&q],
            &c,
            NOW,
        )
        .unwrap();
    let f = frozen(&b, &g);
    assert!(q2.export_snapshot(&p2, &g, &b, &f, &c, NOW).is_ok());
    c.approvals.get_mut("first").unwrap().validity.revoked = true;
    assert!(q2.export_snapshot(&p2, &g, &b, &f, &c, NOW).is_err());
}

#[test]
fn committed_real_export_fixtures_are_strict_and_non_authoritative() {
    let positive = include_bytes!("../fixtures/schema/accepted-stage-valid.json");
    assert!(AcceptedStageSnapshot::from_json(positive).is_ok());
    for bytes in [
        include_bytes!("../fixtures/schema/accepted-stage-unknown-field.json").as_slice(),
        include_bytes!("../fixtures/schema/accepted-stage-unknown-version.json").as_slice(),
        include_bytes!("../fixtures/schema/accepted-stage-empty-identity.json").as_slice(),
        include_bytes!("../fixtures/schema/accepted-stage-tampered-digest.json").as_slice(),
        include_bytes!("../fixtures/schema/accepted-stage-duplicate-key.json").as_slice(),
    ] {
        assert!(AcceptedStageSnapshot::from_json(bytes).is_err());
    }
    let duplicate = std::str::from_utf8(positive).unwrap().replacen(
        "\"stage\":{",
        "\"stage\":{\"owner\":\"forged\",",
        1,
    );
    assert_ne!(duplicate.as_bytes(), positive);
    assert!(AcceptedStageSnapshot::from_json(duplicate.as_bytes()).is_err());
}
