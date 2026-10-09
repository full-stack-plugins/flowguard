# FlowGuard — 详细技术实施方案

> V0.1 设计；新仓无 Rust 代码。既有 flowguard-plugin 的 Python 脚本、十阶段文档模板和 Hooks 是真实兼容目标，不得立即重写后宣称对齐。

## 1. 技术选择

Rust 2024 + Serde/Clap/Tokio（执行/取消）、GuardEngine SDK、按需 SQLite 事件/证据缓存；流程状态权威保持在现有 `docs/` 的项目/功能阶段文档中。OPA/Rego 可作为 GuardEngine 通用策略执行器，由 FlowGuard 拥有阶段策略包；不是将十阶段写死在 GuardEngine 源码里。现有 Python `flowguard_lib` 首版通过进程外版本化 JSON 适配器复用，确认语义一致后再演进 Rust 状态机。

## 2. 目标逻辑结构

~~~text
src/{context,stage,dependencies,approvals,obligations,gate,evidence,cli}.rs
adapters/{flowguard-legacy,speckit,openspec,superpowers,host-hooks,github}/
schemas/{stage-record,task-binding,approval-ref,gate-decision}/
fixtures/{valid,missing-stage,stale-evidence,forged-approval,release-invalidation}/
docs/
openspec/changes/
~~~

ProjectStageDocument 与 FeatureStageDocument 只保存既有 `docs/project`、`docs/features` 的真实路径及摘要，任务正文仍由原生 SDD 工具管理。Snapshot 以实际内容 ID/仓库/候选和解析器版本键控。

## 3. 核心状态与数据对象

- ContextBinding(repoId, worktreeId, sessionId, taskId, featureId, parentFeature, specSourceRef)。
- StageRecord(stageNumber, owner, docRef, state, approvalRef, evidenceRefs, version, inheritedFrom)。
- EvidenceObligation(guardName, ruleSetDigest, candidateOid, coverageRequirement, required, expiryCondition)。
- ApprovalRef(actorVerified, purpose, scope, docDigest, candidateDigest?, issuedAt, expiry, revokedAt)。
- GateDecision(targetAction, candidateDigest, requiredObligations, observedEvidence, unresolvedGaps, decision, reasons)。
- ExecutionGrant 是由受信控制端颁发的**未来协议对象**，普通 Agent 不能自己提交 true 字段骗取执行权。

状态：pending → in_progress → pending_acceptance → accepted；另有 inherited/skipped/invalidated，满足明确的批准条件才可复原。区分“文档验收”“需求批准”“代码候选技术证据”“发布用户验收”，不能一份 accepted 代表所有含义。

## 4. 计划与门禁算法

1. Discover：只读检索真实仓库 SDD 模式、十阶段文档与任务上下文，不执行 init。
2. Bind：唯一化 repository/worktree/task/feature，确认原生规格来源，冲突返回待澄清。
3. BuildGraph：读取原项目/功能文档和依赖，生成阶段适用范围及继承快照。
4. FreezeObligations：在运行检查前固定阶段必须提供的 GuardResult 和批准信息。
5. GatherEvidence：从各 Guard/legacy provider 接收带版本/来源/候选/覆盖的证据，失败与缺失不消失。
6. Validate：分别检查阶段状态、文档摘要、依赖、审批身份、证据新鲜度与覆盖；不能将 CodeReview success 当技术通过。
7. Decide：ALLOW（仅指定阶段动作）、REQUIRE_APPROVAL、BLOCK，并提供缺口和合法修复路径。
8. Execute：写操作只能由对应可信执行器在校验有效 grant/期望 targetOid 后执行，未知结果先对账。

## 5. CLI/MCP/Hook/API（目标，不代表已可运行）

~~~sh
flowguard discover --project .
flowguard context bind --task TASK-104 --feature example
flowguard stage status --feature example
flowguard evidence verify --task TASK-104
flowguard gate check --task TASK-104 --action git-commit
flowguard gate check --task TASK-104 --action release
~~~

继续支持旧 flowguard-plugin 的命令和 Hooks，Rust 接口不与旧产物产生并行的权威状态。Agent Hook 只能提前反馈；MCP 不暴露审批签发/任意 shell；HTTP 长任务 API 需独立身份、幂等、超时、取消和租户范围控制。

## 6. 安全与恢复

候选代码不可接触审批签发凭据；候选文档中的 `accepted` 无法取得可信授权。独立 CI 在正确合入候选重验受保护基线，服务端 Rulesets 要求状态检查。超时、崩溃、网络中断和旧运行迟到：保留多维状态、不可让旧 PASS 覆盖新 FAIL；授权撤销和阶段失效按因果重算。日志脱敏；文件写入仅对原有已授权 docs/ 路径，采用原子替换并检查预期摘要。

## 7. 分阶段实施与兼容门槛

| Wave | 实施内容 | 验收 |
|---|---|---|
| F0 | 固定十阶段 schema、文档 adapter、context & graph | 与 Python 原行为快照一致 |
| F1 | 审批与证据义务、过期/继承/跳过 | 正常/无批准/非法继承/目标变化 |
| F2 | GuardEngine 组合策略 + 跨专业证据 | 缺 TestGuard/SpecGuard 不会误放行 |
| F3 | 原子状态迁移、可信 CI/Hook、受信 grants | Agent 自签和绕过 Hook 均失败 |
| F4 | MCP/API、跨宿主兼容、权限/审计/回滚 | N/N-1、真实宿主、失败恢复 |

实施必须用十阶段真文档与既有 Python fixture 做逐条差分。迁移期间一份规则仅有一个权威 owner；如果 Rust 没通过差分验证，就继续使用已确认的 Python provider，不让两套策略独立判定。
