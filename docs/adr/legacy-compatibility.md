# ADR: fixed-revision legacy survey; migration unsupported

Inspected https://github.com/full-stack-plugins/flowguard-plugin at **13b52b054c31f614dc272b18c195b8fa929aa595**, manifest version **0.4.2**, on 2026-10-09. The read-only cloud clone remains in `/workspace/guard-implementation-ledger/flowguard-legacy`; no changes, notifications or pushes were made. Source license is Apache-2.0 (`LICENSE`); no source code is copied into this crate.

All following citations resolve within that fixed commit, not mutable main:

- [`plugin.json`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/plugin.json) declares name/version/license.
- [`scripts/flowguard_lib/registry.py`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/scripts/flowguard_lib/registry.py) `ARTIFACTS` contains the ten names and project ownership of 02/07/10. It defines dependency edges and FIFO ready-node traversal.
- [`scripts/flowguard_lib/stage_docs.py`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/scripts/flowguard_lib/stage_docs.py) `path_for` uses `docs/project/` and `docs/features/<task>/`; `_metadata` and `_rows` use Chinese Markdown table fields. `LEGAL` allows direct pending/in_progress/invalidated→accepted. `_content_hash` omits selected metadata and evidence sections.
- [`scripts/flowguard_state.py`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/scripts/flowguard_state.py) exposes `init`, `discover`, `context`, `evidence`, `governance`, `stage`, `validate`. Its help was actually executed successfully with Python 3.12. Not all commands are read-only: init/advance/approve/record are not imported or executed by the Rust implementation.
- [`hooks/hooks.json`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/hooks/hooks.json) and [`hooks/__protocol__.md`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/hooks/__protocol__.md) define Python hooks: UserPromptSubmit, PreToolUse, PostToolUse and summaries. Source protocol documents gate exit 0/2; artifact check exit 0. These are source observations, not real-host enforcement evidence.

Executed `test_registry.py`: 4 passed. Executed `test_stage_transitions.py`: 8 passed after explicitly setting PYTHONPATH and FLOWGUARD_STATE_HOME to isolated cloud scratch. Initial import failed with missing flowguard_lib; initial corrected-import run failed because the default home state directory was read-only. Both failures are recorded, not hidden. Full legacy suite and real host were not run. No legacy/new differential acceptance was performed.

Compatibility decisions: ten path names are source-confirmed and reused as a versioned layout. New graph uses lexicographic ready order; new state transition requirements prohibit invalidated→accepted without resubmission; discovery hashes every source byte, unlike legacy metadata exclusions. These are known differences, not parity. Legacy status-table parsing, mutation commands, hook payload compatibility, identity authentication, and automatic migration remain UNSUPPORTED. No rule-owner transfer is authorized by this survey.

## Task 5.2: fixed-revision declaration differential

`adapters/legacy/mod.rs` now observes the surveyed two-column Chinese stage/status rows through the bounded `AllowedRoot`, with an explicit exact legacy revision argument and one of the ten surveyed paths. It returns `declared` state and the complete raw-byte digest. This is not legacy `read()`'s effective state, authenticated task ownership, approval, inheritance validation, migration, or gate eligibility. `migration_supported()` is always false and no mutator or migration command is exposed. The caller's immutable-checkout condition still applies.

`tests/support/capture_legacy.py <existing-pinned-checkout>` refuses another commit or tracked modifications, performs 15 actual old-library reads in disposable directories, and records the old 49-pair LEGAL table in `fixtures/legacy/differential.json`. It neither fetches nor changes legacy source. `tests/legacy_differential.rs` consumes those frozen observations without requiring Python or an external checkout for normal Rust tests. Sources are byte-pinned in the fixture. Execution is local differential evidence, not a claim of real-host compatibility.

| Rule | Sole authority in this profile | Known difference / disposition |
|---|---|---|
| Legacy effective status and excluded-content fingerprint | Fixed legacy `stage_docs.py` | New adapter only reports the literal declaration. A legacy accepted declaration can be effectively invalidated; unsupported migration. |
| Legacy approval, inheritance and skip metadata | Legacy native implementation | Old accepted/inherited/skipped text cannot authorize new evidence-qualified transitions. |
| New transition requirements | Existing FlowGuard `stage_transition::transition` | Exact baseline, protected skip and technical evidence require their existing approval paths; no legacy override. |
| Source byte identity | FlowGuard bounded reader/digest | Full bytes hashed; no legacy metadata exclusion. |
| Duplicate or fenced example metadata | FlowGuard declaration parser | Duplicate recognized fields reject; fenced examples ignored. Legacy regex accepts fenced rows and keeps last duplicate. |
| Missing, directory, invalid UTF-8, unknown status/stage | Bounded read/parser | New adapter rejects; legacy missing/directory reads yield pending, invalid UTF-8 throws. No assumed pending authority. |

Fingerprint recomputation, upstream/project release resolution, real-host hooks, changing ownership, CLI migration and broad old/new behavioral parity remain unsupported. Documented differences are not silently adjudicated into compatibility; every observation remains ineligible for migration. No user documents are changed.
