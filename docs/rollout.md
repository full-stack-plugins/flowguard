# Approved local rollout

The 2026-10-09 user decision adopts ADR D1 and D7–D10 recommendations, with the strict parser, complete-byte identity and sorted graph defaults in D2–D6/D11. Earlier “unapproved” language in the survey is historical and superseded for this scope.

## Reachable entry points and rule owners

| Entry | Owner / result |
| --- | --- |
| `flowguard legacy status --root ROOT --feature FEATURE --stage STAGE --revision 13b52b054c31f614dc272b18c195b8fa929aa595` | Bounded strict document parser plus fixed-revision fingerprint compatibility display. Returns `declared`, `legacy_effective`, `source_digest`, `new_eligibility`. Display never authorizes. |
| `LocalStageRoute::new` / `submit` / `invalidate` / `edited` | Existing `StageDeclaration` and protected stage plan. Old satisfying declarations enter immutable history and start the new flow invalidated. Other declarations start unqualified editing. No old approval import. |
| `LocalStageRoute::qualify` / `consume` | Existing `ProtectedStagePlan::qualify` / `QualifiedStage::consume`; technical evidence and current exact-scope approval remain necessary. Protected skip/exact baseline inheritance use the plan's existing typed mode. |
| `flowguard legacy release-map --root ROOT --mapping mapping.json --expected closure.json` | Exact local mapping contract. Each array row requires `feature_id`, `requirement_id`, and `immutable_09` (content-addressed SHA256 reference). Empty, duplicate, conflicting per-feature references, missing or extra triples refuse. |
| New release qualification | Existing `ReleasePlan`, fresh qualified feature09 receipts and its existing dependency closure checks. A valid mapping file alone grants nothing. |

The fixed legacy effective reader reproduces the surveyed body/evidence/metadata fingerprint distinction for strictly accepted inputs; it also checks upstream fingerprints. Its finite budget is 1 MiB per document, 1,024 reads and 64 release features. Malformed/ambiguous/unavailable inputs fail rather than becoming approval. Strict parser differences (duplicates, fences and unavailable paths) remain intentional; this is not broad old CLI compatibility. Missing inputs continue to surface as unavailable in this CLI, not synthetic pending authority.

## Local rollout and rollback

`LocalStageRoute` is a narrow process-local library entry point for a supplied protected stage plan. It starts advisory. Advisory and shadow evaluate fresh qualification and append observations but cannot return eligibility. Explicit `set_mode(OptIn)` enables consumption. `rollback()` changes routing to advisory; even a previously qualified stage cannot be consumed through the route. Re-enabling the route still refreshes approvals, so revocation survives rollback.

The route exposes history as a borrowed immutable slice. It appends mode changes, invalidations, submissions and qualification outcomes, preserving the original declaration/digest record. Original legacy documents are never rewritten. `edited(new_plan)` invalidates the satisfying state before replacing the protected plan; explicit resubmission and current approval follow. Historical records are not qualified receipts. No Git references are reset and no CodeGuard behavior changes.

This route uses the core's detached qualification profile. Hosts using `MemoryRunStore` must retain their publication-aware `consume_published` checks; this route is not a replacement publication store. The existing audit, invalidation and revocation stores are not migrated, deleted or reset. Process-local history is not a durable cross-process rollout controller, and no such controller is claimed.

No real old release sample was supplied. `release-map` validates an explicit local mapping against an independently supplied expected closure; neither file is authenticated authority. Real old-release owner transfer remains disabled pending that sample and actual closure validation. No controller authentication, protected-host enforcement, production distribution or N/N-1 compatibility is claimed. Those remain the separately scoped 4.5/5.4 and distribution prerequisites.
