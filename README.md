# FlowGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**FlowGuard is the proposed lifecycle and stage-governance member of the six independent Guards built on GuardEngine.** It determines stage obligations, dependency order, approval requirements, evidence freshness, and eligibility for a specific next action. It does not replace specialist analysis or grant merge/release authority.

## Current status and evidence

Inspected baseline: `a6bd25fc38b0a323f880bedd395373d3bb672826` (2026-10-09). The tracked tree contains only this README, its Chinese counterpart, `docs/architecture.md`, and `docs/technical-design.md`. There is **no executable, source tree, manifest, test suite, CI configuration, or OpenSpec tree**. All runtime behavior and commands below are proposals; no runtime tests or OpenSpec validation are claimed.

The external [flowguard-plugin](https://github.com/full-stack-plugins/flowguard-plugin) is an **unverified compatibility target**. Its source, releases, Python `flowguard_lib`, hooks, and [referenced ten-stage specification](https://github.com/full-stack-plugins/flowguard-plugin/blob/main/docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md) were not inspected in this review. Earlier repository documents described these components; that is not evidence that they exist or behave as described. Migration must pin and inspect a real revision before making parity claims.

## Proposed ten-stage compatibility model

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

These retained design assumptions need external compatibility verification. Do not create a second `.flowguard/` project tree or duplicate native OpenSpec/Spec Kit task records. Superpowers is an execution methodology, not an approval authority. Project stage inheritance must bind immutable baseline references and authenticated approval records; a Markdown `accepted` value is insufficient.

## Target workflow and boundaries

Native specification + task/worktree + protected baseline → frozen stage obligations → SpecGuard / ArchGuard / CodeGuard / TestGuard / GitGuard evidence → verification and trusted approval lookup → FlowGuard decision → trusted executor.

- FlowGuard owns stage meaning, applicability, dependencies, and invalidation; specialist Guards own domain parsers and checks. GuardEngine owns generic contract validation, neutral rules, and deterministic evidence computation. There is no GuardCore.
- Inputs bind repository, requirements, task, worktree, exact candidate/base, optional merge group, policy/analyzer revisions, and coverage. Outputs describe a scoped decision, gaps, and audit references in a proposed integration envelope.
- Parallel requirements remain isolated. Candidate/base/merge-group, ruleset, analyzer/coverage, baseline, or approval validity changes invalidate affected results; late results cannot overwrite a newer candidate.
- The trusted merge-queue controller recomputes obligations and evidence for the exact integration candidate. GitGuard checks Git-domain conditions; the hosting platform executes the protected merge. FlowGuard does not merge or release.
- `ALLOW` is a scoped technical decision. Missing approval can yield `REQUIRE_APPROVAL` only when required analysis is complete; incomplete evidence and tool failures cannot be waived by approval. Reads, clarification, and authorized remediation remain available.

Current GuardEngine protocol `guard.partme.ai/v1alpha1` is distinct from the proposed [integration envelope](docs/integration-contract.md). It supports strict GuardContract/GuardFacts/GuardReport and exact `forbid_relation`, not FlowGuard stage or approval objects. Reports are unsigned; recomputation establishes consistency, not provenance or authority.

## Planned CLI — not runnable

```sh
flowguard discover --project .
flowguard stage status --feature example
flowguard gate check --task TASK-104 --action git-commit
flowguard gate check --task TASK-104 --action release
```

No install/build/test command exists here. The proposed check mapping is exit `0` ALLOW, `2` BLOCK, `3` REQUIRE_APPROVAL, `4` invalid input/runtime/verification error, with JSON on stdout and diagnostics on stderr. FlowGuard has no implemented `--report` behavior. Existing CodeGuard legacy exit codes must remain unchanged; an explicit adapter may normalize them.

## Design and implementation path

Read the [architecture](docs/architecture.md), [technical design and acceptance plan](docs/technical-design.md), and [draft integration contract](docs/integration-contract.md). Start with a pinned legacy-source inventory and differential fixtures, then implement stage resolution, approval/evidence validation, concurrency control, and protected-host verification. Each phase requires observable negative cases before authority can transfer. No external side effects are enabled by this documentation.
