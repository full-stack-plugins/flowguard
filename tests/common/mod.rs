#![allow(dead_code)]
use flowguard::{context::*, dependencies::*, obligations::*};
use guardengine::integration::{eligibility::*, *};
use guardengine::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    process::Command,
};
pub const TIME: &str = "2026-10-09T00:00:00Z";
pub const NOW: i64 = 1791504000;
pub fn binding() -> (tempfile::TempDir, ValidatedBinding) {
    let d = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let o = Command::new("/usr/bin/git")
            .arg("-C")
            .arg(d.path())
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
    std::fs::write(d.path().join("a"), "fixture").unwrap();
    git(&["add", "a"]);
    git(&["commit", "-qm", "fixture"]);
    let oid = git(&["rev-parse", "HEAD"]);
    let repo = gitguard::Repository::discover(d.path(), "repo").unwrap();
    let subject = repo
        .resolve_subject(gitguard::subject::SubjectRequest::Commit(oid.clone()))
        .unwrap();
    let scope = gitguard::scope::TaskScope::advisory(
        "task",
        vec!["A".into()],
        vec![b"a".to_vec()],
        &"a".repeat(64),
        None,
    )
    .unwrap();
    let c = repo
        .prepare_candidate(
            &subject,
            &scope,
            &gitguard::candidate::CandidateRequest {
                worktree_id: "w".into(),
                base_oid: oid.clone(),
                merge_group_id: None,
                members: vec![],
            },
        )
        .unwrap();
    let b = bind(
        &InvocationInput {
            repo_candidates: vec!["repo".into()],
            task_candidates: vec!["task".into()],
            worktree_id: "w".into(),
            requirement_ids: vec!["A".into()],
            candidate_oid: oid.clone(),
            base_oid: oid,
        },
        &repo,
        &c,
    )
    .unwrap();
    (d, b)
}
#[derive(Clone)]
pub struct Upstream {
    pub envelope: GuardRunEnvelope,
    pub contract: Vec<u8>,
    pub facts: Vec<u8>,
    pub report: Vec<u8>,
}
impl Upstream {
    pub fn artifacts(&self) -> ArtifactBytes<'_> {
        ArtifactBytes {
            contract: &self.contract,
            facts: &self.facts,
            report: &self.report,
        }
    }
}
pub fn upstream(binding: &ValidatedBinding, enforcement: Enforcement, partial: bool) -> Upstream {
    let c = GuardContract {
        api_version: API_VERSION.into(),
        kind: "GuardContract".into(),
        metadata: ContractMetadata {
            id: "fixture.specialist".into(),
            revision: "1".into(),
        },
        spec: ContractSpec {
            rules: vec![GuardRule {
                id: "review-change".into(),
                description: String::new(),
                enforcement,
                assertion: GuardAssertion::ForbidRelation {
                    subject: "spec".into(),
                    predicate: "changed".into(),
                    object: "baseline".into(),
                },
            }],
        },
    };
    let f = GuardFacts {
        api_version: API_VERSION.into(),
        kind: "GuardFacts".into(),
        analyzer: AnalyzerIdentity {
            id: "fixture.spec".into(),
            version: "1".into(),
        },
        subject: GuardSubject {
            id: "spec".into(),
            snapshot_digest: binding.binding().source_snapshot_digest.clone(),
        },
        completeness: if partial {
            Completeness::Partial
        } else {
            Completeness::Complete
        },
        facts: vec![GuardFact {
            subject: "spec".into(),
            predicate: "changed".into(),
            object: "baseline".into(),
            source: "fixture".into(),
        }],
        diagnostics: if partial {
            vec!["fixture partial".into()]
        } else {
            vec![]
        },
    };
    let report = evaluate(&c, &f).unwrap();
    let contract = serde_json::to_vec(&c).unwrap();
    let facts = serde_json::to_vec(&f).unwrap();
    let report_bytes = serde_json::to_vec(&report).unwrap();
    let artifact = |name: &str, bytes: &[u8]| ArtifactRef {
        uri: format!("artifact://fixture/{name}"),
        digest: flowguard::digest(bytes),
        media_type: "application/json".into(),
    };
    let envelope = GuardRunEnvelope {
        api_version: INTEGRATION_VERSION.into(),
        kind: "GuardRunEnvelope".into(),
        run_id: "specialist-run".into(),
        producer: Producer {
            guard: "specguard".into(),
            version: "1".into(),
            analyzer_id: "fixture.spec".into(),
            analyzer_version: "1".into(),
        },
        binding: binding.binding().clone(),
        run_status: RunStatus::Completed,
        decision: Some(report.decision),
        coverage: Coverage {
            status: if partial {
                CoverageStatus::Partial
            } else {
                CoverageStatus::Complete
            },
            required_scopes: vec!["A".into()],
            observed_scopes: if partial { vec![] } else { vec!["A".into()] },
            missing_scopes: if partial { vec!["A".into()] } else { vec![] },
        },
        artifacts: Artifacts {
            contract: Some(artifact("contract", &contract)),
            facts: Some(artifact("facts", &facts)),
            report: Some(artifact("report", &report_bytes)),
            domain: vec![],
        },
        approval_refs: vec![],
        diagnostics: vec![],
        started_at: TIME.into(),
        finished_at: TIME.into(),
        expires_at: None,
    };
    Upstream {
        envelope,
        contract,
        facts,
        report: report_bytes,
    }
}
pub fn policy(u: &Upstream) -> EligibilityPolicy {
    EligibilityPolicy {
        binding: u.envelope.binding.clone(),
        producer: u.envelope.producer.clone(),
        required_scopes: vec!["A".into()],
        contract_digest: flowguard::digest(&u.contract),
        action: "commit".into(),
        producer_principals: BTreeSet::from(["fixture-producer".into()]),
        approval_principals: BTreeMap::from([(
            "review".into(),
            BTreeSet::from(["fixture-reviewer".into()]),
        )]),
    }
}
pub fn frozen(binding: &ValidatedBinding, u: &Upstream, missing_stage: bool) -> FrozenObligations {
    let graph = build_graph(
        vec![StageRecord {
            version: "flowguard.workflow/v1".into(),
            id: "A/01".into(),
            stage: "01-requirements".into(),
            owner: "a".into(),
            source_digest: flowguard::digest(b"stage"),
            dependencies: BTreeSet::new(),
        }],
        GraphLimits::default(),
    )
    .unwrap();
    freeze(
        &graph,
        vec![EvidenceObligation {
            stage_id: if missing_stage {
                "A/04".into()
            } else {
                "A/01".into()
            },
            guard: "specguard".into(),
            coverage: BTreeSet::from(["A".into()]),
            rules_digest: flowguard::digest(&u.contract),
            analyzer_version: "1".into(),
        }],
        &binding.domain_digest(),
        &flowguard::digest(b"fixture baseline"),
    )
    .unwrap()
}
/// Explicit test double. Never a production authority implementation.
pub struct FixtureAuthority {
    pub approval: Option<guardengine::integration::eligibility::ApprovalRecord>,
    pub unavailable: bool,
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
        Ok(ProducerRecord {
            principal: "fixture-producer".into(),
            producer: e.producer.clone(),
            envelope_digest: d.into(),
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        })
    }
    fn verify_approval(
        &self,
        _: &str,
    ) -> Result<guardengine::integration::eligibility::ApprovalRecord, AuthorityError> {
        self.approval.clone().ok_or(AuthorityError::Untrusted)
    }
}
