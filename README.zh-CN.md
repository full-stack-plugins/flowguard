# FlowGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**FlowGuard 是建立在 GuardEngine 上的六个独立守卫之一的流程与阶段治理设计。** 它判断阶段义务、依赖顺序、审批条件、证据时效和指定下一动作的资格，不替代专业分析，也不授予合并或发布权。

## 当前状态与证据

原始基线 `a6bd25f` 只有设计文档；当前分支已有 Rust 库、四个只读 CLI 命令、真实 GE/GG 接口、显式 fixture 权限端口和本地持久运行存储。**11/30 项**已有独立本地 profile 验收，CLI5.1 已独立验收。详见 [实施进展](docs/implementation-progress.md)。生产权限提供方、宿主强制保护与 merge/release 执行尚未提供。

外部插件已按 SHA `13b52b054c31f614dc272b18c195b8fa929aa595` 调查，见 [兼容性 ADR](docs/adr/legacy-compatibility.md)；不等于完整行为一致或迁移验收。运行 `cargo build --locked --bin flowguard` 构建，依赖同级 GE/GG/SG 源码。命令为 `discover`、`stage status`、`evidence verify`、`gate check`。参数与 fixture 边界见 [CLI 合同](docs/adr/cli-contract.md)，默认权限不可用，未实现 `--report` 输出参数。

## 拟兼容的十阶段模型

| 阶段 | 归属 | 使用方项目 `docs/` 下的文档 |
|---|---|---|
| 01 需求分析 | 功能 | `features/<feature>/01-requirements.md` |
| 02 架构设计 | 项目 | `project/02-architecture.md` |
| 03 技术方案 | 功能 | `features/<feature>/03-solution.md` |
| 04 测试用例 | 功能 | `features/<feature>/04-testcases.md` |
| 05 概要设计 | 功能 | `features/<feature>/05-hld.md` |
| 06 详细设计 | 功能 | `features/<feature>/06-lld.md` |
| 07 编码规范 | 项目 | `project/07-standards.md` |
| 08 代码审查 | 功能 | `features/<feature>/08-review.md` |
| 09 文档交付 | 功能 | `features/<feature>/09-docs.md` |
| 10 发布交付 | 项目 | `project/10-release.md` |

这些保留的设计假设仍需外部兼容验证。不重建 `.flowguard/` 项目目录，不复制 OpenSpec/Spec Kit 的原生任务账；Superpowers 是执行方法，不是审批权威。项目阶段继承必须绑定不可变基线引用和经过认证的批准记录，Markdown 中的 `accepted` 不足以放行。

## 目标流程与边界

原生规格 + 任务/工作树 + 受保护基线 → 冻结阶段义务 → SpecGuard / ArchGuard / CodeGuard / TestGuard / GitGuard 证据 → 验证与可信审批查询 → FlowGuard 决策 → 可信执行端。

- FlowGuard 拥有阶段语义、适用范围、依赖和失效规则；专业守卫拥有领域解析与检查。GuardEngine 提供通用契约校验、中立规则和确定性证据计算，不存在 GuardCore。
- 输入绑定仓库、需求、任务、工作树、精确候选/基底、可选合并组、策略/分析器版本和覆盖范围；输出在拟议集成信封中描述局部决策、缺口和审计引用。
- 并行需求隔离；候选/基底/合并组、规则集、分析器/覆盖范围、基线或审批有效性变化，会使受影响结果失效；迟到结果不能覆盖新候选结果。
- 可信合并队列控制器针对精确集成候选重新计算义务与证据；GitGuard 检查 Git 领域条件，托管平台执行受保护合并。FlowGuard 不执行合并或发布。
- `ALLOW` 仅是指定范围的技术决策。仅在必要分析完整时，缺少审批才可产生 `REQUIRE_APPROVAL`；审批不能豁免不完整分析或工具故障。读取、澄清与已授权修复保持可用。

拟议 FlowGuard gate check 对阶段义务事实生成自己的引擎报告；信封 decision 与退出码反映该报告，不改写专业守卫结论。外部批准后，专业报告的 `REQUIRE_APPROVAL` 保持原样；新的 FlowGuard 评估可以确认指定范围的待审义务已满足。动作资格单独记录在领域附件，执行仍由可信控制器授权。事实投影和受保护门禁规则须先实现并通过等价性样例验证。

当前 GuardEngine 协议 `guard.partme.ai/v1alpha1` 与拟议[集成信封](docs/integration-contract.md) 分离：它支持严格的 GuardContract/GuardFacts/GuardReport 和精确 `forbid_relation`，不支持 FlowGuard 阶段或审批对象。报告未签名；重算验证一致性，不证明来源或授权。

## 规划 CLI——尚不可运行

```sh
flowguard discover --project .
flowguard stage status --feature example
flowguard gate check --task TASK-104 --action git-commit
flowguard gate check --task TASK-104 --action release
```

本仓没有安装、构建或测试命令。拟议 check 退出码为 `0` ALLOW、`2` BLOCK、`3` REQUIRE_APPROVAL、`4` 输入/运行/验证错误；stdout 输出 JSON，stderr 输出诊断。FlowGuard 尚无已实现的 `--report` 行为。CodeGuard 既有 CLI 退出码保持原样，只允许明确的适配层归一化。

## 设计与实施路线

参阅[架构](docs/architecture.md)、[技术方案与验收计划](docs/technical-design.md)、[集成契约草案](docs/integration-contract.md)。先完成锁定版本的旧实现清单和差分样例，再实施阶段解析、审批/证据校验、并发控制及受保护宿主验证。每阶段在转移权威前都必须完成可观察的负例验证。本文档不启用任何外部副作用。


## OpenSpec 实施待办

新增增量 [proposal](openspec/changes/add-evidence-bound-workflow-gates/proposal.md)、[design](openspec/changes/add-evidence-bound-workflow-gates/design.md)、[规范](openspec/changes/add-evidence-bound-workflow-gates/specs/) 与 [tasks](openspec/changes/add-evidence-bound-workflow-gates/tasks.md)，将架构方案拆成待实施工作。参阅[跨仓依赖路线图](openspec/guard-roadmap.md)与[结构验证记录](openspec/validation-2026-10-09.md)。所有新增实施任务保持未勾选；本分支新增规划，不新增产品功能。前文源码树清单和验证限制对应检查基线或较早的架构审阅阶段；本次另行新增 OpenSpec 文档并记录实际 CLI 校验。既有 change 的任务归属和历史完成证据继续保留。
