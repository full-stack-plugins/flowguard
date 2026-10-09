//! Advisory workflow gate: actual engine reports and controller-side eligibility
//! ports, with no production authority provider or execution grant.
use crate::{
    context::ValidatedBinding,
    obligations::{EvidenceObligation, FrozenObligations},
    projection::Gap,
};
use guardengine::{
    Decision,
    integration::{eligibility::*, *},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateVersion {
    #[serde(rename = "flowguard.gate/v1alpha1")]
    V1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateKind {
    GateDecision,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionQualification {
    Eligible,
    Blocked,
    ReviewRequired,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityProfile {
    Advisory,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GateDecision {
    pub api_version: GateVersion,
    pub kind: GateKind,
    pub authority_profile: AuthorityProfile,
    pub mapping_version: String,
    pub action: String,
    pub binding_digest: String,
    pub frozen_obligations_digest: String,
    pub decision: Decision,
    pub qualification: ActionQualification,
    pub gaps: Vec<Gap>,
    pub report: ArtifactRef,
    pub upstream_reports: Vec<ArtifactRef>,
}
pub struct GateRequest {
    pub run_id: String,
    pub action: String,
    pub started_at: String,
}
pub struct SpecialistEvidence<'a> {
    pub scope: &'a str,
    pub envelope: &'a GuardRunEnvelope,
    pub artifacts: ArtifactBytes<'a>,
}
pub struct PendingGate {
    run_id: String,
    attempt: BoundAttempt,
    binding: RunBinding,
    binding_digest: String,
    frozen_digest: String,
    policies: BTreeMap<String, EligibilityPolicy>,
    required: Vec<String>,
    observed: BTreeSet<String>,
    action: String,
}
struct RetainedEvidence {
    envelope: GuardRunEnvelope,
    contract: Vec<u8>,
    facts: Vec<u8>,
    report: Vec<u8>,
    policy: EligibilityPolicy,
}
#[derive(Debug, PartialEq, Eq)]
pub enum GateConsumptionError {
    ActionMismatch,
    IncompleteRun,
    SpecialistChanged(EligibilityCode),
}
pub struct GateRun {
    work_digest: String,
    envelope: GuardRunEnvelope,
    domain: Option<GateDecision>,
    contract: Vec<u8>,
    facts: Vec<u8>,
    report: Vec<u8>,
    retained: Vec<RetainedEvidence>,
}
impl GateRun {
    pub(crate) fn work_digest(&self) -> &str {
        &self.work_digest
    }
    pub fn envelope(&self) -> &GuardRunEnvelope {
        &self.envelope
    }
    pub fn domain(&self) -> Option<&GateDecision> {
        self.domain.as_ref()
    }
    pub fn artifacts(&self) -> Option<ArtifactBytes<'_>> {
        self.domain.as_ref().map(|_| ArtifactBytes {
            contract: &self.contract,
            facts: &self.facts,
            report: &self.report,
        })
    }
    /// Recheck every retained specialist against freshly queried authority at
    /// consumption. A cached FlowGuard ALLOW cannot cache away upstream revocation.
    pub fn evaluate_eligibility(
        &self,
        policy: &EligibilityPolicy,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<EligibilityResult, GateConsumptionError> {
        let domain = self
            .domain
            .as_ref()
            .ok_or(GateConsumptionError::IncompleteRun)?;
        if domain.action != policy.action {
            return Err(GateConsumptionError::ActionMismatch);
        }
        for evidence in &self.retained {
            let current = guardengine::integration::eligibility::evaluate_eligibility(
                &evidence.envelope,
                ArtifactBytes {
                    contract: &evidence.contract,
                    facts: &evidence.facts,
                    report: &evidence.report,
                },
                &evidence.policy,
                provider,
                now,
                None,
            );
            if !current.eligible {
                return Err(GateConsumptionError::SpecialistChanged(current.code));
            }
        }
        let artifacts = self
            .artifacts()
            .ok_or(GateConsumptionError::IncompleteRun)?;
        Ok(guardengine::integration::eligibility::evaluate_eligibility(
            &self.envelope,
            artifacts,
            policy,
            provider,
            now,
            None,
        ))
    }
}
pub fn obligation_scope(o: &EvidenceObligation) -> String {
    format!(
        "flowguard.obligation:{}",
        crate::digest(&serde_json::to_vec(o).expect("closed obligation"))
    )
}
/// Policy and frozen obligations must come from the protected controller, never
/// candidate-edited configuration. This local library does not authenticate them.
pub fn prepare_gate(
    binding: &ValidatedBinding,
    frozen: &FrozenObligations,
    policies: BTreeMap<String, EligibilityPolicy>,
    request: GateRequest,
) -> Result<PendingGate, TransportDiagnostic> {
    prepare_with_sources(binding, frozen, None, policies, request)
}
/// Opt-in scoped profile: expected source pins must be independently frozen by
/// the controller. All other native binding fields retain exact equality.
pub fn prepare_scoped_gate(
    binding: &ValidatedBinding,
    frozen: &FrozenObligations,
    sources: &crate::evidence::ScopedSources,
    policies: BTreeMap<String, EligibilityPolicy>,
    request: GateRequest,
) -> Result<PendingGate, TransportDiagnostic> {
    sources
        .validate_for(binding, frozen)
        .map_err(|_| error("gate.source_profile"))?;
    prepare_with_sources(binding, frozen, Some(sources), policies, request)
}
fn prepare_with_sources(
    binding: &ValidatedBinding,
    frozen: &FrozenObligations,
    sources: Option<&crate::evidence::ScopedSources>,
    policies: BTreeMap<String, EligibilityPolicy>,
    request: GateRequest,
) -> Result<PendingGate, TransportDiagnostic> {
    if frozen.context_digest() != binding.domain_digest()
        || request.action.trim().is_empty()
        || request.action.len() > 128
    {
        return Err(error("gate.binding_mismatch"));
    }
    let mut required = BTreeSet::new();
    let mut observed = BTreeSet::new();
    for anchor in [
        format!("flowguard.frozen:{}", frozen.digest()),
        format!(
            "flowguard.action:{}",
            crate::digest(request.action.as_bytes())
        ),
    ] {
        required.insert(anchor.clone());
        observed.insert(anchor);
    }
    let git_scope = format!("git.scope:{}", binding.candidate_scope_digest());
    required.insert(git_scope.clone());
    observed.insert(git_scope);
    if let Some(sources) = sources {
        let anchor = format!("flowguard.sources:{}", sources.digest());
        required.insert(anchor.clone());
        observed.insert(anchor);
    }
    for obligation in frozen.obligations() {
        let scope = obligation_scope(obligation);
        let policy = policies
            .get(&scope)
            .ok_or_else(|| error("gate.policy_missing"))?;
        let expected = crate::guard_adapters::expected_binding(binding.binding(), sources, &scope)
            .map_err(|_| error("gate.source_profile"))?;
        if policy.binding != expected
            || policy.action != request.action
            || policy.producer.guard != obligation.guard
            || policy.producer.analyzer_version != obligation.analyzer_version
            || policy.contract_digest != obligation.rules_digest
            || policy.required_scopes != obligation.coverage.iter().cloned().collect::<Vec<_>>()
        {
            return Err(error("gate.policy_mismatch"));
        }
        required.insert(scope);
        let stage_scope = format!("flowguard.stage:{}", obligation.stage_id);
        required.insert(stage_scope.clone());
        if !frozen.missing_stages().contains(&obligation.stage_id) {
            observed.insert(stage_scope);
        }
    }
    if policies.len() != frozen.obligations().len()
        || required.len() > 64
        || required.iter().any(|s| s.len() > 256)
    {
        return Err(error("gate.policy_scope"));
    }
    let required: Vec<_> = required.into_iter().collect();
    let run_id = request.run_id.clone();
    let attempt = prepare_attempt(InvocationDraft {
        run_id: request.run_id,
        producer: Some(Producer {
            guard: "flowguard".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            analyzer_id: "flowguard.stage-gates".into(),
            analyzer_version: env!("CARGO_PKG_VERSION").into(),
        }),
        binding: Some(binding.binding().clone()),
        coverage: Some(coverage(&required, &observed)),
        profile: Some(EvidenceProfile::EngineBacked),
        started_at: request.started_at,
    })?;
    Ok(PendingGate {
        run_id,
        attempt,
        binding: binding.binding().clone(),
        binding_digest: binding.domain_digest(),
        frozen_digest: frozen.digest().into(),
        policies,
        required,
        observed,
        action: request.action,
    })
}
impl PendingGate {
    pub(crate) fn store_identity(&self) -> crate::run_store::WorkIdentity {
        let target = crate::digest(
            &serde_json::to_vec(&(
                &self.binding.repo_id,
                &self.binding.task_id,
                &self.binding.worktree_id,
                &self.binding.requirement_ids,
                &self.action,
            ))
            .expect("closed store target"),
        );
        let digest = crate::digest(
            &serde_json::to_vec(&(
                "flowguard.work/v1alpha1",
                &self.binding,
                &self.binding_digest,
                &self.frozen_digest,
                &self.required,
                &self.action,
                &self.policies,
                crate::projection::MAPPING_VERSION,
                env!("CARGO_PKG_VERSION"),
            ))
            .expect("closed work identity"),
        );
        crate::run_store::WorkIdentity {
            target,
            digest,
            run_id: self.run_id.clone(),
            binding: self.binding.clone(),
            required: self.required.clone(),
        }
    }

    pub fn required_scopes(&self) -> &[String] {
        &self.required
    }
    pub fn evaluate(
        mut self,
        evidence: &[SpecialistEvidence<'_>],
        provider: &dyn AuthorityProvider,
        now: i64,
        finished_at: &str,
    ) -> Result<GateRun, TransportDiagnostic> {
        let work_digest = self.store_identity().digest;
        let mut seen = BTreeSet::new();
        let mut gaps = Vec::new();
        let mut upstream_reports = Vec::new();
        let mut approvals = BTreeSet::new();
        let mut retained = Vec::new();
        let mut total_bytes = 0usize;
        if evidence.len() > self.policies.len() {
            return self.failed(RunStatus::Error, "gate.unexpected_evidence", finished_at);
        }
        for input in evidence {
            let Some(policy) = self.policies.get(input.scope) else {
                return self.failed(RunStatus::Error, "gate.unexpected_evidence", finished_at);
            };
            if !seen.insert(input.scope) {
                return self.failed(RunStatus::Error, "gate.duplicate_evidence", finished_at);
            }
            total_bytes = total_bytes
                .saturating_add(input.artifacts.contract.len())
                .saturating_add(input.artifacts.facts.len())
                .saturating_add(input.artifacts.report.len())
                .saturating_add(
                    serde_json::to_vec(input.envelope)
                        .expect("closed envelope")
                        .len(),
                );
            if total_bytes > MAX_ARTIFACT_BYTES {
                return self.failed(RunStatus::Error, "gate.evidence_budget", finished_at);
            }
            let result = guardengine::integration::eligibility::evaluate_eligibility(
                input.envelope,
                ArtifactBytes {
                    contract: input.artifacts.contract,
                    facts: input.artifacts.facts,
                    report: input.artifacts.report,
                },
                policy,
                provider,
                now,
                None,
            );
            use EligibilityCode::*;
            match result.code {
                Eligible => {
                    self.observed.insert(input.scope.into());
                }
                MissingApproval => {
                    self.observed.insert(input.scope.into());
                    gaps.push(Gap {
                        scope: input.scope.into(),
                        kind: crate::projection::GapKind::Review,
                        code: "specialist.approval_missing".into(),
                    });
                }
                TechnicalBlock => {
                    self.observed.insert(input.scope.into());
                    gaps.push(Gap {
                        scope: input.scope.into(),
                        kind: crate::projection::GapKind::Block,
                        code: "specialist.technical_block".into(),
                    });
                }
                Incomplete => {
                    gaps.push(Gap {
                        scope: input.scope.into(),
                        kind: crate::projection::GapKind::Block,
                        code: "specialist.incomplete".into(),
                    });
                }
                _ => {
                    return self.failed(
                        RunStatus::Error,
                        &format!("specialist.{:?}", result.code),
                        finished_at,
                    );
                }
            }
            if let Some(reference) = &input.envelope.artifacts.report {
                upstream_reports.push(reference.clone());
            }
            approvals.extend(input.envelope.approval_refs.iter().cloned());
            retained.push(RetainedEvidence {
                envelope: input.envelope.clone(),
                contract: input.artifacts.contract.to_vec(),
                facts: input.artifacts.facts.to_vec(),
                report: input.artifacts.report.to_vec(),
                policy: policy.clone(),
            });
        }
        for scope in &self.required {
            if !self.observed.contains(scope) && !gaps.iter().any(|g| &g.scope == scope) {
                gaps.push(Gap {
                    scope: scope.clone(),
                    kind: crate::projection::GapKind::Block,
                    code: "workflow.required_observation_missing".into(),
                });
            }
        }
        gaps.sort_by(|a, b| (&a.scope, &a.code).cmp(&(&b.scope, &b.code)));
        upstream_reports.sort_by(|a, b| (&a.digest, &a.uri).cmp(&(&b.digest, &b.uri)));
        upstream_reports.dedup();
        let observed: Vec<_> = self.observed.iter().cloned().collect();
        let projection = match crate::projection::project(
            &self.binding.source_snapshot_digest,
            &self.required,
            &observed,
            &gaps,
        ) {
            Ok(p) => p,
            Err(_) => return self.failed(RunStatus::Error, "gate.projection_invalid", finished_at),
        };
        let report = match guardengine::integration::evaluate_bounded(
            &projection.contract,
            &projection.facts,
        ) {
            Ok(report) => report,
            Err(_) => return self.failed(RunStatus::Error, "gate.engine_error", finished_at),
        };
        let contract_bytes =
            serde_json::to_vec(&projection.contract).map_err(|_| error("gate.encoding"))?;
        let facts_bytes =
            serde_json::to_vec(&projection.facts).map_err(|_| error("gate.encoding"))?;
        let report_bytes = serde_json::to_vec(&report).map_err(|_| error("gate.encoding"))?;
        let report_ref = artifact(&report_bytes);
        let domain = GateDecision {
            api_version: GateVersion::V1,
            kind: GateKind::GateDecision,
            authority_profile: AuthorityProfile::Advisory,
            mapping_version: crate::projection::MAPPING_VERSION.into(),
            action: self.action,
            binding_digest: self.binding_digest,
            frozen_obligations_digest: self.frozen_digest,
            qualification: match report.decision {
                Decision::Allow => ActionQualification::Eligible,
                Decision::Block => ActionQualification::Blocked,
                Decision::RequireApproval => ActionQualification::ReviewRequired,
            },
            decision: report.decision.clone(),
            gaps,
            report: report_ref.clone(),
            upstream_reports,
        };
        let domain_bytes = serde_json::to_vec(&domain).map_err(|_| error("gate.encoding"))?;
        let envelope = self.attempt.finish(AttemptOutput {
            coverage: coverage(&self.required, &self.observed),
            run_status: RunStatus::Completed,
            decision: Some(report.decision),
            artifacts: Artifacts {
                contract: Some(artifact(&contract_bytes)),
                facts: Some(artifact(&facts_bytes)),
                report: Some(report_ref),
                domain: vec![artifact(&domain_bytes)],
            },
            approval_refs: approvals.into_iter().collect(),
            diagnostics: vec![],
            finished_at: finished_at.into(),
            expires_at: None,
        })?;
        verify_engine_artifacts(&envelope, &contract_bytes, &facts_bytes, &report_bytes)
            .map_err(|_| error("gate.self_verification"))?;
        Ok(GateRun {
            work_digest,
            envelope,
            domain: Some(domain),
            contract: contract_bytes,
            facts: facts_bytes,
            report: report_bytes,
            retained,
        })
    }
    pub(crate) fn input_failed(self, finished_at: &str) -> Result<GateRun, TransportDiagnostic> {
        self.failed(RunStatus::Error, "gate.input_unavailable", finished_at)
    }
    pub fn cancel(self, finished_at: &str) -> Result<GateRun, TransportDiagnostic> {
        self.failed(RunStatus::Cancelled, "gate.cancelled", finished_at)
    }
    fn failed(
        self,
        status: RunStatus,
        code: &str,
        finished_at: &str,
    ) -> Result<GateRun, TransportDiagnostic> {
        let work_digest = self.store_identity().digest;
        let envelope = self.attempt.finish(AttemptOutput {
            coverage: coverage(&self.required, &self.observed),
            run_status: status,
            decision: None,
            artifacts: Artifacts {
                contract: None,
                facts: None,
                report: None,
                domain: vec![],
            },
            approval_refs: vec![],
            diagnostics: vec![Diagnostic {
                code: code.into(),
                message: "workflow attempt did not establish usable evidence".into(),
                retryable: false,
                source: None,
            }],
            finished_at: finished_at.into(),
            expires_at: None,
        })?;
        Ok(GateRun {
            work_digest,
            envelope,
            domain: None,
            contract: vec![],
            facts: vec![],
            report: vec![],
            retained: vec![],
        })
    }
}
fn coverage(required: &[String], observed: &BTreeSet<String>) -> Coverage {
    let missing_scopes: Vec<_> = required
        .iter()
        .filter(|s| !observed.contains(*s))
        .cloned()
        .collect();
    Coverage {
        status: if missing_scopes.is_empty() {
            CoverageStatus::Complete
        } else {
            CoverageStatus::Partial
        },
        required_scopes: required.to_vec(),
        observed_scopes: observed.iter().cloned().collect(),
        missing_scopes,
    }
}
fn artifact(bytes: &[u8]) -> ArtifactRef {
    let digest = crate::digest(bytes);
    ArtifactRef {
        uri: format!("artifact://flowguard/{}", &digest[7..]),
        digest,
        media_type: "application/json".into(),
    }
}
fn error(code: &str) -> TransportDiagnostic {
    TransportDiagnostic {
        code: code.into(),
        message: code.into(),
    }
}
/// Structural domain validation only; artifact consistency and authority remain
/// the responsibility of the engine-backed envelope and controller ports.
pub fn load_gate_decision(bytes: &[u8]) -> Result<GateDecision, &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("gate domain budget");
    }
    let domain: GateDecision =
        serde_json::from_slice(bytes).map_err(|_| "invalid gate domain schema")?;
    if domain.mapping_version != crate::projection::MAPPING_VERSION
        || domain.action.trim().is_empty()
        || domain.action.len() > 128
        || !crate::valid_digest(&domain.binding_digest)
        || !crate::valid_digest(&domain.frozen_obligations_digest)
        || !crate::valid_digest(&domain.report.digest)
        || domain.report.media_type.trim().is_empty()
        || domain.report.uri != format!("artifact://flowguard/{}", &domain.report.digest[7..])
        || domain.gaps.len() > 192
        || domain.upstream_reports.len() > 64
    {
        return Err("invalid gate domain binding");
    }
    if domain.gaps.iter().any(|g| {
        g.scope.trim().is_empty()
            || g.scope.len() > 256
            || g.code.trim().is_empty()
            || g.code.len() > 128
    }) || domain.upstream_reports.iter().any(|r| {
        !crate::valid_digest(&r.digest)
            || !r.uri.starts_with("artifact://")
            || r.media_type.trim().is_empty()
    }) {
        return Err("invalid gate domain references");
    }
    let qualifier = match domain.decision {
        Decision::Allow => ActionQualification::Eligible,
        Decision::Block => ActionQualification::Blocked,
        Decision::RequireApproval => ActionQualification::ReviewRequired,
    };
    if domain.qualification != qualifier
        || (domain.decision == Decision::Allow
            && domain
                .gaps
                .iter()
                .any(|g| g.kind != crate::projection::GapKind::Advise))
        || (domain.decision == Decision::RequireApproval
            && (domain
                .gaps
                .iter()
                .any(|g| g.kind == crate::projection::GapKind::Block)
                || !domain
                    .gaps
                    .iter()
                    .any(|g| g.kind == crate::projection::GapKind::Review)))
    {
        return Err("inconsistent gate qualification");
    }
    Ok(domain)
}
