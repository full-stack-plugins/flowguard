# FlowGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**FlowGuard governs engineering lifecycle, human approvals, dependency ordering, stage evidence and controlled transitions for AI-native development.**

> **Status: buildable agent-integration CLI and read-only MCP are available in source.** Professional workflow behavior remains in flowguard-plugin; independent stage gates are not implemented.

Implemented stage artifact prechecks: `stage status`, `stage check`, and MCP `flowguard_stage_check` explicitly adapt the trusted native plugin. Ready artifacts return `requires_approval`, never authorization. See [plugin usage and trust boundary](plugin/README.md).

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

## Agent installation and managed instructions

The shared installer provides `install`, `uninstall`, `context`, `instructions render|sync`, `doctor --integration`, and `serve --mcp` for Codex, Claude Code, and Gemini CLI. Each tool renders its own managed block.

```sh
cargo build --locked
flowguard install --agents codex,claude,gemini --project /absolute/project --bin-dir /absolute/bin --dry-run
flowguard context --project /absolute/project
```

After building, invoke the executable under `target/debug/` (with `.exe` on Windows); short commands above assume PATH is configured. `--bin-dir` copies the current executable; PATH is not changed automatically. Remove `--dry-run` to apply. A sibling `../guardengine/crates/guard-install` source checkout is required.

[Shared design, host mapping, update/uninstall and verification](https://github.com/full-stack-plugins/guardengine/blob/main/docs/agent-installation.md). Configuration success does not prove host loading or quality approval.

## 受信执行证据 / Signed execution evidence

新增只读 evidence check 与 flowguard_evidence_check MCP：显式项目外受信公钥、任务/计划/用例/运行器/有效期策略，重算源码及报告指纹；技术通过仍 requires_approval。完整命令、策略和信任边界参见 [plugin/README.md](plugin/README.md#signed-execution-evidence)。不执行测试、不推进原生阶段。

