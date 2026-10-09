# ADR: advisory engine-backed workflow gate slice

Status: implemented local interfaces, pending independent review. Production specialist providers, authenticated policy/baseline loading, exact queue-controller authorization and FG-GATE are NOT established.

## Inputs and protected boundaries

`prepare_gate` takes an actual GitGuard-validated advisory `ValidatedBinding`, immutable `FrozenObligations`, a complete map of GuardEngine `EligibilityPolicy` objects, and a run/action/time request. The controller must supply protected obligations and policies. This crate does not authenticate caller-provided policy or re-read the repository at execution time.

`ValidatedBinding::domain_digest` includes both the unchanged shared RunBinding and the full GG v1alpha2 candidate digest. That latter digest covers allowed paths, policy and queue members omitted from RunBinding. Frozen obligations must match this domain identity. Each per-obligation policy must match exact binding, Guard/analyzer version, contract byte digest, coverage and action. Missing/changed policy produces a pre-binding diagnostic and no envelope.

Required scopes are fixed before collection: full GG scope identity, full frozen snapshot digest (including graph/source and baseline), action digest, each stage ID and each obligation digest. Missing providers or stages never shrink this set. Stage scopes here reflect presence in the frozen graph, not authenticated accepted-stage transitions; task 2.3 remains partial. An Eligible domain qualification means only the declared local frozen obligations were satisfied, not that every broader business-stage requirement is accepted. The actual reviewed GE `BoundAttempt` owns the lifecycle; final `AttemptOutput.coverage` updates observed/missing coverage while required scopes stay identical.

## Projection and reports

`flowguard.gate-mapping/v1alpha1` maps a closed set of domain gaps to actual engine `forbid_relation` assertions: block→enforce, review→review, advise→advise. Callers cannot downgrade these enforcement levels. Unknown scopes/kinds reject. Missing stage/provider observations produce partial facts with a diagnostic; actual GE evaluates these as BLOCK/INDETERMINATE even when individual gap facts could be empty.

Evidence bytes and envelopes are read without mutation. Real GE `evaluate_eligibility` verifies artifact hashes/recomputation, exact frozen expected producer/binding/coverage/contract, and freshly fetched authority records. Verified MissingApproval becomes a review gap; technical BLOCK stays a blocking gap; incomplete evidence stays partial/BLOCK. Invalid evidence, untrusted producer, stale/revoked authorization, provider outage or execution failure produces bound error/null and no report/domain artifact. Cancellation is cancelled/null. No fabricated approval-none record is introduced.

A new FlowGuard report is computed using real GE `integration::evaluate_bounded` (resource budget checked before report generation). Its envelope decision equals its own report. `flowguard.gate/v1alpha1` GateDecision is a separate strict domain artifact, referenced through `artifacts.domain`, with an explicit `authorityProfile: advisory`, action, full binding/frozen digests, mapping version, qualification, gap references and immutable upstream report references. Its action qualification is not a grant. Artifacts use content-addressed `artifact://flowguard/<sha256>` references; the library returns bytes in memory and does not publish them or create a store.

An approval reference added later requires a separately produced/authenticated upstream envelope with a new runId. The technical report bytes and REQUIRE_APPROVAL verdict remain unchanged, and the original envelope is preserved. Tests explicitly model this with a fixture authority; no real host or identity provider is claimed.

## Consumption and freshness

`GateRun::evaluate_eligibility` first checks the requested action, then re-runs actual GE eligibility for all retained immutable specialist envelopes/bytes/policies against current authority. Only then does it assess its own report through GE. Upstream revocation after a prior FlowGuard ALLOW therefore closes consumption without rewriting either historical report. Errors use `GateConsumptionError`, distinct from a successful query with a negative eligibility result.

The caller must still supply the current protected FlowGuard eligibility policy, current trusted time and current candidate context. Changes to current protected required scopes/contract/binding are rejected by GE. This in-memory check is not a persisted CAS store, queue event authenticator, full source re-observer, cross-process invalidation service or privileged executor.

## Limits and validation

At most 64 total required scopes, 192 projected gap facts, and 16 MiB total retained specialist bytes plus serialized envelopes are accepted per gate. Engine validation retains its own stricter artifact/recomputation budgets. General cancellation/timeouts outside explicit cancellation and filesystem publication remain future work. Unexpected internal serialization/engine-finalization failures can return an explicit TransportDiagnostic; this slice has not completed all task 3.5 failure-normalization scenarios.

GateDecision's JSON schema and validated loader reject unknown version/fields, unsupported mapping version, invalid digest shape and inconsistent decision/qualification. Structural loading does not authenticate the artifact; consumers require its exact envelope artifact digest and real report/authority verification. Existing StageRecord remains the narrower graph-node schema, so task 1.2 as a whole remains partial.

`fixtures/gate_mapping/` captures a real GE-evaluated local fixture with actual temporary Git objects verified while generating the binding. The captured fixture later proves byte consistency only; those temporary objects and the test approval port are not production provenance. Live integration tests recreate actual Git repositories. SG's native schema adapter remains tested against the real sibling library, but these gate specialist reports are explicitly synthetic GE fixtures; task 3.6 real AG/CG/TG/SG/GG interoperability is not claimed.
