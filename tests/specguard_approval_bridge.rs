#[allow(dead_code)]
#[path = "../examples/approval_bridge_support/mod.rs"]
mod native;
#[path = "support/approval_bridge.rs"]
mod support;
use guardengine::{
    Decision,
    integration::{RunStatus, eligibility::*},
};
use specguard::integration::{approval::Profile, freshness::*, runtime::CancellationToken};
use support::*;
struct Clock;
impl ConsumptionClock for Clock {
    fn now(&self) -> Result<i64, String> {
        Ok(NOW)
    }
}
fn captures() -> Vec<Captured> {
    PROVIDERS.iter().map(|name| Captured::new(name)).collect()
}
#[test]
fn actual_review_and_four_producers_have_one_candidate_but_distinct_fg_scope() {
    let f = Fixture::new();
    let captures = captures();
    let original: specguard::integration::producer::ProducedRun = read("specguard/original.json");
    original.verify().unwrap();
    assert_eq!(original.envelope.decision, Some(Decision::RequireApproval));
    assert!(original.envelope.approval_refs.is_empty());
    assert_eq!(
        captures[4].envelope.decision,
        Some(Decision::RequireApproval)
    );
    let mut detached = captures[4].envelope.clone();
    detached.approval_refs.clear();
    assert_eq!(detached, original.envelope);
    assert_eq!(original.report.as_ref().unwrap(), &captures[4].report);
    assert_eq!(original.facts.as_ref().unwrap(), &captures[4].facts);
    assert_ne!(
        flowguard::digest(&serde_json::to_vec(&original.envelope).unwrap()),
        flowguard::digest(&serde_json::to_vec(&captures[4].envelope).unwrap())
    );
    for c in &captures {
        assert_eq!(
            c.envelope.binding.candidate_oid,
            f.binding.binding().candidate_oid
        );
        assert_eq!(c.envelope.binding.base_oid, f.binding.binding().base_oid);
        guardengine::integration::verify_engine_artifacts(
            &c.envelope,
            &c.contract,
            &c.facts,
            &c.report,
        )
        .unwrap();
    }
    let evidence: Vec<_> = captures
        .iter()
        .zip(PROVIDERS)
        .map(|(c, name)| c.evidence(&f.scopes[name]))
        .collect();
    let result = f
        .pending()
        .evaluate(&evidence, &FixtureAuthority::new(), NOW, TIME)
        .unwrap();
    assert_eq!(result.envelope().decision, Some(Decision::Allow));
    assert_ne!(
        result.envelope().coverage.required_scopes,
        original.envelope.coverage.required_scopes
    );
    assert_eq!(result.envelope().producer.guard, "flowguard");
    assert_eq!(
        read::<specguard::integration::producer::ProducedRun>("specguard/original.json")
            .envelope
            .decision,
        Some(Decision::RequireApproval)
    );
}
#[test]
fn approval_missing_expired_revoked_wrong_purpose_action_and_old_digest_never_open_gate() {
    let f = Fixture::new();
    let original = bytes("specguard", "original.json");
    for case in 0..10 {
        let mut authority = FixtureAuthority::new();
        let mut captures = captures();
        match case {
            0 => authority.approval = None,
            1 => authority.approval.as_mut().unwrap().validity.revoked = true,
            2 => authority.approval.as_mut().unwrap().validity.expires_at = NOW,
            3 => authority.approval.as_mut().unwrap().purpose = "other".into(),
            4 => authority.approval.as_mut().unwrap().action = "merge".into(),
            5 => authority.approval.as_mut().unwrap().binding.candidate_oid = "f".repeat(40),
            6 => {
                let old: specguard::integration::producer::ProducedRun =
                    serde_json::from_slice(&original).unwrap();
                let record = authority
                    .records
                    .remove(&flowguard::digest(
                        &serde_json::to_vec(&captures[4].envelope).unwrap(),
                    ))
                    .unwrap();
                let digest = flowguard::digest(&serde_json::to_vec(&old.envelope).unwrap());
                authority.records.insert(
                    digest.clone(),
                    ProducerRecord {
                        envelope_digest: digest,
                        ..record
                    },
                );
            }
            7 => captures[4].envelope.approval_refs.clear(),
            8 => authority.baseline_auth.revoked = true,
            _ => authority.baseline_auth.expires_at = NOW,
        }
        let evidence: Vec<_> = captures
            .iter()
            .zip(PROVIDERS)
            .map(|(c, name)| c.evidence(&f.scopes[name]))
            .collect();
        let result = f
            .pending()
            .evaluate(&evidence, &authority, NOW, TIME)
            .unwrap();
        assert_ne!(
            result.envelope().decision,
            Some(Decision::Allow),
            "case {case}"
        );
        assert_eq!(bytes("specguard", "original.json"), original);
    }
}
#[test]
fn live_sg_attachment_reauthenticates_baseline_and_producer_without_rewriting_history() {
    let f = Fixture::new();
    let (prepared, review, policy) = native::prepared(&f.repo_root);
    let target = prepared.work_key().target().clone();
    let expected = CurrentExpectation::freeze(&prepared, 1, &policy, Profile::Fixture).unwrap();
    let mut history = RunHistory::default();
    let complete = history
        .register(prepared, 0)
        .unwrap()
        .execute("2026-10-09T00:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    let before = serde_json::to_vec(complete.output()).unwrap();
    assert_eq!(before, bytes("specguard", "original.json"));
    history.append(&complete).unwrap();
    history.publish(&complete).unwrap();
    let attachment = history
        .attach_current(&expected, &["fixture:actual-spec-review".into()])
        .unwrap();
    let authority = FixtureAuthority::new();
    assert_eq!(attachment.envelope(), &envelope("specguard"));
    let mut baseline = native::baseline_authority(&review.baseline);
    assert!(
        history
            .consume_attached(
                &expected,
                &attachment,
                &authority,
                &Clock,
                Some(BaselineCheck {
                    baseline: &review.baseline,
                    authority: &baseline
                })
            )
            .unwrap()
            .eligible()
    );
    baseline.0.revoked = true;
    assert!(
        history
            .consume_attached(
                &expected,
                &attachment,
                &authority,
                &Clock,
                Some(BaselineCheck {
                    baseline: &review.baseline,
                    authority: &baseline
                })
            )
            .is_err()
    );
    baseline.0.revoked = false;
    baseline.0.expires_at = NOW;
    assert!(
        history
            .consume_attached(
                &expected,
                &attachment,
                &authority,
                &Clock,
                Some(BaselineCheck {
                    baseline: &review.baseline,
                    authority: &baseline
                })
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_vec(history.current(&target).unwrap().output()).unwrap(),
        before
    );
}
#[test]
fn native_partial_and_tool_error_cannot_be_repaired_by_approval() {
    let f = Fixture::new();
    for partial in [true, false] {
        let (prepared, _, policy) = native::prepared_mode(&f.repo_root, partial);
        let output = if partial {
            prepared
                .execute("2026-10-09T00:00:01Z", &CancellationToken::new(), |_| {})
                .unwrap()
        } else {
            prepared.fail("2026-10-09T00:00:01Z").unwrap()
        };
        if partial {
            assert_ne!(
                output.envelope.coverage.status,
                guardengine::integration::CoverageStatus::Complete
            );
        } else {
            assert_eq!(output.envelope.run_status, RunStatus::Error);
            assert!(output.envelope.decision.is_none());
        }
        let mut captures = captures();
        captures[4] = Captured {
            envelope: output.envelope.clone(),
            contract: output.contract.unwrap_or_default(),
            facts: output.facts.unwrap_or_default(),
            report: output.report.unwrap_or_default(),
        };
        // The real SG output has no approval refs; even a matching locally registered
        // producer record and valid grant record cannot supply complete evidence.
        let mut authority = FixtureAuthority::new();
        let digest = flowguard::digest(&serde_json::to_vec(&captures[4].envelope).unwrap());
        authority.records.insert(
            digest.clone(),
            ProducerRecord {
                principal: "fixture-ci".into(),
                producer: policy.producer,
                envelope_digest: digest,
                validity: Validity {
                    issued_at: 0,
                    expires_at: i64::MAX,
                    revoked: false,
                },
            },
        );
        let evidence: Vec<_> = captures
            .iter()
            .zip(PROVIDERS)
            .map(|(c, name)| c.evidence(&f.scopes[name]))
            .collect();
        let result = f
            .pending()
            .evaluate(&evidence, &authority, NOW, TIME)
            .unwrap();
        assert_ne!(result.envelope().decision, Some(Decision::Allow));
    }
    // A cancellation in real history also cannot get a borrowed attachment.
    let (prepared, _, policy) = native::prepared(&f.repo_root);
    let expected = CurrentExpectation::freeze(&prepared, 1, &policy, Profile::Fixture).unwrap();
    let mut history = RunHistory::default();
    let token = CancellationToken::new();
    token.cancel();
    let completion = history
        .register(prepared, 0)
        .unwrap()
        .execute("2026-10-09T00:00:01Z", &token, |_| {})
        .unwrap();
    history.append(&completion).unwrap();
    history.publish(&completion).unwrap();
    assert!(
        history
            .attach_current(&expected, &["fixture:actual-spec-review".into()])
            .is_err()
    );
}

#[test]
fn original_capture_hashes_and_all_required_providers_remain_bound() {
    let provenance: serde_json::Value = read("PROVENANCE.json");
    for section in ["artifacts", "protectedBeforeResults"] {
        for (path, digest) in provenance[section].as_object().unwrap() {
            assert_eq!(
                flowguard::digest(&std::fs::read(root().join(path)).unwrap())
                    .strip_prefix("sha256:")
                    .unwrap(),
                digest.as_str().unwrap(),
                "{path}"
            );
        }
    }
    let f = Fixture::new();
    let captures = captures();
    for omitted in 0..PROVIDERS.len() {
        let evidence: Vec<_> = captures
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != omitted)
            .map(|(i, c)| c.evidence(&f.scopes[PROVIDERS[i]]))
            .collect();
        let result = f
            .pending()
            .evaluate(&evidence, &FixtureAuthority::new(), NOW, TIME)
            .unwrap();
        assert_eq!(result.envelope().decision, Some(Decision::Block));
        assert_eq!(
            result.envelope().coverage.status,
            guardengine::integration::CoverageStatus::Partial
        );
    }
}
