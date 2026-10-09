#![allow(dead_code)]
use archguard::integration::{binding::GitCargoEvidence, projection::ProtectedCargoPolicy};
use flowguard::{context::*, dependencies::*, gate::obligation_scope, obligations::*};
use guardengine::{
    integration::{eligibility::*, *},
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    process::Command,
};
pub const TIME: &str = "2026-10-09T00:00:00Z";
pub struct Prepared {
    pub directory: tempfile::TempDir,
    pub repo: gitguard::Repository,
    pub candidate: gitguard::candidate::CandidateSnapshot,
    pub binding: ValidatedBinding,
    pub frozen: FrozenObligations,
    pub policies: BTreeMap<String, EligibilityPolicy>,
    pub key: String,
    pub source_pin: String,
    pub producer: GitCargoEvidence,
}
pub fn fixture(enforcement: Enforcement, partial: bool) -> Prepared {
    let directory = tempfile::tempdir().unwrap();
    for (path, content) in [
        (
            "Cargo.toml",
            "[workspace]\nmembers=[\"app\",\"core\"]\nresolver=\"2\"\n",
        ),
        (
            "app/Cargo.toml",
            "[package]\nname=\"app\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[dependencies]\ncore={path=\"../core\"}\n",
        ),
        (
            "core/Cargo.toml",
            "[package]\nname=\"core\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        ),
        ("app/src/lib.rs", "pub fn app() {}\n"),
        ("core/src/lib.rs", "pub fn core() {}\n"),
    ] {
        let path = directory.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    let git = |args: &[&str]| {
        let o = Command::new("/usr/bin/git")
            .arg("-C")
            .arg(directory.path())
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8(o.stdout).unwrap().trim().to_string()
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "actual Cargo source"]);
    let oid = git(&["rev-parse", "HEAD"]);
    let contract = GuardContract {
        api_version: API_VERSION.into(),
        kind: "GuardContract".into(),
        metadata: ContractMetadata {
            id: "local-architecture".into(),
            revision: "1".into(),
        },
        spec: ContractSpec {
            rules: vec![GuardRule {
                id: "dependency".into(),
                description: "fixture".into(),
                enforcement,
                assertion: GuardAssertion::ForbidRelation {
                    subject: "app".into(),
                    predicate: "depends_on".into(),
                    object: "core".into(),
                },
            }],
        },
    };
    let members = if partial {
        vec!["absent-member".into()]
    } else {
        vec![]
    };
    let protected = ProtectedCargoPolicy::freeze(contract.clone(), members.clone()).unwrap();
    let repo = gitguard::Repository::discover(directory.path(), "repo").unwrap();
    let scope = gitguard::scope::TaskScope::advisory(
        "task",
        vec!["R".into()],
        vec![b"app".to_vec(), b"core".to_vec(), b"Cargo.toml".to_vec()],
        protected.profile_digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let subject = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(oid.clone()))
        .unwrap();
    let candidate = repo
        .prepare_candidate(
            &subject,
            &scope,
            &gitguard::candidate::CandidateRequest {
                worktree_id: "worktree".into(),
                base_oid: oid.clone(),
                merge_group_id: None,
                members: vec![],
            },
        )
        .unwrap();
    let binding = bind(
        &InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec!["task".into()],
            worktree_id: "worktree".into(),
            requirement_ids: vec!["R".into()],
            candidate_oid: oid.clone(),
            base_oid: oid,
        },
        &repo,
        &candidate,
    )
    .unwrap();
    let mut coverage = BTreeSet::from([
        "cargo.declarations".into(),
        format!("cargo.profile:{}", protected.profile_digest()),
    ]);
    // Contract endpoints are controller inputs, fixed before preparing evidence.
    for member in members.iter().map(String::as_str).chain(["app", "core"]) {
        coverage.insert(format!(
            "cargo.member:{}",
            flowguard::digest(member.as_bytes())
        ));
    }
    coverage.insert(format!(
        "cargo.relation:{}",
        flowguard::digest(b"depends_on")
    ));
    let producer = GitCargoEvidence::prepare(&repo, &candidate, protected).unwrap();
    // Independent controller pin BEFORE analysis/result collection.
    let expected = producer.binding().clone();
    let source_pin = expected.source_snapshot_digest.clone();
    let contract_digest =
        flowguard::digest(&serde_json::to_vec(&serde_json::to_value(&contract).unwrap()).unwrap());
    let graph = build_graph(
        vec![StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "architecture".into(),
            stage: "02-architecture".into(),
            owner: "project".into(),
            source_digest: flowguard::digest(b"protected-stage"),
            dependencies: BTreeSet::new(),
        }],
        GraphLimits::default(),
    )
    .unwrap();
    let frozen = freeze(
        &graph,
        vec![EvidenceObligation {
            stage_id: "architecture".into(),
            guard: "ArchGuard".into(),
            coverage: coverage.clone(),
            rules_digest: contract_digest.clone(),
            analyzer_version: "0.1.0".into(),
        }],
        &binding.domain_digest(),
        &flowguard::digest(b"fixture-baseline"),
    )
    .unwrap();
    let key = obligation_scope(frozen.obligations().first().unwrap());
    let policy = EligibilityPolicy {
        binding: expected,
        producer: Producer {
            guard: "ArchGuard".into(),
            version: "0.1.0".into(),
            analyzer_id: "archguard.cargo.declarations".into(),
            analyzer_version: "0.1.0".into(),
        },
        required_scopes: coverage.into_iter().collect(),
        contract_digest,
        action: "commit".into(),
        producer_principals: BTreeSet::from(["fixture-issuer".into()]),
        approval_principals: BTreeMap::new(),
    };
    Prepared {
        directory,
        repo,
        candidate,
        binding,
        frozen,
        policies: BTreeMap::from([(key.clone(), policy)]),
        key,
        source_pin,
        producer,
    }
}
pub struct FixtureIssuer(pub ProducerRecord);
impl FixtureIssuer {
    pub fn for_envelope(e: &GuardRunEnvelope) -> Self {
        Self(ProducerRecord {
            principal: "fixture-issuer".into(),
            producer: e.producer.clone(),
            envelope_digest: flowguard::digest(&serde_json::to_vec(e).unwrap()),
            validity: Validity {
                issued_at: 0,
                expires_at: i64::MAX,
                revoked: false,
            },
        })
    }
}
impl AuthorityProvider for FixtureIssuer {
    fn verify_producer(
        &self,
        e: &GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if e.producer == self.0.producer && digest == self.0.envelope_digest {
            Ok(self.0.clone())
        } else {
            Err(AuthorityError::Untrusted)
        }
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }
}
/// Explicit local fixture clock, rounded up for GE's whole-second consumption port.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + 1
}
