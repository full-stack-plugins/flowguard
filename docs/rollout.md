# Approved local rollout

The 2026-10-09 user decision adopts ADR D1 and D7–D10 recommendations, with the strict parser, complete-byte identity and sorted graph defaults in D2–D6/D11. Earlier “unapproved” language in the survey is historical and superseded for this scope.

## Reachable entry points and rule owners

| Entry | Owner / result |
| --- | --- |
| `flowguard legacy status --root ROOT --feature FEATURE --stage STAGE --revision 13b52b054c31f614dc272b18c195b8fa929aa595` | Bounded strict document parser plus fixed-revision fingerprint compatibility display. Returns `declared`, `legacy_effective`, `source_digest`, `new_eligibility`. Display never authorizes. |
| `LocalStageRoute::new` / `submit` / `invalidate` / `edited` | Existing `StageDeclaration` and protected stage plan. Old satisfying declarations enter immutable history and start the new flow invalidated. Other declarations start unqualified editing. No old approval import. |
| `LocalStageRoute::new_published` / `qualify` / `consume` | Binds the existing `MemoryRunStore`; qualification and consumption call `QualifiedStage::consume_published`, including every prerequisite and fresh authority. Protected skip/exact baseline inheritance retain the existing typed modes. |
| `flowguard legacy release-map --root ROOT --mapping mapping.json --expected closure.json` | Exact local mapping contract. Each array row requires `feature_id`, `requirement_id`, and `immutable_09` (content-addressed SHA256 reference). Empty, duplicate, conflicting per-feature references, missing or extra triples refuse. |
| `ReleasePlan::consume_routed_published` | Reads the actual opted-in feature09 routes bound to the same store, then calls existing `consume_published` for exact frozen feature plans and complete current publication/authority checks. Rollback, missing routes, detached routes and edited plans refuse. A valid mapping file alone grants nothing. |

The fixed legacy effective reader reproduces the surveyed body/evidence/metadata fingerprint distinction for strictly accepted inputs; it also checks upstream fingerprints. Its finite budget is 1 MiB per document, 1,024 reads and 64 release features. Malformed/ambiguous/unavailable inputs fail rather than becoming approval. Strict parser differences (duplicates, fences and unavailable paths) remain intentional; this is not broad old CLI compatibility. Missing inputs continue to surface as unavailable in this CLI, not synthetic pending authority.

## Local rollout and rollback

`LocalStageRoute` is a narrow process-local library entry point for a supplied protected stage plan. It starts advisory. Advisory and shadow evaluate fresh qualification and append observations but cannot return eligibility. Explicit `set_mode(OptIn)` enables consumption. `rollback()` changes routing to advisory; even a previously qualified stage cannot be consumed through the route. Re-enabling the route still refreshes approvals, so revocation survives rollback.

The route exposes history as a borrowed immutable slice. It appends mode changes, invalidations, submissions and qualification outcomes, preserving the original declaration/digest record. Original legacy documents are never rewritten. `edited(new_plan)` invalidates the satisfying state before replacing the protected plan; explicit resubmission and current approval follow. Historical records are not qualified receipts. No Git references are reset and no CodeGuard behavior changes.

Use `LocalStageRoute::new_published` for the publication-aware path. Its retained store reference makes the existing `qualify` and `consume` methods recheck current publications, including advisory/shadow observations. Release callers use `ReleasePlan::consume_routed_published` with their actual routes and that same store; its complete fan-in check runs before and after external authority callbacks. The original `LocalStageRoute::new` retains detached compatibility and is rejected by routed release consumption. This route is not a replacement publication store. The existing audit, invalidation and revocation stores are not migrated, deleted or reset. Process-local history is not a durable cross-process rollout controller, and no such controller is claimed.

No real old release sample was supplied. `release-map` validates an explicit local mapping against an independently supplied expected closure; neither file is authenticated authority. The publication-aware local stage/release wiring is independently testable without a real old-release sample. Real old-release migration is not claimed; it would require the actual sample and closure validation. No controller authentication, protected-host enforcement, production distribution or N/N-1 compatibility is claimed. Those remain the separately scoped 4.5/5.4 and distribution prerequisites.

### Bounded history shutdown behavior

At the 1,024-entry limit, rollback and switches to advisory/shadow still stop consumption; satisfying-state invalidation clears the receipt and sets invalidated before attempting its history append. These operations return `Budget` to report the missing audit append, but cannot leave eligibility active. Existing history remains byte-for-byte intact, and exhausted history cannot enable opt-in or submission. `edited(new_plan)` has the same fail-closed invalidation behavior for satisfying stages. Already-invalidated imports and repeated edits can adopt a different same-stage plan without qualification of the obsolete plan, preserving history and requiring submission/current approval for the latest plan.

### Published local verification (2026-10-10)

`tests/published_routing.rs` exercises unpublished evidence, transitive withdrawal, fresh prerequisite revocation, changed-plan resubmission and publication, and fail-closed rollback at the history limit. `tests/release_qualified.rs::published_release_routes_four_phases_and_rechecks_every_feature` executes advisory, shadow, opt-in and rollback over two real generated GateRuns appended and published in `MemoryRunStore`; it emits actual plan/envelope digests and immutable route history. This uses explicit local fixture authority and native Git fixture binding, not production authentication. The release regressions also reject missing/swapped features, another store, detached routes and freshly qualified plans outside the frozen release.

### Authority composition boundary

A `QualifiedStage` prerequisite refreshes its publication and authority. It does not
retain or observe an unrelated `LocalStageRoute` mode. Routed release consumption
checks the actual feature09 routes supplied to it; arbitrary upstream route rollback
requires publication withdrawal or a fresh authority refusal to affect dependents.

`tests/published_archguard.rs` demonstrates the latter with actual Git-bound
`GitCargoEvidence`. Its test-only authority invokes ArchGuard's `check_stage` on
every specialist producer refresh, under the independently frozen policy and the
same contract used for the native/enhanced decision comparison. Disabling the
adapter, actual native analyzer fact mismatch, or revoking the ArchGuard-specific
required approval after rollback/re-enable refuses the retained FlowGuard receipt.
The original bundle/history are preserved. This is an explicit local caller
composition, not a production provider installation or implicit propagation of all
upstream route modes.

The same actual AG/FG consumer test rejects an unsupported required capability,
a missing domain artifact and authenticated partial coverage when consuming the
retained published FlowGuard receipt. It keeps both protected required-scope sets
and original bundle/baseline bytes unchanged, and verifies baseline recovery between
each mutation. These are consumer-boundary refusals, not standalone schema checks.
