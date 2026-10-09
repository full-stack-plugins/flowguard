//! Declarative transition requirements, never an authorization result.
use serde::{Deserialize, Serialize};
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    Pending,
    InProgress,
    PendingAcceptance,
    Accepted,
    Inherited,
    Skipped,
    Invalidated,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TransitionCheck {
    DeclarationAllowed,
    RequiresCurrentTechnicalEvidenceAndApproval,
    RequiresProtectedSkipAndApproval,
    RequiresExactBaselineAndApproval,
    Illegal,
}
pub fn transition(from: StageState, to: StageState) -> TransitionCheck {
    use StageState::*;
    use TransitionCheck::*;
    match (from, to) {
        (Pending, InProgress)
        | (InProgress, PendingAcceptance)
        | (Invalidated, PendingAcceptance)
        | (Accepted | Inherited | Skipped, Invalidated) => DeclarationAllowed,
        (PendingAcceptance, Accepted) => RequiresCurrentTechnicalEvidenceAndApproval,
        (Pending | InProgress | PendingAcceptance, Skipped) => RequiresProtectedSkipAndApproval,
        (Pending | InProgress | PendingAcceptance, Inherited) => RequiresExactBaselineAndApproval,
        _ => Illegal,
    }
}
