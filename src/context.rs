//! Candidate binding is validated against real GitGuard objects and byte snapshots.
//! Local caller identity remains advisory; failures are pre-binding diagnostics.
use gitguard::{Repository, candidate::CandidateSnapshot};
use guardengine::integration::RunBinding;
#[derive(Debug, Clone)]
pub struct InvocationInput {
    pub repo_candidates: Vec<String>,
    pub task_candidates: Vec<String>,
    pub worktree_id: String,
    pub requirement_ids: Vec<String>,
    pub candidate_oid: String,
    pub base_oid: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreBindingDiagnostic {
    AmbiguousIdentity,
    BindingMismatch,
    InvalidCandidate,
    DirtySnapshot,
}
#[derive(Debug, Clone)]
pub struct ValidatedBinding {
    binding: RunBinding,
    candidate_scope_digest: String,
}
impl ValidatedBinding {
    pub fn binding(&self) -> &RunBinding {
        &self.binding
    }
    pub fn candidate_scope_digest(&self) -> String {
        self.candidate_scope_digest.clone()
    }
    pub fn domain_digest(&self) -> String {
        crate::digest(
            &serde_json::to_vec(&(
                "flowguard.binding/v1",
                &self.binding,
                &self.candidate_scope_digest,
            ))
            .expect("closed binding"),
        )
    }
    pub fn is_advisory(&self) -> bool {
        true
    }
}
pub fn bind(
    input: &InvocationInput,
    repo: &Repository,
    candidate: &CandidateSnapshot,
) -> Result<ValidatedBinding, PreBindingDiagnostic> {
    use PreBindingDiagnostic::*;
    if input.repo_candidates.len() != 1
        || input.task_candidates.len() != 1
        || input.repo_candidates[0].trim().is_empty()
        || input.task_candidates[0].trim().is_empty()
    {
        return Err(AmbiguousIdentity);
    }
    if input.worktree_id.trim().is_empty()
        || input.requirement_ids.is_empty()
        || input.requirement_ids.len() > 4096
        || input.requirement_ids.iter().any(|r| r.trim().is_empty())
        || input.requirement_ids.windows(2).any(|w| w[0] >= w[1])
        || input.repo_candidates[0] != candidate.repo_id()
        || input.task_candidates[0] != candidate.task_id()
        || input.worktree_id != candidate.worktree_id()
        || input.requirement_ids != candidate.requirement_ids()
        || input.candidate_oid != candidate.candidate_oid()
        || input.base_oid != candidate.base_oid()
    {
        return Err(BindingMismatch);
    }
    candidate.validate(repo).map_err(|_| InvalidCandidate)?;
    let current = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(
            candidate.candidate_oid().into(),
        ))
        .map_err(|_| InvalidCandidate)?;
    if !candidate.clean()
        || !current.clean()
        || current.digest() != candidate.source_snapshot_digest()
    {
        return Err(DirtySnapshot);
    }
    let binding = RunBinding {
        repo_id: candidate.repo_id().into(),
        task_id: candidate.task_id().into(),
        worktree_id: candidate.worktree_id().into(),
        requirement_ids: candidate.requirement_ids().to_vec(),
        candidate_oid: candidate.candidate_oid().into(),
        base_oid: candidate.base_oid().into(),
        merge_group_id: candidate.merge_group_id().map(str::to_owned),
        source_snapshot_digest: format!("sha256:{}", candidate.source_snapshot_digest()),
        baseline_digest: candidate.baseline_digest().map(|d| format!("sha256:{d}")),
    };
    Ok(ValidatedBinding {
        binding,
        candidate_scope_digest: format!("sha256:{}", candidate.binding_digest()),
    })
}
