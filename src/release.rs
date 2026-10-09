//! Exact project10 dependency composition, not execution authority.
use crate::{
    context::ValidatedBinding,
    dependencies::StageGraph,
    stage_qualification::{ProtectedStagePlan, QualifiedStage},
};
use guardengine::integration::eligibility::AuthorityProvider;
use std::collections::BTreeMap;
pub struct ReleasePlan {
    context: String,
    features: BTreeMap<String, String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ReleaseError {
    InvalidPlan,
    Binding,
    Features,
    Qualification,
}
impl ReleasePlan {
    /// Local profile: project10 direct dependencies are exactly all feature09
    /// nodes in the protected graph. Other prerequisite kinds belong upstream.
    pub fn freeze(
        graph: &StageGraph,
        release_stage: &str,
        binding: &ValidatedBinding,
        features: &BTreeMap<String, &ProtectedStagePlan>,
    ) -> Result<Self, ReleaseError> {
        use ReleaseError::*;
        binding.check_budget().map_err(|_| Binding)?;
        if graph.nodes().len() > 64 || features.is_empty() || features.len() > 64 {
            return Err(InvalidPlan);
        }
        if graph.nodes().values().any(|n| {
            n.id.len() > 256
                || n.owner.len() > 256
                || n.dependencies.len() > 64
                || n.dependencies.iter().any(|d| d.len() > 256)
        }) {
            return Err(InvalidPlan);
        }
        let release = graph.nodes().get(release_stage).ok_or(InvalidPlan)?;
        if release.stage != "10-release" || release.owner != "project" {
            return Err(InvalidPlan);
        }
        let docs: BTreeMap<_, _> = graph
            .nodes()
            .values()
            .filter(|n| n.stage == "09-docs")
            .map(|n| (n.owner.as_str(), n))
            .collect();
        let ids: std::collections::BTreeSet<_> = docs.values().map(|n| n.id.as_str()).collect();
        if !release
            .dependencies
            .iter()
            .map(String::as_str)
            .eq(ids.iter().copied())
            || !docs.keys().copied().eq(features.keys().map(String::as_str))
        {
            return Err(Features);
        }
        let graph_digest = crate::digest(&serde_json::to_vec(graph).map_err(|_| InvalidPlan)?);
        for (feature, plan) in features {
            let node = docs.get(feature.as_str()).ok_or(Features)?;
            let parent = &plan.approval_policy().binding;
            let release_binding = binding.binding();
            if plan.stage_id() != node.id
                || plan.graph_digest() != graph_digest
                || parent.repo_id != release_binding.repo_id
                || parent.candidate_oid != release_binding.candidate_oid
                || parent.base_oid != release_binding.base_oid
                || parent.merge_group_id != release_binding.merge_group_id
            {
                return Err(Binding);
            }
        }
        Ok(Self {
            context: binding.domain_digest(),
            features: features
                .iter()
                .map(|(f, p)| (f.clone(), p.digest().into()))
                .collect(),
        })
    }
    pub fn consume(
        &self,
        current: &ValidatedBinding,
        features: &BTreeMap<String, (&QualifiedStage<'_>, &ProtectedStagePlan)>,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<(), ReleaseError> {
        current.check_budget().map_err(|_| ReleaseError::Binding)?;
        if current.domain_digest() != self.context {
            return Err(ReleaseError::Binding);
        }
        if features.len() > 64 || !features.keys().eq(self.features.keys()) {
            return Err(ReleaseError::Features);
        }
        for (feature, (receipt, plan)) in features {
            if self.features.get(feature).map(String::as_str) != Some(plan.digest()) {
                return Err(ReleaseError::Binding);
            }
            receipt
                .consume(plan, provider, now)
                .map_err(|_| ReleaseError::Qualification)?;
        }
        Ok(())
    }
}
