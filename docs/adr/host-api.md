# Local read-only host profile

The `flowguard-mcp` binary exposes the existing Rust CLI/library through MCP stdio.
Launch with `--local-readonly-v1 ROOT PROFILE_RELATIVE_PATH EXPECTED_SHA256`.
The profile is an immutable, digest-pinned trusted-launcher input, for example:

```json
{"version":"flowguard.host/v1","tenant":"local-team","jobs":{"inventory":["discover","--root","/absolute/project","--feature","a"]}}
```

The controller supplies all job arguments. The client supplies only `job` and
`requestKey` to `flowguard_read`; it cannot select paths, tenants, CLI arguments,
authentication, approvals, or commands. One process belongs to one controller
scope. The tenant label is not proof of identity. OS process/pipe ownership and
profile/digest provisioning are trusted deployment inputs; this slice does not
implement or claim a remote issuer, tenant authentication, or multiuser isolation.
Profile paths must be contained in the launcher root. Job roots themselves are
explicit trusted-controller choices, bounded by the existing CLI path rules.

| Capability | Local profile |
| --- | --- |
| discover; stage status; evidence verify; gate check | Controller-frozen jobs, same CLI core |
| Freshness | Repeat keys recompute; changed files do not reuse success |
| Idempotency | Key cannot bind to another job; 256 keys maximum |
| Cancellation | Atomic flag checked before and after library call; canceled results withheld; retry permitted |
| stdio asynchronous cancellation | Not advertised; sequential worker has no preemption |
| shell, self-approve, grant, execution, refs, release writes | Absent; unknown fields/methods rejected |
| HTTP | Not selected; no listening socket |
| host hooks / installed UI | Not installed or tested |

Gate jobs retain their controller-pinned request `now`. Their results are technical
observations at that request time, not current wall-clock authorization or grants.
The wrapper always reports `authorization: not_evaluated`. It does not consume
LocalAuditStore artifacts or replace its tenant/retention consumption boundary.
Raw CLI stderr is withheld. Native evidence/source output is visible only within
the trusted launcher's chosen read scope; do not expose this pipe publicly.

The server implements MCP 2025-11-25 initialize/version negotiation, initialized,
ping, tools/list, and tools/call. Ordinary clientInfo metadata and capability
objects are accepted as informational declarations, never permissions. Unknown
methods and tools are rejected. Only one tool is advertised, readOnlyHint=true,
destructiveHint=false, taskSupport=forbidden. Sources are never written.
Requests are capped at 8192 bytes; oversized frames are drained before the next
request. Duplicate JSON keys are rejected recursively. Profile limit 256 KiB,
16 jobs, 32 arguments/16 KiB per job, and CLI output limit 8 MiB bound this profile.
The output limit is checked after the bounded CLI call, not a streaming quota.

`tests/host_interface.rs` runs actual binary stdio using installed Python MCP SDK
1.29.0 (test dependency, not runtime dependency), checks initialized/list/call,
permission-field refusal, unchanged source hashes and changed-source reread. It
also tests negotiated version fallback, metadata, key rebinding/capacity,
cancel/retry, traversal argument refusal, malformed/oversized JSON recovery and
wrong launch digest. No Claude/Codex UI or any other client version is certified.
The full original task remains pending independent review and scope acceptance.

Protocol references: [lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
[stdio](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).
