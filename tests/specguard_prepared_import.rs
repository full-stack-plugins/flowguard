mod common;
use flowguard::{context::*, dependencies::*, gate::*, obligations::*, specguard_adapter::*};
use specguard::model::Identity;
use std::collections::{BTreeMap, BTreeSet};
const RAW: &[u8] = include_bytes!("../fixtures/providers/specguard-actual/obligations.json");
const RAW_PIN: &str = "sha256:7b6c1c513e338924a275499b3b05fd7c5358a1bc166fa7e6e08442374df04b08";
fn expected() -> ExpectedExport {
    ExpectedExport {
        baseline_digest: "sha256:9b1a5665b5fe39136cd5f5f014e1c87293b14fd98719bff86bb58f164380ac9f"
            .into(),
        source_digest: "sha256:2b2a13bb45c17cad31380c581eb6265e43b32a3d23628c5ca388aadfcc64b4bd"
            .into(),
        candidate_oid: "8df2938fd39ad94c098691912ab1e2c9b0aff0b7".into(),
        scope: BTreeSet::from([identity("R1"), identity("R2")]),
    }
}
fn identity(id: &str) -> Identity {
    Identity {
        namespace: "demo".into(),
        id: id.into(),
    }
}
fn mapping() -> BTreeMap<Identity, String> {
    BTreeMap::from([
        (identity("R1"), "CURRENT-R1".into()),
        (identity("R2"), "CURRENT-R2".into()),
    ])
}
fn template() -> FixtureEvidenceRequirement {
    FixtureEvidenceRequirement {
        stage_id: "A/01".into(),
        rules_digest: flowguard::digest(b"protected SG engine rules"),
        analyzer_version: "protected-sg-version".into(),
    }
}
fn binding() -> (tempfile::TempDir, ValidatedBinding) {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("/usr/bin/git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    std::fs::write(root.path().join("work"), "current flow context").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "current"]);
    let oid = git(&["rev-parse", "HEAD"]);
    let repo = gitguard::Repository::discover(root.path(), "current-repo").unwrap();
    let ids = vec!["CURRENT-R1".into(), "CURRENT-R2".into()];
    let scope = gitguard::scope::TaskScope::advisory(
        "task",
        ids.clone(),
        vec![b"work".to_vec()],
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
                worktree_id: "current-worktree".into(),
                base_oid: oid.clone(),
                merge_group_id: None,
                members: vec![],
            },
        )
        .unwrap();
    let b = bind(
        &InvocationInput {
            repo_candidates: vec!["current-repo".into()],
            task_candidates: vec!["task".into()],
            worktree_id: "current-worktree".into(),
            requirement_ids: ids,
            candidate_oid: oid.clone(),
            base_oid: oid,
        },
        &repo,
        &candidate,
    )
    .unwrap();
    (root, b)
}
#[test]
fn actual_export_preserves_three_stable_ids_and_freezes_independent_history_and_current() {
    let (_root, b) = binding();
    assert_ne!(b.binding().candidate_oid, expected().candidate_oid);
    let mut pins = expected();
    let mut links = mapping();
    let p = PreparedFixtureImport::prepare(&pins, RAW_PIN, &b, &links, &template()).unwrap();
    pins.scope.clear();
    links.clear();
    let imported = p.consume(RAW, &b).unwrap();
    assert_eq!(imported.obligations().len(), 3);
    for o in imported.obligations() {
        assert_eq!(
            o.id,
            format!(
                "obligation:{}",
                specguard::model::digest(&(&o.requirement, &o.acceptance))
            )
        );
    }
    let required = imported.evidence_obligation(&b).unwrap();
    assert_eq!(required.guard, "specguard");
    assert_eq!(required.coverage.len(), 3);
    let (_other, c) = common::binding();
    assert!(p.consume(RAW, &c).is_err());
    assert!(imported.evidence_obligation(&c).is_err());
}
#[test]
fn independent_raw_pin_schema_scope_and_all_source_bindings_reject_drift() {
    let (_root, b) = binding();
    let p =
        PreparedFixtureImport::prepare(&expected(), RAW_PIN, &b, &mapping(), &template()).unwrap();
    for key in [
        "apiVersion",
        "authenticationProfile",
        "scope",
        "sourceDigest",
        "baselineDigest",
        "candidateOid",
        "sources",
        "unknown",
    ] {
        let mut v: serde_json::Value = serde_json::from_slice(RAW).unwrap();
        match key {
            "scope" | "sources" => v[key] = serde_json::json!([]),
            _ => v[key] = serde_json::json!("changed"),
        };
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(p.consume(&bytes, &b).is_err(), "raw {key}");
        let rehashed = PreparedFixtureImport::prepare(
            &expected(),
            &flowguard::digest(&bytes),
            &b,
            &mapping(),
            &template(),
        )
        .unwrap();
        assert!(rehashed.consume(&bytes, &b).is_err(), "semantic {key}");
    }
    for n in 0..4 {
        let mut e = expected();
        match n {
            0 => e.baseline_digest = flowguard::digest(b"other"),
            1 => e.source_digest = flowguard::digest(b"other"),
            2 => e.candidate_oid = "f".repeat(40),
            _ => {
                e.scope.remove(&identity("R2"));
            }
        };
        let result = PreparedFixtureImport::prepare(&e, RAW_PIN, &b, &mapping(), &template())
            .and_then(|p| p.consume(RAW, &b));
        assert!(result.is_err());
    }
}
#[test]
fn mapping_and_metadata_reject_expansion_or_candidate_self_reduction() {
    let (_root, b) = binding();
    for n in 0..4 {
        let mut m = mapping();
        match n {
            0 => {
                m.remove(&identity("R2"));
            }
            1 => {
                m.insert(identity("R3"), "CURRENT-R3".into());
            }
            2 => {
                m.insert(identity("R2"), "CURRENT-R1".into());
            }
            _ => {
                m.insert(identity("R2"), "outside".into());
            }
        };
        assert!(PreparedFixtureImport::prepare(&expected(), RAW_PIN, &b, &m, &template()).is_err());
    }
    let mut e = expected();
    e.scope.insert(Identity {
        namespace: "n".repeat(1_048_576),
        id: "x".into(),
    });
    assert!(PreparedFixtureImport::prepare(&e, RAW_PIN, &b, &mapping(), &template()).is_err());
    let mut t = template();
    t.stage_id = "x".repeat(1_048_576);
    assert!(PreparedFixtureImport::prepare(&expected(), RAW_PIN, &b, &mapping(), &t).is_err());
}
#[test]
fn missing_actual_sg_requirement_cannot_be_replaced_by_other_guard_allow() {
    let (_root, b) = binding();
    let imported =
        PreparedFixtureImport::prepare(&expected(), RAW_PIN, &b, &mapping(), &template())
            .unwrap()
            .consume(RAW, &b)
            .unwrap();
    let sg = imported.evidence_obligation(&b).unwrap();
    let mut other = common::upstream(&b, guardengine::Enforcement::Advise, false);
    other.envelope.producer.guard = "codeguard".into();
    assert_eq!(other.envelope.decision, Some(guardengine::Decision::Allow));
    let cg = EvidenceObligation {
        stage_id: "A/01".into(),
        guard: "codeguard".into(),
        coverage: BTreeSet::from(["A".into()]),
        rules_digest: flowguard::digest(&other.contract),
        analyzer_version: "1".into(),
    };
    let graph = build_graph(
        vec![StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "A/01".into(),
            stage: "01-requirements".into(),
            owner: "fixture".into(),
            source_digest: flowguard::digest(b"stage"),
            dependencies: BTreeSet::new(),
        }],
        GraphLimits::default(),
    )
    .unwrap();
    let frozen = freeze(
        &graph,
        vec![sg.clone(), cg.clone()],
        &b.domain_digest(),
        &flowguard::digest(b"separate workflow baseline"),
    )
    .unwrap();
    let mut sgpolicy = common::policy(&other);
    sgpolicy.producer.guard = "specguard".into();
    sgpolicy.producer.analyzer_version = sg.analyzer_version.clone();
    sgpolicy.contract_digest = sg.rules_digest.clone();
    sgpolicy.required_scopes = sg.coverage.iter().cloned().collect();
    let cgkey = obligation_scope(&cg);
    let sgkey = obligation_scope(&sg);
    let pending = prepare_gate(
        &b,
        &frozen,
        BTreeMap::from([
            (sgkey.clone(), sgpolicy),
            (cgkey.clone(), common::policy(&other)),
        ]),
        GateRequest {
            run_id: "fg-import-gate".into(),
            action: "commit".into(),
            started_at: common::TIME.into(),
        },
    )
    .unwrap();
    let result = pending
        .evaluate(
            &[SpecialistEvidence {
                scope: &cgkey,
                envelope: &other.envelope,
                artifacts: other.artifacts(),
            }],
            &common::FixtureAuthority {
                approval: None,
                unavailable: false,
            },
            common::NOW,
            common::TIME,
        )
        .unwrap();
    assert_eq!(
        result.envelope().decision,
        Some(guardengine::Decision::Block)
    );
    assert!(result.envelope().coverage.observed_scopes.contains(&cgkey));
    assert!(result.envelope().coverage.missing_scopes.contains(&sgkey));
    assert_eq!(frozen.obligations().len(), 2);
}

