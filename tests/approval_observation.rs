use flowguard::approvals::*;
use std::collections::BTreeSet;
struct FixtureProvider(Result<Option<ApprovalRecord>, ProviderError>);
impl ApprovalProvider for FixtureProvider {
    fn fetch(&self, _reference: &str) -> Result<Option<ApprovalRecord>, ProviderError> {
        self.0.clone()
    }
}
fn request() -> ApprovalRequest {
    ApprovalRequest {
        reference: "record:1".into(),
        issuer: "controller".into(),
        role: "reviewer".into(),
        action: "accept".into(),
        repo_id: "repo".into(),
        requirements: BTreeSet::from(["A".into()]),
        target_digest: flowguard::digest(b"binding"),
        policy_digest: flowguard::digest(b"policy"),
        baseline_revision: "revision-1".into(),
        now: 100,
    }
}
fn record() -> ApprovalRecord {
    let q = request();
    ApprovalRecord {
        reference: q.reference,
        issuer: q.issuer,
        role: q.role,
        action: q.action,
        repo_id: q.repo_id,
        requirements: q.requirements,
        target_digest: q.target_digest,
        policy_digest: q.policy_digest,
        baseline_revision: q.baseline_revision,
        issued_at: 90,
        expires_at: 110,
        revoked: false,
    }
}
#[test]
fn observation_requires_exact_fresh_scoped_provider_record() {
    let q = request();
    assert!(
        observe(&FixtureProvider(Ok(Some(record()))), &q)
            .unwrap()
            .is_some()
    );
    for case in 0..9 {
        let mut r = record();
        match case {
            0 => r.expires_at = 100,
            1 => r.revoked = true,
            2 => r.requirements = BTreeSet::from(["B".into()]),
            3 => r.action = "release".into(),
            4 => r.target_digest = flowguard::digest(b"other"),
            5 => r.issued_at = 101,
            6 => r.issuer = "agent".into(),
            7 => r.policy_digest = flowguard::digest(b"other"),
            _ => r.baseline_revision = "other".into(),
        };
        assert!(
            observe(&FixtureProvider(Ok(Some(r))), &q).is_err(),
            "case {case}"
        );
    }
    assert!(observe(&FixtureProvider(Err(ProviderError::Unavailable)), &q).is_err());
    assert!(observe(&FixtureProvider(Ok(None)), &q).unwrap().is_none());
    let mut self_asserted = serde_json::to_value(record()).unwrap();
    self_asserted["actorVerified"] = true.into();
    assert!(serde_json::from_value::<ApprovalRecord>(self_asserted).is_err());
}
#[test]
fn previously_observed_approval_must_refresh_at_consumption() {
    let q = request();
    let seen = observe(&FixtureProvider(Ok(Some(record()))), &q)
        .unwrap()
        .unwrap();
    let mut revoked = record();
    revoked.revoked = true;
    assert!(
        seen.refresh(&FixtureProvider(Ok(Some(revoked))), 100)
            .is_err()
    );
    assert!(
        seen.refresh(&FixtureProvider(Ok(Some(record()))), 110)
            .is_err()
    );
}
