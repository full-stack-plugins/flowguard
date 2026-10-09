# FlowGuard — 详细技术实施方案

> 全部为实施提案；检查基线 `a6bd25fc38b0a323f880bedd395373d3bb672826`，2026-10-09。实际树只有四份 Markdown 文档，无运行时、测试、配置或 OpenSpec。架构事实边界见[architecture.md](architecture.md)。

## 1. 技术选择与依赖准入

拟使用 Rust 2024、Serde/Clap，异步执行与取消确有需要时再引入 Tokio；通过 GuardEngine SDK 使用通用规则/证据能力。没有 Cargo.toml，以上不是现有依赖。先用不可变事件文件验证模型；多进程需要事务时再选 SQLite。缓存不作批准权威，文档描述阶段意图，可信外部审批记录提供授权依据。

旧设计中的 Python `flowguard_lib`、JSON 适配器和 Hooks 尚未核实存在或协议。迁移 F0 必须先获取授权可读的固定外部修订，检查许可证、源码、入口、版本、fixture 和真实宿主。若没有结构化协议，则设计独立适配器并报告能力差异，不假定旧插件已输出目标格式。不确认旧行为时保持未验证状态，不把新模型宣布为兼容替换。OPA/Rego 仅是远期可选方案；当前 GuardEngine 协议不因此获得通用脚本执行或插件扩展能力。

## 2. 拟议目录与对象

以下目录均未创建：

```text
src/{context,stage,dependencies,approvals,obligations,gate,evidence,cli}.rs
adapters/{flowguard-legacy,speckit,openspec,superpowers,host-hooks,github}/
schemas/{stage-record,task-binding,approval-ref,gate-decision}/
fixtures/{valid,missing-stage,stale-evidence,forged-approval,release-invalidation}/
openspec/changes/
```

| 内部对象（提案） | 必需意义与不变量 |
|---|---|
| ContextBinding | repo/task/worktree、非空需求集合、精确 candidate/base、可选 merge group；session 只追踪 |
| BaselineRef | 不可变修订和内容摘要、所属项目/阶段、适用需求范围、可信批准引用 |
| StageRecord | 阶段号、项目/功能归属、源路径/摘要、声明状态、版本、依赖/继承边、证据和批准引用 |
| EvidenceObligation | 守卫、规则摘要、精确调用绑定、分析器及覆盖要求、必需性、失效条件；先冻结后执行 |
| ApprovalRef | 外部不可变记录标识；控制器解析其主体、权限、目的、目标摘要、范围、期限、撤销状态 |
| GateDecision | 指定动作、冻结义务摘要、观察证据、未解决缺口、作用范围与决策 |
| AuditEvent | 输入/状态版本、因果引用、失效或转换理由、控制器身份、执行对账引用 |

这些对象不是 `guard.partme.ai/v1alpha1` 的可接受字段。批准结构不设置可由调用者自证的 `actorVerified` 布尔值。ProjectStageDocument/FeatureStageDocument 只引用原生文档，不复制需求正文。

## 3. 协议与退出行为

当前共享引擎只接受严格 GuardContract YAML、GuardFacts JSON、GuardReport JSON，精确 `forbid_relation` 和 enforcement `enforce/review/advise`，未知字段拒绝；引擎不解析阶段图或签发授权。partial facts 为 BLOCK/INDETERMINATE；complete 限于声明分析范围。报告未签名，verify 是重算一致性校验，不是可信身份校验。本仓无引擎源码，以上为跨仓冻结契约；接入前需固定引擎版本并执行真实契约测试。

使用独立草案 [GuardRunEnvelope](integration-contract.md)，版本 `guard.integration/v1alpha1`；精确字段以该文件为准。本设计只要求：运行状态 completed/error/cancelled 与 decision 分离，失败 decision 为空；调用绑定含 repoId/taskId/worktreeId/requirementIds/candidateOid/baseOid/mergeGroupId；包含 contract/facts/report 摘要引用、分析器/覆盖、审批引用与诊断。错误不能伪装为合法 GuardReport。信封版本、crate semver、策略修订各自演进，不能混为一个版本号。

未来 FlowGuard check 映射：`0` ALLOW，`2` BLOCK，`3` REQUIRE_APPROVAL，`4` 输入/运行/验证错误；拟用 stdout JSON、stderr 诊断。取消在集成层使用 cancelled，CLI 暂拟退出 4，待实现时固定并测量。无现有 `--report`，不提供其行为保证。CodeGuard 旧 CLI 保留历史退出码，通过显式、版本化适配器归一化，不静默修改其外部行为。

## 4. 门禁算法与状态转换

