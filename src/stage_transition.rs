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

/// An observed declaration, never a technical verdict or execution permission.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StageDeclaration {
    state: StageState,
}
impl StageDeclaration {
    pub fn observed(state: StageState) -> Self {
        Self { state }
    }
    pub fn state(&self) -> StageState {
        self.state
    }
    /// Read-only declaration application. Protected transitions return requirements.
    pub fn apply_declaration(&self, to: StageState) -> Result<Self, TransitionCheck> {
        match transition(self.state, to) {
            TransitionCheck::DeclarationAllowed => Ok(Self { state: to }),
            requirement => Err(requirement),
        }
    }
}
