# FlowGuard — 研发流程守卫总体架构

> Partme Guard | 目标架构 V0.1 | 新独立产品待实施；既有 [flowguard-plugin](https://github.com/full-stack-plugins/flowguard-plugin) 目前承担已验证的流程语义与入口 | 2026-10-09

## 1. 产品定位与架构红线

FlowGuard 管理“哪些工程阶段必须完成、谁有权批准、哪些证据足够新鲜、什么时候允许进入下一阶段”，但**不能代替其他专业守卫声称技术正确**。它只消费 SpecGuard、ArchGuard、CodeGuard、TestGuard、GitGuard 产生的结构化证据；公共规则、契约和证据基础设施归 GuardEngine。FlowGuard 管理阶段与批准策略，不复制各 Guard 的检查算法。

Agent 负责提出状态推进申请；FlowGuard 验证状态机、前置阶段和真实批准来源。编码 Agent、仓库文档中的 approved 字段和模型自述都不能签发可信放行凭据。本地 Hook 只是提前反馈，服务端强制合入需要受保护 CI 与 GitGuard/可信执行端。

## 2. 已确认的十阶段流程（必须兼容原生 FlowGuard）

沿用原 flowguard-plugin [正式规格](https://github.com/full-stack-plugins/flowguard-plugin/blob/main/docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md) 的十阶段、文档归属与状态语义。**不是另外设计一套新阶段**：

| Stage | 中文 | 唯一文档事实源 |
|---|---|---|
| 01 | 需求分析 | `docs/features/<feature>/01-requirements.md` |
| 02 | 架构设计 | `docs/project/02-architecture.md` |
| 03 | 技术方案 | `docs/features/<feature>/03-solution.md` |
| 04 | 测试用例 | `docs/features/<feature>/04-testcases.md` |
| 05 | 概要设计 | `docs/features/<feature>/05-hld.md` |
| 06 | 详细设计 | `docs/features/<feature>/06-lld.md` |
| 07 | 编码规范 | `docs/project/07-standards.md` |
| 08 | 代码审查 | `docs/features/<feature>/08-review.md` |
| 09 | 文档交付 | `docs/features/<feature>/09-docs.md` |
| 10 | 发布交付 | `docs/project/10-release.md` |

项目级 02/07/10 由子功能继承；普通任务仍写在 OpenSpec/Spec Kit/Superpowers 原生 tasks，不在 FlowGuard 重建第二本任务账。**旧 `.flowguard/` 项目目录兼容线已于既有插件 v0.4.0 下线**，新产品不得将其重新引入。

已有阶段状态为 `pending`、`in_progress`、`pending_acceptance`、`accepted`、`inherited`、`skipped`、`invalidated`；继承/跳过必须附可验证来源或批准，文档和底层代码变更可使已接受阶段及下游阶段失效。

## 3. 逻辑组件与时序

~~~text
Task Request / Native Spec / Session & Worktree Context
                          │
                          ▼
            Stage Graph + Source Resolution
           docs/ project + features/ references
                          │
                          ▼
               Mandatory Evidence Plan
     Required Spec/Arch/Code/Test/Git Guard outputs
                          │
             ┌────────────┴─────────────┐
             ▼                          ▼
        Evidence Verifier        Approval Registry
      source/candidate/coverage    actor/scope/expiry
             └────────────┬─────────────┘
                          ▼
                   Transition Policy
                 GuardEngine evaluation
                          │
                   FlowGuard Decision
            ADVANCE / BLOCK / REQUIRE_APPROVAL
                          │
               Trusted Action Adapter
       GitGuard + branch protection / release owner
~~~

### 3.1 Stage Graph 与继承

ContextKey = repository + worktree + session + task/change + feature；所有证据必须绑定此上下文或经过明确且仍有效的共享批准。项目级阶段继承通过文档父引用，不能复制一份“临时 accepted”。阶段依赖图须无环，确认受影响上游改变时将下游标记 invalidated。

### 3.2 Approval Registry

ApprovalRef 需要可验证的身份、作用范围、具体目标（需求、候选代码、发布动作）、审核意见、时点、有效期限及撤销信息。已有合法范围批准可以复用，但代码候选修改后仍必须重新验证技术证据。文档中的 `user: approved` 或 `--actor user` 不属于可信批准。

### 3.3 Gate Orchestrator

“写代码之前”需要 01—07 相应阶段满足；允许为 TDD 预先写测试，不得因此绕过需求与设计确认。“提交之前”需要 08/09 以及对应候选的 TestGuard/CodeGuard/ArchGuard 审查与 Git 范围证据。“发布之前”需要 10、必要用户验收、所有列入的功能 09 和相关依赖处于有效状态。读取、澄清、补测试、修复阻断等安全操作不应被全面禁止。

## 4. 与 GuardEngine 与其他守卫的分工

GuardEngine 统一解析合同、执行通用规则、验证报告内容一致、调度验证器；FlowGuard 定义**哪些阶段需要哪些专业检查/批准/动作**。专业 Guard 独立产生结果，不能让 FlowGuard 重算架构/测试/代码判定。CodeReview 产生 REVIEW 风险，不提供签名权。用户确认事件必须从可信宿主/控制端取得，模型自写文本不能接受。

v1alpha1 GuardEngine 尚无原生 ExecutionGrant 或完整阶段图引擎，FlowGuard 的状态与批准模型是后续领域协议设计，不宣称已在 GuardEngine 中实现。

## 5. 迁移与系统 ADR

- FG-ADR-001：保留旧 flowguard-plugin 十阶段与 docs/ 事实源，迁移时不创建第二套审批状态机。
- FG-ADR-002：FlowGuard 只消费证据并控制状态，不自行替技术守卫作 PASS。
- FG-ADR-003：来源与版本一旦失效，批准和技术证据分别按各自条件失效。
- FG-ADR-004：审批身份不能由 Agent 自述，受保护合并与发布由可信执行端落实。
- FG-ADR-005：原有 Python flowguard_lib 为迁移前权威语义，Rust 实现通过合法/违规/未知差分才切换 owner。
- FG-ADR-006：安全修复路径保持可用；策略异常默认不放行高风险写操作。

## 6. 验收矩阵

| 场景 | 预期 |
|---|---|
| 十阶段继承、功能级依赖完整且批准有效 | 满足对应阶段资格 |
| 01—07 缺批准直接执行业务写入 | BLOCK 并提示待办 |
| 08/09 证据对应旧提交 | 旧证据失效，阻止提交 |
| 项目级 10 对应功能 09 内容变化 | 旧发布验收失效 |
| 模型自述用户已批准 | 拒绝可信批准映射 |
| 未部署 SpecGuard/TestGuard 却要求其义务 | missing provider 不能当 PASS |
| 修复代码/补测试所需读取 | 正常允许，不全局封禁 |
| GitHub 未启用 required check | 不宣称服务端 enforced |

具体状态机和技术方案见 [technical-design.md](technical-design.md)。
