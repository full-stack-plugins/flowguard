mod common;
use common::*;
use flowguard::{
    evidence::{BaselineScope, ScopedSource, ScopedSources},
    gate::{GateRequest, SpecialistEvidence, obligation_scope, prepare_scoped_gate},
};
use std::collections::BTreeMap;
#[test]
fn explicit_baseline_profile_accepts_only_the_frozen_workflow_baseline() {
    let (_root, binding) = binding();
    let mut up = upstream(&binding, guardengine::Enforcement::Advise, false);
    let frozen = frozen(&binding, &up, false);
    let key = obligation_scope(frozen.obligations().first().unwrap());
    let mut p = policy(&up);
    p.binding.baseline_digest = Some(frozen.baseline_digest().into());
    let request = || GateRequest {
        run_id: "baseline-profile".into(),
        action: "commit".into(),
        started_at: TIME.into(),
    };
    let sources = ScopedSources::freeze(
        &binding,
        &frozen,
        BTreeMap::from([(key.clone(), p.binding.source_snapshot_digest.clone())]),
    )
    .unwrap();
    assert!(
        prepare_scoped_gate(
            &binding,
            &frozen,
            &sources,
            BTreeMap::from([(key.clone(), p.clone())]),
            request()
        )
        .is_err()
    );
    let protected = ScopedSources::freeze_contexts(
        &binding,
        &frozen,
        BTreeMap::from([(
            key.clone(),
            ScopedSource {
                source_snapshot_digest: p.binding.source_snapshot_digest.clone(),
                baseline: BaselineScope::FrozenWorkflowBaseline,
            },
        )]),
    )
    .unwrap();
    let prepare = |p| {
        prepare_scoped_gate(
            &binding,
            &frozen,
            &protected,
            BTreeMap::from([(key.clone(), p)]),
            request(),
        )
    };
    let mut wrong = p.clone();
    wrong.binding.baseline_digest = Some(flowguard::digest(b"different baseline"));
    assert!(prepare(wrong).is_err());
    let mut wrong = p.clone();
    wrong.binding.candidate_oid = "f".repeat(40);
    assert!(prepare(wrong).is_err());
    up.envelope.binding.baseline_digest = p.binding.baseline_digest.clone();
    let run = prepare(p)
        .unwrap()
        .evaluate(
            &[SpecialistEvidence {
                scope: &key,
                envelope: &up.envelope,
                artifacts: up.artifacts(),
            }],
            &FixtureAuthority {
                approval: None,
                unavailable: false,
            },
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(run.envelope().decision, Some(guardengine::Decision::Allow));
    assert!(
        run.envelope()
            .coverage
            .required_scopes
            .contains(&format!("flowguard.sources:{}", protected.digest()))
    );
}

#[test]
fn v1_wire_and_digest_remain_exact_and_v2_rejects_implicit_or_foreign_baselines() {
    let (_root, binding) = binding();
    let up = upstream(&binding, guardengine::Enforcement::Advise, false);
    let frozen = frozen(&binding, &up, false);
    let key = obligation_scope(frozen.obligations().first().unwrap());
    let source = up.envelope.binding.source_snapshot_digest.clone();
    let v1 = ScopedSources::freeze(
        &binding,
        &frozen,
        BTreeMap::from([(key.clone(), source.clone())]),
    )
    .unwrap();
    // Pre-v2 payload order and bytes, independent of the new implementation's DTO.
    let payload = format!(
        r#"{{"version":"flowguard.specialist-sources/v1alpha1","context_digest":"{}","frozen_digest":"{}","pins":[{{"scope":"{}","source_snapshot_digest":"{}"}}]}}"#,
        binding.domain_digest(),
        frozen.digest(),
        key,
        source
    );
    assert_eq!(v1.digest(), flowguard::digest(payload.as_bytes()));
    let expected = format!(
        "{},\"digest\":\"{}\"}}",
        &payload[..payload.len() - 1],
        v1.digest()
    );
    assert_eq!(serde_json::to_string(&v1).unwrap(), expected);
    let v2 = ScopedSources::freeze_contexts(
        &binding,
        &frozen,
        BTreeMap::from([(
            key,
            ScopedSource {
                source_snapshot_digest: source,
                baseline: BaselineScope::FrozenWorkflowBaseline,
            },
        )]),
    )
    .unwrap();
    assert_ne!(v1.digest(), v2.digest());
    let original = serde_json::to_value(v2).unwrap();
    for case in 0..8 {
        let mut value = original.clone();
        match case {
            0 => {
                value
                    .as_object_mut()
                    .unwrap()
                    .remove("workflow_baseline_digest");
            }
            1 => value["workflow_baseline_digest"] = serde_json::Value::Null,
            2 => {
                value["workflow_baseline_digest"] =
                    serde_json::json!(flowguard::digest(b"foreign baseline"))
            }
            3 => {
                value["pins"][0].as_object_mut().unwrap().remove("baseline");
            }
            4 => value["pins"][0]["baseline"] = serde_json::Value::Null,
            5 => value["pins"][0]["baseline"] = serde_json::json!("arbitrary"),
            6 => value["version"] = serde_json::json!("flowguard.specialist-sources/v99"),
            _ => value["version"] = serde_json::json!("flowguard.specialist-sources/v1alpha1"),
        }
        assert!(
            ScopedSources::from_json(&serde_json::to_vec(&value).unwrap(), &binding, &frozen)
                .is_err(),
            "case {case}"
        );
    }
    // Explicit null is not an omitted old-v1 field.
    let mut value = serde_json::to_value(v1).unwrap();
    value["workflow_baseline_digest"] = serde_json::Value::Null;
    assert!(
        ScopedSources::from_json(&serde_json::to_vec(&value).unwrap(), &binding, &frozen).is_err()
    );
}
