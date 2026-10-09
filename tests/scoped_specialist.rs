mod scoped_support;
use flowguard::{evidence::ScopedSources, gate::*};
use guardengine::{Decision, Enforcement};
use scoped_support::*;
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
fn request() -> GateRequest {
    GateRequest {
        run_id: "scoped-source-run".into(),
        action: "commit".into(),
        started_at: TIME.into(),
    }
}
#[test]
fn actual_archguard_source_scope_is_frozen_before_evidence_and_consumed_without_relabelling() {
    let f = fixture(Enforcement::Advise, false);
    assert_ne!(f.source_pin, f.binding.binding().source_snapshot_digest);
    assert!(prepare_gate(&f.binding, &f.frozen, f.policies.clone(), request()).is_err());
    let sources = ScopedSources::freeze(
        &f.binding,
        &f.frozen,
        BTreeMap::from([(f.key.clone(), f.source_pin.clone())]),
    )
    .unwrap();
    let pending = prepare_scoped_gate(
        &f.binding,
        &f.frozen,
        &sources,
        f.policies.clone(),
        request(),
    )
    .unwrap();
    let result = f.producer.run(&AtomicBool::new(false)).unwrap();
    result.verify(&f.repo, &f.candidate).unwrap();
    let raw = serde_json::to_vec(&result).unwrap();
    let verified =
        archguard::integration::binding::GitEvidenceBundle::load(&raw, &f.repo, &f.candidate)
            .unwrap();
    let evidence = verified.cargo();
    assert_eq!(
        evidence.envelope.binding.source_snapshot_digest,
        f.source_pin
    );
    let c = serde_json::to_vec(evidence.contract.as_ref().unwrap()).unwrap();
    let facts = serde_json::to_vec(evidence.facts.as_ref().unwrap()).unwrap();
    let report = serde_json::to_vec(evidence.report.as_ref().unwrap()).unwrap();
    let run = pending
        .evaluate(
            &[SpecialistEvidence {
                scope: &f.key,
                envelope: &evidence.envelope,
                artifacts: guardengine::integration::eligibility::ArtifactBytes {
                    contract: &c,
                    facts: &facts,
                    report: &report,
                },
            }],
            &FixtureIssuer::for_envelope(&evidence.envelope),
            now(),
            TIME,
        )
        .unwrap();
    assert_eq!(
        run.envelope().decision,
        Some(Decision::Allow),
        "{:?}",
        run.envelope().diagnostics
    );
    assert_eq!(run.envelope().binding, *f.binding.binding());
    let mut revoked = FixtureIssuer::for_envelope(&evidence.envelope);
    revoked.0.validity.revoked = true;
    assert!(matches!(
        run.evaluate_eligibility(f.policies.get(&f.key).unwrap(), &revoked, now()),
        Err(GateConsumptionError::SpecialistChanged(_))
    ));
    // Changing a controller pin changes the complete work identity even when
    // candidate/task/run identity remain unchanged.
    let changed_pin = flowguard::digest(b"different protected source");
    let changed_sources = ScopedSources::freeze(
        &f.binding,
        &f.frozen,
        BTreeMap::from([(f.key.clone(), changed_pin.clone())]),
    )
    .unwrap();
    let mut changed_policies = f.policies.clone();
    changed_policies
        .get_mut(&f.key)
        .unwrap()
        .binding
        .source_snapshot_digest = changed_pin;
    let changed = prepare_scoped_gate(
        &f.binding,
        &f.frozen,
        &changed_sources,
        changed_policies,
        request(),
    )
    .unwrap();
    assert_ne!(
        changed.required_scopes(),
        run.envelope().coverage.required_scopes
    );
    let store = flowguard::run_store::MemoryRunStore::default();
    let generation = store.advance(&changed, 0).unwrap();
    store.reserve(&changed, "changed", generation).unwrap();
    assert!(store.append(&run).is_err());
    let directory = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let disk = flowguard::durable_run_store::DurableRunStore::create(directory.path()).unwrap();
    let generation = disk.advance(&changed, 0).unwrap();
    disk.reserve(&changed, "changed", generation).unwrap();
    drop(disk);
    let disk = flowguard::durable_run_store::DurableRunStore::open(directory.path()).unwrap();
    assert!(disk.append(&run).is_err());
}

