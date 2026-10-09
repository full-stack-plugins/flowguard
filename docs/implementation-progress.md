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

## Root accepted-task registration

Root's independent review and fix recheck accepted exactly **1.1, 1.3, 1.5, 1.6, 2.1, 2.4** (6/30) at their documented local/advisory scope. Only those six OpenSpec task boxes are checked. Evidence: `/workspace/guard-implementation-ledger/flowguard-review.md`, review probes, `flowguard-review-fixes-tests.log` (24 locked tests) and root's explicit acceptance after `ed6d19d`. Task 1.4 remains unchecked: its original behavioral RED capture gap is retained honestly.

## Next small slice plan: local engine-backed gate evaluation

- 3.3: versioned closed gap projection into actual GuardEngine forbid_relation types; unknown/missing/incomplete required observations cannot yield empty-facts ALLOW. Test legal/enforce/review/advise/partial and unmapped finding.
- 3.4 plus 1.2 substep: strict versioned GateDecision domain artifact, actual new FlowGuard GuardReport; immutable upstream report bytes and independently derived action eligibility. Test external approval before/after REQUIRE_APPROVAL without rewriting upstream; enforce and partial remain blocking.
- 3.5 substep: prepare real GE BoundAttempt before collection with frozen required scopes; finish with observed coverage using reviewed AttemptOutput API, required scopes unchanged; error/cancelled keep null decision. No second engine DTOs.
- 3.2/GE-TRUST local substep: consume GE verify_engine_artifacts/evaluate_eligibility ports on exact frozen expected binding/producer/coverage/contract and fixture authority, never boolean self-certification. Real production provider/cross-guard 3.6 remain absent.
- Integration: retain actual GG v1alpha2 candidate scope digest within frozen scope identity to avoid using narrower RunBinding as reuse key. SG export remains fixture-only at its current reviewed schema.

All new task claims remain partial until root review; no production FG-GATE or execution grant. Execute TDD and commit this slice for review before broadening.

## Gate slice outcome pending review

Implemented closed actual-GE projection, a versioned GateDecision schema/loader, real own-report generation, GE BoundAttempt final coverage, immutable upstream retention, exact policy/evidence verification through GE eligibility, bound provider/error/cancel outcomes, and consumption-time upstream revocation rechecks. Full GG v1alpha2 and full frozen/action identities are carried in required scopes. All required scopes are fixed before collection.

TDD: named executable RED precedes projection/report/schema changes; additional RED→GREEN cases found narrower binding scope identity, missing frozen-snapshot identity, cross-action consumption, and cached upstream revocation. Specialist evidence regression covers changed base/group/requirements, tampered bytes, changed analyzer, unavailable/untrusted producer and execution failure. Captured JSON fixture is produced through the real engine, with explicitly synthetic specialist/authority fixtures.

No new task box is checked. Tasks 1.2/3.1/3.2/3.4/3.5 remain partial until full specified scope and review; 3.3's local mapping is ready for review. No production provider, protected loader, full stage eligibility transition, persistent run/invalidation/queue layer, CLI/host enforcement or real task 3.6 cross-provider acceptance. See `docs/adr/gate-contract.md` and `/workspace/guard-implementation-ledger/flowguard-gate-slice-report.md` for exact tests, compatibility revisions and remaining limits.

## Controller-pinned local policy slice (review pending)

`policy::load_and_freeze` adds explicit authorized-root reads, exact raw-byte pin checks, closed versioned policy decoding and exact context/baseline/graph matching before existing obligation freezing. No candidate-side auto-discovery or storage write is introduced. Six new policy tests plus existing suite pass (41 integration tests); focused RED evidence is retained in external implementation ledger. See [local-policy-loader ADR](adr/local-policy-loader.md). Tasks 2.5 and 3.1 remain partial; no new checkbox is accepted. The controller still owns pin provisioning, action applicability and upstream GE eligibility policies; this local integrity profile does not authenticate a production configuration source.

## Second independent acceptance registration

Root accepted local tasks **3.3 and 3.4** at commit `0a5e912`, based on `/workspace/guard-implementation-ledger/flowguard-gate-independent-review.md`. Total accepted: **8/30** (1.1/1.3/1.5/1.6/2.1/2.4/3.3/3.4). These are real engine projection and independent report semantics with synthetic authority fixtures, not production FG-GATE. Earlier pending-review statements above are historical. Task 1.4's RED gap remains; policy loader `bc4c315` awaits separate review.

## Process-local run store slice (review pending)

Added `MemoryRunStore`: actual PendingGate-derived full work identity, separate idempotency/run IDs, immutable terminal-envelope bytes, mutex-atomic generation advance, single-assignment publication, and independent target histories. Seven tests cover duplicates/conflicts, authority-policy drift, changed-work late actual ALLOW after BLOCK, concurrent CAS/publication, and A/B use of one real Git worktree. Full suite: 48 locked integration tests passed; fmt/clippy/diff checks passed against updated GE c80ec32 local tree. See [memory store ADR](adr/memory-run-store.md).

This completes only the explicit single-process memory profile of 4.1/4.2; tasks remain unchecked because persistent authorized-directory/schema, crash/restart fault tests and multiprocess behavior are absent. Storage validates structure/binding and never caches authority or grants eligibility. No push or execution writes.

## Durable local attempt slice (review pending)

Added opt-in Linux `DurableRunStore`, a strict versioned attempt-log schema, controlled owner-private storage, descriptor-anchored access, cross-process flock, fsync/rename persistence and replay through existing MemoryRunStore. Actual A/head1→A/head2 Git commits and independent B prove late ALLOW cannot overwrite current BLOCK or cross-satisfy B after reopens. Actual subprocesses test one-winner CAS and before/after-rename interruptions. A generation read supports uncertain-outcome reconciliation. See [durable store ADR](adr/durable-run-store.md).

No new checkboxes: 4.1/4.2 are now presented for local-profile acceptance review, not self-accepted. Production authority, remote coordination/hardware power-loss and execution intent remain out of scope. Current total accepted stays 8/30.
