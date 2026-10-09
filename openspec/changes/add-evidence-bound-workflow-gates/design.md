## Context

原始实现基线 `a6bd25fc38b0a323f880bedd395373d3bb672826` 为四文档仓库；规划前复查 `de60563f9d80f5ad0ef835191ae907c469c04e38` 的源码状态仍无 runtime/manifest/tests/OpenSpec。已审阅的[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[共享集成契约](../../../docs/integration-contract.md)是本计划输入，不是运行验证。外部 legacy 未检查，本设计的十阶段/docs 配置保留为调查前假设。

## Goals / Non-Goals

目标是只读、可解释、候选绑定的阶段技术门禁；独立保留专业报告与授权记录，给后续真实宿主提供可信控制器可核实的证据。非目标包括引擎新增领域类型、专业代码/测试分析、写 refs、发消息、发布、批准签发、复制任务账或自动迁移历史文档。

## Dependency DAG

按照[共享路线图](../../guard-roadmap.md)实施。GE-CONTRACT 冻结 envelope/能力/错误合同后，GE-ADAPTER 提供通用校验；GE-TRUST 提供认证证据引用与新鲜度端口，不把身份签发放入引擎。SG-BASELINE 提供稳定需求/批准基线和义务来源。GG-CANDIDATE 的只读候选绑定是前置输入。AG-EVIDENCE、CG-ADAPTER、TG-EVIDENCE 仅按冻结义务消费，缺任一必需项不得放行。

```text
本地调查/图 fixture ─────────────────────┐
SG-BASELINE + GG-CANDIDATE(read-only) ────┤
GE-CONTRACT → GE-ADAPTER → GE-TRUST ──────┤→ FG-GATE
必需 AG-EVIDENCE / CG-ADAPTER / TG-EVIDENCE┘      |
                                              v
                               外部控制器 / 可选 GitGuard 写端
```

图/legacy 调查可与所有上游并行；本地模拟不是跨仓验收。FG-GATE 不依赖 GitGuard privileged executor，避免 gate↔merge 循环。END-TO-END 等相关门槛具备后运行。开发可使用固定引擎源 SHA；生产独立分发等待 GE-RELEASE。

跨仓依据：[GE](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)、[SG](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis)、[AG](https://github.com/full-stack-plugins/archguard/tree/docs/guard-design-20261009/openspec/changes/extend-architecture-analysis-and-evidence)、[CG](https://github.com/full-stack-plugins/codeguard/tree/docs/guard-design-20261009/openspec/changes/add-guardengine-compatibility-adapter)、[TG](https://github.com/full-stack-plugins/testguard/tree/docs/guard-design-20261009/openspec/changes/add-test-obligation-evidence-pipeline)、[GG](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance)。

## Decisions

### 1. Proposed module and contract boundaries

所有以下文件均为未来路径；Rust 2024/Serde/Clap 是候选技术，锁定版本在首次实现决定；Tokio/SQLite 仅在测得异步/事务需求后引入。

| 拟议模块 | 输入 → 输出 | 核心约束 |
|---|---|---|
| `src/context.rs` | InvocationInput + Git candidate observation → ValidatedBinding 或 PreBindingDiagnostic | 不猜 repo/task/requirements，不伪造 OID |
| `src/stage.rs`、`src/dependencies.rs` | 固定 docs 源 +配置 → StageGraph/coverage | DAG、完整解析范围、稳定节点、原生引用 |
| `src/baseline.rs` | BaselineRef + graph → InheritanceSnapshot | 不可变 revision/digest、范围校验 |
| `src/approvals.rs` | 外部引用 + binding + action → VerifiedApprovalObservation 或错误 | 可信端认证，不能由布尔值自证 |
| `src/obligations.rs` | graph + protected policy → FrozenObligations | 先冻结，失败 provider 不消失 |
| `src/evidence.rs` | obligations + 专业 envelope/工件 → ValidatedEvidenceSet | 核对摘要/覆盖/精确候选/生产者，保留原报告 |
| `src/projection.rs`、`src/gate.rs` | 阶段观察 +证据+批准观察 → 自身 facts/report + GateDecision 附件 | 中立精确关系、自己报告的 decision、非执行授权 |
| `src/run_store.rs` | 完整绑定 +事件 + expected generation → immutable attempt/history | CAS、幂等、只失效资格、不改历史 |
| `src/cli.rs` | 明确命令/参数 → 文档化 JSON 或诊断 | check 0/2/3/4，其他命令逐个固定 |
| `adapters/{legacy,openspec,speckit,superpowers,host_hooks,mcp}/` | 声明版本的来源/宿主输入 → 领域对象 | 每个适配器单独能力/权限/失败矩阵，未实现不宣传 |

领域 schema 版本、引擎 `guard.partme.ai/v1alpha1`、集成 `guard.integration/v1alpha1`、crate semver、policy revision 不混用。不向现有 GuardContract/Facts/Report 添加阶段、审批、错误字段。信封按 GE-CONTRACT 冻结的精确字段实现，不自行扩展共享字段或保证 N/N-1。

### 2. Stage graph and baseline meaning

保留十阶段候选配置：功能 01/03/04/05/06/08/09 位于 `docs/features/<feature>/`，项目 02/07/10 位于 `docs/project/`，具体文件名以架构表为输入 fixture。项目 02/07 继承确切基线；10 固定发布功能集合和各 09 摘要。原生 OpenSpec/Spec Kit task 只引用，不复制，不创建 `.flowguard/`。

StageRecord 声明状态为 pending/in_progress/pending_acceptance/accepted/inherited/skipped/invalidated。声明不等于已核验状态；accepted 需要完整技术义务与必要批准，inherited 绑定父 digest/ref 和覆盖本需求的批准，skipped 需要受保护策略和明确范围批准。invalidated 重新提交到 pending_acceptance 并重算，不能手改回 accepted。授权修复动作与交付动作分别判定。

BaselineRef 包含 scope、不可变内容/修订、policy digest、批准引用；控制器验证身份、action、范围、issue/expiry/revocation 与目标绑定。批准服务不可用是验证失败，不是已确认“无批准”。Markdown accepted、模型声称或 actorVerified=true 不形成权威。

### 3. Independent gate evaluation and external authorization

FreezeObligations 在收集前固定全部必需阶段、专业 Guard、覆盖和批准。FlowGuard 不重算专业算法，只验证其工件与作用域。专业报告无论是 ALLOW/BLOCK/REQUIRE_APPROVAL 都原样保留；unsigned verify 只证明一致性，来源仍须可信控制器认证。

FlowGuard 领域投影对阶段缺口、证据缺失和待审义务形成版本化的精确 `forbid_relation` 事实。受保护规则不能来自候选自己改弱的拷贝；每个强制缺口须有匹配断言。领域 incomplete 产生合法 partial/BLOCK 或 error，不能用空 facts 隐去缺口。未经认证的批准、不完整分析、专业 enforce/BLOCK 均不能因“人批准”转为通过。

同一专业 REQUIRE_APPROVAL 在批准前后字节不变；批准后新 FlowGuard 运行可观察该待审义务已满足，产生自己的 ALLOW 报告。自己的 completed envelope decision 必须等于自己引用的 GuardReport.decision，下游报告通过领域附件关联。GateDecision 动作资格通过 `artifacts.domain`，控制器执行授权是独立记录。FlowGuard 不签 grant、不写 ref、不发布；退出 0 不证明下游全部 ALLOW，也不证明动作执行。

### 4. Errors, attempts and freshness

完整 binding/producer/frozen coverage 之前失败：独立 transport diagnostic，无 GuardRunEnvelope，不填空 repo/OID；CLI 4/stderr。之后工具失败、核验失败为 error/decision=null，取消为 cancelled/null/目标 CLI 4；强杀可能没有输出。有效 partial facts 可以 completed/BLOCK/2。完整且缺真实批准为 REQUIRE_APPROVAL/3，全部流程义务满足为 ALLOW/0。诊断不得藏在旧报告文件存在这一事实后面。

runId 是每次尝试身份，重试新建；幂等键标识相同不可变工作请求，不等于 runId。结果追加，当前指针以 binding+generation CAS；旧运行只归档。键覆盖 repo/task/worktree/requirements/candidate/base/group/source/baseline/contract/analyzer/config/coverage/action。每次门禁消费重新核验批准时效，不把撤销缓存掉。

candidate/base/merge-group、内容、规则、分析器/配置/覆盖、基线和依赖变化、批准过期/撤销都传播资格失效；历史报告不改字节。A/B 需求不交叉满足，确切同一父基线且批准范围包含两者才可分别继承。10 失效由发布依赖集合决定。

### 5. Exact queue candidate and permission boundary

可信队列控制器提供认证事件与 GG-CANDIDATE 解析的精确集成提交/基底/组，冻结该候选所需需求集合并安排必要重算。FlowGuard 消费精确绑定证据，不把分支 head 结果拼接成队列 ALLOW。组成员重排/基底推进必须新运行。只读 Git 候选输入不要求 Git 写端可用；写端若将来部署只在 FG-GATE 和独立授权之后消费结果。

默认读取授权根，限制 symlink/path traversal、文档大小、图深度/数量、解析预算、工件存储 URI；不运行文档指令或仓库策略脚本。候选进程无审批/合并凭据，MCP 不暴露任意 shell 或审批签发。check 无隐式文档写入；可选本地事件持久化须显式启用授权目录，不扩展到源文档或外部服务。日志脱敏并按租户限权，保留策略先决策再实现。

## Risks / Trade-offs

- 旧行为可能与十阶段假设不一致：固定 legacy SHA/许可/入口再建立差异裁决；没有来源则仅称独立设计。
- 过度复用批准造成范围越权：默认确切作用域、不自动 latest 继承；测试含 A/B 独立与共享基线。
- 事实投影遗漏使空事实误放行：以义务全覆盖矩阵审查，未映射 finding 直接不支持。
- 本地 Hook 可绕过：先 advisory/shadow；真实 required check 与权限测试完成才声明强制性。
- 存储复杂度：首期单进程 append-only 原型，跨进程部署必须事务/CAS/崩溃测试后启用，不能把 JSON 当分布式锁。

## Migration Plan

任务组 1–2 构成 F0/F1 的调查、来源、图与基线；组 3 构成 F2 的引擎投影与专业证据；组 4 构成 F3 的并发/失效/队列信任；组 5 构成 F4 的真实宿主、只读接口和分阶段推广。所有任务未完成。差分能力不足时保留已核实旧 provider 或禁用新门禁，不能双重独立权威。

先固定合法/违规/未知/故障 fixture，后 shadow 比较，再 opt-in protected check；转移每条规则 owner 前须无未解释差异、精确候选认证及撤销测试通过。回滚停止新适配器/控制器路由，保留历史工件，不回退代码 refs、不丢批准/失效审计，不更改 CodeGuard 原 CLI。END-TO-END 含两需求、队列重组、批准撤销、基线漂移、晚结果、CodeGuard 原生/适配旁车一致和回滚。

## Open Questions

首个 legacy/source 版本与许可证、审批 provider/身份映射/刷新时限、阶段适用性与 TDD 例外、领域 canonicalization、存储引擎/保留时长、绑定 CLI 参数、MCP/API 认证及首个宿主。任务 1.1–1.3、2.4、4.4、5.1/5.3 要产出明确 ADR 或能力拒绝；不把未决依赖假设成已安装。
