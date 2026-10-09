# Local controller inheritance and release composition

Existing resolve_baseline and validate_release are structural checks; they are not
authentication. FrozenBaseline adds actual Git parent verification with the reviewed
GitGuard reader. The parent CandidateSnapshot is fully validated when freezing;
revision, repository and full requirement set must match the protected BaselineRef.
Only docs/project/02-architecture.md and07-standards.md regular blobs are supported.
Their exact committed bytes must hash to the reference. The frozen identity commits
the entire GG candidate digest (including scope/members/policy) and full baseline
reference. Caller repo IDs remain controller identities, not host authentication.

InheritedEdge binds that immutable parent to the validated complete child context,
including independent task/worktree/candidate/scope and requirements. It queries the
existing authenticated FG ApprovalProvider port for baseline.inherit.local-controller,
exact target digest, policy/revision, issuer/role and child scope. Approval records
covering A/B may share parent identity, but A/B edges cannot substitute. The edge
cannot deserialize, contains no accepted boolean and grants no stage/execution
permission. Stage qualification remains a separate explicitly protected interface;
controllers must consume relevant inheritance edges when using this native baseline
profile. No automatic raw-reference-to-stage qualification adapter is installed.

Every edge consumption supplies the current protected parent and current child,
checks complete identities, reopens immutable candidate/base/member objects and
rereads the exact parent file, then refreshes approval. Current parent/child changes,
unavailable objects, wrong bytes, expired/revoked/wrong-scope approvals and provider
outage fail closed. At consumption the already validated parent snapshot is matched
by its full frozen digest; today's worktree cleanliness is not compared with the
historical commit. A newer HEAD does not alter an old immutable reference. A newly
selected parent requires a new independently approved edge. Source reads retain
GitGuard budgets; references cap256 requirements and256-byte identities before
cloning. This is a local controller profile, not authenticated SG production baseline
integration.

ReleasePlan provides read-only project10 dependency composition. Its local profile
requires the release node's direct dependency set to equal every feature09 node in
the protected graph. Additional prerequisites belong upstream of those09 nodes.
Features are keyed by graph owner; missing, extra and unknown features reject.
Expected values are exact ProtectedStagePlan digests, not caller claims about doc
hashes. Plans must match graph,09 node and release repository/candidate/base/group.
Feature tasks/worktrees/requirement subsets may differ and remain committed in each
plan. consume checks current full release binding and exact current feature plans,
then calls each typed QualifiedStage.consume, refreshing issuer/approval/dependencies.
No serialized accepted string or selfreported digest can satisfy this path. This
composes prerequisites only; it neither qualifies the10 stage itself nor executes a
release. At most64 graph nodes/features with bounded graph identities are accepted.

Fixture tests use real committed parent files and native GE GateRuns, plus explicitly
synthetic controller authority. Provider identity, trusted time and supplying the
current protected policy/parent/plan remain controller responsibilities. Existing
schemas and declaration parsing do not acquire new authority.

## Candidate and context admission budgets

All incoming CandidateSnapshot values are borrowed-preflighted before Git validation,
cloning or digest allocation, including on inherited-edge consumption. The same
helper protects bind before creating any ValidatedBinding. Text fields are capped
at256 bytes, requirements at4096, merge members at64 (64 bytes each), allowed paths
at256 (4096 bytes each), and aggregate visible metadata/path bytes at64KiB. Counts
are checked before visiting members. A non-allocating512KiB counting JSON sink also
bounds private GG schema/object-format strings and escaped/numeric wire expansion
before validation/hash; no encoded Vec is produced. This limit is an FG admission
profile, not a claim that all larger native GG inputs are invalid.

InvocationInput uses the same256-byte text/64KiB aggregate/4096-requirement bounds.
Opaque ValidatedBinding context is checked at inheritance, release, stage-plan and
gate boundaries before hashing; its only constructor now enforces these budgets.
Budget failures are prebinding InputBudget diagnostics or domain InvalidParent/
Child/Binding errors, never a truncated complete result. Existing narrower baseline
scope limits remain. Allocator tests cover17MiB task/schema/object-format fields;
metadata/path/count/aggregate tests cover the full visible candidate shape.
