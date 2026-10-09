# Five actual Guard artifacts through the CLI

From a coordinated, pinned FlowGuard checkout with its sibling crates:

```sh
cargo build --locked --bin flowguard
python3 examples/cli-five-guards/run.py --flowguard target/debug/flowguard --output /tmp/flowguard-five-guards-demo
```

Choose a new output directory. The script requires Python3 and `/usr/bin/git`, installs nothing, and uses the existing real five-provider `approval-bridge` corpus. It clones the committed `source.bundle` into that private directory. It does not rerun candidate programs or native analyzers. Their original captured bytes and provenance hashes are verified and preserved.

The output includes per-case `request-<case>.json`, `authority-<case>.json`, raw CLI stdout/stderr and `summary.json` with complete commands. Each command is directly replayable, for example:

```sh
python3 - /tmp/flowguard-five-guards-demo/summary.json <<'PY'
import json, subprocess, sys
command = json.load(open(sys.argv[1]))[0]['command']
raise SystemExit(subprocess.run(command).returncode)
PY
```

All invocations are the existing `flowguard gate check` binary command. Each explicitly uses `--local-fixture-authority` and pinned request/authority files. No receipt authenticates a production identity; the protected clock is the fixture request's explicit `now`. The SG producer validity is the intersection of its original record, real baseline model and separately declared fixture baseline-issuer interval, preserving the repaired f769cca boundary. A longer review approval cannot keep an expired baseline eligible. Online issuer refresh and production authorization are not implemented by this example.

Primary outcomes:

| Case | Exit | CLI result |
|---|---:|---|
| normal | 0 | complete ALLOW, execution_authorized=false |
| missing SG evidence | 2 | partial BLOCK |
| revoked SG review approval | 4 | bound error, decision=null |
| expired SG baseline issuer interval, longer review approval | 4 | bound error, decision=null |
| controller selects a real later commit but supplied candidate object remains old | 4 | prebinding diagnostic on stderr, empty stdout |

Cancellation and missing report-file cases additionally prove the bound null result retains the same v2 required scopes. Paired-reference, explicit-null, wrong digest and unknown profile/schema negatives reject before evidence consumption. The existing CLI tests retain unscoped v1 behavior and exits0/2/3/4.

The request format remains `flowguard.cli-request/v1alpha1`, with optional paired string fields `sources` and `sources_digest`. The digest is SHA256 of the exact referenced file bytes, including any whitespace. Both fields must be omitted for legacy behavior or both non-null and valid. The referenced existing ScopedSources loader validates version, raw profile semantics, frozen obligations, exact context and profile digest. V2 changes only the independently protected per-producer source/baseline mapping; repo/task/worktree/requirements/candidate/base/group remain exact. Unknown keys are rejected. Cancellation and fallback never downgrade the profile.

The script's external setup deliberately creates one empty commit for the drift case. Before/after every CLI invocation it checks refs, HEAD, index, tracked file bytes and status are unchanged. The CLI validates the exact **requested** candidate, which may legitimately be a non-HEAD synthetic candidate; selecting the current queue candidate remains the controller's responsibility. Thus replaying the preserved normal request still means its original exact candidate, not an automatic selection of later HEAD.

Native SG remains REQUIRE_APPROVAL in both its original and reference-bearing envelopes; FlowGuard produces a separate ALLOW only when the scoped fixture approval is valid. CodeGuard original aggregate exit3 and native report bytes remain unchanged. No grant, ref update, release command, hosted required check, new framework or MCP endpoint is introduced.
