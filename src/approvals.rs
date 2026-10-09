//! Controller-side port only. Implementations MUST authenticate their channel and
//! records before returning them. There is no production implementation in this slice.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    Unavailable,
    InvalidRecord,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRecord {
    pub reference: String,
    pub issuer: String,
    pub role: String,
    pub action: String,
    pub repo_id: String,
    pub requirements: BTreeSet<String>,
    pub target_digest: String,
    pub policy_digest: String,
    pub baseline_revision: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked: bool,
}
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub reference: String,
    pub issuer: String,
    pub role: String,
    pub action: String,
    pub repo_id: String,
    pub requirements: BTreeSet<String>,
    pub target_digest: String,
    pub policy_digest: String,
    pub baseline_revision: String,
    pub now: u64,
}
pub trait ApprovalProvider {
    fn fetch(&self, reference: &str) -> Result<Option<ApprovalRecord>, ProviderError>;
}
/// Non-serializable validation result. Authority is only as strong as the injected provider.
#[derive(Debug, Clone)]
pub struct VerifiedApprovalObservation {
    request: ApprovalRequest,
}
impl VerifiedApprovalObservation {
    pub fn refresh(
        &self,
        p: &impl ApprovalProvider,
        now: u64,
    ) -> Result<Option<Self>, ProviderError> {
        let mut q = self.request.clone();
        q.now = now;
        observe(p, &q)
    }
}
pub fn observe(
    provider: &impl ApprovalProvider,
    q: &ApprovalRequest,
) -> Result<Option<VerifiedApprovalObservation>, ProviderError> {
    if [
        &q.reference,
        &q.issuer,
        &q.role,
        &q.action,
        &q.repo_id,
        &q.baseline_revision,
    ]
    .iter()
    .any(|s| s.trim().is_empty())
        || q.requirements.is_empty()
        || q.requirements.iter().any(|s| s.trim().is_empty())
        || !crate::valid_digest(&q.target_digest)
        || !crate::valid_digest(&q.policy_digest)
    {
        return Err(ProviderError::InvalidRecord);
    }
    let Some(r) = provider.fetch(&q.reference)? else {
        return Ok(None);
    };
    if r.reference != q.reference
        || r.issuer != q.issuer
        || r.role != q.role
        || r.action != q.action
        || r.repo_id != q.repo_id
        || r.requirements.iter().any(|s| s.trim().is_empty())
        || !q.requirements.is_subset(&r.requirements)
        || r.target_digest != q.target_digest
        || r.policy_digest != q.policy_digest
        || r.baseline_revision != q.baseline_revision
        || r.issued_at > q.now
        || r.expires_at <= q.now
        || r.expires_at <= r.issued_at
        || r.revoked
    {
        return Err(ProviderError::InvalidRecord);
    }
    Ok(Some(VerifiedApprovalObservation { request: q.clone() }))
}
