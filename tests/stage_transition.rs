use flowguard::stage_transition::{StageState::*, TransitionCheck, transition};
#[test]
fn seven_states_have_explicit_edges_without_eligibility() {
    let all = [
        Pending,
        InProgress,
        PendingAcceptance,
        Accepted,
        Inherited,
        Skipped,
        Invalidated,
    ];
    for from in all {
        for to in all {
            let allowed = matches!(
                (from, to),
                (Pending, InProgress)
                    | (InProgress, PendingAcceptance)
                    | (Invalidated, PendingAcceptance)
                    | (Accepted | Inherited | Skipped, Invalidated)
            );
            let result = transition(from, to);
            assert_eq!(
                matches!(result, TransitionCheck::DeclarationAllowed),
                allowed,
                "{from:?} -> {to:?}"
            );
        }
    }
    assert_eq!(
        transition(PendingAcceptance, Accepted),
        TransitionCheck::RequiresCurrentTechnicalEvidenceAndApproval
    );
    assert_eq!(
        transition(Pending, Skipped),
        TransitionCheck::RequiresProtectedSkipAndApproval
    );
    assert_eq!(
        transition(Pending, Inherited),
        TransitionCheck::RequiresExactBaselineAndApproval
    );
    assert_eq!(transition(Invalidated, Accepted), TransitionCheck::Illegal);
}
#[test]
fn arbitrary_status_is_not_a_supported_declaration() {
    assert!(
        serde_json::from_str::<flowguard::stage_transition::StageState>("\"eligible\"").is_err()
    );
}

#[test]
fn complete_matrix_applies_only_plain_declarations_and_preserves_every_rejection() {
    use flowguard::stage_transition::{StageDeclaration, TransitionCheck::*};
    let states = [
        Pending,
        InProgress,
        PendingAcceptance,
        Accepted,
        Inherited,
        Skipped,
        Invalidated,
    ];
    // Literal independent expected table, columns and rows in states order.
    let i = Illegal;
    let d = DeclarationAllowed;
    let a = RequiresCurrentTechnicalEvidenceAndApproval;
    let b = RequiresExactBaselineAndApproval;
    let s = RequiresProtectedSkipAndApproval;
    let expected = [
        [i, d, i, i, b, s, i],
        [i, i, d, i, b, s, i],
        [i, i, i, a, b, s, i],
        [i, i, i, i, i, i, d],
        [i, i, i, i, i, i, d],
        [i, i, i, i, i, i, d],
        [i, i, d, i, i, i, i],
    ];
    for (row, from) in states.into_iter().enumerate() {
        let declaration = StageDeclaration::observed(from);
        for (column, to) in states.into_iter().enumerate() {
            let requirement = expected[row][column];
            assert_eq!(transition(from, to), requirement, "{from:?} -> {to:?}");
            let actual = declaration.apply_declaration(to);
            if requirement == d {
                assert_eq!(actual.unwrap().state(), to);
            } else {
                assert_eq!(actual, Err(requirement), "{from:?} -> {to:?}");
            }
            assert_eq!(declaration.state(), from);
        }
    }
}

#[test]
fn self_reported_terminal_states_cannot_skip_recomputation_or_approval() {
    use flowguard::stage_transition::{StageDeclaration, TransitionCheck::*};
    for label in ["accepted", "inherited", "skipped"] {
        let state = serde_json::from_str(&format!("\"{label}\"")).unwrap();
        let reported = StageDeclaration::observed(state);
        assert_eq!(reported.apply_declaration(Accepted), Err(Illegal));
        let invalidated = reported.apply_declaration(Invalidated).unwrap();
        assert_eq!(invalidated.apply_declaration(Accepted), Err(Illegal));
        assert_eq!(invalidated.apply_declaration(Skipped), Err(Illegal));
        let resubmitted = invalidated.apply_declaration(PendingAcceptance).unwrap();
        assert_eq!(
            resubmitted.apply_declaration(Accepted),
            Err(RequiresCurrentTechnicalEvidenceAndApproval)
        );
        assert_eq!(
            resubmitted.apply_declaration(Skipped),
            Err(RequiresProtectedSkipAndApproval)
        );
        assert_eq!(
            resubmitted.apply_declaration(Inherited),
            Err(RequiresExactBaselineAndApproval)
        );
    }
}

#[test]
fn wire_states_roundtrip_without_accepting_technical_or_execution_status() {
    use flowguard::stage_transition::StageState;
    for (state, label) in [
        (Pending, "pending"),
        (InProgress, "in_progress"),
        (PendingAcceptance, "pending_acceptance"),
        (Accepted, "accepted"),
        (Inherited, "inherited"),
        (Skipped, "skipped"),
        (Invalidated, "invalidated"),
    ] {
        let bytes = serde_json::to_vec(&state).unwrap();
        assert_eq!(bytes, format!("\"{label}\"").as_bytes());
        assert_eq!(serde_json::from_slice::<StageState>(&bytes).unwrap(), state);
    }
    for label in [
        "ALLOW",
        "BLOCK",
        "REQUIRE_APPROVAL",
        "completed",
        "error",
        "cancelled",
        "eligible",
        "approved",
    ] {
        assert!(serde_json::from_str::<StageState>(&format!("\"{label}\"")).is_err());
    }
}
