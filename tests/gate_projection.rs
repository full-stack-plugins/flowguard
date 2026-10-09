use flowguard::projection::*;
use guardengine::{Decision, evaluate, load_contract_yaml, load_facts_json};
#[test]
fn closed_gap_mapping_uses_actual_engine_precedence_and_partial_semantics() {
    let scopes = vec!["stage:A/01".to_string()];
    for (kind, decision) in [
        (None, Decision::Allow),
        (Some(GapKind::Block), Decision::Block),
        (Some(GapKind::Review), Decision::RequireApproval),
        (Some(GapKind::Advise), Decision::Allow),
    ] {
        let gaps = kind
            .into_iter()
            .map(|kind| Gap {
                scope: scopes[0].clone(),
                kind,
                code: "fixture.observation".into(),
            })
            .collect::<Vec<_>>();
        let p = project(&flowguard::digest(b"snapshot"), &scopes, &scopes, &gaps).unwrap();
        let contract = load_contract_yaml(&serde_json::to_vec(&p.contract).unwrap()).unwrap();
        let facts = load_facts_json(&serde_json::to_vec(&p.facts).unwrap()).unwrap();
        assert_eq!(evaluate(&contract, &facts).unwrap().decision, decision);
    }
    let partial = project(&flowguard::digest(b"snapshot"), &scopes, &[], &[]).unwrap();
    assert_eq!(
        evaluate(&partial.contract, &partial.facts)
            .unwrap()
            .decision,
        Decision::Block
    );
}
#[test]
fn rejects_unmapped_scopes_unknown_kinds_and_mutable_coverage() {
    let scopes = vec!["required".to_string()];
    assert!(
        project(
            &flowguard::digest(b"snapshot"),
            &scopes,
            &scopes,
            &[Gap {
                scope: "unmapped".into(),
                kind: GapKind::Block,
                code: "must.not.disappear".into()
            }]
        )
        .is_err()
    );
    assert!(
        project(
            &flowguard::digest(b"snapshot"),
            &scopes,
            &["other".into()],
            &[]
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<Gap>(r#"{"scope":"required","kind":"unknown","code":"unknown"}"#)
            .is_err()
    );
    assert!(project(&flowguard::digest(b"snapshot"), &[], &[], &[]).is_err());
}
