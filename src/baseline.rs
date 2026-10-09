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

/// Native Git parent, explicitly local-controller authority profile.
/// This identity is not itself an approval or an inherited-stage qualification.
pub struct FrozenBaseline {
    reference: BaselineRef,
    parent_candidate_digest: String,
    digest: String,
}
/// A child-bound edge; no deserialization or accepted-state field exists.
pub struct InheritedEdge {
    parent: String,
    child: String,
    request: crate::approvals::ApprovalRequest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InheritanceError {
    InvalidParent,
    Source,
    Child,
    Approval,
    Unavailable,
}
impl FrozenBaseline {
    pub fn from_candidate(
        repo: &gitguard::Repository,
        candidate: &gitguard::candidate::CandidateSnapshot,
        reference: &BaselineRef,
    ) -> Result<Self, InheritanceError> {
        use InheritanceError::*;
        crate::admission::candidate(candidate).map_err(|_| InvalidParent)?;
        if reference.requirements.is_empty()
            || reference.requirements.len() > 256
            || reference.requirements.iter().any(|r| r.len() > 256)
            || reference.repo_id.len() > 256
            || reference.approval_ref.len() > 256
        {
            return Err(InvalidParent);
        }
        candidate.validate(repo).map_err(|_| Source)?;
        if reference.revision != candidate.candidate_oid()
            || reference.repo_id != candidate.repo_id()
            || !reference
                .requirements
                .iter()
                .map(String::as_str)
                .eq(candidate.requirement_ids().iter().map(String::as_str))
        {
            return Err(InvalidParent);
        }
        for requirement in &reference.requirements {
            resolve_baseline(
                reference,
                candidate.repo_id(),
                requirement,
                candidate.candidate_oid(),
                &reference.content_digest,
                &reference.policy_digest,
            )
            .map_err(|_| InvalidParent)?;
        }
        verify_parent_source(repo, reference)?;
        let parent_candidate_digest = candidate.binding_digest();
        let digest = crate::digest(
            &serde_json::to_vec(&(
                "flowguard.local-baseline/v1alpha1",
                &parent_candidate_digest,
                reference,
            ))
            .map_err(|_| InvalidParent)?,
        );
        Ok(Self {
            reference: reference.clone(),
            parent_candidate_digest,
            digest,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn inherit(
        &self,
        child: &crate::context::ValidatedBinding,
        issuer: &str,
        role: &str,
        provider: &impl crate::approvals::ApprovalProvider,
        now: u64,
    ) -> Result<InheritedEdge, InheritanceError> {
        child.check_budget().map_err(|_| InheritanceError::Child)?;
        let b = child.binding();
        if b.repo_id != self.reference.repo_id
            || b.requirement_ids
                .iter()
                .any(|r| !self.reference.requirements.contains(r))
        {
            return Err(InheritanceError::Child);
        }
        if issuer.trim().is_empty()
            || role.trim().is_empty()
            || issuer.len() > 256
            || role.len() > 256
        {
            return Err(InheritanceError::Approval);
        }
        let request = crate::approvals::ApprovalRequest {
            reference: self.reference.approval_ref.clone(),
            issuer: issuer.into(),
            role: role.into(),
            action: "baseline.inherit.local-controller".into(),
            repo_id: b.repo_id.clone(),
            requirements: b.requirement_ids.iter().cloned().collect(),
            target_digest: self.digest.clone(),
            policy_digest: self.reference.policy_digest.clone(),
            baseline_revision: self.reference.revision.clone(),
            now,
        };
        verify_parent_approval(provider, &request)?;
        Ok(InheritedEdge {
            parent: self.digest.clone(),
            child: child.domain_digest(),
            request,
        })
    }
}
fn verify_parent_source(
    repo: &gitguard::Repository,
    reference: &BaselineRef,
) -> Result<(), InheritanceError> {
    use InheritanceError::*;
    if repo.repo_id() != reference.repo_id {
        return Err(Source);
    }
    let path = format!("docs/project/{}.md", reference.stage);
    let files = repo
        .read_commit_files(&reference.revision)
        .map_err(|_| Source)?;
    let file = files
        .iter()
        .find(|f| f.path() == path.as_bytes())
        .ok_or(Source)?;
    if !matches!(file.mode(), "100644" | "100755")
        || crate::digest(file.contents()) != reference.content_digest
    {
        return Err(Source);
    }
    Ok(())
}
fn verify_parent_approval(
    provider: &impl crate::approvals::ApprovalProvider,
    request: &crate::approvals::ApprovalRequest,
) -> Result<(), InheritanceError> {
    match crate::approvals::observe(provider, request) {
        Ok(Some(_)) => Ok(()),
        Err(crate::approvals::ProviderError::Unavailable) => Err(InheritanceError::Unavailable),
        _ => Err(InheritanceError::Approval),
    }
}
impl InheritedEdge {
    /// Supply the current protected parent and child, plus a current accessible
    /// repository observation. No historical object discovers current policy.
    pub fn consume(
        &self,
        parent: &FrozenBaseline,
        repo: &gitguard::Repository,
        candidate: &gitguard::candidate::CandidateSnapshot,
        current_child: &crate::context::ValidatedBinding,
        provider: &impl crate::approvals::ApprovalProvider,
        now: u64,
    ) -> Result<(), InheritanceError> {
        crate::admission::candidate(candidate).map_err(|_| InheritanceError::InvalidParent)?;
        current_child
            .check_budget()
            .map_err(|_| InheritanceError::Child)?;
        if self.parent != parent.digest || self.child != current_child.domain_digest() {
            return Err(InheritanceError::Child);
        }
        if candidate.binding_digest() != parent.parent_candidate_digest {
            return Err(InheritanceError::InvalidParent);
        }
        // The parent candidate was validated at freezing. Its exact digest above
        // must remain unchanged; refresh immutable objects, not today's worktree
        // cleanliness relative to a historical candidate commit.
        repo.commit(candidate.base_oid())
            .map_err(|_| InheritanceError::Source)?;
        for member in candidate.members() {
            repo.commit(member).map_err(|_| InheritanceError::Source)?;
        }
        verify_parent_source(repo, &parent.reference)?;
        let mut request = self.request.clone();
        request.now = now;
        verify_parent_approval(provider, &request)
    }
}
