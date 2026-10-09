# Read-only CLI v1alpha1

Status: implemented, task5.1 local-profile independent review pending. No host service, network listener, persistent store activation, approval issuance or Git execution writer. Build with `cargo build --locked --bin flowguard` in the coordinated sibling checkout; executable is `target/debug/flowguard`.

## Commands and exit status

```
flowguard discover --root /absolute/project --feature feature-slug
flowguard stage status --root /absolute/project --feature feature-slug
flowguard evidence verify --root /absolute/evidence --envelope envelope.json --contract contract.json --facts facts.json --report-file report.json
flowguard gate check --repo /absolute/git-worktree --input-root /absolute/controller-input --request request.json --request-digest sha256:<exact-request-bytes> --run-id unique-attempt-id
```

Only these commands/options exist; duplicates, unknown options (including `--report`), missing values, extra positional arguments and non-UTF8 argv reject. `--report-file` is exclusively an input to evidence verify, never an output destination. All artifact/request paths are relative to the explicitly supplied authorized root. No implicit candidate inference from task IDs, no config auto-discovery, no output file writes.

| Command | stdout | Exit |
|---|---|---|
| discover | versioned native inventory | 0 all ten source reads complete;2 missing/rejected sources;4 invalid command/root/feature |
| stage status | native source inventory and qualification `not_evaluated` | same as discover |
| evidence verify | versioned `valid:true`, technical decision, authority `not_evaluated` | 0 exact native artifacts verify, regardless technical decision;4 input/verification error |
| gate check | versioned advisory own-envelope/artifact bundle | own ALLOW0, BLOCK2, REQUIRE_APPROVAL3; bound error/cancelled4 with decision null |

Prebinding errors have **empty stdout**, redacted diagnostic on stderr and exit4. A completed result carries its own engine decision; a bound error is an actual GE error envelope with no report/domain/decision. No unrelated logging on stdout. A failed stdout write returns4 and cannot guarantee a complete JSON document; this is an OS stream failure, not a fabricated report.

Source `Read` does not mean stage accepted. Stage status intentionally exposes the implemented source-observation layer and makes qualification unassessed; authenticated stage transition execution is a separate pending task. `evidence verify` checks exact raw bytes, native recomputation and envelope equality through GE, not producer identity, Git provenance or action authority.

## Gate request

`request.json` is a closed `flowguard.cli-request/v1alpha1` object with these required controller inputs:

```
{
  "version": "flowguard.cli-request/v1alpha1",
  "invocation": {
    "repo_candidates": ["repo-label"], "task_candidates": ["task-label"],
    "worktree_id": "worktree-label", "requirement_ids": ["requirement"],
    "candidate_oid": "<full real Git OID>", "base_oid": "<full real Git OID>"
  },
  "candidate": "candidate.json",
  "frozen": "frozen.json", "frozen_digest": "sha256:<frozen domain digest>",
  "action": "commit",
  "started_at": "2026-10-09T00:00:00Z", "finished_at": "2026-10-09T00:00:00Z",
  "now": 1791504000,
  "specialists": [{
    "scope": "flowguard.obligation:sha256:<full obligation digest>",
    "producer": {"guard":"specguard","version":"1","analyzerId":"fixture.spec","analyzerVersion":"1"},
    "producer_principals": ["fixture-producer"], "approval_principals": {},
    "evidence": {"envelope":"upstream/envelope.json","contract":"upstream/contract.json","facts":"upstream/facts.json","report":"upstream/report.json"}
  }]
}
```

Placeholders are not runnable identities. Candidate is actual GitGuard v1alpha2 serialized CandidateSnapshot verified against `--repo`; frozen is the existing strict FrozenObligations serialization and must match its independently pinned domain digest. Full context equality includes the GG scope binding. One specialist config per frozen obligation is required; producer identity/principals come from pinned request, while binding/action/contract digest/required coverage derive from exact frozen controller inputs, never returned evidence. `evidence:null` (or absent evidence) represents a missing provider, retains required scope and gives partial BLOCK. Every file read uses bounded authorized-root access. Aggregate controller/evidence reads16MiB, request1MiB, at most64specialists; gate's own stricter scope/projection budgets remain.

The controller supplies whole-second UTC `...Z` timestamps and Unix `now`; both RFC3339 values are checked and finished>=started before binding. These are explicit controller/fixture clocks, not authenticated host time. `--run-id` is explicit and must be fresh for a new attempt; this stateless CLI does not enforce uniqueness across processes or reopen a result store. Every invocation rebinds Git, rereads bytes and evaluates anew; no cached ALLOW. `--cancel` on gate check requests cancellation after successful binding and before collection, producing a real cancelled/null envelope. Signal-based process interruption/abrupt termination and deadlines are not implemented by this synchronous local CLI; generic process supervision remains task3.5/host scope.

## Explicit fixture authority

Default authority is unavailable, so supplied completed evidence cannot authenticate itself. To exercise local synthetic records, explicitly add all four options:

```
--local-fixture-authority --fixture-authority-root /absolute/fixture-controller --fixture-authority receipts.json --fixture-authority-digest sha256:<exact-receipt-bytes>
```

This marks stdout `authority_profile:local_fixture`; output always includes `execution_authorized:false`. It never authenticates the real caller or host. File has closed version `flowguard.authority-fixture/v1alpha1`, `producers` and `approvals` arrays (<=64 each,1MiB file). Producer record fields: principal, exact native producer, envelope_digest (SHA256 of canonical native envelope serialization), issued_at, expires_at, revoked. Approval record fields: reference, principal, purpose, action, native binding, contract_digest, issued_at, expires_at, revoked. Timestamps are Unix seconds. Duplicate producer digest or approval reference rejects. GE checks exact scoped records, allowed principals, validity and revocation; no boolean self-approval replaces its checks. Empty/missing approval does not invent a confirmed anonymous actor. Receipt lookup is exact; an unmatched envelope is untrusted. Each invocation reloads receipts, so changed revocation is observed. Pins protect byte integrity only; a caller controlling both bytes and pins can replace all fixture configuration. This is not a production provider.

## Output and preservation

All successful JSON output has `apiVersion:flowguard.cli/v1alpha1`. Gate bundle contains native `envelope` and `artifacts` with **exact JSON strings** `contract`, `facts`, `report`, `domain` (null artifacts for error/cancel). Extract the strings' UTF8 bytes to verify references; do not reserialize parsed report objects and assume byte identity. Domain remains advisory GateDecision. Upstream source files/reports/envelopes are untouched, and no controller grant is issued. CLI does not serialize the GateRun private completion identity or accept bytes as a fresh typed store completion.

Tests launch the actual binary against real temporary Git candidates and real GE-generated synthetic specialist reports. All0/2/3/4 outcomes, exact artifact verification, partial, default authority unavailable, cancelled/null, malformed bound evidence, prebinding errors, pinned configuration, changed revocation and unsupported --report are exercised. Production providers, host required-check enforcement and full authenticated stage semantics remain separate tasks.
