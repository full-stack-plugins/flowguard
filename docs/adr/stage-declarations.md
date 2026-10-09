# Stage declarations are not qualifications

`StageDeclaration::observed` wraps an untrusted observed state. It has no
qualification getter, approval boolean or conversion into a GateRun. Even an
observed accepted/inherited/skipped label is only a declaration.

`apply_declaration` is read-only: ordinary allowed edges return a new declaration;
protected edges return the exact requirements from the existing transition table;
illegal edges reject. Every rejection preserves the original declaration.
Pending acceptance requires current technical evidence and approval to become
accepted; skip requires protected policy and approval; inheritance requires an
exact baseline and approval. This API does not execute any of those promotions.

All49 ordered state pairs are tested against a literal expected matrix. Terminal
states can be invalidated, then resubmitted to pending_acceptance; resubmission
never restores prior acceptance. Technical ALLOW/BLOCK and execution
completed/error/cancelled are not valid stage-state strings.

The pinned legacy revision13b52b054c31f614dc272b18c195b8fa929aa595 allowed several
direct accepted edges and used in_progress as reopening; FlowGuard deliberately
requires invalidation/resubmission and explicit qualification. The existing
fixtures/legacy/differential.json preserves those differences. No parity or
migration claim is added. Full qualification consumption is a separate local
controller integration, not implied by this declaration wrapper.
