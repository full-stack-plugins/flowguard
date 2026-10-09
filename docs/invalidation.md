# WIP: FG4.3 local invalidation API and boundaries

**Preservation commit only; task 4.3 remains incomplete and unchecked.** The current implementation has not been compiled or tested. Ten tests are authored, but only earlier missing-API RED and two assertions against a compiling no-op stub were executed. Those RED results do not validate the implementation below. No GREEN or acceptance is claimed.

Known outstanding issues:

- Comparing entire `Freshness` currently invalidates work when only `observed_at` advances; the intended no-op behavior below is not implemented correctly.
- Per-node observation rollback is not yet rejected separately from snapshot-time rollback.
- Clearing MemoryRunStore current does not revoke detached GateRun/QualifiedStage objects. Current-publication validation is not automatically connected to every qualification consumption path. A controller must still check current and perform fresh GE/provider validation; a future interface solution has not been implemented.

The remainder describes the proposed design, not verified guarantees. Work was paused before current-source compilation, focused tests, full regression or independent review. No new execution or production authentication is enabled.

FrozenInputs is an opaque controller snapshot of a bounded StageGraph and an exact mapping of every node to a real prepared PendingGate, generation, ancillary ContextDigests and Freshness. It does not accept returned success reports as expected inputs. Full work comes from existing PendingGate.store_identity (full binding/policies/frozen obligations/required coverage/action/mapping/runtime versions); graph/context identity is independently protected by the controller. Unique full store targets prevent ambiguity between graph nodes. No credentials or qualification receipts are produced.

ContextDigests commits source/rules/analyzer/config/coverage/baseline/dependency observations; freshness commits expiry/revocation/artifact availability observed at a protected time. These are trusted deployment inputs, not self-authentication. All reports still need existing fresh GE/provider checks when consumed. A new positive observation cannot itself restore a publication.

changes compares old/current snapshots and protected now; it detects work/run/generation, graph membership/record/edge, ancillary digest and invalid-state changes. Propagate on both old and new edges, so removing a dependency cannot hide its formerly downstream qualification. New nodes propagate but only previously frozen old heads are withdrawn. Union may be cyclic despite each graph being a DAG; finite visited-set closure handles this without executing stages. Project10 release fan-in is just an ordinary downstream node. Unrelated feature branches remain untouched. Merely later observed_at with otherwise identical live state must be a no-op; per-node/snapshot clock rollback rejects.

Invalidation holds an immutable affected set and private old WorkIdentity/generation batch. MemoryRunStore checks every expected digest/generation and overflow under its existing mutex, then increments generations and clears publications with no remaining fallible action. Any stale/missing head rejects the entire batch before mutation. Empty batch is a safe no-op. Original attempts/report bytes remain in history and late completion append remains possible, but old publication is stale. No automatic fallback to a prior successful attempt is performed.

Limits: <=256 nodes per snapshot, <=512KiB serialized graph, <=512KiB borrowed serialized gate input per node, <=16MiB aggregate measured inputs; typed graph retains its existing edge/depth bounds. Budget before graph/identity clones or digest Vec creation. Minimal crate-private PendingGate accounting helper accesses otherwise private fields; no public getter or serialized wire is added. Only MemoryRunStore is supported. No durable replay, cross-process CAS, distributed current authority, filesystem isolation or production identity is claimed.
