# FlowGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**FlowGuard is the proposed lifecycle and stage-governance member of the six independent Guards built on GuardEngine.** It determines stage obligations, dependency order, approval requirements, evidence freshness, and eligibility for a specific next action. It does not replace specialist analysis or grant merge/release authority.

## Current status and evidence

The original baseline `a6bd25f` was documentation-only. This branch now has a Rust library, read-only CLI, actual GuardEngine/GitGuard integration, explicit fixture authority, and local durable attempt storage. **11/30 tasks** have independent local-profile acceptance; CLI5.1 has independent acceptance. See [implementation progress](docs/implementation-progress.md). No production provider, host enforcement or merge/release execution is provided.

The external plugin was inspected at fixed SHA `13b52b054c31f614dc272b18c195b8fa929aa595`; see the [compatibility ADR](docs/adr/legacy-compatibility.md). This is source-backed investigation, not full legacy parity or migration acceptance.

Build: `cargo build --locked --bin flowguard` in the coordinated checkout with sibling guardengine/gitguard/specguard crates. Available commands: `discover`, `stage status`, `evidence verify`, `gate check`. See the [exact CLI contract, inputs and fixture limits](docs/adr/cli-contract.md). Gate defaults to unavailable authority; synthetic authority requires explicit fixture opt-in. No `--report` output flag exists.

## Ten-stage source layout

| Stage | Scope | Document under the consuming project's `docs/` |
|---|---|---|
| 01 Requirements analysis | Feature | `features/<feature>/01-requirements.md` |
| 02 Architecture design | Project | `project/02-architecture.md` |
| 03 Technical solution | Feature | `features/<feature>/03-solution.md` |
| 04 Test cases | Feature | `features/<feature>/04-testcases.md` |
| 05 High-level design | Feature | `features/<feature>/05-hld.md` |
| 06 Low-level design | Feature | `features/<feature>/06-lld.md` |
| 07 Coding standards | Project | `project/07-standards.md` |
| 08 Code review | Feature | `features/<feature>/08-review.md` |
| 09 Documentation delivery | Feature | `features/<feature>/09-docs.md` |
| 10 Release delivery | Project | `project/10-release.md` |

Source discovery implements this fixed layout; full legacy behavior compatibility remains unverified. Do not create a second `.flowguard/` project tree or duplicate native OpenSpec/Spec Kit task records. Superpowers is an execution methodology, not an approval authority. Project stage inheritance must bind immutable baseline references and authenticated approval records; a Markdown `accepted` value is insufficient.

## Target workflow and boundaries

Native specification + task/worktree + protected baseline → frozen stage obligations → SpecGuard / ArchGuard / CodeGuard / TestGuard / GitGuard evidence → verification and trusted approval lookup → FlowGuard decision → trusted executor.

- FlowGuard owns stage meaning, applicability, dependencies, and invalidation; specialist Guards own domain parsers and checks. GuardEngine owns generic contract validation, neutral rules, and deterministic evidence computation. There is no GuardCore.
- Inputs bind repository, requirements, task, worktree, exact candidate/base, optional merge group, policy/analyzer revisions, and coverage. Outputs describe a scoped decision, gaps, and audit references in a proposed integration envelope.
- Parallel requirements remain isolated. Candidate/base/merge-group, ruleset, analyzer/coverage, baseline, or approval validity changes invalidate affected results; late results cannot overwrite a newer candidate.
- The trusted merge-queue controller recomputes obligations and evidence for the exact integration candidate. GitGuard checks Git-domain conditions; the hosting platform executes the protected merge. FlowGuard does not merge or release.
- `ALLOW` is a scoped technical decision. Missing approval can yield `REQUIRE_APPROVAL` only when required analysis is complete; incomplete evidence and tool failures cannot be waived by approval. Reads, clarification, and authorized remediation remain available.

FlowGuard’s local gate check produces its own engine-backed report over stage-obligation facts. Its envelope decision and exit code reflect that report, not a rewritten specialist verdict. A specialist `REQUIRE_APPROVAL` remains unchanged after external approval; a new FlowGuard evaluation may find that the scoped review obligation is satisfied. Action eligibility is recorded separately in a domain artifact, and the trusted controller still authorizes execution. The local projection uses real engine fixtures; production protected policy/provider parity remains separate.

Current GuardEngine protocol `guard.partme.ai/v1alpha1` is distinct from the proposed [integration envelope](docs/integration-contract.md). It supports strict GuardContract/GuardFacts/GuardReport and exact `forbid_relation`, not FlowGuard stage or approval objects. Reports are unsigned; recomputation establishes consistency, not provenance or authority.

## Implemented read-only CLI

```sh
cargo build --locked --bin flowguard
cargo test --locked
target/debug/flowguard discover --root /absolute/project --feature example
target/debug/flowguard stage status --root /absolute/project --feature example
target/debug/flowguard evidence verify --root /absolute/evidence --envelope envelope.json --contract contract.json --facts facts.json --report-file report.json
target/debug/flowguard gate check --repo /absolute/git-worktree --input-root /absolute/controller-input --request request.json --request-digest sha256:EXACT_REQUEST_DIGEST --run-id unique-attempt-id
```

Use coordinated sibling GuardEngine/GitGuard/SpecGuard sources. Replace the example paths and digest with real bound inputs; see the [CLI contract and request format](docs/adr/cli-contract.md). Gate outputs its own ALLOW/BLOCK/REQUIRE_APPROVAL as exits `0`/`2`/`3`; errors and requested cancellation use `4`, with empty stdout before binding and null decision after binding. Other commands have their own documented exit semantics. `--report` is rejected; `--report-file` is an evidence input only. No task-ID inference or execution command is implemented. CodeGuard native exits remain command-specific and unchanged: native aggregate `3` must never be interpreted as FlowGuard REQUIRE_APPROVAL.

## Design and implementation path

Read the [architecture](docs/architecture.md), [technical design and acceptance plan](docs/technical-design.md), and [draft integration contract](docs/integration-contract.md). Start with a pinned legacy-source inventory and differential fixtures, then implement stage resolution, approval/evidence validation, concurrency control, and protected-host verification. Each phase requires observable negative cases before authority can transfer. No external side effects are enabled by this documentation.


## OpenSpec implementation backlog

The incremental [proposal](openspec/changes/add-evidence-bound-workflow-gates/proposal.md), [design](openspec/changes/add-evidence-bound-workflow-gates/design.md), [requirements](openspec/changes/add-evidence-bound-workflow-gates/specs/) and [tasks](openspec/changes/add-evidence-bound-workflow-gates/tasks.md) translate the architecture into pending implementation work. See the [cross-repository dependency roadmap](openspec/guard-roadmap.md) and [structural validation record](openspec/validation-2026-10-09.md). Reviewed local task acceptances are recorded in the task checklist; remaining work stays unchecked. Earlier source-tree inventories and validation limitations describe the inspected baseline or earlier architecture-review stage; this planning stage adds OpenSpec artifacts and separately records actual CLI validation. Existing change ownership and historical completion evidence remain intact.