#[test]
fn source_profile_rejects_unknown_missing_extra_tampered_and_moved_context() {
    let f = fixture(Enforcement::Advise, false);
    let pins = BTreeMap::from([(f.key.clone(), f.source_pin.clone())]);
    assert!(ScopedSources::freeze(&f.binding, &f.frozen, BTreeMap::new()).is_err());
    let mut extra = pins.clone();
    extra.insert("unknown-obligation".into(), f.source_pin.clone());
    assert!(ScopedSources::freeze(&f.binding, &f.frozen, extra).is_err());
    let mut bad = pins.clone();
    bad.insert(f.key.clone(), "not-a-digest".into());
    assert!(ScopedSources::freeze(&f.binding, &f.frozen, bad).is_err());
    let profile = ScopedSources::freeze(&f.binding, &f.frozen, pins).unwrap();
    let original = serde_json::to_value(&profile).unwrap();
    let bytes = serde_json::to_vec(&profile).unwrap();
    assert_eq!(
        ScopedSources::from_json(&bytes, &f.binding, &f.frozen)
            .unwrap()
            .digest(),
        profile.digest()
    );
    for field in ["version", "digest", "context_digest", "frozen_digest"] {
        let mut v = original.clone();
        v[field] = serde_json::json!("changed");
        assert!(
            ScopedSources::from_json(&serde_json::to_vec(&v).unwrap(), &f.binding, &f.frozen)
                .is_err()
        );
    }
    let mut v = original.clone();
    v["trusted"] = serde_json::json!(true);
    assert!(
        ScopedSources::from_json(&serde_json::to_vec(&v).unwrap(), &f.binding, &f.frozen).is_err()
    );
    let mut v = original;
    let p = v["pins"][0].clone();
    v["pins"].as_array_mut().unwrap().push(p);
    assert!(
        ScopedSources::from_json(&serde_json::to_vec(&v).unwrap(), &f.binding, &f.frozen).is_err()
    );
    assert!(ScopedSources::from_json(&vec![b' '; 65537], &f.binding, &f.frozen).is_err());
}

