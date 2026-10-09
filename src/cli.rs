//! Read-only command transport. No installed host or execution authority.
use crate::gate::*;
use crate::input_limits::AllowedRoot;
use guardengine::integration::eligibility::*;
use guardengine::integration::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub struct CliOutput {
    pub code: u8,
    pub stdout: Vec<u8>,
    pub stderr: String,
}
fn failure() -> CliOutput {
    crate::transport::prebinding_error()
}
fn json(code: u8, value: impl Serialize) -> Result<CliOutput, ()> {
    let mut stdout = serde_json::to_vec(&value).map_err(|_| ())?;
    stdout.push(b'\n');
    Ok(CliOutput {
        code,
        stdout,
        stderr: String::new(),
    })
}
struct Options {
    values: BTreeMap<String, String>,
    flags: BTreeSet<String>,
}
impl Options {
    fn parse(args: &[String], values: &[&str], flags: &[&str]) -> Result<Self, ()> {
        let mut out = Self {
            values: BTreeMap::new(),
            flags: BTreeSet::new(),
        };
        let mut i = 0;
        while i < args.len() {
            let name = &args[i];
            if flags.contains(&name.as_str()) {
                if !out.flags.insert(name.clone()) {
                    return Err(());
                }
                i += 1;
            } else if values.contains(&name.as_str()) {
                let value = args
                    .get(i + 1)
                    .filter(|v| !v.starts_with("--") && !v.is_empty())
                    .ok_or(())?;
                if out.values.insert(name.clone(), value.clone()).is_some() {
                    return Err(());
                }
                i += 2;
            } else {
                return Err(());
            }
        }
        Ok(out)
    }
    fn get(&self, name: &str) -> Result<&str, ()> {
        self.values.get(name).map(String::as_str).ok_or(())
    }
}
pub fn run(args: Vec<String>) -> CliOutput {
    if args.len() > 32 || args.iter().map(String::len).sum::<usize>() > 16384 {
        return failure();
    }
    dispatch(&args).unwrap_or_else(|_| failure())
}
fn dispatch(args: &[String]) -> Result<CliOutput, ()> {
    let command = args.first().ok_or(())?;
    match (command.as_str(), args.get(1).map(String::as_str)) {
        ("discover", _) => discovery(&args[1..], false),
        ("stage", Some("status")) => discovery(&args[2..], true),
        ("evidence", Some("verify")) => verify(&args[2..]),
        ("gate", Some("check")) => gate(&args[2..]),
        _ => Err(()),
    }
}
fn discovery(args: &[String], status: bool) -> Result<CliOutput, ()> {
    let o = Options::parse(args, &["--root", "--feature"], &[])?;
    let root = AllowedRoot::new(o.get("--root")?, 1024 * 1024).map_err(|_| ())?;
    let inventory =
        crate::stage::discover(&root, o.get("--feature")?, "flowguard.docs/v1").map_err(|_| ())?;
    json(
        if inventory.complete() { 0 } else { 2 },
        serde_json::json!({"apiVersion":"flowguard.cli/v1alpha1","command":if status{"stage status"}else{"discover"},"qualification":"not_evaluated","inventory":inventory}),
    )
}
fn verify(args: &[String]) -> Result<CliOutput, ()> {
    let o = Options::parse(
        args,
        &[
            "--root",
            "--envelope",
            "--contract",
            "--facts",
            "--report-file",
        ],
        &[],
    )?;
    let root = AllowedRoot::new(o.get("--root")?, MAX_ARTIFACT_BYTES).map_err(|_| ())?;
    let mut budget = 0;
    let envelope = load_envelope_json(
        &read(&root, o.get("--envelope")?, &mut budget)?,
        EvidenceProfile::EngineBacked,
    )
    .map_err(|_| ())?;
    let contract = read(&root, o.get("--contract")?, &mut budget)?;
    let facts = read(&root, o.get("--facts")?, &mut budget)?;
    let report = read(&root, o.get("--report-file")?, &mut budget)?;
    let checked = verify_engine_artifacts(&envelope, &contract, &facts, &report).map_err(|_| ())?;
    json(
        0,
        serde_json::json!({"apiVersion":"flowguard.cli/v1alpha1","command":"evidence verify","valid":true,"authority":"not_evaluated","decision":checked.decision}),
    )
}
fn read(root: &AllowedRoot, path: &str, total: &mut usize) -> Result<Vec<u8>, ()> {
    let bytes = root.read(path).map_err(|_| ())?;
    *total = total.saturating_add(bytes.len());
    if *total > MAX_ARTIFACT_BYTES {
        return Err(());
    }
    Ok(bytes)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GateInput {
    version: String,
    invocation: crate::context::InvocationInput,
    candidate: String,
    frozen: String,
    frozen_digest: String,
    action: String,
    started_at: String,
    finished_at: String,
    now: i64,
    specialists: Vec<SpecialistInput>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecialistInput {
    scope: String,
    producer: Producer,
    producer_principals: BTreeSet<String>,
    approval_principals: BTreeMap<String, BTreeSet<String>>,
    evidence: Option<EvidencePaths>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidencePaths {
    envelope: String,
    contract: String,
    facts: String,
    report: String,
}
struct Evidence {
    scope: String,
    envelope: GuardRunEnvelope,
    contract: Vec<u8>,
    facts: Vec<u8>,
    report: Vec<u8>,
}
fn pinned(
    root: &AllowedRoot,
    path: &str,
    expected: &str,
    total: &mut usize,
) -> Result<Vec<u8>, ()> {
    let bytes = read(root, path, total)?;
    if !crate::valid_digest(expected) || crate::digest(&bytes) != expected {
        return Err(());
    }
    Ok(bytes)
}
fn gate(args: &[String]) -> Result<CliOutput, ()> {
    let o = Options::parse(
        args,
        &[
            "--repo",
            "--input-root",
            "--request",
            "--request-digest",
            "--run-id",
            "--fixture-authority-root",
            "--fixture-authority",
            "--fixture-authority-digest",
        ],
        &["--cancel", "--local-fixture-authority"],
    )?;
    let fixture = o.flags.contains("--local-fixture-authority");
    let authority_names = [
        "--fixture-authority-root",
        "--fixture-authority",
        "--fixture-authority-digest",
    ];
    if fixture {
        for name in authority_names {
            o.get(name)?;
        }
    } else if authority_names
        .iter()
        .any(|name| o.values.contains_key(*name))
    {
        return Err(());
    }
    let root = AllowedRoot::new(o.get("--input-root")?, MAX_ARTIFACT_BYTES).map_err(|_| ())?;
    let mut budget = 0;
    let bytes = pinned(
        &root,
        o.get("--request")?,
        o.get("--request-digest")?,
        &mut budget,
    )?;
    if bytes.len() > 1024 * 1024 {
        return Err(());
    }
    let input: GateInput = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if input.version != "flowguard.cli-request/v1alpha1"
        || input.specialists.len() > 64
        || input.invocation.repo_candidates.len() != 1
        || input.started_at.len() != 20
        || input.finished_at.len() != 20
        || !input.started_at.ends_with('Z')
        || !input.finished_at.ends_with('Z')
        || input.finished_at < input.started_at
    {
        return Err(());
    }
    let repo = gitguard::Repository::discover(
        std::path::Path::new(o.get("--repo")?),
        &input.invocation.repo_candidates[0],
    )
    .map_err(|_| ())?;
    let candidate: gitguard::candidate::CandidateSnapshot =
        serde_json::from_slice(&read(&root, &input.candidate, &mut budget)?).map_err(|_| ())?;
    let binding = crate::context::bind(&input.invocation, &repo, &candidate).map_err(|_| ())?;
    let frozen =
        crate::obligations::FrozenObligations::from_json(&read(&root, &input.frozen, &mut budget)?)
            .map_err(|_| ())?;
    if frozen.digest() != input.frozen_digest {
        return Err(());
    }
    let mut configs = BTreeMap::new();
    for config in &input.specialists {
        if configs.insert(config.scope.clone(), config).is_some() {
            return Err(());
        }
    }
    if configs.len() != frozen.obligations().len() {
        return Err(());
    }
    let mut policies = BTreeMap::new();
    for obligation in frozen.obligations() {
        let scope = obligation_scope(obligation);
        let config = configs.get(&scope).ok_or(())?;
        policies.insert(
            scope,
            EligibilityPolicy {
                binding: binding.binding().clone(),
                producer: config.producer.clone(),
                required_scopes: obligation.coverage.iter().cloned().collect(),
                contract_digest: obligation.rules_digest.clone(),
                action: input.action.clone(),
                producer_principals: config.producer_principals.clone(),
                approval_principals: config.approval_principals.clone(),
            },
        );
    }
    // Validate both controller-supplied timestamps before establishing the actual attempt.
    prepare_gate(
        &binding,
        &frozen,
        policies.clone(),
        GateRequest {
            run_id: o.get("--run-id")?.into(),
            action: input.action.clone(),
            started_at: input.finished_at.clone(),
        },
    )
    .map_err(|_| ())?;
    let fallback_policies = policies.clone();
    let pending = prepare_gate(
        &binding,
        &frozen,
        policies,
        GateRequest {
            run_id: o.get("--run-id")?.into(),
            action: input.action.clone(),
            started_at: input.started_at.clone(),
        },
    )
    .map_err(|_| ())?;
    if o.flags.contains("--cancel") {
        return gate_json(pending.cancel(&input.finished_at).map_err(|_| ())?, fixture);
    }
    let authority = if fixture {
        match fixture_authority(&o) {
            Ok(provider) => provider,
            Err(_) => {
                return gate_json(
                    pending.input_failed(&input.finished_at).map_err(|_| ())?,
                    true,
                );
            }
        }
    } else {
        FixtureAuthority::default()
    };
    let mut evidence = Vec::new();
    for config in &input.specialists {
        if let Some(paths) = &config.evidence {
            let mut load = || -> Result<Evidence, ()> {
                Ok(Evidence {
                    scope: config.scope.clone(),
                    envelope: load_envelope_json(
                        &read(&root, &paths.envelope, &mut budget)?,
                        EvidenceProfile::EngineBacked,
                    )
                    .map_err(|_| ())?,
                    contract: read(&root, &paths.contract, &mut budget)?,
                    facts: read(&root, &paths.facts, &mut budget)?,
                    report: read(&root, &paths.report, &mut budget)?,
                })
            };
            match load() {
                Ok(e) => evidence.push(e),
                Err(_) => {
                    return gate_json(
                        pending.input_failed(&input.finished_at).map_err(|_| ())?,
                        fixture,
                    );
                }
            }
        }
    }
    let views: Vec<_> = evidence
        .iter()
        .map(|e| SpecialistEvidence {
            scope: &e.scope,
            envelope: &e.envelope,
            artifacts: ArtifactBytes {
                contract: &e.contract,
                facts: &e.facts,
                report: &e.report,
            },
        })
        .collect();
    let run = pending
        .evaluate(&views, &authority, input.now, &input.finished_at)
        .or_else(|_| {
            // The same already-resolved immutable inputs support bound transport even
            // if the evaluator rejects its final encoding/verification output.
            prepare_gate(
                &binding,
                &frozen,
                fallback_policies,
                GateRequest {
                    run_id: o.get("--run-id").expect("validated option").into(),
                    action: input.action.clone(),
                    started_at: input.started_at.clone(),
                },
            )?
            .input_failed(&input.finished_at)
        })
        .map_err(|_| ())?;
    gate_json(run, fixture)
}
fn gate_json(run: GateRun, fixture: bool) -> Result<CliOutput, ()> {
    crate::transport::encode_gate(&run, fixture).map_err(|_| ())
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureAuthority {
    version: String,
    producers: Vec<ProducerReceipt>,
    approvals: Vec<ApprovalReceipt>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProducerReceipt {
    principal: String,
    producer: Producer,
    envelope_digest: String,
    issued_at: i64,
    expires_at: i64,
    revoked: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalReceipt {
    reference: String,
    principal: String,
    purpose: String,
    action: String,
    binding: RunBinding,
    contract_digest: String,
    issued_at: i64,
    expires_at: i64,
    revoked: bool,
}
fn fixture_authority(options: &Options) -> Result<FixtureAuthority, ()> {
    let root =
        AllowedRoot::new(options.get("--fixture-authority-root")?, 1024 * 1024).map_err(|_| ())?;
    let bytes = pinned(
        &root,
        options.get("--fixture-authority")?,
        options.get("--fixture-authority-digest")?,
        &mut 0,
    )?;
    let provider: FixtureAuthority = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if provider.version != "flowguard.authority-fixture/v1alpha1"
        || provider.producers.len() > 64
        || provider.approvals.len() > 64
    {
        return Err(());
    }
    let mut digests = BTreeSet::new();
    for record in &provider.producers {
        if !crate::valid_digest(&record.envelope_digest)
            || record.principal.trim().is_empty()
            || !digests.insert(&record.envelope_digest)
        {
            return Err(());
        }
    }
    let mut refs = BTreeSet::new();
    for record in &provider.approvals {
        if record.reference.trim().is_empty()
            || record.principal.trim().is_empty()
            || !crate::valid_digest(&record.contract_digest)
            || !refs.insert(&record.reference)
        {
            return Err(());
        }
    }
    Ok(provider)
}
impl AuthorityProvider for FixtureAuthority {
    fn verify_producer(
        &self,
        envelope: &GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if self.version.is_empty() {
            return Err(AuthorityError::Unavailable);
        }
        let receipt = self
            .producers
            .iter()
            .find(|r| r.envelope_digest == digest && r.producer == envelope.producer)
            .ok_or(AuthorityError::Untrusted)?;
        Ok(ProducerRecord {
            principal: receipt.principal.clone(),
            producer: receipt.producer.clone(),
            envelope_digest: receipt.envelope_digest.clone(),
            validity: Validity {
                issued_at: receipt.issued_at,
                expires_at: receipt.expires_at,
                revoked: receipt.revoked,
            },
        })
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        if self.version.is_empty() {
            return Err(AuthorityError::Unavailable);
        }
        let receipt = self
            .approvals
            .iter()
            .find(|r| r.reference == reference)
            .ok_or(AuthorityError::Untrusted)?;
        Ok(ApprovalRecord {
            principal: receipt.principal.clone(),
            purpose: receipt.purpose.clone(),
            action: receipt.action.clone(),
            binding: receipt.binding.clone(),
            contract_digest: receipt.contract_digest.clone(),
            validity: Validity {
                issued_at: receipt.issued_at,
                expires_at: receipt.expires_at,
                revoked: receipt.revoked,
            },
        })
    }
}
