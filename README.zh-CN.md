# FlowGuard — 流程守卫

[English](README.md) · [简体中文](README.zh-CN.md)

FlowGuard 是 Partme Guard 的**研发流程、审批、阶段状态和交付门禁**组件。它使 AI 能按工程规范自主推进，但不能自行修改流程条件后给自己放行。

> **当前状态：已制定独立 FlowGuard 的详细架构与技术方案，尚未实现新的产品运行时。** 现有 [flowguard-plugin](https://github.com/full-stack-plugins/flowguard-plugin) 的十阶段语义、文档位置和真实兼容入口继续保留，不能直接宣称已迁移。

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
