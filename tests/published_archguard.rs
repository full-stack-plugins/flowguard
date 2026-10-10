mod qualification_support;
mod scoped_support;
use archguard::integration::{projection::EvidenceBundle, rollout::*, trust::ConsumptionPolicy};
use flowguard::{
    evidence::ScopedSources, gate::*, local_migration::*, run_store::MemoryRunStore,
    stage_qualification::*, stage_transition::StageState,
};
use guardengine::{
    Enforcement, GuardAnalyzer, GuardFacts,
    integration::{GuardRunEnvelope, eligibility::*},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::AtomicBool,
};

// Test-only protected caller composes existing AG/GE authority ports. It is not
// an installed production provider or an additional rollout state owner.
struct JointAuthority<'a> {
    records: qualification_support::Controller,
    bundle: &'a EvidenceBundle,
    policy: ConsumptionPolicy,
    native: GuardFacts,
    enabled: bool,
    now: i64,
}
impl AuthorityProvider for JointAuthority<'_> {
    fn verify_producer(
        &self,
        envelope: &GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if envelope.producer.guard == "ArchGuard"
            && !check_stage(
                self.bundle,
                &self.policy,
                &self.records,
                self.now,
                Selection {
                    stage: Stage::OptInProtected,
                    adapter_enabled: self.enabled,
                    native_facts: Some(&self.native),
                },
            )
            .required_check_satisfied
        {
            return Err(AuthorityError::Untrusted);
        }
        self.records.verify_producer(envelope, digest)
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.records.verify_approval(reference)
    }
}

