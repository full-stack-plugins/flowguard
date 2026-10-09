//! Applicability comes from controller-protected policy. Results describe local
//! requirements, not grants; the external controller owns execution authority.
use crate::approvals::{ApprovalProvider, ApprovalRequest, ProviderError, observe};
use std::collections::BTreeSet;
#[derive(Debug, Clone)]
pub struct ActionPolicy {
    pub required_stages: BTreeSet<String>,
    pub allow_test_repair: bool,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Action {
    Read,
    Clarify,
    WriteTests,
    Deliver,
    Skip,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ActionAssessment {
    ReadOnly,
    ScopedRepairEligible,
    ApprovalMissing,
    DeliveryGateRequired,
    ProtectedSkipGateRequired,
    RepairNotApplicable,
}
pub fn assess(
    action: Action,
    policy: &ActionPolicy,
    provider: &impl ApprovalProvider,
    request: &ApprovalRequest,
) -> Result<ActionAssessment, ProviderError> {
    let requirement = requirements(action, policy);
    if requirement != ActionAssessment::ApprovalMissing {
        return Ok(requirement);
    }
    if policy.required_stages.is_empty()
        || policy.required_stages.len() > 64
        || policy
            .required_stages
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 256)
        || request.action != "write-tests"
    {
        return Err(ProviderError::InvalidRecord);
    }
    Ok(if observe(provider, request)?.is_some() {
        ActionAssessment::ScopedRepairEligible
    } else {
        ActionAssessment::ApprovalMissing
    })
}
// Both legacy descriptive assessment and protected receipt use one action
// classification. Only the protected API freezes concrete applicability/paths.
pub(crate) fn requirements(action: Action, policy: &ActionPolicy) -> ActionAssessment {
    match action {
        Action::Read | Action::Clarify => ActionAssessment::ReadOnly,
        Action::Deliver => ActionAssessment::DeliveryGateRequired,
        Action::Skip => ActionAssessment::ProtectedSkipGateRequired,
        Action::WriteTests if policy.allow_test_repair => ActionAssessment::ApprovalMissing,
        Action::WriteTests => ActionAssessment::RepairNotApplicable,
    }
}

mod repair;
pub use repair::{ProtectedRepairPlan, RepairContext, RepairError, RepairReceipt, RepairRequest};
