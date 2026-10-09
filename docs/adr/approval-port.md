# ADR: controller-side read-only approval port

Status: local port accepted for fixture testing; production provider UNSELECTED.

`ApprovalProvider::fetch` must be implemented inside a trusted controller, over an authenticated channel. The Rust trait is not a trust sandbox: a caller that injects a dishonest implementation can fabricate observations. This crate ships no production provider, accepts no workspace `actorVerified` boolean, and signs no approval. Test `FixtureProvider` types are explicit test doubles.

`observe` verifies exact reference, issuer, role, action, repository, target digest, protected policy digest and baseline revision; required requirement IDs must be a subset of the record scope. It rejects empty scope/identity, future issue time, expiry at or before consumption, and revocation. `None` means a completed query found no approval. `Unavailable` is distinct and propagates as an error. Observations are not deserializable and refresh requires another provider query; no persistent cache exists. The controller supplies a trusted clock and exact target binding digest.

`assess` returns scoped repair eligibility only after a fresh port query for `write-tests`. Delivery and skip always retain their separate gate requirements. Neither this result nor an observed approval is an execution grant. The stage transition table returns unmet requirements, never converts an unverified declaration to accepted eligibility.

Production acceptance requires GE-TRUST authentication, a reviewed provider/identity mapping, real records, and consumer-time checks. No claim of FG-GATE or hostile-agent enforcement is made.
