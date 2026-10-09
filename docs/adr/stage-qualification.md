# Local protected stage qualification

`StageIntent::freeze` freezes a validated DAG node, target mode and exact direct
prerequisite plan digests. The derived stage action commits graph/source, stage,
mode and dependencies. It is generated before gate preparation. `ProtectedStagePlan`
then binds the intent to actual PendingGate full-work identity, validated candidate
context and frozen graph. The expected GE policy is generated from FlowGuard's
existing deterministic projection and required scopes before execution; no returned
GateRun supplies expected bindings or contract hashes.

Modes are Accept, ProtectedSkip with policy digest, and LocalControllerInheritance
with exact BaselineRef. These are controller-protected inputs, never candidate
configuration. Inheritance is an explicit local-controller profile: the dedicated
stage.inherit.local-controller approval attests the exact baseline included in the
action, including its reference, revision, scope and digests. The baseline reference
alone is not authenticated SG baseline evidence. The controller's current stage
approval must authorize this particular inheritance; no production baseline-provider
claim or old accepted-label migration is made.

`qualify` accepts only a valid declared transition, typed GateRun, approval reference
and exact prerequisite receipts. It checks private original full-work identity,
re-runs GateRun's retained specialist eligibility and own GE eligibility, and requires
an additional explicit stage approval even for technical ALLOW. Approval records
come from GE AuthorityProvider and are validated by GE's
validate_approval_record for the exact purpose, action, native binding, contract,
authorized principal, time and revocation. No cloned generic approval validator or
boolean qualification input exists. Skip and inherit require complete eligible
technical evidence as well; neither repairs partial/BLOCK/error results.

QualifiedStage has private fields and cannot deserialize. `consume` requires the
current protected plan and refreshes all prerequisite issuers/approvals as well as
its own. Current plan changes must be supplied by the controller: old plans are
historical values, not a source of current policy truth. Requirements include exact
plan digest, validated full candidate context, same graph and exact dependency set.
The validated DAG rejects cycles. Limits are64 graph nodes,64 direct dependencies,
256-byte identities,64 principals per set,64 recursion depth and1024 visits per
consumption. Bounds are checked before relevant clone/serialization/traversal;
repeated DAG paths count against the visit limit. Budget failures never qualify.

Trusted time and authenticated provider implementations are controller obligations.
Tests use an explicit fixture controller with exact envelope-digest producer records
and approval records; this is local capability verification, not host authentication
or permission to commit/merge/release. The receipt remains read-only and execution
is external. CLI has no stage-promotion endpoint.
