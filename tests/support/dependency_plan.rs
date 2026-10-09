//! Fixed bounded local integration graph, assembled from protected obligations.
use flowguard::{
    context::ValidatedBinding, dependencies::*, evidence::*, gate::obligation_scope, obligations::*,
};
use guardengine::integration::eligibility::EligibilityPolicy;
use std::collections::{BTreeMap, BTreeSet};

pub struct ReadPlan {
    pub graph: StageGraph,
    pub frozen: FrozenObligations,
    pub sources: ScopedSources,
    pub policies: BTreeMap<String, EligibilityPolicy>,
    pub scopes: BTreeMap<String, String>,
}
impl ReadPlan {
    /// No evidence or completed report is an input to this protected plan.
    pub fn freeze(
        binding: &ValidatedBinding,
        original: &FrozenObligations,
        policies: &BTreeMap<String, EligibilityPolicy>,
    ) -> Self {
        assert!(!policies.is_empty() && policies.len() <= 5);
        assert_eq!(policies.len(), original.obligations().len());
        let has_sg = original
            .obligations()
            .iter()
            .any(|o| o.guard.eq_ignore_ascii_case("specguard"));
        let mut records = Vec::new();
        let mut obligations = Vec::new();
        let mut selected = BTreeMap::new();
        let mut pins = BTreeMap::new();
        let mut names = BTreeMap::new();
        for old in original.obligations() {
            let name = old.guard.to_ascii_lowercase();
            let stage = match name.as_str() {
                "specguard" => "01-requirements",
                "archguard" => "02-architecture",
                "codeguard" => "07-standards",
                "testguard" => "04-testcases",
                "gitguard" => "08-review",
                _ => panic!("unknown fixture reader"),
            };
            let policy = &policies[&obligation_scope(old)];
            assert_eq!(policy.contract_digest, old.rules_digest);
            let mut obligation = old.clone();
            obligation.stage_id = name.clone();
            let scope = obligation_scope(&obligation);
            let baseline = if let Some(digest) = &policy.binding.baseline_digest {
                assert_eq!(digest, original.baseline_digest());
                BaselineScope::FrozenWorkflowBaseline
            } else {
                BaselineScope::Absent
            };
            records.push(StageRecord {
                version: "flowguard.workflow/v1".into(),
                id: name.clone(),
                stage: stage.into(),
                owner: if ["archguard", "codeguard"].contains(&name.as_str()) {
                    "project"
                } else {
                    "feature-R"
                }
                .into(),
                source_digest: policy.binding.source_snapshot_digest.clone(),
                dependencies: if has_sg && name != "specguard" {
                    BTreeSet::from(["specguard".into()])
                } else {
                    BTreeSet::new()
                },
            });
            assert!(names.insert(name, scope.clone()).is_none());
            selected.insert(scope.clone(), policy.clone());
            pins.insert(
                scope,
                ScopedSource {
                    source_snapshot_digest: policy.binding.source_snapshot_digest.clone(),
                    baseline,
                },
            );
            obligations.push(obligation);
        }
        records.push(StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "gate".into(),
            stage: "10-release".into(),
            owner: "project".into(),
            source_digest: flowguard::digest(b"protected read-only DAG fixture v1"),
            dependencies: names.keys().cloned().collect(),
        });
        let graph = build_graph(
            records,
            GraphLimits {
                max_nodes: 6,
                max_edges: 9,
                max_depth: 3,
            },
        )
        .unwrap();
        let frozen = freeze(
            &graph,
            obligations,
            &binding.domain_digest(),
            original.baseline_digest(),
        )
        .unwrap();
        let sources = ScopedSources::freeze_contexts(binding, &frozen, pins).unwrap();
        Self {
            graph,
            frozen,
            sources,
            policies: selected,
            scopes: names,
        }
    }
}
