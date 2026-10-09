use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineRef {
    pub version: String,
    pub repo_id: String,
    pub stage: String,
    pub revision: String,
    pub content_digest: String,
    pub policy_digest: String,
    pub approval_ref: String,
    pub requirements: BTreeSet<String>,
}
/// Exact structural inheritance. This is NOT an authenticated approval or eligibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InheritanceSnapshot {
    requirement: String,
    parent: BaselineRef,
}
impl InheritanceSnapshot {
    pub fn parent_digest(&self) -> &str {
        &self.parent.content_digest
    }
}
pub fn resolve_baseline(
    baseline: &BaselineRef,
    repo: &str,
    requirement: &str,
    revision: &str,
    digest: &str,
    policy: &str,
) -> Result<InheritanceSnapshot, &'static str> {
    if baseline.version != "flowguard.workflow/v1"
        || !matches!(baseline.stage.as_str(), "02-architecture" | "07-standards")
        || repo.trim().is_empty()
        || baseline.repo_id != repo
        || requirement.trim().is_empty()
        || !baseline.requirements.contains(requirement)
        || baseline.requirements.iter().any(|r| r.trim().is_empty())
    {
        return Err("baseline scope/version");
    }
    if !matches!(baseline.revision.len(), 40 | 64)
        || !baseline
            .revision
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || baseline.revision != revision
        || !crate::valid_digest(&baseline.content_digest)
        || !crate::valid_digest(&baseline.policy_digest)
        || baseline.content_digest != digest
        || baseline.policy_digest != policy
        || baseline.approval_ref.trim().is_empty()
    {
        return Err("unbound baseline");
    }
    Ok(InheritanceSnapshot {
        requirement: requirement.into(),
        parent: baseline.clone(),
    })
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseRef {
    pub version: String,
    pub features: BTreeMap<String, String>,
}
pub fn validate_release(
    release: &ReleaseRef,
    observed: &BTreeMap<String, String>,
) -> Result<(), &'static str> {
    if release.version != "flowguard.workflow/v1"
        || release.features.is_empty()
        || release
            .features
            .iter()
            .any(|(id, d)| id.trim().is_empty() || !crate::valid_digest(d))
        || &release.features != observed
    {
        return Err("release feature set/digest mismatch");
    }
    Ok(())
}