#[test]
fn scoped_policy_still_requires_every_non_source_binding_field_and_exact_source_pin() {
    let f = fixture(Enforcement::Advise, false);
    let profile = ScopedSources::freeze(
        &f.binding,
        &f.frozen,
        BTreeMap::from([(f.key.clone(), f.source_pin.clone())]),
    )
    .unwrap();
    for case in 0..9 {
        let mut policies = f.policies.clone();
        let b = &mut policies.get_mut(&f.key).unwrap().binding;
        match case {
            0 => b.repo_id.push('x'),
            1 => b.task_id.push('x'),
            2 => b.worktree_id.push('x'),
            3 => b.requirement_ids = vec!["other".into()],
            4 => b.candidate_oid = "1".repeat(40),
            5 => b.base_oid = "2".repeat(40),
            6 => b.merge_group_id = Some("other".into()),
            7 => b.baseline_digest = Some(flowguard::digest(b"other")),
            _ => b.source_snapshot_digest = flowguard::digest(b"other"),
        };
        assert!(
            prepare_scoped_gate(&f.binding, &f.frozen, &profile, policies, request()).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn actual_partial_and_block_stay_closed_and_upstream_cancel_is_bound_error() {
    for (enforcement, partial, cancel) in [
        (Enforcement::Enforce, false, false),
        (Enforcement::Advise, true, false),
        (Enforcement::Advise, false, true),
    ] {
        let f = fixture(enforcement, partial);
        let sources = ScopedSources::freeze(
            &f.binding,
            &f.frozen,
            BTreeMap::from([(f.key.clone(), f.source_pin.clone())]),
        )
        .unwrap();
        let pending = prepare_scoped_gate(
            &f.binding,
            &f.frozen,
            &sources,
            f.policies.clone(),
            request(),
        )
        .unwrap();
        let required = pending.required_scopes().to_vec();
        let result = f.producer.run(&AtomicBool::new(cancel)).unwrap();
        result.verify(&f.repo, &f.candidate).unwrap();
        let e = result.cargo();
        let bytes = |v: &Option<serde_json::Value>| {
            v.as_ref()
                .map(|v| serde_json::to_vec(v).unwrap())
                .unwrap_or_default()
        };
        let c = bytes(&e.contract);
        let facts = bytes(&e.facts);
        let report = bytes(&e.report);
        let run = pending
            .evaluate(
                &[SpecialistEvidence {
                    scope: &f.key,
                    envelope: &e.envelope,
                    artifacts: guardengine::integration::eligibility::ArtifactBytes {
                        contract: &c,
                        facts: &facts,
                        report: &report,
                    },
                }],
                &FixtureIssuer::for_envelope(&e.envelope),
                now(),
                TIME,
            )
            .unwrap();
        assert_eq!(run.envelope().coverage.required_scopes, required);
        if cancel {
            assert_eq!(
                run.envelope().run_status,
                guardengine::integration::RunStatus::Error
            );
            assert!(run.envelope().decision.is_none());
        } else {
            assert_eq!(
                run.envelope().decision,
                Some(Decision::Block),
                "{:?}",
                run.envelope().diagnostics
            );
        }
    }
}

#[test]
fn actual_specialist_binding_drift_and_revoked_issuer_never_open_gate() {
    let f = fixture(Enforcement::Advise, false);
    let sources = ScopedSources::freeze(
        &f.binding,
        &f.frozen,
        BTreeMap::from([(f.key.clone(), f.source_pin.clone())]),
    )
    .unwrap();
    let result = f.producer.run(&AtomicBool::new(false)).unwrap();
    result.verify(&f.repo, &f.candidate).unwrap();
    let e = result.cargo();
    let c = serde_json::to_vec(e.contract.as_ref().unwrap()).unwrap();
    let facts = serde_json::to_vec(e.facts.as_ref().unwrap()).unwrap();
    let report = serde_json::to_vec(e.report.as_ref().unwrap()).unwrap();
    for case in 0..8 {
        let pending = prepare_scoped_gate(
            &f.binding,
            &f.frozen,
            &sources,
            f.policies.clone(),
            request(),
        )
        .unwrap();
        let required = pending.required_scopes().to_vec();
        let mut envelope = e.envelope.clone();
        match case {
            0 => envelope.binding.source_snapshot_digest = flowguard::digest(b"foreign source"),
            1 => envelope.binding.candidate_oid = "1".repeat(40),
            2 => envelope.binding.base_oid = "2".repeat(40),
            3 => envelope.binding.merge_group_id = Some("foreign group".into()),
            _ => (),
        }
        // Even an independently trusted fixture issuer cannot override binding.
        let mut issuer = FixtureIssuer::for_envelope(&envelope);
        if case == 4 {
            issuer.0.validity.revoked = true;
        }
        if case == 5 {
            issuer.0.envelope_digest = flowguard::digest(b"untrusted");
        }
        let mut checked_facts = facts.clone();
        if case == 6 {
            checked_facts.push(b' ');
        }
        if case == 7 {
            envelope.producer.analyzer_version = "unsupported".into();
        }
        let run = pending
            .evaluate(
                &[SpecialistEvidence {
                    scope: &f.key,
                    envelope: &envelope,
                    artifacts: guardengine::integration::eligibility::ArtifactBytes {
                        contract: &c,
                        facts: &checked_facts,
                        report: &report,
                    },
                }],
                &issuer,
                now(),
                TIME,
            )
            .unwrap();
        assert_eq!(
            run.envelope().run_status,
            guardengine::integration::RunStatus::Error
        );
        assert!(run.envelope().decision.is_none(), "case {case}");
        assert_eq!(run.envelope().coverage.required_scopes, required);
    }
}