1. **Discover**：只读定位仓库、原生 SDD、十阶段文档；不 init、不运行仓库脚本。
2. **Bind**：验证所有身份和调用范围，规范化路径、固定 Git 对象及需求集合；歧义报错，不猜测当前目录代表的任务。
3. **BuildGraph**：解析阶段与基线、验证唯一归属和 DAG、读取固定版本；无引用证据的 inherited 无效。
4. **Freeze**：从受保护策略计算适用阶段、专业守卫、覆盖和批准要求，冻结摘要；provider 缺失也保留其义务。
5. **Gather**：按独立运行绑定收集专业报告和摘要，对每个 provider 保留 complete/partial/error/cancelled 状态；绝不通过删除失败项缩减义务。
6. **Validate**：重算报告一致性；核对来源通道、候选/基底/合并组、规则/分析器/覆盖和基线；控制器独立验证批准身份、范围、期限、撤销。Code review 文本成功不能替代专业技术证据。
7. **Decide**：工具失败/取消产生无放行决策的错误/取消信封；必要证据缺失或不完整、范围不符、违规或失效为 BLOCK；仅审批不足而技术条件齐备时 REQUIRE_APPROVAL；全部满足才 ALLOW。
8. **Commit state**：以 expected state version + 输入摘要执行条件写入；冲突重新绑定重算，不覆盖并发结果。默认 check 只读，写事件或文档的命令未来需单独定义授权。
9. **Execute externally**：可信控制器在执行瞬间再次核对候选、基线与审批状态，宿主执行授权动作并审计。未知执行结果先对账；FlowGuard 不自行写 Git refs 或发布。

| 转换 | 前置条件 | 失败处理 |
|---|---|---|
| pending → in_progress | 有明确任务范围、合法开始动作 | 保留 pending 并解释缺口 |
| in_progress → pending_acceptance | 文档和必需证据已提交，等待验证/批准 | 不自动 accepted |
| pending_acceptance → accepted | 当前依赖、完整技术证据和有效批准全满足 | BLOCK 或 REQUIRE_APPROVAL |
| 任意适用阶段 → inherited | 精确父基线和批准覆盖本需求 | 拒绝模糊引用/范围越权 |
| 任意适用阶段 → skipped | 受保护策略允许、明确跳过批准仍有效 | 默认不跳过 |
| accepted/inherited/skipped → invalidated | 绑定或其依赖失效 | 记录原因并传播相关下游 |
| invalidated → pending_acceptance | 更新材料、重建义务并重新提交 | 禁止直接恢复 accepted |

## 5. 失效图、基线与多需求隔离

内部缓存键拟由规范化调用绑定、动作、义务摘要、基线摘要、规则/分析器/覆盖版本计算；序列化规范与摘要算法待 schema 落地时冻结。批准查询结果需携带刷新信息，在执行前重新验证；不能只因缓存键不变就复用过期批准。

失效根包括 candidate/base/merge group、需求/阶段内容、基线版本、规则集、分析器/覆盖变化，以及批准过期/撤销。沿明确依赖边失效：A 的需求改变使 A 下游失效；B 仅在依赖 A 或同一变更基线时受影响。任何基线修订都使依赖旧绑定的资格失效，不能“语义相似”自动复用。02/07 共享通过确切基线引用，10 固定发布需求清单和功能 09 摘要集合。

并行 A/B 各有 task/worktree/requirementIds。对 A 的批准必须明确覆盖 B 才能成为 B 的可用输入，且 B 仍独立决策。状态更新使用聚合版本 CAS；同一幂等键重复相同输入返回同一记录，复用键但输入不同报冲突。旧运行可追加审计，不可将当前指针从 head2 回退到 head1。项目文档并发修改遇到摘要冲突必须重试解析，禁止最后写入覆盖。

队列控制器构造精确集成候选 M，冻结合并组中的需求并重跑所有受影响守卫，FlowGuard 重算 M 的阶段资格，GitGuard检查候选范围，托管平台核验 required checks 并执行合并。队列重组/基底推进必须生成新绑定，旧分支结果只能作历史参考。执行前比较 M 与当前队列候选，不匹配立即失效。发布同理固定最终交付对象，不复用任意早期提交的验收。

## 6. 错误、恢复与安全要求

以下诊断分类是提案，不是已实现的错误码或引擎字段：

| 类别 | 结果 | 可执行恢复 |
|---|---|---|
| INPUT_INVALID / GRAPH_CYCLE / BINDING_AMBIGUOUS | runStatus error、空 decision | 修正输入/依赖，重新发现 |
| PROVIDER_MISSING / EVIDENCE_STALE / COVERAGE_INCOMPLETE | 已完成资格检查为 BLOCK；真实工具故障为 error | 安装经核实 provider 或针对当前候选重跑 |
| APPROVAL_MISSING | 技术条件满足时 REQUIRE_APPROVAL | 通过真实宿主请求批准 |
| APPROVAL_INVALID / BASELINE_CHANGED | BLOCK | 刷新外部记录/基线，重新验证依赖 |
| TIMEOUT / PROVIDER_CRASH / VERIFY_ERROR | error、空 decision | 有界重试；不复用旧 ALLOW |
| CANCELLED | cancelled、空 decision | 保留部分审计，重新开始独立运行 |
| STATE_CONFLICT / EXECUTION_UNKNOWN | 不放行新动作 | CAS 重算或查询宿主对账 |

