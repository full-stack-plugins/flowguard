//! Actual fixed producer artifacts; fixture identity does not establish production authority.
use guardengine::integration::{
    EvidenceProfile, GuardRunEnvelope, load_envelope_json, verify_engine_artifacts,
};
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/providers/cross-guard/actual")
}
fn bytes(provider: &str, name: &str) -> Vec<u8> {
    std::fs::read(root().join(provider).join(name)).expect("actual generated provider artifact")
}
fn envelope(provider: &str) -> GuardRunEnvelope {
    load_envelope_json(
        &bytes(provider, "envelope.json"),
        EvidenceProfile::EngineBacked,
    )
    .unwrap()
}
#[test]
fn actual_four_producer_engine_artifacts_are_verified_without_rewriting() {
    for provider in ["archguard", "codeguard", "testguard", "gitguard"] {
        let e = envelope(provider);
        verify_engine_artifacts(
            &e,
            &bytes(provider, "contract.json"),
            &bytes(provider, "facts.json"),
            &bytes(provider, "report.json"),
        )
        .unwrap();
        assert_eq!(e.decision, Some(guardengine::Decision::Allow), "{provider}");
    }
}

use flowguard::{context::*, evidence::ScopedSources, gate::*, obligations::FrozenObligations};
use guardengine::integration::eligibility::*;
use std::collections::BTreeMap;
const TIME: &str = "2033-05-18T03:33:20Z";
const NOW: i64 = 2_000_000_000;
const PROVIDERS: [&str; 4] = ["archguard", "codeguard", "testguard", "gitguard"];
fn read<T: serde::de::DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&std::fs::read(root().join(name)).unwrap()).unwrap()
}
struct Fixture {
    _directory: tempfile::TempDir,
    binding: ValidatedBinding,
    frozen: FrozenObligations,
    sources: ScopedSources,
    policies: BTreeMap<String, EligibilityPolicy>,
    scopes: BTreeMap<String, String>,
}
impl Fixture {
    fn new() -> Self {
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
            _directory: directory,
            binding,
            frozen,
            sources,
            policies: protected_policies(),
            scopes: read("scopes.json"),
        }
    }
    fn pending(&self) -> PendingGate {
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
fn request() -> GateRequest {
    GateRequest {
        run_id: "actual-four-producer-fixture".into(),
        action: "commit".into(),
        started_at: TIME.into(),
    }
}
struct Captured {
    envelope: GuardRunEnvelope,
    contract: Vec<u8>,
    facts: Vec<u8>,
    report: Vec<u8>,
}
impl Captured {
    fn new(name: &str) -> Self {
        Self {
            envelope: envelope(name),
            contract: bytes(name, "contract.json"),
            facts: bytes(name, "facts.json"),
            report: bytes(name, "report.json"),
        }
    }
    fn evidence<'a>(&'a self, scope: &'a str) -> SpecialistEvidence<'a> {
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
struct FixtureAuthority {
    records: BTreeMap<String, ProducerRecord>,
    unavailable: bool,
}
impl FixtureAuthority {
    fn new() -> Self {
        Self {
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
        self.records
            .get(d)
            .filter(|r| r.producer == e.producer)
            .cloned()
            .ok_or(AuthorityError::Untrusted)
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }
}
#[test]
fn actual_same_candidate_four_way_gate_and_each_missing_provider_fail_closed() {
    let f = Fixture::new();
    let captures: Vec<_> = PROVIDERS.iter().map(|n| Captured::new(n)).collect();
    for omitted in 0..=4 {
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
        assert_eq!(
            result.envelope().decision,
            Some(if omitted == 4 {
                guardengine::Decision::Allow
            } else {
                guardengine::Decision::Block
            }),
            "omitted={omitted} {:?}",
            result.envelope().diagnostics
        );
        if omitted < 4 {
            assert_eq!(
                result.envelope().coverage.status,
                guardengine::integration::CoverageStatus::Partial
            );
        }
    }
}
#[test]
fn actual_evidence_binding_artifact_weak_profile_and_identity_fail_closed() {
    let f = Fixture::new();
    for case in 0..9 {
        let mut captures: Vec<_> = PROVIDERS.iter().map(|n| Captured::new(n)).collect();
        let mut authority = FixtureAuthority::new();
        match case {
            0 => captures[0].envelope.binding.candidate_oid = "f".repeat(40),
            1 => {
                captures[2].envelope.binding.baseline_digest =
                    Some(flowguard::digest(b"other baseline"))
            }
            2 => {
                captures[1].envelope.binding.source_snapshot_digest =
                    flowguard::digest(b"foreign source")
            }
            3 => captures[1].report.push(b' '),
            4 => captures[2].envelope.producer.analyzer_version = "unqualified".into(),
            5 => {
                let v: serde_json::Value = read("codeguard/prepare.stdout.json");
                captures[1].envelope = serde_json::from_value(v["envelope"].clone()).unwrap();
                captures[1].contract.clear();
                captures[1].facts.clear();
                captures[1].report.clear();
            }
            6 => authority.unavailable = true,
            7 => {
                for r in authority.records.values_mut() {
                    r.validity.revoked = true
                }
            }
            _ => captures[3].contract = bytes("archguard", "contract.json"),
        }
        let evidence: Vec<_> = captures
            .iter()
            .enumerate()
            .map(|(i, c)| c.evidence(&f.scopes[PROVIDERS[i]]))
            .collect();
        let result = f
            .pending()
            .evaluate(&evidence, &authority, NOW, TIME)
            .unwrap();
        assert_eq!(
            result.envelope().run_status,
            guardengine::integration::RunStatus::Error,
            "case {case}"
        );
        assert_eq!(result.envelope().decision, None, "case {case}");
    }
}
#[test]
fn native_baselines_require_explicit_v2_and_changed_profile_cannot_reuse_completed_work() {
    let f = Fixture::new();
    let pins = f
        .policies
        .iter()
        .map(|(k, p)| (k.clone(), p.binding.source_snapshot_digest.clone()))
        .collect();
    let v1 = ScopedSources::freeze(&f.binding, &f.frozen, pins).unwrap();
    assert!(
        prepare_scoped_gate(&f.binding, &f.frozen, &v1, f.policies.clone(), request()).is_err()
    );
    for field in ["candidate", "baseline"] {
        let mut policies = f.policies.clone();
        let tg = policies.get_mut(&f.scopes["testguard"]).unwrap();
        if field == "candidate" {
            tg.binding.candidate_oid = "f".repeat(40)
        } else {
            tg.binding.baseline_digest = Some(flowguard::digest(b"foreign baseline"))
        }
        assert!(
            prepare_scoped_gate(&f.binding, &f.frozen, &f.sources, policies, request()).is_err()
        );
    }
    let captures: Vec<_> = PROVIDERS.iter().map(|n| Captured::new(n)).collect();
    let evidence: Vec<_> = captures
        .iter()
        .enumerate()
        .map(|(i, c)| c.evidence(&f.scopes[PROVIDERS[i]]))
        .collect();
    let run = f
        .pending()
        .evaluate(&evidence, &FixtureAuthority::new(), NOW, TIME)
        .unwrap();
    let mut pins: BTreeMap<_, _> = f
        .policies
        .iter()
        .map(|(key, p)| {
            (
                key.clone(),
                flowguard::evidence::ScopedSource {
                    source_snapshot_digest: p.binding.source_snapshot_digest.clone(),
                    baseline: if p.binding.baseline_digest.is_some() {
                        flowguard::evidence::BaselineScope::FrozenWorkflowBaseline
                    } else {
                        flowguard::evidence::BaselineScope::Absent
                    },
                },
            )
        })
        .collect();
    pins.get_mut(&f.scopes["testguard"]).unwrap().baseline =
        flowguard::evidence::BaselineScope::Absent;
    let changed = ScopedSources::freeze_contexts(&f.binding, &f.frozen, pins).unwrap();
    let mut policies = f.policies.clone();
    policies
        .get_mut(&f.scopes["testguard"])
        .unwrap()
        .binding
        .baseline_digest = None;
    let pending =
        prepare_scoped_gate(&f.binding, &f.frozen, &changed, policies, request()).unwrap();
    assert_ne!(
        pending.required_scopes(),
        run.envelope().coverage.required_scopes
    );
    let store = flowguard::run_store::MemoryRunStore::default();
    let generation = store.advance(&pending, 0).unwrap();
    store
        .reserve(&pending, "different-profile", generation)
        .unwrap();
    assert!(store.append(&run).is_err());
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
                    approval_principals: BTreeMap::new(),
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

#[test]
fn recorded_native_bytes_and_pre_result_expectations_keep_original_hashes() {
    let provenance: serde_json::Value = read("PROVENANCE.json");
    for section in ["artifacts", "protectedBeforeResults"] {
        for (path, hash) in provenance[section].as_object().unwrap() {
            let actual = flowguard::digest(&std::fs::read(root().join(path)).unwrap());
            assert_eq!(
                actual.strip_prefix("sha256:").unwrap(),
                hash.as_str().unwrap(),
                "{section}/{path}"
            );
        }
    }
    assert_eq!(
        bytes("codeguard", "native.json"),
        bytes("codeguard", "domain.json")
    );
    for provider in PROVIDERS {
        assert_eq!(
            bytes(provider, "prepared-contract.json"),
            bytes(provider, "contract.json")
        );
    }
}
