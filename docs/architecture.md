# FlowGuard — 研发流程守卫总体架构

> 目标设计；无产品运行时。检查日期 2026-10-09，基线 `a6bd25fc38b0a323f880bedd395373d3bb672826`。

## 1. 证据边界与产品定位

检查 `git ls-tree -r HEAD` 得到且仅得到 `README.md`、`README.zh-CN.md`、`docs/architecture.md`、`docs/technical-design.md`。无源码、依赖清单、配置、测试、CI 或 OpenSpec 文件可证明运行时行为。本文件所有组件、接口、存储和状态转换均为目标设计。

旧文档引用外部 [flowguard-plugin](https://github.com/full-stack-plugins/flowguard-plugin) 和 Python `flowguard_lib`，并声称插件 v0.4.0 已移除 `.flowguard/`。本次没有检查该外部源码或版本记录，因此不能确认这些陈述，也不能称外部插件为“已验证权威”。拟保留十阶段和 `docs/` 布局作为兼容假设；外部源码固定版本、许可证、入口、真实样例和行为测试都属于迁移准入条件。

FlowGuard 回答“此任务对这一候选、这一基线和这一动作，还缺哪些阶段证据或批准”。它拥有阶段意义与依赖，不能替代专业守卫证明代码正确。适用于跨阶段交付、多个需求并行、共享项目架构继承、基线变更后的失效传播和发布验收。它不创建第二份任务正文、不自动批准、不直接合并或部署。

## 2. 六个守卫与 GuardEngine 的边界

| 参与者 | 职责 | 不承担 |
|---|---|---|
| SpecGuard | 需求/规格领域解析和证据 | 流程批准 |
| ArchGuard | 架构领域分析和约束 | 项目审批身份认证 |
| CodeGuard | 代码领域检查，保留自身既有 CLI 行为 | 签发用户批准 |
| TestGuard | 测试领域分析与覆盖证据 | 将测试成功等同发布批准 |
| GitGuard | Git 范围、候选与仓库领域条件 | 单凭本地 Hook 完成服务端保护 |
| FlowGuard | 阶段、义务、继承、审批语义、流程决策 | 重跑其他守卫领域算法、执行合并 |
| GuardEngine | 通用 Contract 校验、中立 Rule 计算、确定性 Evidence | 领域审批、宿主调度、阶段状态机、合并执行 |
| 可信控制器/宿主 | 身份与权限校验、受保护策略装载、队列运行、实际执行 | 接受模型自签授权 |

没有 GuardCore。当前共享引擎协议（跨仓约束，非本仓源码证明）是 `guard.partme.ai/v1alpha1`：GuardContract YAML、GuardFacts JSON、GuardReport JSON，唯一精确规则类型 `forbid_relation`；未知字段拒绝，不能加入任意扩展。规则 enforcement 为 `enforce/review/advise`；决策 `ALLOW/BLOCK/REQUIRE_APPROVAL`。部分 facts 为 `BLOCK/INDETERMINATE`；complete 仅表示分析器声明的范围，并非全仓或完整业务证明。报告未签名，verify 重算不认证主体、来源、审批或执行权。

阶段图与可信审批无法通过向现有 GuardReport 填字段实现。目标 [GuardRunEnvelope 集成契约](integration-contract.md) 使用独立草案版本 `guard.integration/v1alpha1`，由未来适配器实现，现有引擎不解析它。版本号独立于 crate semver 和策略修订。

## 3. 阶段、原生来源与继承

以下是保留的兼容设计，尚未验证外部历史行为：

| 阶段 | 文档（使用方仓库） | 目标语义 |
|---|---|---|
| 01 需求分析 | `docs/features/<feature>/01-requirements.md` | 固定需求范围、验收条件 |
| 02 架构设计 | `docs/project/02-architecture.md` | 项目架构基线，功能显式继承 |
| 03 技术方案 | `docs/features/<feature>/03-solution.md` | 功能方案与依赖 |
| 04 测试用例 | `docs/features/<feature>/04-testcases.md` | 可追溯验证计划 |
| 05 概要设计 | `docs/features/<feature>/05-hld.md` | 功能分解与边界 |
| 06 详细设计 | `docs/features/<feature>/06-lld.md` | 实施接口与错误路径 |
| 07 编码规范 | `docs/project/07-standards.md` | 项目规范基线，功能显式继承 |
| 08 代码审查 | `docs/features/<feature>/08-review.md` | 候选范围审查证据 |
| 09 文档交付 | `docs/features/<feature>/09-docs.md` | 功能交付内容与证据引用 |
| 10 发布交付 | `docs/project/10-release.md` | 发布清单与指定功能集合验收 |

02/07/10 为项目级，01/03/04/05/06/08/09 为功能级；10 的共享记录不能被理解为任意功能均自动满足发布条件。OpenSpec/Spec Kit 保留原生任务记录，Superpowers 保留执行方法。FlowGuard 只保存引用、摘要和审计事件，不另建任务账或 `.flowguard/` 目录。

基线对象需要不可变文档摘要、修订引用、适用 requirementIds、授权来源和批准记录引用。文档 `accepted` 是候选声明，不是授权；外部可信记录才提供批准身份与时效。继承固定到确切父版本，禁止解析“latest accepted”后隐式替换。依赖必须无环；源缺失、引用冲突、循环或不明确的需求绑定均停止该门禁并返回诊断。

## 4. 目标数据流与接口边界

```text
Native specs + stage documents + authenticated invocation
                        |
             Source resolver / context binding
                        |
           DAG + immutable baseline snapshot
                        |
            Frozen obligations and policy digest
                        |
        Specialist evidence     External approval references
                    \             /
             Evidence + approval validation
                        |
             FlowGuard scoped gate decision
                        |
       Trusted controller revalidation + protected executor
```

输入包含 repoId/taskId/worktreeId/requirementIds/candidateOid/baseOid/mergeGroupId（如适用），文档与基线不可变引用、规则版本、分析器与覆盖要求。sessionId 可用于追踪，不能作为稳定授权键。领域对象包括 StageRecord、BaselineRef、DependencyEdge、EvidenceObligation、ApprovalRef、GateDecision；它们是 FlowGuard 未来内部对象，不是当前引擎格式。

输出包含目标动作、缺失义务、证据/批准引用和范围决策。集成信封将 `runStatus=completed/error/cancelled` 与决策分开；失败时 decision 为空，不能制造成功报告。错误详情使用独立诊断对象，不扩展 v1alpha1 GuardReport。FlowGuard 记录动作资格，不签发执行权；若将来定义 ExecutionGrant，应由可信控制器单独签发，仍需协议和威胁模型评审。

### 技术决策与动作资格分离

目标 gate check 将 FlowGuard 自己的阶段/义务观察投影成兼容 facts，由受保护的流程规则生成独立 GuardReport；该次 GuardRunEnvelope.decision 必须等于所引用的 FlowGuard 报告 decision。专业守卫报告各自保留原决策：外部批准不会把其 REQUIRE_APPROVAL 改写成 ALLOW。批准状态变化产生新的调用尝试和 FlowGuard 事实快照；新的流程报告可以不同于其引用的专业报告，因为检查范围不同，而不是审批重写了技术证据。

GateDecision 中的动作资格是独立领域结果，通过既有 `artifacts.domain` 引用，不能覆盖信封 decision 或添加未定义的共享字段。可信控制器综合有效技术报告、真实批准和权限决定是否授权动作。下游 enforce/BLOCK、partial 和工具错误不能因批准而被判为已满足。投影必须用已支持的精确关系覆盖所有必需缺口；尚无可验证投影时报告能力未支持，不生成任意 ALLOW。

## 5. 状态机与行动资格

目标阶段状态保留 `pending → in_progress → pending_acceptance → accepted`，旁路为 `inherited/skipped/invalidated`。accepted 要求完整且有效的领域证据、必要的真实批准和有效前置阶段；inherited 要求满足相同要求的确切父基线；skipped 需要经过认证、限范围且未过期的跳过批准。invalidated 必须重新计算义务和验证，不能手改回 accepted。阶段完成度、检查运行状态和技术决策是三个独立维度。

拟议默认动作策略：业务编码要求适用的 01—07；提交要求 08/09 和冻结的专业证据；发布要求 10、指定功能集合的 09、相关依赖与发布验收。阶段不是机械地要求所有任务填写所有文档：适用性和 TDD 前置测试例外必须由受保护策略明确，不能由 Agent 临时缩减义务。读取、澄清、补测试和已授权修复有独立动作资格，不因交付门禁阻断而全部封禁。

FlowGuard 自身门禁规则的目标决策优先级（不改写下游报告）：工具故障/取消先输出运行状态且不放行；分析缺失、不完整、范围不符或确定违规为 BLOCK；分析完整且其他条件满足但缺可信批准为 REQUIRE_APPROVAL；全部义务满足才为 ALLOW。批准不能覆盖不完整分析或工具失败。ALLOW 不等于合并或发布授权。

## 6. 失效、并发需求与合并队列

| 变化 | 失效范围与响应 |
|---|---|
| candidateOid/baseOid/mergeGroupId 变化 | 对应运行和动作资格失效；对新精确候选重算 |
| 规则集、分析器版本或覆盖范围变化 | 依赖该证据的阶段/运行重验 |
| 项目基线修订、需求或阶段内容变化 | 精确依赖边的继承与下游失效；10 重验相关功能集合 |
| 批准过期或撤销 | 相应批准及依赖阶段资格失效；执行前再次查询 |
| 输入损坏、provider 不可用或中断 | 保留失败，不转换为通过或空义务 |

多个需求 A/B 使用独立 task/worktree 和 requirementIds；A 的 accepted 不意味着 B 已批准。共享 02/07 只有在批准覆盖相应范围且摘要相同的情况下可复用，不能复制状态字符串。共享项目文档由控制器串行更新或使用 expected digest 条件写入；并发写冲突需重新读取与重算，不用最后写入获胜。

队列将 A/B 与新基底合成 M 时，可信队列控制器负责构造 M、重建冻结义务并安排各守卫针对 M 重算；FlowGuard 验证 M 的完整依赖集合和阶段资格，GitGuard 核实 Git 领域范围，平台执行受保护检查后的合并。分支 head 证据不能直接冒充 M 证据。出队重组、基底推进或合并组变化均使旧结果过期。缓存只能复用不可变、范围相符的中间材料，最终门禁必须重新判断。

结果写入采用不可变绑定键和期望版本比较；迟到的 A/head1 结果保留审计记录，不能覆盖 A/head2 或 B 的状态。幂等重试不重复状态迁移；实际合并结果未知时先查询宿主对账，不能重放未经确认的副作用。

## 7. 安全、扩展与审计

默认只读、无外部副作用。候选代码不能持有审批/合并凭据；受保护策略不从未经审查的候选分支替换。可信控制器校验批准主体、作用域、不可变目标、有效期和撤销信息，不能信任 `actorVerified: true`、`--actor user` 或 Markdown 文本。

适配器只解析声明式数据，禁止执行仓库自带策略脚本；限制文件根目录、符号链接越界、输入大小、图节点数、子进程时间与输出大小。远程引用按许可来源获取，凭据最小只读权限；诊断脱敏，审计避免存储密钥和完整私密需求正文。未来文档更新只在已授权路径采用原子替换和摘要前置条件。

审计事件需记录调用绑定、源摘要、规则/分析器版本、覆盖、外部审批查询结果引用、决策理由、失效原因、状态版本和执行对账结果。日志不是授权；生产需受保护存储与保留策略。Native-spec、legacy、MCP、Hook、CI 适配器使用版本化协议；不允许任意插件字段穿透引擎的严格 schema。本地 Hook 可绕过，只有正确候选的 required check 与宿主权限保护才构成强制门禁。

## 8. 架构决策和未决项

FG-ADR-001：保留十阶段/docs 布局作为待验证兼容目标，不宣称迁移完成。FG-ADR-002：领域 Guard 保持独立，FlowGuard 不复制专业检查。FG-ADR-003：技术证据与批准各按范围失效，依赖图传播资格变化。FG-ADR-004：外部可信身份与服务端保护负责授权。FG-ADR-005：外部源码固定、差分验证、宿主验证通过后才转移规则所有权。FG-ADR-006：保留授权修复路径，未知状态不放行受保护动作。

未决项：外部旧实现实际能力；审批提供方及身份映射；签名/撤销刷新与审计保留策略；队列宿主具体 API；阶段适用性与共享基线范围格式。可逆默认是只读发现、拒绝不明确继承、无自动跳过、无自动合并。实施与可测验收见[技术方案](technical-design.md)。
