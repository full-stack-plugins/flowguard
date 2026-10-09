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

## Independent full-work completion review fix

Root reproduced old ALLOW completion accepted by a new same-run/candidate/coverage reservation whose producer principals had changed. Structural envelope checks were insufficient. Maintained RED reproduces this P2; GateRun now freezes original private full-work identity before evaluation, and both public stores accept only typed GateRun completion. Internal persistent replay verifies the recorded original work digest. Log schema advances to v1alpha2, old logs reject; no caller digest rebinding API. Fresh changed-policy evaluation remains correctly closed and can append its own error outcome. No task acceptance added pending independent fix review.

## Durable independent acceptance registration

Root closed full-work completion P2 at `1ffd34a` and accepted local **4.1/4.2** after independent59-test/3-schema-vector recheck; evidence `/workspace/guard-implementation-ledger/flowguard-full-work-independent-review.md`. Accepted total is **10/30**. Owner-private cooperative Linux local-filesystem limitations remain: no hardware power-loss/network filesystem/hostile same-UID claim. Earlier pending-review entries are historical. CLI5.1 is now in progress and remains unchecked.

## Read-only CLI5.1 slice (review pending)

Added actual binary and all four command paths: discover, stage status (source-observation scope, qualification unassessed), evidence verify (integrity only), gate check (real GG binding and GE FlowGuard report). Strict pinned request and explicit separate fixture-authority options; no production actor inference, output writes or persistent-store activation. Actual binary tests cover0/2/3/4, malformed prebinding empty stdout, bound error/null and requested cancellation, default unavailable authority, partial missing provider, revoked records reloaded, unknown --report, exact output artifact bytes and immutable upstream REQUIRE_APPROVAL when a new approved envelope is supplied.

The local CLI profile is ready for independent review; task5.1 remains unchecked. Signal/process-death supervision and authenticated host clocks remain outside the synchronous CLI profile. See [CLI ADR](adr/cli-contract.md).

## Read-only CLI independent acceptance registration

Root accepted local task **5.1** at `8c62afb`, based on `/workspace/guard-implementation-ledger/flowguard-cli-independent-review.md`: 67 existing tests plus 2 independent probes, no reproducible P1/P2. Accepted total **11/30**. This is the explicit read-only CLI profile; fixture receipts do not authenticate production identities or authorize execution. Earlier pending-review entries above are historical.

## Fixed legacy declaration differential (review pending)

Task5.2 local slice adds bounded read-only parsing of the surveyed exact legacy revision and fifteen captured actual legacy reads, including accepted/inherited/skipped/invalidated, malformed/unknown/fault inputs and duplicate/fenced-table differences. Forty-nine legacy transition pairs are compared to the existing FlowGuard transition owner. No old declaration supplies new evidence authority; migration stays unsupported. Capture replay is byte-identical. Four focused tests and the full71-test pinned regression pass; exact source pins and the initial moving-GitGuard regression failure are recorded in `/workspace/guard-implementation-ledger/flowguard-legacy-slice-report.md`. Task5.2 remains unchecked pending independent review; accepted total stays11/30.

## Legacy parser allocation review fix (review pending)

Root's independent2MiB delimiter-row probe reproduced a32MiB column-vector allocation before ignoring an unknown field. Replaced full split collection with borrowed field/value iteration and one third-column check. Existing ignore-unknown/reject-known-extra semantics remain. Real allocator regression covers both inputs; maximum observed allocation is4MiB for bounded source buffering, compared with32MiB before. Pinned full72-test suite plus unchanged original independent probe pass (73 total), with Clippy/fmt checks. No task acceptance added;5.2 remains pending independent fix review.

## Legacy differential independent acceptance registration

Root closed allocation P2 at `7b70bda` and accepted local **5.2**, based on `/workspace/guard-implementation-ledger/flowguard-legacy-independent-review.md`:72 product tests plus unchanged original allocator probe pass; actual15-read/49-transition legacy capture reproduces exact fixture SHA. Accepted total **12/30**. Scope is fixed-revision declaration differential with unsupported migration, not complete legacy CLI/host compatibility or authorization. Earlier pending-review statements are historical.

## Scoped specialist independent acceptance registration

Root accepted local **3.2** at `ef916d8`, based on `/workspace/guard-implementation-ledger/flowguard-scoped-specialist-independent-review.md`:77 tests plus2 independent probes, Clippy/fmt/OpenSpec pass. Accepted total **13/30**. This is protected-controller scoped-source consumption with real AG/GG provenance and explicit fixture authority; actual SG/TG scoped chains and scoped CLI are not claimed.

## Stage qualification independent acceptance registration

Root accepted local **2.3** at `24ea862`, based on `/workspace/guard-implementation-ledger/flowguard-stage-independent-review.md`:88 tests plus an independent18-node shared-DAG budget probe, Clippy/fmt/OpenSpec pass. Accepted total **14/30**. The probe rejects at1024 visits after113 seconds; this is a work bound, not a latency SLA. Scope is protected local controller qualification; production authority remains external.


## Reviewed immutable inheritance acceptance

Task 2.2 accepted at `e68b93082b621d2fbf0da60a144e3b44dd0998e7` after the independent 17MiB metadata P2 was closed. The original three probes pass unchanged; 98 maintained tests plus two additional boundary probes pass. Borrowed metadata/count/aggregate and serialization budgets run before hashing/cloning full candidates. Real committed 02/07 parent references, independent child requirement scopes, current approval refresh and exact typed09 prerequisite composition remain intact. Historical parents are explicit immutable references; no implicit latest substitution is permitted.

This acceptance covers the local controller inheritance/composition profile, not production SG authority, stage10 qualification or release execution. Independent allocation probes establish bounded call-site behavior, not global RSS. Evidence: cloud ledger `flowguard-baseline-budget-independent-review.md` and original `flowguard-baseline-independent-review.md`.


## Final local discovery acceptance

Task 1.4 accepted for the existing immutable-checkout local profile after root's targeted recheck (three discovery tests passed; code byte-identical to the independently reviewed archive). Exact ten-stage paths/owners/digests, missing/oversized/unknown-version outcomes, forged completeness rejection and unchanged complete source-tree bytes are verified. No `.flowguard/` is created. The original implementation's missing initial RED log remains an explicit historical process gap; no retrospective RED claim is made. Root accepts the functional deliverable without manufacturing a rewrite to fabricate history. Hostile concurrently mutable filesystems and approval authority are outside this profile. Evidence: cloud ledger `flowguard-stage-discovery-final-review.md` and the original independent `flowguard-review.md`.
