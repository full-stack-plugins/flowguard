# Joint local five-producer acceptance

The original task 5.5 remains unchecked until independent acceptance. This slice
builds fresh actual SG/AG/CG/TG/GG artifacts for A, B, M1 and M2. It does not edit
or relabel the earlier approval-bridge corpus. The fixture is controlled local
execution with fixture authority, not authenticated production CI or a hosted
required-check installation.

`fixtures/providers/joint/capture.py` constructs committed A and B changes from a
common source, then real merge-tree/commit-tree M1 and M2 objects. M2 advances the
base and reverses the ordered member list. Separate clean checkouts are required
for current GitGuard byte validation; no candidate cleanliness check is bypassed.
A and B have independent task/requirement bindings. M1/M2 share the queue task,
but their candidate/base/group/order bindings differ. The local store holds A/B
reservations concurrently; producer processes themselves run sequentially.

The project `02-architecture` node uses the same project baseline file digest.
Each requirement closure has its own actual SpecGuard ApprovedBaseline record;
A/B digests differ and are never interchangeable. The project node in the joint
invalidation graph is an unexecuted controller source tracker, not a fabricated
accepted stage or an approval. Its change withdraws A/B/M2 publications through
explicit edges. The existing accepted-stage APIs are not exercised by this node.

| Actual exercise | Required result |
| --- | --- |
| Fresh five-provider A, B, M1, M2 | Four CLI exit 0 / ALLOW; execution unauthorized |
| A artifacts presented to B or M1 | Bound error / null decision / exit 4 |
| M1 artifacts presented to M2 | Bound error / null decision / exit 4 |
| A baseline authority expiry; B review revocation | Bound error / null decision / exit 4 |
| A protected rule/analyzer descriptors change | Withdraw only A; B remains published |
| M1 completes after M2 publication | M1 history retained, stale publication rejected; A/B untouched |
| Shared project node changes | Withdraw A/B/M2, causal paths retained, history bytes identical |
| Native CodeGuard and projection | Same pinned native executable and lint argv, schema 0.13.0 / native exit 3 retained, domain bytes exactly equal native stdout, current projection ALLOW; no native CLI/schema edits |

The native CodeGuard observation is only the fixed Ruff fixture below. It does
not establish parity for other tools, CodeGuard versions, or all native commands.
Rule/analyzer drift changes protected controller descriptors and checks withdrawal;
it does not run a second analyzer version or certify cross-version compatibility.
M1/M2 are real local objects with a controlled member event description; this is
not production controller-event authentication. No grant, merge, ref update or
release executor is called during provider/consumer windows. Git writes occur
only in explicit fixture setup between those windows.

## Reproduction and retained evidence

Source `/workspace/guard-toolchain/env.sh`. Build `generate_joint` and
`joint_invalidation` examples in the existing FlowGuard target with jobs 1 and
`CARGO_INCREMENTAL=0`. The standalone `fixtures/providers/joint/helper` Cargo
package compiles against TestGuard `2d2189f9b083ff719667613a7b0f44c3b78c0088`
and GE `b0e4b445e1a20a9354d55ebb41bbedaf16ec8a74`; it adds no FlowGuard dependency.
Its locked helper executable is copied to the ledger fixed-tools directory,
SHA256 `d9547e4ceba0770c16d64666f1ef9294bb5900932eebe06d38fd4e5d921b4a16`.
Do not acquire another worker's target without a lease. The helper and native
binaries are explicit test prerequisites, never silently installed.

```sh
FLOWGUARD_JOINT_OUTPUT=/absolute/new-capture-directory \
  CARGO_INCREMENTAL=0 cargo test -j1 --test joint_gates -- --ignored --nocapture
```

For an archived source checkout sharing the original target, set
`FLOWGUARD_JOINT_GENERATOR` and `FLOWGUARD_JOINT_INVALIDATION` to the explicitly
built example paths. Fixed consumer FlowGuard source is `56a2c9576caf382a6bb5f716c707c77c94606599`,
SHA256 `b49af236e8fdd3e41742d8c4168bf0dffce7cb463ea9a049fc5bc6b0708a5048`.
CG native `f5661d51dd1b2629f3b87b726fb09a9060ce7958`, projection
`72d80ae453c0380ad7061c10847595c5a4ac1736` and Ruff executable hashes are enforced
by the capture runner, preserving the original fixture tool versions.

`producer-manifest.json` records source pins/dirty states, generator input hashes,
tool hashes, native command argv/exits/stdout/stderr hashes, exact four profiles,
and every retained pre-consumption artifact digest. `result.json` records actual
consumer commands/exits and result hashes; each raw stdout/stderr is retained.
`invalidation.stdout.json` records causal paths, scoped baseline digests and
historical-byte preservation. Consumer replay rechecks every captured artifact
hash. Author mixed-working-tree runs remain supplemental; independent review must
rebuild from fixed source. Existing capture directories are never overwritten by
the generator. This fixture does not replace required production host/issuer work
or resolve the legacy 13b52b05 migration differences.

The manifest labels live Git identities as runtime observations, not tool build
provenance. Archives report no Git identity rather than accidentally attributing
the enclosing checkout. The author's final example build record is
`/workspace/guard-implementation-ledger/flowguard-joint-build.json`: it checks all
Rust source/Cargo file hashes before and after building with GE b0e4b445, SG
8fc3f9b0, AG d6c8235d and GG f6c7368c. TG helper source 2d2189f is unchanged in
current a61975c for src/Cargo files. Independent reviewers supply their archived
source pins alongside the generated per-file and executable hashes.
