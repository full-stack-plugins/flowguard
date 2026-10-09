#![allow(dead_code)]
//! Actual five-provider fixture. No production authority.
use guardengine::integration::{EvidenceProfile, GuardRunEnvelope, load_envelope_json};
use std::path::PathBuf;
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/providers/approval-bridge/actual")
}
pub fn bytes(provider: &str, name: &str) -> Vec<u8> {
    std::fs::read(root().join(provider).join(name)).expect("actual generated provider artifact")
}
pub fn envelope(provider: &str) -> GuardRunEnvelope {
    load_envelope_json(
        &bytes(provider, "envelope.json"),
        EvidenceProfile::EngineBacked,
    )
    .unwrap()
}

use flowguard::{context::*, evidence::ScopedSources, gate::*, obligations::FrozenObligations};
use guardengine::integration::eligibility::*;
use std::collections::BTreeMap;
pub const TIME: &str = "2033-05-18T03:33:20Z";
pub const NOW: i64 = 2_000_000_000;
pub const PROVIDERS: [&str; 5] = [
    "archguard",
    "codeguard",
    "testguard",
    "gitguard",
    "specguard",
];
pub fn read<T: serde::de::DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&std::fs::read(root().join(name)).unwrap()).unwrap()
}
pub struct Fixture {
    pub repo_root: PathBuf,
    _directory: tempfile::TempDir,
    pub binding: ValidatedBinding,
    pub frozen: FrozenObligations,
    pub sources: ScopedSources,
    pub policies: BTreeMap<String, EligibilityPolicy>,
    pub scopes: BTreeMap<String, String>,
}
impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let repo_path = directory.path().join("repo");
        let output = std::process::Command::new("/usr/bin/git")
            .args(["clone", "--quiet"])
            .arg(root().join("source.bundle"))
            .arg(&repo_path)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let repo = gitguard::Repository::discover(&repo_path, "cross-guard-repo").unwrap();
        let candidate: gitguard::candidate::CandidateSnapshot = read("gitguard/candidate.json");
        candidate.validate(&repo).unwrap();
        let ac: gitguard::candidate::CandidateSnapshot = read("archguard/candidate.json");
        archguard::integration::binding::GitEvidenceBundle::load(
            &bytes("archguard", "bundle.json"),
            &repo,
            &ac,
        )
        .unwrap();
        assert_ne!(candidate.policy_digest(), ac.policy_digest());
        let b: guardengine::integration::RunBinding = read("binding.json");
        let binding = bind(
            &InvocationInput {
                repo_candidates: vec![b.repo_id],
                task_candidates: vec![b.task_id],
                worktree_id: b.worktree_id,
                requirement_ids: b.requirement_ids,
                candidate_oid: b.candidate_oid,
                base_oid: b.base_oid,
            },
            &repo,
            &candidate,
        )
        .unwrap();
        let frozen =
            FrozenObligations::from_json(&std::fs::read(root().join("frozen.json")).unwrap())
                .unwrap();
        let sources = ScopedSources::from_json(
            &std::fs::read(root().join("sources.json")).unwrap(),
            &binding,
            &frozen,
        )
        .unwrap();
        Self {
            repo_root: repo_path,
            _directory: directory,
            binding,
            frozen,
            sources,
            policies: protected_policies(),
            scopes: read("scopes.json"),
        }
    }
    pub fn pending(&self) -> PendingGate {
        prepare_scoped_gate(
            &self.binding,
            &self.frozen,
            &self.sources,
            self.policies.clone(),
            request(),
        )
        .unwrap()
    }
}
pub fn request() -> GateRequest {
    GateRequest {
        run_id: "actual-four-producer-fixture".into(),
        action: "commit".into(),
        started_at: TIME.into(),
    }
}
pub struct Captured {
    pub envelope: GuardRunEnvelope,
    pub contract: Vec<u8>,
    pub facts: Vec<u8>,
    pub report: Vec<u8>,
}
impl Captured {
    pub fn new(name: &str) -> Self {
        Self {
            envelope: envelope(name),
            contract: bytes(name, "contract.json"),
            facts: bytes(name, "facts.json"),
            report: bytes(name, "report.json"),
        }
    }
    pub fn evidence<'a>(&'a self, scope: &'a str) -> SpecialistEvidence<'a> {
        SpecialistEvidence {
            scope,
            envelope: &self.envelope,
            artifacts: ArtifactBytes {
                contract: &self.contract,
                facts: &self.facts,
                report: &self.report,
            },
        }
    }
}
/// Fixed local records, never an echo of the query or a production issuer.
pub struct FixtureAuthority {
    pub baseline: specguard::baseline::ApprovedBaseline,
    pub baseline_auth: specguard::integration::approval::Authentication,
    pub approval: Option<ApprovalRecord>,
    pub records: BTreeMap<String, ProducerRecord>,
    pub unavailable: bool,
}
impl FixtureAuthority {
    pub fn new() -> Self {
        let baseline: specguard::baseline::ApprovedBaseline = read("specguard/baseline.json");
        let baseline_auth = specguard::integration::approval::Authentication {
            issuer: "fixture-baseline-issuer".into(),
            purpose: "specification-baseline".into(),
            repository: baseline.repository.clone(),
            scope: baseline.scope.clone(),
            baseline_digest: specguard::model::digest(&baseline),
            policy_digest: baseline.policy_digest.clone(),
            issued_at: NOW - 10,
            expires_at: NOW + 100,
            revoked: false,
        };
        Self {
            baseline,
            baseline_auth,
            approval: Some(approval_record()),
            records: PROVIDERS
                .into_iter()
                .map(|name| {
                    let e = envelope(name);
                    let d = flowguard::digest(&serde_json::to_vec(&e).unwrap());
                    (
                        d.clone(),
                        ProducerRecord {
                            principal: "fixture-ci".into(),
                            producer: e.producer,
                            envelope_digest: d,
                            validity: Validity {
                                issued_at: 0,
                                expires_at: i64::MAX,
                                revoked: false,
                            },
                        },
                    )
                })
                .collect(),
            unavailable: false,
        }
    }
}
impl AuthorityProvider for FixtureAuthority {
    fn verify_producer(
        &self,
        e: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if self.unavailable {
            return Err(AuthorityError::Unavailable);
        }
        if e.producer.guard == "SpecGuard" {
            specguard::integration::approval::authenticate(
                &self.baseline,
                &BaselinePort(self.baseline_auth.clone()),
                specguard::integration::approval::Profile::Fixture,
                NOW,
            )
            .map_err(|_| AuthorityError::Untrusted)?;
        }

        self.records
            .get(d)
            .filter(|r| r.producer == e.producer)
            .cloned()
            .ok_or(AuthorityError::Untrusted)
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        if reference != "fixture:actual-spec-review" {
            return Err(AuthorityError::Untrusted);
        }
        self.approval.clone().ok_or(AuthorityError::Unavailable)
    }
}