#[test]
fn actual_archguard_trusted_refresh_controls_published_flowguard_route() {
    let f = scoped_support::fixture(Enforcement::Advise, false);
    let now = scoped_support::now();
    let intent = StageIntent::freeze(
        &f.graph,
        "architecture",
        StageMode::Accept,
        &BTreeMap::new(),
    )
    .unwrap();
    let mut policies = f.policies.clone();
    policies.get_mut(&f.key).unwrap().action = intent.action().into();
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
        policies.clone(),
        GateRequest {
            run_id: "actual-ag-published-fg".into(),
            action: intent.action().into(),
            started_at: scoped_support::TIME.into(),
        },
    )
    .unwrap();
    let plan = ProtectedStagePlan::freeze(
        intent,
        &f.binding,
        &f.frozen,
        &pending,
        BTreeSet::from(["fixture-issuer".into()]),
        BTreeSet::from(["stage-reviewer".into()]),
    )
    .unwrap();
    let result = f.producer.run(&AtomicBool::new(false)).unwrap();
    result.verify(&f.repo, &f.candidate).unwrap();
    let bundle = result.cargo();
    let original = serde_json::to_vec(&result).unwrap();
    let native = archguard::CargoWorkspaceAnalyzer
        .analyze(f.directory.path(), "native same input")
        .unwrap();
    let contract: guardengine::GuardContract =
        serde_json::from_value(bundle.contract.clone().unwrap()).unwrap();
    let enhanced: GuardFacts = serde_json::from_value(bundle.facts.clone().unwrap()).unwrap();
    // Only compare decisions under the exact same frozen contract; legacy and
    // enhanced snapshot identities intentionally differ.
    assert_eq!(
        guardengine::evaluate(&contract, &native).unwrap().decision,
        guardengine::evaluate(&contract, &enhanced)
            .unwrap()
            .decision
    );
    let record = scoped_support::FixtureIssuer::for_envelope(&bundle.envelope).0;
    let mut records = qualification_support::Controller::default();
    records
        .issuers
        .insert(record.envelope_digest.clone(), record);
    qualification_support::approve(&mut records, &plan, "stage-review");
    records.approvals.get_mut("stage-review").unwrap().validity = Validity {
        issued_at: now - 1,
        expires_at: now + 100,
        revoked: false,
    };
    let mut ag_policy = policies[&f.key].clone();
    ag_policy.approval_principals.insert(
        "architecture.review".into(),
        BTreeSet::from(["architecture-reviewer".into()]),
    );
    records.approvals.insert(
        "architecture-review".into(),
        ApprovalRecord {
            principal: "architecture-reviewer".into(),
            purpose: "architecture.review".into(),
            action: ag_policy.action.clone(),
            binding: ag_policy.binding.clone(),
            contract_digest: ag_policy.contract_digest.clone(),
            validity: Validity {
                issued_at: now - 1,
                expires_at: now + 100,
                revoked: false,
            },
        },
    );
    let mut authority = JointAuthority {
        records,
        bundle,
        native,
        enabled: true,
        now,
        policy: ConsumptionPolicy {
            eligibility: ag_policy,
            required_capabilities: BTreeSet::from([
                "cargo.declarations".into(),
                "cargo.depends_on".into(),
            ]),
            required_approvals: BTreeMap::from([(
                "architecture.review".into(),
                "architecture-review".into(),
            )]),
        },
    };
    assert!(
        check_stage(
            bundle,
            &authority.policy,
            &authority.records,
            now,
            Selection {
                stage: Stage::OptInProtected,
                adapter_enabled: true,
                native_facts: Some(&authority.native)
            }
        )
        .required_check_satisfied
    );
    let bytes = |v: &Option<serde_json::Value>| serde_json::to_vec(v.as_ref().unwrap()).unwrap();
    let c = bytes(&bundle.contract);
    let facts = bytes(&bundle.facts);
    let report = bytes(&bundle.report);
    let store = MemoryRunStore::default();
    store.advance(&pending, 0).unwrap();
    store.reserve(&pending, "joint", 1).unwrap();
    let run = pending
        .evaluate(
            &[SpecialistEvidence {
                scope: &f.key,
                envelope: &bundle.envelope,
                artifacts: ArtifactBytes {
                    contract: &c,
                    facts: &facts,
                    report: &report,
                },
            }],
            &authority,
            now,
            scoped_support::TIME,
        )
        .unwrap();
    let record = scoped_support::FixtureIssuer::for_envelope(run.envelope()).0;
    authority
        .records
        .issuers
        .insert(record.envelope_digest.clone(), record);
    store.append(&run).unwrap();
    store.publish(&run.envelope().run_id, 1).unwrap();
    // Actual native analyzer observes a changed dependency; the stored bundle
    // remains the original immutable commit and cannot pass parity comparison.
    let manifest = f.directory.path().join("app/Cargo.toml");
    let original_manifest = std::fs::read(&manifest).unwrap();
    std::fs::write(
        &manifest,
        "[package]\nname=\"app\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .unwrap();
    let mismatch = archguard::CargoWorkspaceAnalyzer
        .analyze(f.directory.path(), "changed native input")
        .unwrap();
    std::fs::write(&manifest, original_manifest).unwrap();
    let mut route = LocalStageRoute::new_published(
        &plan,
        StageState::Accepted,
        flowguard::digest(b"legacy architecture"),
        &store,
    )
    .unwrap();
    route.submit().unwrap();
    for mode in [RouteMode::Advisory, RouteMode::Shadow] {
        route.set_mode(mode).unwrap();
        assert!(
            !route
                .qualify(&run, "stage-review", vec![], &authority, now)
                .unwrap()
        );
        assert!(route.consume(&authority, now).is_err());
    }
    route.set_mode(RouteMode::OptIn).unwrap();
    let original_native = std::mem::replace(&mut authority.native, mismatch.clone());
    assert!(
        route
            .qualify(&run, "stage-review", vec![], &authority, now)
            .is_err()
    );
    assert!(route.consume(&authority, now).is_err());
    authority.native = original_native;
    route
        .qualify(&run, "stage-review", vec![], &authority, now)
        .unwrap();
    assert_eq!(
        route.consume(&authority, now).unwrap(),
        StageState::Accepted
    );
    let frozen_required = run.envelope().coverage.required_scopes.clone();
    let ag_required = authority.policy.eligibility.required_scopes.clone();
    authority
        .policy
        .required_capabilities
        .insert("java.semantic-api.unavailable".into());
    assert!(route.consume(&authority, now).is_err());
    authority
        .policy
        .required_capabilities
        .remove("java.semantic-api.unavailable");
    assert!(route.consume(&authority, now).is_ok());

    let bundle_bytes = serde_json::to_vec(bundle).unwrap();
    let mut missing_domain: EvidenceBundle = serde_json::from_slice(&bundle_bytes).unwrap();
    missing_domain.domain = None;
    authority.bundle = &missing_domain;
    assert!(route.consume(&authority, now).is_err());
    authority.bundle = bundle;
    assert!(route.consume(&authority, now).is_ok());

    let mut missing_coverage: EvidenceBundle = serde_json::from_slice(&bundle_bytes).unwrap();
    let omitted = missing_coverage
        .envelope
        .coverage
        .observed_scopes
        .pop()
        .unwrap();
    missing_coverage
        .envelope
        .coverage
        .missing_scopes
        .push(omitted);
    missing_coverage.envelope.coverage.status = guardengine::integration::CoverageStatus::Partial;
    // Authenticate the exact incomplete observation too: refusal must not rely
    // merely on an unknown digest or a reduced protected required set.
    let partial_record = scoped_support::FixtureIssuer::for_envelope(&missing_coverage.envelope).0;
    authority
        .records
        .issuers
        .insert(partial_record.envelope_digest.clone(), partial_record);
    authority.bundle = &missing_coverage;
    assert!(route.consume(&authority, now).is_err());
    authority.bundle = bundle;
    assert!(route.consume(&authority, now).is_ok());
    assert_eq!(run.envelope().coverage.required_scopes, frozen_required);
    assert_eq!(authority.policy.eligibility.required_scopes, ag_required);
    assert_eq!(serde_json::to_vec(bundle).unwrap(), bundle_bytes);
    assert_eq!(original, serde_json::to_vec(&result).unwrap());
    authority.enabled = false;
    assert!(route.consume(&authority, now).is_err());
    authority.enabled = true;
    assert!(route.consume(&authority, now).is_ok());
    let original_native = std::mem::replace(&mut authority.native, mismatch);
    assert!(route.consume(&authority, now).is_err());
    authority.native = original_native;
    assert!(route.consume(&authority, now).is_ok());
    let history = serde_json::to_value(route.history()).unwrap();
    route.rollback().unwrap();
    authority
        .records
        .approvals
        .get_mut("architecture-review")
        .unwrap()
        .validity
        .revoked = true;
    route.set_mode(RouteMode::OptIn).unwrap();
    assert!(route.consume(&authority, now).is_err());
    assert_eq!(
        &serde_json::to_value(route.history())
            .unwrap()
            .as_array()
            .unwrap()[..history.as_array().unwrap().len()],
        history.as_array().unwrap()
    );
    assert_eq!(original, serde_json::to_vec(&result).unwrap());
    eprintln!(
        "FLOWGUARD_ARCHGUARD_JOINT {}",
        serde_json::json!({
            "schema": "flowguard.archguard-local-joint/v1", "authority": "test-only protected caller",
            "archguard_envelope_digest": flowguard::digest(&serde_json::to_vec(&bundle.envelope).unwrap()),
            "flowguard_envelope_digest": flowguard::digest(&serde_json::to_vec(run.envelope()).unwrap()),
            "plan_digest": plan.digest(), "trusted_check_each_refresh": true, "same_contract_native_parity": true, "unsupported_capability_refused": true, "missing_domain_refused": true, "missing_coverage_refused": true, "frozen_required_set_preserved": true,
            "actual_native_mismatch_refused": true, "mismatch_promotion_refused": true, "advisory_shadow_no_transfer": true, "disabled_adapter_refused": true,
            "rollback_reenable_revocation_refused": true, "original_bundle_preserved": true, "history_prefix_preserved": true,
            "host_enforced": false
        })
    );
}
