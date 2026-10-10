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

Review regression checks added after independent findings:

- `exhausted_history_never_prevents_stopping_qualified_consumption` fills all 1,024 history slots on a qualified opt-in route and exercises rollback, invalidation, edited-plan invalidation, shadow and advisory switches. Each reports the unavailable audit append, refuses subsequent consumption, preserves historical bytes and refuses opt-in reactivation.
- `invalidated_import_and_repeated_edits_keep_history_and_require_latest_approval` imports each old satisfying state, changes the protected plan twice while invalidated, preserves the original history prefix, refuses pre-submission qualification and the prior plan's approval, and accepts only the latest plan's fresh approval after submission.

## Publication-aware routing exercise (2026-10-10)

Run `cargo test --test published_routing --test release_qualified --test approved_routing`.
The original detached API regressions remain intact. New store-bound routes perform
all qualification/consumption via the existing published receipt checks.

The release four-phase test generates two feature09 GateRuns from frozen obligations,
reserves/appends/publishes them in MemoryRunStore, then consumes actual routes through
`ReleasePlan::consume_routed_published`. Advisory and shadow refuse stage/release
consumption; opt-in succeeds; rollback refuses while retaining the history prefix.
The test then re-enables opt-in, revokes one feature approval, and withdraws the other
feature publication through real `FrozenInputs::changes().apply()`; each independently
refuses release consumption and leaves the unaffected feature and original envelopes
intact. `--nocapture` emits `FLOWGUARD_PUBLISHED_ROLLOUT` JSON evidence with the actual
plan/envelope digests, four observed outcomes and full route history.

Additional tests reject unpublished evidence in all modes, withdrawn transitive
prerequisites, fresh prerequisite revocation, missing/swapped features, detached routes,
a different publication store and a freshly qualified changed plan outside the frozen
release. Changed plans require resubmission, exact fresh approval and publication.
The store-bound rollback exercise also fills the 1,024 history slots and proves that
Budget prevents reactivation without preventing shutdown or altering existing history.

This completes local publication-aware wiring; it does not require or claim a real old
release migration, authenticated controller (4.5), protected host (5.4), durable routing,
production distribution or N/N-1 compatibility.