fn protected_policies() -> BTreeMap<String, EligibilityPolicy> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Expected {
        binding: guardengine::integration::RunBinding,
        producer: guardengine::integration::Producer,
        required_scopes: Vec<String>,
        contract_digest: String,
    }
    let scopes: BTreeMap<String, String> = read("scopes.json");
    let policies: BTreeMap<_, _> = PROVIDERS
        .into_iter()
        .map(|name| {
            let e: Expected = read(&format!("{name}/expected.json"));
            (
                scopes[name].clone(),
                EligibilityPolicy {
                    binding: e.binding,
                    producer: e.producer,
                    required_scopes: e.required_scopes,
                    contract_digest: e.contract_digest,
                    action: "commit".into(),
                    producer_principals: std::collections::BTreeSet::from(["fixture-ci".into()]),
                    approval_principals: if name == "specguard" {
                        BTreeMap::from([(
                            "review".into(),
                            std::collections::BTreeSet::from(["fixture-reviewer".into()]),
                        )])
                    } else {
                        BTreeMap::new()
                    },
                },
            )
        })
        .collect();
    assert_eq!(
        serde_json::to_value(&policies).unwrap(),
        read::<serde_json::Value>("policies.json")
    );
    policies
}

pub fn approval_record() -> ApprovalRecord {
    let p = protected_policies();
    let scopes: BTreeMap<String, String> = read("scopes.json");
    let p = &p[&scopes["specguard"]];
    ApprovalRecord {
        principal: "fixture-reviewer".into(),
        purpose: "review".into(),
        action: "commit".into(),
        binding: p.binding.clone(),
        contract_digest: p.contract_digest.clone(),
        validity: Validity {
            issued_at: NOW - 10,
            expires_at: NOW + 100,
            revoked: false,
        },
    }
}

struct BaselinePort(specguard::integration::approval::Authentication);
impl specguard::integration::approval::ApprovalValidationPort for BaselinePort {
    fn profile(&self) -> specguard::integration::approval::Profile {
        specguard::integration::approval::Profile::Fixture
    }
    fn validate(
        &self,
        _: &specguard::baseline::ApprovedBaseline,
    ) -> Result<
        specguard::integration::approval::Authentication,
        specguard::integration::approval::ApprovalError,
    > {
        Ok(self.0.clone())
    }
}