缺失分析义务与分析器执行故障要明确区分。阶段门禁 BLOCK 不妨碍只读发现、解释和已授权修复。重试采用有限次数/超时，不反复刷新直到忽略失败。报告内容合法但来源不可验证仍不能成为可信外部授权。

固定受保护 contract 与 provider 版本；不运行不可信策略脚本，不把 repo 文档路径拼成 shell 命令。路径必须留在授权根目录，拒绝符号链接逃逸；为文档、图、报告、子进程和网络请求设资源限额。审批凭据仅存在可信控制器，普通适配器只读、最小权限。审计需访问控制、脱敏、可追踪不可变输入与失效链；保留时长和删除机制由部署方明确。

## 7. CLI、MCP、Hook 和 API 提案

下面命令均不存在，不可作为当前使用说明执行：

```sh
flowguard discover --project .
flowguard context bind --task TASK-104 --feature example
flowguard stage status --feature example
flowguard evidence verify --task TASK-104
flowguard gate check --task TASK-104 --action git-commit
flowguard gate check --task TASK-104 --action release
```

这里是用户交互草案；候选、基底和需求绑定必须由经过验证的上下文解析，不可仅靠 TASK-104 或工作目录推断。显式绑定参数及缺省规则在实现前定版。discover 返回来源、能力和缺口；status 返回带版本的声明/验证状态；verify 不签发授权；gate 返回范围决策与合法补救路径。

MCP 首版仅暴露只读发现/状态/检查，不暴露任意 shell 或签发审批。HTTP 长任务需独立身份、租户边界、幂等键、超时/取消和结果绑定；宿主权限校验不能只依赖 taskId。Hooks 只作早期反馈；真实宿主验证必须测试关闭 Hook 后受保护门禁仍生效。旧插件命令和 Hook 是否可兼容由 F0 证据决定；本设计不承诺当前已有适配器。

## 8. 分阶段实施和可测验收

| 阶段 | 交付物 | 必须通过的可观察验收 |
|---|---|---|
| F0 兼容调查 | 外部固定 SHA/许可/入口清单、十阶段真实样例、差异表 | 每个保留行为都指向已检查源码或样例；所有未知项显式记录；无资料则不宣称兼容 |
| F1 来源与图 | 版本化 schema、只读 resolver、DAG、基线快照 | 10 阶段和项目/功能布局全覆盖；缺文档、循环、歧义、越界路径均拒绝；重复输入得到相同快照 |
| F2 证据与审批 | 冻结义务、专业适配器、可信批准查询 | 无 provider、partial、错误、伪造 accepted、过期/撤销、范围不符均不 ALLOW；只有技术完整的缺审批返回 REQUIRE_APPROVAL |
| F3 并发与失效 | CAS、幂等、依赖失效、精确队列候选 | A/B 隔离；旧结果迟到不覆盖；candidate/base/group/规则/分析器/覆盖/基线逐项变化均失效；M 重算而非复用分支 PASS |
| F4 宿主与迁移 | 真实 required check、权限/审计/恢复、兼容适配器 | 禁用 Hook 无法绕过；执行前撤销批准被拒；未知执行先对账；N/N-1 支持矩阵逐项测试后才承诺兼容 |

每个阶段产出执行记录、固定输入摘要、预期/实际结果和未通过项；所有列出的负例必须通过，无未解释兼容差异才能转移相应规则所有权。一条领域规则同一阶段只能有一个权威 owner；迁移未完成时保留已核实旧 provider 或阻断该能力，不能让两套实现各自宣布通过。

未来测试层次：解析与图单元测试、真实 GuardEngine 协议契约测试、旧新行为差分、并发/撤销故障注入、精确候选宿主端到端。当前无测试入口，本次只能执行文档链接/命名/一致性/差异检查，不声称上述验收已通过。

## 9. 尚待定版的决策

审批后端、身份映射与撤销刷新时限；摘要规范与 schema；默认阶段适用性/TDD 范围；CLI 绑定参数；可信执行凭据或 grant 的协议；存储/审计保留；外部旧实现能力和宿主版本。默认选择只读、无自动副作用、拒绝不明确绑定、不自动继承/跳过；真实验证完成后再扩展功能。
