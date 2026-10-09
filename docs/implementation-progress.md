# Slice ledger — add-evidence-bound-workflow-gates

Base: 1f8666d361102f8f113a2ef72d9b2a7f3dc4d6e9. Branch: impl/guard-roadmap-20261009.
Baseline is documentation-only; no runtime/test command exists.

## Executable slice

| Task | Edits and acceptance | Prerequisite/status |
|---|---|---|
| 1.1 | Fixed legacy inventory, source citations, actual read-only CLI survey | Authorized cloud clone; in progress |
| 1.2 | Strict independent domain types + canonical digest fixtures | Local schema only |
| 1.3 | Cargo library and validated context | Binding integration awaits GE/GG; partial bootstrap |
| 1.4, 1.6 | Bounded authorized-root reader, ten-stage native discovery; missing/escape/size/read-only tests | Local immutable fixture only |
| 1.5 | Fixed OpenSpec native task references; reject mixed authority | Independent native text fixture, no installation |
| 2.1 | Stable graph topological order; missing edge/cycle/duplicate/node budget tests | Stage schema |
| 2.2 | Exact immutable parent/scope inheritance and release 09 references | Local references; authentication external |
| 2.3 | Seven-state transition table, no self-declared eligibility | External observations required |
| 2.4 | Read-only approval port and expiry/revocation/action/scope checks | Test provider explicitly nonproduction; GE-TRUST pending |
| 2.5 | Protected applicability and separately scoped repair | Local policy input must be protected by caller |
| 2.6 | Actual SG export integration | SG-BASELINE dependency pending |
| 3.1 | Freeze graph/policy obligations, deterministic digest and missing-provider gaps | Local domain only; shared binding pending |

Ruling: use this authorized ledger rather than skill-managed ephemeral workspace; parent requires persistent reviewable ledger and controls review/checkboxes. No tasks checked before root review.
Preflight: 1.2 supplies stage schema to 1.4/2.1/2.2/2.3. Canonical digest must be independent of input order; ordered maps/sets and explicit version define encoding. 1.3/2.6 cannot invent shared candidate/export contracts; independent graph fixtures proceed and adapters wait.
Ruling: this slice is a synchronous std/Serde library, no CLI/provider/network runtime. Limits constrain file bytes and graph nodes/edges/depth; immutable checkout required during reads (path checks alone cannot defeat hostile concurrent filesystem mutation).

## Slice implementation outcome (pending root review)

Implemented and tested independent portions: 1.1 fixed legacy survey; 1.3 local advisory binding through actual GitGuard objects into unchanged GE RunBinding; 1.4/1.6 bounded discovery; 1.5 numeric OpenSpec task references; 2.1 graph DAG; 2.2 structural exact baseline/release references; 2.3 declaration-only transition requirements; 2.4 controller port fixture checks; 2.5 separate repair assessment; 2.6 actual SG export fixture reader; 3.1 immutable local obligations.

Not full task acceptance: 1.2 has three domain schemas but no GateDecision/full evidence-bearing StageRecord; 2.2 does not authenticate a baseline; 2.3 does not yet execute evidence-backed accepted/inherited/skipped transitions; 2.4 production provider and GE-TRUST remain absent; 2.5 applicability is controller-supplied and not yet connected to the delivery evaluator; 2.6 accepts explicitly fixture-only data, production remains unsupported; 3.1 context digest is local input pending protected policy/evidence orchestration. No FG-GATE claim. All OpenSpec checkboxes remain unchecked.

Ruling: preserve stricter approved transition design despite legacy direct acceptance edges; cost is no legacy parity or migration until differential review.
Ruling: graph ordering is lexicographic instead of legacy FIFO ready ordering; deterministic independent fixture semantics only, possible display-order differences.
Ruling: reject dirty candidates in the current advisory adapter; GitGuard commit digest does not represent dirty worktree bytes, so no invented dirty-snapshot binding. Future explicit dirty-snapshot coverage requires separate contract.
Ruling: use local sibling GuardEngine/GitGuard/SpecGuard development APIs, not release compatibility claims. Exact source hashes and available commit IDs are recorded externally; production distribution waits GE-RELEASE.

TDD evidence in `/workspace/guard-implementation-ledger/flowguard-*.log`: Most behavior groups have a failing executable test before implementation (initial missing-import compilations are not counted as behavioral RED). The stage-discovery tests were written before implementation, but their separate executable RED was not captured because Cargo stopped at the earlier input_limits test failure; this TDD evidence gap is explicitly retained for review. Whole suite was rerun after each group. Hardening found FIFO open-before-type-check and duplicate JSON Value normalization: both reproduced RED and fixed GREEN. Transient context editing syntax error was fixed before tests; no unresolved failure is omitted.

Legacy survey: SHA 13b52b054c31f614dc272b18c195b8fa929aa595, Apache-2.0, source manifest 0.4.2, CLI help ran; registry 4 and transition 8 tests passed with isolated cloud state. Full legacy/host compatibility remains unverified.

Final verification commands: `cargo test --locked`; `cargo fmt --check`; `cargo clippy --all-targets --no-deps -- -D warnings`; JSON Schema validator 4.26.0 over nine fixtures; `openspec validate add-evidence-bound-workflow-gates --strict --no-interactive --json`. Detailed command outcomes and final commit belong in the external slice report. Root owns independent review; no subagents, push, main merge, release or notifications performed.

## Independent review follow-up

Root review accepted local tasks 1.1/1.6/2.1/2.4; task boxes remain root-owned. Review P2 probes for duplicate/digestless inventory completeness and nested-length Markdown fences were reproduced as failing maintained tests before fixes.

- `SourceInventory::complete` now validates the known inventory version, exact unique ten-stage set, valid source digests, read outcomes, expected project/feature paths and coherent feature ownership. It still does not authenticate serialized source observations.
- Native task reader now retains the opening fence marker and length, requires a matching sufficiently long whitespace-terminated closer, respects the supported zero-to-three-space block indentation, and ignores four-space-indented examples. This remains a declared native subset, not a full Markdown implementation.
- Refreshed Cargo.lock offline against coordinated sibling dependencies; GitGuard now depends on GuardEngine/time and the enabled time feature adds itoa. No unrelated crate version changes.
- Promoted independent GG v1alpha2 scope assertions into context_binding: canonical allowed_paths [[97]], old schema rejected, unsorted paths rejected, both real SHA-1/SHA-256 repositories.

Fresh commands after fixes: `cargo test --locked` (24 passed), `cargo fmt --check` (exit 0), `cargo clippy --all-targets --no-deps -- -D warnings` (exit 0). Test logs and exact source HEADs are in the external review-fixes report. All prior production/FG-GATE limitations remain; no task is checked on the strength of this fix alone.
