# Executed local rollback exercise

Runnable evidence: `cargo test --test approved_routing`.

`shadow_opt_in_rollback_preserve_history_and_fresh_revocation` uses actual native Git fixture bindings, frozen graph/obligations, generated technical gate evidence and the explicit local fixture authority. It verifies:

1. Historical accepted state is retained but starts invalidated; direct qualification refuses.
2. Submission reaches pending_acceptance. Advisory/shadow return no route eligibility.
3. Opt-in qualifies and consumes through the protected core.
4. Rollback refuses consumption of a previously qualified receipt through the route.
5. Editing requires invalidation and resubmission; a revoked approval refuses qualification.
6. Rollback preserves the entire previous history prefix. Re-enabling opt-in does not revive the revoked approval.

`edited_satisfied_stage_invalidates_before_resubmitting_new_plan` changes a graph source digest, freezes a new protected plan, invalidates the satisfied route, refuses pre-submission qualification and old approval, and accepts only the fresh new-plan approval.

No real-host experiment or production authentication is represented by the fixture authority. These tests do not run or migrate legacy mutators, reset Git refs, alter external revocation records or modify CodeGuard. Historical old inherited/skipped states use the same history-only import path; authorization for either new mode remains in the already-tested typed qualification core.
