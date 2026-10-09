mod common;
use common::*;
use flowguard::{gate::*, run_store::*};
use std::collections::BTreeMap;
#[test]
fn old_completed_bytes_cannot_be_relabelled_under_new_authority_policy() {
    let (_dir, b) = binding();
    let u = upstream(&b, guardengine::Enforcement::Advise, false);
    let frozen = frozen(&b, &u, false);
    let scope = obligation_scope(frozen.obligations().first().unwrap());
    let request = || GateRequest {
        run_id: "same-run".into(),
        action: "commit".into(),
        started_at: TIME.into(),
    };
    let old = prepare_gate(
        &b,
        &frozen,
        BTreeMap::from([(scope.clone(), policy(&u))]),
        request(),
    )
    .unwrap();
    let result = old
        .evaluate(
            &[SpecialistEvidence {
                scope: &scope,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            &FixtureAuthority {
                approval: None,
                unavailable: false,
            },
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(
        result.envelope().decision,
        Some(guardengine::Decision::Allow)
    );
    let mut changed = policy(&u);
    changed.producer_principals = std::collections::BTreeSet::from(["different-controller".into()]);
    let new = prepare_gate(
        &b,
        &frozen,
        BTreeMap::from([(scope.clone(), changed.clone())]),
        request(),
    )
    .unwrap();
    assert_eq!(
        result.envelope().coverage.required_scopes,
        new.required_scopes()
    );
    let store = MemoryRunStore::default();
    let generation = store.advance(&new, 0).unwrap();
    store.reserve(&new, "request", generation).unwrap();
    assert!(
        store.append(&result).is_err(),
        "accepted old result under changed full work identity"
    );
    let directory = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    let disk = flowguard::durable_run_store::DurableRunStore::create(directory.path()).unwrap();
    let generation = disk.advance(&new, 0).unwrap();
    disk.reserve(&new, "request", generation).unwrap();
    assert!(disk.append(&result).is_err());
    let current = new
        .evaluate(
            &[SpecialistEvidence {
                scope: &scope,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            &FixtureAuthority {
                approval: None,
                unavailable: false,
            },
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(
        current.envelope().run_status,
        guardengine::integration::RunStatus::Error
    );
    store.append(&current).unwrap();
    disk.append(&current).unwrap();
    disk.publish("same-run", generation).unwrap();
    drop(disk);
    let disk = flowguard::durable_run_store::DurableRunStore::open(directory.path()).unwrap();
    let query = prepare_gate(&b, &frozen, BTreeMap::from([(scope, changed)]), request()).unwrap();
    assert_eq!(
        disk.current(&query).unwrap(),
        Some(serde_json::to_vec(current.envelope()).unwrap())
    );
}
