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
    use ActionAssessment::*;
    match action {
        Action::Read | Action::Clarify => Ok(ReadOnly),
        Action::Deliver => Ok(DeliveryGateRequired),
        Action::Skip => Ok(ProtectedSkipGateRequired),
        Action::WriteTests => {
            if !policy.allow_test_repair {
                return Ok(RepairNotApplicable);
            }
            if request.action != "write-tests" {
                return Err(ProviderError::InvalidRecord);
            }
            Ok(if observe(provider, request)?.is_some() {
                ScopedRepairEligible
            } else {
                ApprovalMissing
            })
        }
    }
}
