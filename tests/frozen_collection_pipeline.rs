//! Real Git binding + protected file pin + actual GE evaluations; authority is fixture-only.
mod common;
use flowguard::{dependencies::*, gate::*, input_limits::AllowedRoot, obligations::*, policy::*};
use guardengine::{Decision, Enforcement};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn frozen_collection_retains_missing_testguard_across_permutations_and_deleted_inputs() {
    let (_git, binding) = common::binding();
    let directory = tempfile::tempdir().unwrap();
    let mut upstreams = Vec::new();
    let mut obligations = Vec::new();
    let mut stages = Vec::new();
    for (index, guard) in ["specguard", "codeguard", "testguard"]
        .into_iter()
        .enumerate()
    {
        let mut upstream = common::upstream(&binding, Enforcement::Advise, false);
        upstream.envelope.producer.guard = guard.into();
        upstream.envelope.run_id = format!("fixture-{guard}");
        let stage = StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: format!("A/0{}", index + 1),
            stage: flowguard::stage::STAGES[[0, 2, 3][index]].0.into(),
            owner: "A".into(),
            source_digest: flowguard::digest(guard.as_bytes()),
            dependencies: BTreeSet::new(),
        };
        std::fs::write(
            directory.path().join(format!("{}.json", index)),
            serde_json::to_vec(&stage).unwrap(),
        )
        .unwrap();
        obligations.push(EvidenceObligation {
            stage_id: stage.id.clone(),
            guard: guard.into(),
            coverage: BTreeSet::from(["A".into()]),
            rules_digest: flowguard::digest(&upstream.contract),
            analyzer_version: "1".into(),
        });
        stages.push(stage);
        upstreams.push(upstream);
    }
    let graph = build_graph(stages.clone(), GraphLimits::default()).unwrap();
    let baseline = flowguard::digest(b"protected baseline");
    let bytes = serde_json::to_vec(&serde_json::json!({
        "version":"flowguard.policy/v1alpha1", "context_digest":binding.domain_digest(),
        "baseline_digest":baseline, "graph_digest":flowguard::digest(&serde_json::to_vec(&graph).unwrap()),
        "obligations":obligations,
    })).unwrap();
    std::fs::write(directory.path().join("policy.json"), &bytes).unwrap();
    let root = AllowedRoot::new(directory.path(), 1_048_576).unwrap();
    let load = || {
        load_and_freeze(
            &root,
            "policy.json",
            &flowguard::digest(&bytes),
            &binding.domain_digest(),
            &baseline,
            &graph,
        )
    };
    let snapshot = load().unwrap();
    let before = serde_json::to_vec(snapshot.frozen()).unwrap();
    let scopes: Vec<_> = obligations.iter().map(obligation_scope).collect();
    let policies: BTreeMap<_, _> = scopes
        .iter()
        .cloned()
        .zip(upstreams.iter().map(common::policy))
        .collect();
    let evaluate = |frozen: &FrozenObligations, order: &[usize]| {
        let evidence: Vec<_> = order
            .iter()
            .map(|&i| SpecialistEvidence {
                scope: &scopes[i],
                envelope: &upstreams[i].envelope,
                artifacts: upstreams[i].artifacts(),
            })
            .collect();
        prepare_gate(
            &binding,
            frozen,
            policies.clone(),
            GateRequest {
                run_id: "collection-attempt".into(),
                action: "commit".into(),
                started_at: common::TIME.into(),
            },
        )
        .unwrap()
        .evaluate(
            &evidence,
            &common::FixtureAuthority {
                approval: None,
                unavailable: false,
            },
            common::NOW,
            common::TIME,
        )
        .unwrap()
    };
    let full = evaluate(snapshot.frozen(), &[0, 1, 2]);
    assert_eq!(full.envelope().decision, Some(Decision::Allow));
    for order in [[2, 1, 0], [1, 0, 2], [0, 2, 1]] {
        let result = evaluate(snapshot.frozen(), &order);
        assert_eq!(result.envelope().decision, Some(Decision::Allow));
        assert_eq!(result.envelope().coverage, full.envelope().coverage);
    }
    for order in [&[0, 1][..], &[1, 0][..], &[][..]] {
        let result = evaluate(snapshot.frozen(), order);
        assert_eq!(result.envelope().decision, Some(Decision::Block));
        assert!(
            result
                .envelope()
                .coverage
                .missing_scopes
                .contains(&scopes[2])
        );
        assert!(
            result
                .domain()
                .unwrap()
                .gaps
                .iter()
                .any(|gap| gap.scope == scopes[2])
        );
        assert_eq!(
            result.envelope().coverage.required_scopes,
            full.envelope().coverage.required_scopes
        );
    }
    // Candidate rewrites/deletes cannot modify the already frozen denominator or satisfy a new load.
    std::fs::write(directory.path().join("policy.json"), b"{}").unwrap();
    assert!(load().is_err());
    std::fs::remove_file(directory.path().join("policy.json")).unwrap();
    std::fs::remove_file(directory.path().join("2.json")).unwrap();
    assert!(load().is_err());
    assert_eq!(before, serde_json::to_vec(snapshot.frozen()).unwrap());
    let subset = evaluate(snapshot.frozen(), &[1, 0]);
    assert_eq!(subset.envelope().decision, Some(Decision::Block));
    assert!(
        subset
            .envelope()
            .coverage
            .missing_scopes
            .contains(&scopes[2])
    );
    // Re-discovery of remaining stage records also retains the protected obligation for the deleted stage.
    let remaining: Vec<StageRecord> = (0..3)
        .filter_map(|i| {
            std::fs::read(directory.path().join(format!("{i}.json")))
                .ok()
                .map(|b| serde_json::from_slice(&b).unwrap())
        })
        .collect();
    let reduced_graph = build_graph(remaining, GraphLimits::default()).unwrap();
    let frozen = freeze(
        &reduced_graph,
        obligations.clone(),
        &binding.domain_digest(),
        &baseline,
    )
    .unwrap();
    assert_eq!(frozen.obligations().len(), 3);
    assert!(frozen.missing_stages().contains(&stages[2].id));
    let result = evaluate(&frozen, &[2, 0, 1]);
    assert_eq!(result.envelope().decision, Some(Decision::Block));
    assert!(
        result
            .envelope()
            .coverage
            .missing_scopes
            .contains(&format!("flowguard.stage:{}", stages[2].id))
    );
}
