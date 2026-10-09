# Controller-frozen specialist source scopes

The optional library entrypoint `prepare_scoped_gate` accepts a validated
`ScopedSources` profile (`flowguard.specialist-sources/v1alpha1`). GE's
`sourceSnapshotDigest` names the declared source scope: actual ArchGuard Cargo
inventory differs from GitGuard's complete candidate snapshot. All remaining
native RunBinding fields still require exact equality. `prepare_gate` and the
CLI retain the previous complete-binding equality behavior.

The controller freezes one source digest per frozen obligation before collecting
an envelope. The profile commits the validated context, frozen obligations, exact
sorted obligation keys and source digests; missing/extra/duplicate pins, unknown
fields/versions, malformed digests and changed context are rejected. Decode is
bounded to 64 KiB, 64 pins and 256 bytes per obligation key. JSON Schema describes
the wire shape; the library additionally checks ordering, exact obligation sets
and the canonical payload SHA-256. A profile digest is byte identity, not authority.
Controllers must protect its input independently of candidate-edited content.
Never extract expected pins from an envelope under evaluation.

The integration tests prepare actual GitGuard candidates and actual ArchGuard
GitCargoEvidence, read its immutable `binding()` before `run`, then verify the
result against the repository/candidate through ArchGuard's native bundle loader.
Coverage expectations come from the independently chosen controller contract and
profile. No specialist analysis is reproduced in FlowGuard. ArchGuard is a test
only dependency. Producer authentication uses explicitly synthetic local fixture
records; no host identity or production provider is asserted.

The source-profile digest becomes a required coverage anchor, and native exact
per-specialist policies remain included in the private full-work digest. Thus
source changes invalidate reservations, stored completion and current selection,
including after reopening the local durable store. Required scopes stay frozen
for errors/cancellation. Retained exact policies are checked again at consumption,
so revocation cannot be bypassed by a cached FlowGuard ALLOW. Native GE validates
versions, bindings, raw artifact digests, coverage and authority; incomplete or
blocked evidence stays closed and invalid evidence produces a bound error with
null decision. This profile does not authenticate raw domain bytes or add a
production authority provider. Actual SG/TG scoped producer chains are outside
this slice; their independent policies still use the same native GE port.
