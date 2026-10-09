# Queue adapter implementation plan — paused draft

Status: **unimplemented, uncompiled, and not accepted**. FG task 4.5 remains incomplete.

The following is the existing proposed plan, not a report of implemented behavior.
No merge-queue adapter or library export was written. No queue test was run.

The original API test draft at `tests/queue_candidate.rs` is preserved byte-for-byte
as `docs/drafts/queue_candidate.rs.disabled`. It references APIs that do not exist.
It is outside Cargo test discovery so that preserving the draft does not break
normal compilation. It is not an executable acceptance test or a passing fixture.

Original and archived draft SHA-256:
`e680b9fb15fafb46ed0ea08498b535674b54b6a45a3841ff1d03bcdd5f5599e7`.

Implementation and verification were paused before starting this slice. Resume
only under the next authorized implementation scope. Existing local-fixture and
production-authority boundaries remain unchanged.

---

# FG4.5 local authenticated-controller event adapter

Approved bounded scope: readonly actual GG candidate binding; explicit fixture authority only, no product object creation, refs writes, executor, grant or release.

ProtectedQueue freezes controller principal/profile, queue/repo/task/worktree identity, GG policy digest and exact allowed paths. ControllerEventPort resolves a reference against its independently owned current event registry. A caller payload/hash is never treated as proof. Fixed fixture authority registers only events it actually emitted after creating real temporary Git candidate objects.

Each current event records version, event reference, sequence, validity/revocation, merge group/base/candidate, ordered member OIDs and per-member requirement IDs. The adapter derives the canonical union of member requirements, verifies event identity/budgets, compares all fields to actual CandidateSnapshot, calls GG validate and FG bind. It does not reproduce merge algorithms or synthesize objects itself.

Session uses exclusive &mut access, monotonic time and event sequence. Time is observed before budget/authority failures. Once a correctly identified controller event is observed, its sequence high-water mark advances before expiry/candidate failure; stale events cannot revive after failure. Revalidation refreshes current port, clock and actual candidate on each consumption; observations cannot serialize into authority or grant.

Tests: actual base/A/B → M1, advance base and compose M2; exact M2 succeeds, head/M1/old requirements/wrong member order cannot replace it. Wrong issuer/profile/reference, replay, expiry/revocation and clock rollback fail. Expired/bad candidate new event prevents old sequence resurrection. Read-only refs/index/source invariance and bounded rejection are checked. SHA object IDs are validated through GG, not shape alone.

Owned files: adapters/merge_queue/mod.rs; tests/queue_candidate.rs; docs/adr/0013-merge-queue-events.md; precise src/lib.rs export only. No bridge/DAG/invalidation edits. Shared target build window coordinated with root.
