# Partme FlowGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**FlowGuard governs engineering lifecycle, human approvals, dependency ordering, stage evidence and controlled transitions for AI-native development.**

> **Status:** detailed architecture and technical blueprint prepared. This new independent FlowGuard repository has **no released executable runtime yet**. The pre-existing [flowguard-plugin](https://github.com/full-stack-plugins/flowguard-plugin) currently owns the ten-stage semantics and host integrations; it remains authoritative until verified migration.

## Ten established stages

1. Requirements analysis (feature-level).
2. Architecture design (project-level).
3. Technical solution (feature-level).
4. Test cases (feature-level).
5. High-level design (feature-level).
6. Low-level design (feature-level).
7. Coding standards (project-level).
8. Code review (feature-level).
9. Documentation delivery (feature-level).
10. Release delivery (project-level).

All stage artifacts live under the project's ¤docs/¤ tree. **Do not reintroduce the legacy ¤.flowguard/¤ project directory.** OpenSpec/Spec Kit remain specification sources; Superpowers is an execution methodology.

## Ownership and enforcement

~~~text
Native spec + docs/ stage graph + session/worktree/task binding
                         ↓
             Frozen stage obligations
                         ↓
  SpecGuard / ArchGuard / CodeGuard / TestGuard / GitGuard
                         ↓
         GuardEngine evidence and rules
                         ↓
    Verified approval identity + stage policy
                         ↓
           FlowGuard stage decision
                         ↓
        trusted executor / protected CI
~~~

FlowGuard **does not** rerun other Guards' domain rules or create its own accepted status from a model statement. Missing technical evidence and a missing trusted approval are different blockers. Local hooks can provide feedback; actual merge/release enforcement requires a trusted controller and hosting-platform protections.

## Documents

- [Detailed architecture and ten-stage boundary](docs/architecture.md)
- [Technical design, migration and validation plan](docs/technical-design.md)
- [Original FlowGuard ten-stage specification](https://github.com/full-stack-plugins/flowguard-plugin/blob/main/docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md)
- [GuardEngine](https://github.com/full-stack-plugins/guardengine)

## Planned CLI (not yet executable)

~~~sh
flowguard discover --project .
flowguard stage status --feature example
flowguard gate check --task TASK-104 --action git-commit
flowguard gate check --task TASK-104 --action release
~~~

Implementation begins by matching the existing Python flowguard_lib behaviors using real ten-stage documents and negative tests. Rule ownership will transfer only after differential testing and real host verification. Planning documents do not imply execution or trusted gating.
