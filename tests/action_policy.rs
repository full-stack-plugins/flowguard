use flowguard::{action_policy::*, approvals::*};
use std::collections::BTreeSet;
struct Fixture(Option<ApprovalRecord>);
impl ApprovalProvider for Fixture {
    fn fetch(&self, _reference: &str) -> Result<Option<ApprovalRecord>, ProviderError> {
        Ok(self.0.clone())
    }
}
fn request() -> ApprovalRequest {
    ApprovalRequest {
        reference: "approval:repair".into(),
        issuer: "controller".into(),
        role: "repairer".into(),
        action: "write-tests".into(),
        repo_id: "repo".into(),
        requirements: BTreeSet::from(["A".into()]),
        target_digest: flowguard::digest(b"candidate"),
        policy_digest: flowguard::digest(b"policy"),
        baseline_revision: "baseline".into(),
        now: 100,
    }
}
#[test]
fn repair_is_independent_of_delivery_but_requires_current_scoped_approval() {
    let q = request();
    let r = ApprovalRecord {
        reference: q.reference.clone(),
        issuer: q.issuer.clone(),
        role: q.role.clone(),
        action: q.action.clone(),
        repo_id: q.repo_id.clone(),
        requirements: q.requirements.clone(),
        target_digest: q.target_digest.clone(),
        policy_digest: q.policy_digest.clone(),
        baseline_revision: q.baseline_revision.clone(),
        issued_at: 90,
        expires_at: 110,
        revoked: false,
    };
    let policy = ActionPolicy {
        required_stages: BTreeSet::from(["A/01".into(), "A/04".into()]),
        allow_test_repair: true,
    };
    assert_eq!(
        assess(Action::Deliver, &policy, &Fixture(Some(r.clone())), &q).unwrap(),
        ActionAssessment::DeliveryGateRequired
    );
    assert_eq!(
        assess(Action::WriteTests, &policy, &Fixture(Some(r)), &q).unwrap(),
        ActionAssessment::ScopedRepairEligible
    );
    assert_eq!(policy.required_stages.len(), 2);
    assert_eq!(
        assess(Action::WriteTests, &policy, &Fixture(None), &q).unwrap(),
        ActionAssessment::ApprovalMissing
    );
    assert_eq!(
        assess(Action::Skip, &policy, &Fixture(None), &q).unwrap(),
        ActionAssessment::ProtectedSkipGateRequired
    );
    assert_eq!(
        assess(Action::Read, &policy, &Fixture(None), &q).unwrap(),
        ActionAssessment::ReadOnly
    );
    let mut wrong = q;
    wrong.action = "release".into();
    assert!(assess(Action::WriteTests, &policy, &Fixture(None), &wrong).is_err());
}
