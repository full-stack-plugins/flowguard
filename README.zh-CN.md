# FlowGuard — 流程守卫

已实现 `stage status`、`stage check` 与 MCP `flowguard_stage_check`，只读适配显式信任的原插件。产物齐备仅返回 `requires_approval`，不授予审批或 Git 写入权限。见[插件使用与可信边界](plugin/README.md)。

[English](README.md) · [简体中文](README.zh-CN.md)

FlowGuard 是 Partme Guard 的**研发流程、审批、阶段状态和交付门禁**组件。它使 AI 能按工程规范自主推进，但不能自行修改流程条件后给自己放行。

> **当前状态：源码已提供可构建的智能体接入 CLI 和只读 MCP。** 专业流程仍在既有 flowguard-plugin，独立阶段门禁尚未实现。

## 正式十阶段

| 阶段 | 名称 | 文档归属 |
|---|---|---|
| 01 | 需求分析 | 功能 |
| 02 | 架构设计 | 项目 |
| 03 | 技术方案 | 功能 |
| 04 | 测试用例 | 功能 |
| 05 | 概要设计 | 功能 |
| 06 | 详细设计 | 功能 |
| 07 | 编码规范 | 项目 |
| 08 | 代码审查 | 功能 |
| 09 | 文档交付 | 功能 |
| 10 | 发布交付 | 项目 |

文档继续使用项目原有的 ¤docs/project¤、¤docs/features/<feature>¤ 路径，不重建旧 ¤.flowguard/¤ 目录。用户/Agent 的任务正文仍采用 OpenSpec、Spec Kit、Superpowers 等已有事实源。

## 技术定位

FlowGuard 负责阶段图、继承与依赖、人工批准、证据有效期和下一动作资格；它**不负责代替 SpecGuard/ArchGuard/CodeGuard/TestGuard/GitGuard 重新检查代码或签发技术 PASS**。GuardEngine 提供通用契约、规则和证据基础能力；真正受保护的 Git 合并由 GitGuard/可信 CI 完成。

~~~text
项目十阶段 + 任务上下文 → 冻结所需证据
                               │
                    各专业守卫独立检查
                               │
                       GuardEngine 验证
                               │
                     FlowGuard 阶段决策
                               │
                       可信执行与审计
~~~

## 完整设计文档

[总体架构](docs/architecture.md) · [技术方案](docs/technical-design.md)

## 规划命令（尚不可执行）

~~~sh
flowguard discover --project .
flowguard stage status --feature example
flowguard gate check --task TASK-104 --action git-commit
~~~

开发时优先验证现有 Python 十阶段实现与未来 Rust 实现的行为一致性；再接入可信审批来源、跨守卫证据和服务端强制门禁。没有外部批准和有效技术证据的候选不能通过流程，修复、澄清和测试等合法解除阻断动作必须保持可用。

## 智能体安装与提示注入

已接入共享安装器：`install`、`uninstall`、`context`、`instructions render|sync`、`doctor --integration`、`serve --mcp`。支持 Codex、Claude Code 和 Gemini CLI；按本工具职责生成独立标记区块。

```sh
cargo build --locked
flowguard install --agents codex,claude,gemini --project /absolute/project --bin-dir /absolute/bin --dry-run
flowguard context --project /absolute/project
```

构建后请先通过 `target/debug/工具名`（Windows 加 `.exe`）调用；上面的短命令假定程序已加入 PATH。指定 `--bin-dir` 会复制当前二进制，安装器不自动修改 PATH；去掉 `--dry-run` 才实际安装。需要同级 `../guardengine/crates/guard-install` 源码。

[统一设计、宿主文件映射、更新、卸载和验证说明](https://github.com/full-stack-plugins/guardengine/blob/main/docs/agent-installation.md)。安装成功不代表宿主已加载或专业检查通过。

## 受信执行证据 / Signed execution evidence

新增只读 evidence check 与 flowguard_evidence_check MCP：显式项目外受信公钥、任务/计划/用例/运行器/有效期策略，重算源码及报告指纹；技术通过仍 requires_approval。完整命令、策略和信任边界参见 [plugin/README.md](plugin/README.md#signed-execution-evidence)。不执行测试、不推进原生阶段。

