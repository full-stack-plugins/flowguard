# FlowGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**FlowGuard 是建立在 GuardEngine 上的六个独立守卫之一的流程与阶段治理设计。** 它判断阶段义务、依赖顺序、审批条件、证据时效和指定下一动作的资格，不替代专业分析，也不授予合并或发布权。

## 当前状态与证据

本次检查基线：`a6bd25fc38b0a323f880bedd395373d3bb672826`（2026-10-09）。受版本控制的文件只有本 README、英文版、`docs/architecture.md` 和 `docs/technical-design.md`。**没有可执行程序、源码目录、包清单、测试集、CI 配置或 OpenSpec 目录。** 下文的运行行为与命令均为提案；未宣称执行了运行时测试或 OpenSpec 校验。

外部 [flowguard-plugin](https://github.com/full-stack-plugins/flowguard-plugin) 是**尚未验证的兼容目标**。本次未检查其源码、发布版本、Python `flowguard_lib`、Hooks 或[既有文档引用的十阶段规格](https://github.com/full-stack-plugins/flowguard-plugin/blob/main/docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md)。旧设计文档提及这些组件，不等于证明其存在或行为正确；迁移前必须锁定并检查真实修订。

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
