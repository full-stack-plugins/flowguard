# Prepared SpecGuard baseline fixture import

This opt-in adapter consumes actual version-pinned SG parser/export output with
explicit fixture approval. Protected controller inputs are independently chosen:
raw export digest, historical baseline/source/candidate/scope, current validated
FG binding, a bijection from historical requirement identities to the current
required requirement set, and the required stage/rules/analyzer descriptor.
PreparedFixtureImport privately copies these after borrowed admission. It cannot
be restored from uploaded JSON. A matching raw hash proves byte consistency,
not producer identity or approved production baseline authority.

Historical export candidate and current workflow candidate are separate values.
The actual fixture was exported at historical8df2938; tests bind a different real
current Git candidate. No equality is inferred between their source snapshots or
baseline digests. Mapping must cover both sets exactly without duplicate targets.
Any current binding change rejects use of the prepared/imported value. The caller
cannot shrink requirements by changing input collections after preparation.

Consumption checks size/current binding/raw digest before schema interpretation.
It then reuses the native versioned SG schema, verifies source completion, exact
historical pins/scope, stable requirement/acceptance-derived obligation IDs, source
references and full requirement coverage. It imports references/digests, not
specification text. Legacy import_fixture remains an explicitly untrusted fixture
convenience; it does not gain protected provenance or production eligibility.
Production authenticationProfile is rejected by both paths.

The import-context digest includes all historical pins, raw digest, current domain
binding digest, complete requirement mapping and stage/rules/analyzer descriptor.
The returned EvidenceObligation only ADDS a required SpecGuard obligation. Each
coverage token commits this import context and one stable native obligation ID.
It returns no specialist report, ALLOW, approval grant or completed observation.
This prevents baseline/mapping changes from yielding interchangeable gate work.

## Deliberately incomplete success bridge

These are FlowGuard combination-obligation tokens, NOT scopes already emitted by
the current SG structural producer. Current SG structural completion must not be
claimed to satisfy them automatically. Existing FrozenObligations has no spare
protected import-context field (its context_digest must equal current binding),
and changing actual SG contract hashes or relabeling historical source bytes would
be unsound. This slice therefore implements actual baseline import, frozen required
references, and missing-evidence rejection. A usable successful SG envelope bridge
needs a separately versioned combination adapter or protected context extension;
that integration is NOT implemented here. GE wire schemas are unchanged.

Integration tests freeze the imported SG obligation alongside another guard. That
other guard's actual GE-evaluated fixture report is ALLOW and is observed by FG,
but missing SG evidence still leaves its full required scope missing and FG BLOCK.
This is not a claimed production or successful SG-provider integration. If task2.6
acceptance requires that success bridge, this slice is partial and the task must
remain unchecked pending that work and independent review.

## Resource and trust boundaries

Export bytes cap1MiB. Protected scope/mapping entries cap256; namespace/id/target
strings256 bytes, expected metadata64KiB, combined mapping metadata128KiB;
stage/analyzer128 bytes. Checks precede controller-input clones and serialization.
Current binding uses existing borrowed FG admission. Parsed sources cap4096,
obligations8192 and source×obligation lookups1,048,576; source paths are safe relative
UTF-8 paths capped4096 bytes; trace links cap256 per obligation. Prepared output
caps1024 obligations before coverage expansion, matching FrozenObligations limits.
Stable IDs are validated before coverage construction, making token expansion
bounded. These limits do not authenticate a local controller or sandbox a runner.

The actual corpus is fixtures/providers/specguard-actual; the prior single-obligation
specguard-fixture-only.json is synthetic and unchanged. Full original source,
producer generator and baseline/export bytes are retained for test provenance;
product imports do not copy specification prose into workflow obligations.
