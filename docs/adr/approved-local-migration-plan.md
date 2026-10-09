# Approved local migration implementation — 2026-10-09

User decision “FlowGuard 按建议” supersedes the unapproved recommendations in the earlier ledger. Keep declared document state, fixed-revision legacy effective state and new eligibility distinct. Full-byte identity, strict duplicate/fenced-input handling and deterministic graph ordering remain new-core defaults.

1. Add bounded fixed-revision effective-state observation and reachable CLI display; validate body, excluded metadata/evidence and upstream invalidation.
2. Route local submission/invalidation and qualification through existing typed stage APIs. Preserve immutable imported history; approval references never grant qualification. Exercise advisory/shadow, opt-in and rollback without resetting history or revocation.
3. Validate explicit feature/requirement/immutable09 release mappings against current closure; missing or ambiguous mappings refuse. No real legacy release sample exists, so this is a local contract, not real migration evidence.
4. Run RED/GREEN for new behavior, full Rust suite, record logs and local commits. No remote writes, production controller authentication or actual-host enforcement claims.