#[test]
fn mapping_permutation_changes_required_gate_identity_without_rewriting_native_ids() {
    let (_root, b) = binding();
    let first = PreparedFixtureImport::prepare(&expected(), RAW_PIN, &b, &mapping(), &template())
        .unwrap()
        .consume(RAW, &b)
        .unwrap();
    let swapped = BTreeMap::from([
        (identity("R1"), "CURRENT-R2".into()),
        (identity("R2"), "CURRENT-R1".into()),
    ]);
    let second = PreparedFixtureImport::prepare(&expected(), RAW_PIN, &b, &swapped, &template())
        .unwrap()
        .consume(RAW, &b)
        .unwrap();
    assert_eq!(first.obligations(), second.obligations());
    assert_ne!(
        first.evidence_obligation(&b).unwrap(),
        second.evidence_obligation(&b).unwrap()
    );
}
#[test]
fn raw_corpus_hashes_are_exact_and_unsafe_rehashed_source_is_rejected() {
    let provenance: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../fixtures/providers/specguard-actual/PROVENANCE.json"
    ))
    .unwrap();
    for (name, bytes) in [
        (
            "source.bundle",
            include_bytes!("../fixtures/providers/specguard-actual/source.bundle").as_slice(),
        ),
        (
            "source.md",
            include_bytes!("../fixtures/providers/specguard-actual/source.md").as_slice(),
        ),
        (
            "generator.rs",
            include_bytes!("../fixtures/providers/specguard-actual/generator.rs").as_slice(),
        ),
        (
            "baseline.json",
            include_bytes!("../fixtures/providers/specguard-actual/baseline.json").as_slice(),
        ),
        ("obligations.json", RAW),
    ] {
        assert_eq!(
            flowguard::digest(bytes),
            format!("sha256:{}", provenance["sha256"][name].as_str().unwrap())
        );
    }
    let (_root, b) = binding();
    let mut v: serde_json::Value = serde_json::from_slice(RAW).unwrap();
    v["sources"][0]["path"] = "../outside.md".into();
    for o in v["obligations"].as_array_mut().unwrap() {
        o["source"]["path"] = "../outside.md".into();
    }
    let bytes = serde_json::to_vec(&v).unwrap();
    let p = PreparedFixtureImport::prepare(
        &expected(),
        &flowguard::digest(&bytes),
        &b,
        &mapping(),
        &template(),
    )
    .unwrap();
    assert!(p.consume(&bytes, &b).is_err());
}
