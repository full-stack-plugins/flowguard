//! Joint actual producer consumption in the explicitly local MemoryRunStore profile.
use flowguard::{
    context::*, dependencies::*, evidence::*, gate::*, invalidation::*, obligations::*,
    run_store::*,
};
use guardengine::integration::{eligibility::*, *};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
const NOW: i64 = 2_000_000_000;
const TIME: &str = "2033-05-18T03:33:20Z";
const NAMES: [&str; 5] = [
    "archguard",
    "codeguard",
    "testguard",
    "gitguard",
    "specguard",
];
fn read<T: serde::de::DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}
fn bytes(p: impl AsRef<Path>) -> Vec<u8> {
    std::fs::read(p).unwrap()
}
fn validity(v: &Value) -> Validity {
    Validity {
        issued_at: v["issued_at"].as_i64().unwrap(),
        expires_at: v["expires_at"].as_i64().unwrap(),
        revoked: v["revoked"].as_bool().unwrap(),
    }
}
struct Authority(Value);
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        e: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        let r = self.0["producers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| {
                r["envelope_digest"] == d
                    && r["producer"] == serde_json::to_value(&e.producer).unwrap()
            })
            .ok_or(AuthorityError::Untrusted)?;
        Ok(ProducerRecord {
            principal: r["principal"].as_str().unwrap().into(),
            producer: serde_json::from_value(r["producer"].clone()).unwrap(),
            envelope_digest: d.into(),
            validity: validity(r),
        })
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        let r = self.0["approvals"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["reference"] == reference)
            .ok_or(AuthorityError::Untrusted)?;
        Ok(ApprovalRecord {
            principal: r["principal"].as_str().unwrap().into(),
            purpose: r["purpose"].as_str().unwrap().into(),
            action: r["action"].as_str().unwrap().into(),
            binding: serde_json::from_value(r["binding"].clone()).unwrap(),
            contract_digest: r["contract_digest"].as_str().unwrap().into(),
            validity: validity(r),
        })
    }
}
struct Case {
    path: PathBuf,
    binding: ValidatedBinding,
    frozen: FrozenObligations,
    sources: ScopedSources,
    policies: BTreeMap<String, EligibilityPolicy>,
    scopes: BTreeMap<String, String>,
    authority: Authority,
}
impl Case {
    fn new(root: &Path, name: &str) -> Self {
        let path = root.join(name);
        let candidate: gitguard::candidate::CandidateSnapshot =
            read(path.join("gitguard/candidate.json"));
        let repo = gitguard::Repository::discover(
            &root.join(format!("worktree-{name}")),
            "cross-guard-repo",
        )
        .unwrap();
        let b: RunBinding = read(path.join("binding.json"));
        let binding = bind(
            &InvocationInput {
                repo_candidates: vec![b.repo_id],
                task_candidates: vec![b.task_id],
                worktree_id: b.worktree_id,
                requirement_ids: b.requirement_ids,
                candidate_oid: b.candidate_oid,
                base_oid: b.base_oid,
            },
            &repo,
            &candidate,
        )
        .unwrap();
        let frozen = FrozenObligations::from_json(&bytes(path.join("frozen.json"))).unwrap();
        let sources =
            ScopedSources::from_json(&bytes(path.join("sources.json")), &binding, &frozen).unwrap();
        let wire: Value = read(path.join("policies.json"));
        let policies = wire
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    EligibilityPolicy {
                        binding: serde_json::from_value(v["binding"].clone()).unwrap(),
                        producer: serde_json::from_value(v["producer"].clone()).unwrap(),
                        required_scopes: serde_json::from_value(v["required_scopes"].clone())
                            .unwrap(),
                        contract_digest: v["contract_digest"].as_str().unwrap().into(),
                        action: "commit".into(),
                        producer_principals: serde_json::from_value(
                            v["producer_principals"].clone(),
                        )
                        .unwrap(),
                        approval_principals: serde_json::from_value(
                            v["approval_principals"].clone(),
                        )
                        .unwrap(),
                    },
                )
            })
            .collect();
        let scopes = read(path.join("scopes.json"));
        let authority = Authority(read(root.join(format!("authority-{name}.json"))));
        Self {
            path,
            binding,
            frozen,
            sources,
            policies,
            scopes,
            authority,
        }
    }
    fn pending(&self, id: &str, action: &str) -> PendingGate {
        let mut p = self.policies.clone();
        for v in p.values_mut() {
            v.action = action.into();
        }
        prepare_scoped_gate(
            &self.binding,
            &self.frozen,
            &self.sources,
            p,
            GateRequest {
                run_id: id.into(),
                action: action.into(),
                started_at: TIME.into(),
            },
        )
        .unwrap()
    }
    fn evaluate(&self, id: &str) -> GateRun {
        let captures: Vec<_> = NAMES
            .iter()
            .map(|n| {
                let p = self.path.join(n);
                (
                    read::<GuardRunEnvelope>(p.join("envelope.json")),
                    bytes(p.join("contract.json")),
                    bytes(p.join("facts.json")),
                    bytes(p.join("report.json")),
                )
            })
            .collect();
        let evidence: Vec<_> = captures
            .iter()
            .zip(NAMES)
            .map(|((e, c, f, r), n)| SpecialistEvidence {
                scope: &self.scopes[n],
                envelope: e,
                artifacts: ArtifactBytes {
                    contract: c,
                    facts: f,
                    report: r,
                },
            })
            .collect();
        let run = self
            .pending(id, "commit")
            .evaluate(&evidence, &self.authority, NOW, TIME)
            .unwrap();
        assert_eq!(run.envelope().decision, Some(guardengine::Decision::Allow));
        run
    }
    fn context(&self, source: String) -> ContextDigests {
        ContextDigests {
            source,
            rules: flowguard::digest(&bytes(self.path.join("policies.json"))),
            analyzer: flowguard::digest(
                &serde_json::to_vec(
                    &self
                        .policies
                        .iter()
                        .map(|(k, p)| (k, &p.producer))
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            ),
            config: flowguard::digest(&bytes(self.path.join("profile.json"))),
            coverage: flowguard::digest(&bytes(self.path.join("scopes.json"))),
            baseline: flowguard::digest(&bytes(self.path.join("specguard/baseline.json"))),
            dependencies: flowguard::digest(&bytes(self.path.join("frozen.json"))),
        }
    }
}
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap());
    let a = Case::new(&root, "A");
    let b = Case::new(&root, "B");
    let m1 = Case::new(&root, "M1");
    let m2 = Case::new(&root, "M2");
    let store = MemoryRunStore::default();
    let mut plans = BTreeMap::new();
    let mut runs = BTreeMap::new();
    for (id, case) in [("A", &a), ("B", &b), ("M1", &m1)] {
        let plan = case.pending(id, "commit");
        store.advance(&plan, 0).unwrap();
        store.reserve(&plan, id, 1).unwrap();
        plans.insert(id.to_string(), plan);
        runs.insert(id.to_string(), case.evaluate(id));
    }
    // A and B reservations exist concurrently. M1 completion is withheld until M2 publishes.
    for id in ["A", "B"] {
        store.append(&runs[id]).unwrap();
        store.publish(id, 1).unwrap();
    }
    let late_before = store.history(&plans["A"]).unwrap();
    let b_before = store.history(&plans["B"]).unwrap();
    let p2 = m2.pending("M2", "commit");
    store.advance(&p2, 1).unwrap();
    store.reserve(&p2, "M2", 2).unwrap();
    let r2 = m2.evaluate("M2");
    store.append(&r2).unwrap();
    store.publish("M2", 2).unwrap();
    store.append(&runs["M1"]).unwrap();
    assert_eq!(store.publish("M1", 1), Err(StoreError::Stale));
    assert_eq!(store.history(&plans["A"]).unwrap(), late_before);
    assert_eq!(store.history(&plans["B"]).unwrap(), b_before);
    plans.remove("M1");
    plans.insert("M2".into(), p2);
    runs.insert("M2".into(), r2);
    // Root is a registered controller source observation, never an invented positive gate.
    let project = a.pending("project-source-observation", "project-baseline-observation");
    store.advance(&project, 0).unwrap();
    plans.insert("project02".into(), project);
    let shared = flowguard::digest(&bytes(root.join("worktree-A/baseline.json")));
    assert_eq!(
        shared,
        flowguard::digest(&bytes(root.join("worktree-B/baseline.json")))
    );
    let graph = build_graph(
        plans
            .keys()
            .map(|id| StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: id.clone(),
                stage: if id == "project02" {
                    "02-architecture"
                } else {
                    "09-docs"
                }
                .into(),
                owner: if id == "project02" { "project" } else { id }.into(),
                source_digest: if id == "project02" {
                    shared.clone()
                } else {
                    flowguard::digest(id.as_bytes())
                },
                dependencies: if id == "project02" {
                    BTreeSet::new()
                } else {
                    BTreeSet::from(["project02".into()])
                },
            })
            .collect(),
        GraphLimits::default(),
    )
    .unwrap();
    let contexts = BTreeMap::from([
        (
            "A".to_string(),
            a.context(a.binding.binding().source_snapshot_digest.clone()),
        ),
        (
            "B".into(),
            b.context(b.binding.binding().source_snapshot_digest.clone()),
        ),
        (
            "M2".into(),
            m2.context(m2.binding.binding().source_snapshot_digest.clone()),
        ),
        ("project02".into(), a.context(shared.clone())),
    ]);
    let snapshot = |contexts: &BTreeMap<String, ContextDigests>| {
        FrozenInputs::freeze(
            &graph,
            &plans
                .iter()
                .map(|(id, p)| NodeInput {
                    node_id: id,
                    gate: p,
                    generation: if id == "M2" { 2 } else { 1 },
                    context: &contexts[id],
                    freshness: Freshness {
                        observed_at: NOW,
                        expires_at: NOW + 60,
                        revoked: false,
                        artifacts_available: true,
                    },
                })
                .collect::<Vec<_>>(),
            NOW,
        )
        .unwrap()
    };

    for (dimension, file) in [
        ("rules", "rules-revision2.json"),
        ("analyzer", "analyzer-revision2.json"),
    ] {
        let isolated = MemoryRunStore::default();
        for (id, p) in &plans {
            isolated.advance(p, 0).unwrap();
            let generation = if id == "M2" {
                isolated.advance(p, 1).unwrap();
                2
            } else {
                1
            };
            if id != "project02" {
                isolated.reserve(p, id, generation).unwrap();
                isolated.append(&runs[id]).unwrap();
                isolated.publish(id, generation).unwrap();
            }
        }
        let before: BTreeMap<_, _> = plans
            .iter()
            .map(|(id, p)| (id.clone(), isolated.history(p).unwrap()))
            .collect();
        let mut drift = contexts.clone();
        let changed = flowguard::digest(&bytes(root.join(file)));
        if dimension == "rules" {
            drift.get_mut("A").unwrap().rules = changed
        } else {
            drift.get_mut("A").unwrap().analyzer = changed
        }
        let delta = snapshot(&contexts).changes(&snapshot(&drift), NOW).unwrap();
        assert_eq!(delta.affected(), &BTreeSet::from(["A".into()]));
        delta.apply(&isolated).unwrap();
        assert!(isolated.current(&plans["A"]).unwrap().is_none());
        assert!(isolated.current(&plans["B"]).unwrap().is_some());
        for (id, p) in &plans {
            assert_eq!(isolated.history(p).unwrap(), before[id]);
        }
    }
    let old = snapshot(&contexts);
    let mut changed = contexts.clone();
    changed.get_mut("project02").unwrap().source =
        flowguard::digest(&bytes(root.join("project-baseline-revision2.json")));
    let delta = old.changes(&snapshot(&changed), NOW).unwrap();
    assert_eq!(
        delta.affected(),
        &BTreeSet::from(["project02".into(), "A".into(), "B".into(), "M2".into()])
    );
    let history: BTreeMap<_, _> = plans
        .iter()
        .map(|(id, p)| (id.clone(), store.history(p).unwrap()))
        .collect();
    delta.apply(&store).unwrap();
    for (id, p) in &plans {
        assert!(store.current(p).unwrap().is_none());
        assert_eq!(store.history(p).unwrap(), history[id]);
    }
    println!(
        "{}",
        serde_json::json!({"shared_project_invalidation":true,"causal_paths":delta.causal_paths(),"shared_project_digest":shared,"scoped_approved_baselines":{"A":contexts["A"].baseline,"B":contexts["B"].baseline,"M2":contexts["M2"].baseline},"rule_and_analyzer_drift_withdraw_A_only":true,"old_M1_publication_refused":true,"historical_bytes_unchanged":true,"project_node":"unexecuted controller source tracker, no positive gate or authority"})
    );
}
