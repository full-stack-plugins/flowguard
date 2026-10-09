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
