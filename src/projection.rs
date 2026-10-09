//! Closed, versioned workflow mapping. No caller-controlled enforcement downgrade.
use guardengine::{GuardContract, GuardFacts};
use serde::{Deserialize, Serialize};
pub const MAPPING_VERSION: &str = "flowguard.gate-mapping/v1alpha1";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    Block,
    Review,
    Advise,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gap {
    pub scope: String,
    pub kind: GapKind,
    pub code: String,
}
pub struct Projection {
    pub contract: GuardContract,
    pub facts: GuardFacts,
}
pub fn project(
    snapshot: &str,
    required: &[String],
    observed: &[String],
    gaps: &[Gap],
) -> Result<Projection, &'static str> {
    use guardengine::*;
    if !crate::valid_digest(snapshot)
        || required.is_empty()
        || required.len() > 64
        || gaps.len() > 192
        || required.windows(2).any(|w| w[0] >= w[1])
        || observed.windows(2).any(|w| w[0] >= w[1])
        || required
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 256)
        || observed.iter().any(|s| required.binary_search(s).is_err())
    {
        return Err("invalid frozen projection scope");
    }
    let mut rules = Vec::new();
    for scope in required {
        for (name, enforcement) in [
            ("block", Enforcement::Enforce),
            ("review", Enforcement::Review),
            ("advise", Enforcement::Advise),
        ] {
            rules.push(GuardRule {
                id: format!("{}.{}", crate::digest(scope.as_bytes()), name),
                description: format!("workflow {name} observation"),
                enforcement,
                assertion: GuardAssertion::ForbidRelation {
                    subject: scope.clone(),
                    predicate: "flowguard.gap".into(),
                    object: name.into(),
                },
            });
        }
    }
    let mut facts = Vec::new();
    for gap in gaps {
        if required.binary_search(&gap.scope).is_err()
            || gap.code.trim().is_empty()
            || gap.code.len() > 128
        {
            return Err("unmapped workflow gap");
        }
        facts.push(GuardFact {
            subject: gap.scope.clone(),
            predicate: "flowguard.gap".into(),
            object: match gap.kind {
                GapKind::Block => "block",
                GapKind::Review => "review",
                GapKind::Advise => "advise",
            }
            .into(),
            source: gap.code.clone(),
        });
    }
    facts.sort();
    facts.dedup();
    let complete = required == observed;
    Ok(Projection {
        contract: GuardContract {
            api_version: API_VERSION.into(),
            kind: "GuardContract".into(),
            metadata: ContractMetadata {
                id: "flowguard.workflow-gate".into(),
                revision: MAPPING_VERSION.into(),
            },
            spec: ContractSpec { rules },
        },
        facts: GuardFacts {
            api_version: API_VERSION.into(),
            kind: "GuardFacts".into(),
            analyzer: AnalyzerIdentity {
                id: "flowguard.stage-gates".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            subject: GuardSubject {
                id: "flowguard.workflow".into(),
                snapshot_digest: snapshot.into(),
            },
            completeness: if complete {
                Completeness::Complete
            } else {
                Completeness::Partial
            },
            facts,
            diagnostics: if complete {
                vec![]
            } else {
                vec!["required workflow coverage is incomplete".into()]
            },
        },
    })
}
