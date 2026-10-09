# FlowGuard Evidence-bound Workflow Gates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. 本次仅编写计划，所有任务未实施；用户指定 OpenSpec 位置优先于 skill 默认目录。

**Goal:** 实现绑定精确候选、不可变基线和真实批准观察的阶段技术门禁，保持专业报告不可变且执行授权外置。

**Architecture:** FlowGuard 只读解析原生 docs 与阶段 DAG，冻结义务后验证专业证据，再通过自己的受保护关系规则生成独立报告。动作资格是领域附件；可信控制器认证审批、复查新鲜度并决定执行。并发和队列结果按不可变绑定隔离。

**Tech Stack:** 拟用 Rust 2024、Serde/Clap 与固定版本 GuardEngine；依赖未安装、manifest 尚不存在。Tokio、SQLite、身份提供方和宿主库均需任务内 ADR/验收后选择，不以计划代替兼容证据。

**Spec:** [阶段与基线](specs/workflow-stage-baselines/spec.md)、[证据门禁](specs/workflow-evidence-gates/spec.md)、[运行隔离](specs/workflow-run-isolation/spec.md)、[本变更设计](design.md)。实施前同读[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[共享集成草案](../../../docs/integration-contract.md)和[共享路线图](../../guard-roadmap.md)。

## Global Constraints

- 原始 main `a6bd25fc38b0a323f880bedd395373d3bb672826` 只有四份文档；规划前复查文档分支 `de60563f9d80f5ad0ef835191ae907c469c04e38`，无 runtime、Cargo、tests 或旧 OpenSpec。本表所有 `src/`、`tests/`、`schemas/`、`adapters/`、ADR 路径均为拟新建。
- 保留 `guard.partme.ai/v1alpha1` 严格协议与精确 `forbid_relation`；集成 `guard.integration/v1alpha1` 独立，无任意字段与隐式 N/N-1。引擎不拥有阶段、审批签发或 Git 写入。
- 自身 engine-backed envelope decision 等于自身 GuardReport；下游 REQUIRE_APPROVAL 不因批准改写；GateDecision 使用 domain 附件，外部授权单独记录。
- 前绑定错误无信封；完整 binding/producer/coverage 后的 error/cancelled decision=null。合法 partial 为 BLOCK；批准不覆盖工具故障、不完整分析或 enforce/BLOCK。
- 默认只读/no external side effects；无 `.flowguard/`，不复制原生任务；CodeGuard 原 CLI、报告、退出码不修改。专业域规则不在 FlowGuard 重实现。
- 每个实现任务先建立所列失败样例，再实现并记录实际回归输出；新工程建立后执行该任务对应 Rust 测试。规划勾选、文件存在、模拟 provider 不能算真实集成通过。

## Review Focus

- 看似只改文档但偷偷换成 latest 继承：任务 2.2 断言固定父摘要与范围。
- 有效批准与下游 REQUIRE_APPROVAL 被错误重写：任务 3.3/3.4 比较原报告字节并区分自身报告。
- 同一 worktree 路径复用或迟到成功污染新需求：任务 4.1/4.2 核对绑定与 CAS。
- 门禁满足后批准撤销、队列 base 变化：任务 4.3/4.5 重验真实消费时点。
- 旧插件无法核实或本地 Hook 被绕过：任务 1.1/5.4 保留未验证状态并验证独立宿主保护。

## Dependency Gates

组 1 与本地组 2 图 fixture 无需等待其他仓库。组 2 的真实规格导入等待 SG-BASELINE；组 3 真实互操作等待 GE-CONTRACT/GE-ADAPTER、SG-BASELINE、GG-CANDIDATE 和策略要求的 AG-EVIDENCE/CG-ADAPTER/TG-EVIDENCE；组 4 可信消费等待 GE-TRUST。组 4 完成达到 FG-GATE；组 5 END-TO-END 等相关门槛，生产独立发行再等待 GE-RELEASE。固定源 SHA 可支持前期开发。

依赖计划：[GE](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)、[SG](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis)、[AG](https://github.com/full-stack-plugins/archguard/tree/docs/guard-design-20261009/openspec/changes/extend-architecture-analysis-and-evidence)、[CG](https://github.com/full-stack-plugins/codeguard/tree/docs/guard-design-20261009/openspec/changes/add-guardengine-compatibility-adapter)、[TG](https://github.com/full-stack-plugins/testguard/tree/docs/guard-design-20261009/openspec/changes/add-test-obligation-evidence-pipeline)、[GG](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance)。GitGuard **只读**候选先于 FG-GATE；可选写端在外部授权后消费门禁结果，不是 FlowGuard 前置。

## 1. F0 调查、来源和合同基础

接口：`discover(AllowedRoot, SourceProfile) -> SourceInventory`；`bind(InvocationInput, CandidateObservation) -> ValidatedBinding | PreBindingDiagnostic`。本组输出本地 fixture 与明确能力边界，不宣称跨仓集成。

- [x] 1.1 在 `docs/adr/legacy-compatibility.md` 和 `fixtures/legacy/inventory.json` 记录可核实外部 flowguard-plugin 固定 SHA、许可证、实际命令/格式/Hook 与十阶段来源；为拿不到源码/接口不符建立调查案例并保留 unverified。验收：每个兼容声明都有源码/样例引用，无资料时无迁移承诺。对应 **Evidence-based legacy compatibility investigation**。
- [x] 1.2 在 `docs/adr/domain-schema.md` 冻结 StageRecord、BaselineRef、FrozenObligations、GateDecision 的独立 schema/规范编码与能力版本，保存 `schemas/workflow/` 及 `fixtures/schema/` 正反例。验收：未知字段/版本、空身份、变更摘要均被拒绝且不扩展 engine 对象。对应 **Versioned stage graph and immutable inheritance**、**Independent gate report and immutable upstream verdicts**。
- [x] 1.3 在首个 `Cargo.toml`、`src/lib.rs`、`src/context.rs` 建立最小库和 `bind`，固定候选 Rust/依赖及资源预算 ADR；`tests/context_binding.rs` 覆盖明确/歧义 repo/task、缺 OID、SHA 对象格式与 dirty snapshot。验收：仅可核实完整输入产生 ValidatedBinding，前绑定错误不造 envelope。对应 **Binding-aware error and decision transport**。
- [x] 1.4 在 `src/stage.rs` 实现 docs 布局 resolver 和 SourceInventory；`tests/stage_discovery.rs` 对十阶段归属/路径/摘要逐项断言，并覆盖缺文档与未知版本。验收：所有必需源有终态、前后源树摘要一致、不创建 `.flowguard/`。对应 **Read-only native stage discovery**。
- [x] 1.5 在 `adapters/openspec/mod.rs` 实现首个固定来源版本的引用读取，给 `adapters/{speckit,superpowers}/` 仅声明尚未支持的能力；`tests/source_adapter.rs` 验证原生 task ID、来源版本、混合权威源冲突。验收：正文不复制、不执行安装或文档指令；后续适配器逐个验收。对应 **Read-only native stage discovery**。
- [x] 1.6 在 `src/input_limits.rs` 接入授权根/符号链接/大小/图输入预算；`tests/input_limits.rs` 覆盖越界 symlink、恶意文档指令、超限文本及可执行工件 URI。验收：全部拒绝或有界失败、日志脱敏、不把截断表示 complete。对应 **Bounded access and auditable stage evidence**。

## 2. F1 阶段图、基线和批准观察

接口：`build_graph(SourceInventory, StageProfile) -> StageGraph`；`resolve_baseline(BaselineRef, Scope) -> InheritanceSnapshot`；`ApprovalProvider.observe(ApprovalRef, ValidatedBinding, Action) -> VerifiedApprovalObservation | ProviderError`。真实需求 ID/义务导入等待 SG-BASELINE，批准端口此时仅测试替身。

- [x] 2.1 在 `src/dependencies.rs` 建立稳定节点和显式 DAG 校验；`tests/stage_graph.rs` 覆盖十阶段、悬空边、环、重复归属及乱序输入。验收：图排序确定、定位冲突且循环有限结束。对应 **Versioned stage graph and immutable inheritance**。
- [x] 2.2 在 `src/baseline.rs` 实现不可变继承，`tests/baseline_inheritance.rs` 比较 A/B 独立范围、共享 02/07、latest 替换、父摘要变化与发布 10 的功能 09 集合。验收：只共享确切有效引用，不能复制 accepted 或隐式换父版本。对应 **Versioned stage graph and immutable inheritance**、**Dependency-aware eligibility invalidation**。
- [x] 2.3 在 `src/stage_transition.rs` 实现声明/资格分离的转换表；`tests/stage_transition.rs` 覆盖全部七状态、伪 accepted、未批准 skipped 和 invalidated 重新提交。验收：非法边均拒绝，阶段状态不是技术/执行状态。对应 **Explicit stage transitions and remediation scope**。
- [x] 2.4 在 `docs/adr/approval-port.md` 与 `src/approvals.rs` 固定只读批准端口、identity/action/scope/digest/expiry/revocation 校验和刷新策略；`tests/approval_observation.rs` 注入过期、撤销、越权、布尔自证、服务故障。验收：故障不等于确认无批准，生产 provider 选择仍需 GE-TRUST 与独立审查。对应 **Authenticated approval observations**。
- [ ] 2.5 在 `src/action_policy.rs` 定义受保护阶段适用性、TDD 前置测试例外及修复动作范围；`tests/action_policy.rs` 覆盖提交受阻但授权补测试可用、未授权 skip、候选缩减适用阶段。验收：合法修复不被全局封禁，交付资格不因此放松。对应 **Explicit stage transitions and remediation scope**、**Frozen domain evidence obligations**。
- [x] 2.6 在 `adapters/specguard/mod.rs` 接入 SG-BASELINE 稳定 requirement IDs、固定批准基线和义务引用；`tests/specguard_baseline_contract.rs` 保存真实版本样例并覆盖范围缺失/版本不符。验收：真实与模拟样例标记分离，不复制规格正文，无 SG 能力时相应门禁保持未满足。对应 **Versioned stage graph and immutable inheritance**、**Frozen domain evidence obligations**。

## 3. F2 专业证据与自身引擎报告

接口：`freeze(StageGraph, ProtectedPolicy, ValidatedBinding) -> FrozenObligations`；`validate_evidence(FrozenObligations, ArtifactRefs) -> ValidatedEvidenceSet`；`evaluate_gate(StageObservations, ValidatedEvidenceSet, ApprovalObservations) -> OwnReportAndDomainDecision`。GE-CONTRACT/GE-ADAPTER 定版前只做本地样例，不发兼容声明。

- [x] 3.1 在 `src/obligations.rs` 实现收集前冻结阶段/Guard/覆盖义务与摘要；`tests/frozen_obligations.rs` 删除 provider、文档、规则并乱序返回子集。验收：必需集合不缩减、每个缺口可定位；缺 TestGuard 不能拿其他 Guard 成功替代。对应 **Frozen domain evidence obligations**。
- [x] 3.2 在 `src/evidence.rs` 与 `adapters/guards/mod.rs` 使用 GE-ADAPTER 验证版本、能力、完整绑定、工件摘要、coverage；`tests/specialist_evidence.rs` 含 wrong-candidate/base/group、partial、伪造摘要、unsigned 一致但无可信来源。验收：所有错误证据不满足义务，专业领域算法不复制。对应 **Scope-bound specialist evidence validation**。
- [x] 3.3 在 `src/projection.rs` 和 `fixtures/gate_mapping/` 建立版本化阶段缺口→精确事实/受保护断言映射；`tests/gate_projection.rs` 对合法/enforce/review/advise/partial/未映射缺口逐项校验。验收：强制缺口零漏映射，未知不以空 facts 放行，实际引擎 schema 可加载。对应 **Independent gate report and immutable upstream verdicts**。
- [x] 3.4 在 `src/gate.rs` 生成自己的 GuardReport 与 domain GateDecision；`tests/independent_gate_report.rs` 固定上游 REQUIRE_APPROVAL 字节，分别输入批准前后观察及 enforce/BLOCK。验收：自身新报告可变但上游字节不变，信封等于自身报告，批准不能越过 enforce/partial，资格不签执行权。对应 **Independent gate report and immutable upstream verdicts**。
- [ ] 3.5 在 `src/transport.rs` 按 GE-CONTRACT 编码完整绑定后 envelope 与独立前绑定诊断；`tests/error_transport.rs` 覆盖参数歧义、未冻结范围、运行崩溃/超时/取消、有效 partial 和 malformed schema。验收：前绑定无 envelope，error/cancelled null，partial BLOCK；stdout 不混诊断。对应 **Binding-aware error and decision transport**。
- [ ] 3.6 在 `tests/cross_guard_contract.rs` 接入政策要求的 AG-EVIDENCE/CG-ADAPTER/TG-EVIDENCE 和 GG-CANDIDATE 真实固定工件；保留 `fixtures/providers/` 能力清单。验收：CodeGuard 原生 command/flags/schema/退出码不改写，弱 profile 在要求引擎验证时被拒，缺任何必需 provider 不声称 FG-GATE。对应 **Scope-bound specialist evidence validation**、**Phased validation migration and reversible rollout**。

## 4. F3 信任、并发、失效与队列

接口：`RunStore.append(AttemptEvent)`、`publish_if_current(Binding, ExpectedGeneration, ResultRef)`；`invalidate(ChangeCause, DependencyGraph) -> AffectedEligibility`。组 3 的所需真实 provider 加 GE-TRUST 是可信消费前提；本组不引入 privileged Git executor。

- [x] 4.1 在 `src/run_store.rs` 与 `schemas/workflow/attempt.json` 分离 runId/幂等键，定义追加记录与授权存储目录；`tests/attempt_identity.rs` 覆盖同请求去重、重试新 runId、同键异摘要、worktree 路径复用。验收：无跨需求覆盖，check 未启用持久化时不隐式写源文档。对应 **Immutable attempt isolation and conditional publication**。
- [x] 4.2 在 `src/run_store.rs` 实现 binding+generation CAS，`tests/concurrent_publication.rs` 同时运行 A/head1、A/head2、B 并延迟旧 ALLOW，注入写中断/重启。验收：旧结果仅历史、当前指针不回退；多进程支持只有事务实现与故障验证通过后才能声明。对应 **Immutable attempt isolation and conditional publication**。
- [ ] 4.3 在 `src/invalidation.rs` 实现依赖资格失效传播；`tests/invalidation_matrix.rs` 逐项变更 candidate/base/group/source/rules/analyzer/config/coverage/baseline/dependency/expiry/revocation。验收：受影响闭包精确、无关需求不变、旧报告原字节保留，10 的发布集合联动正确。对应 **Dependency-aware eligibility invalidation**。
- [ ] 4.4 在 `src/trust.rs`、`src/audit.rs` 接 GE-TRUST 的认证引用端口，并在 `docs/adr/audit-retention.md` 决定授权存储/脱敏/保留策略；`tests/trust_audit.rs` 覆盖 forged producer、凭据泄露、跨租户访问、必要工件过期及批准缓存后撤销。验收：复核当前批准而非缓存授权，缺工件失去资格，不由引擎签身份。对应 **Authenticated approval observations**、**Bounded access and auditable stage evidence**。
- [ ] 4.5 在 `adapters/merge_queue/mod.rs` 消费认证控制器事件和 GG-CANDIDATE 只读对象；`tests/queue_candidate.rs` 以 A/B 分支合成 M1，再推进 base/重组为 M2。验收：只认精确 M2 及其需求集重算证据，head/M1 不顶替，错误事件身份拒绝。对应 **Exact merge-queue composition without executor dependency**。
- [ ] 4.6 在 `tests/gate_dependency_dag.rs` 建立无 Git 写端的完整只读 FG-GATE 场景与控制器授权 stub；检查输出及调用记录。验收：所需读取证据齐全时生成技术结果，不等待 merge executor，不调用 grant/refs/release；执行前失效时外部控制器拒绝。对应 **Exact merge-queue composition without executor dependency**、**Read-only interfaces and separated execution authority**。

## 5. F4 接口、真实宿主与迁移推广

接口：只读 CLI/MCP/Hook 复用同一库；控制器消费 OwnReportAndDomainDecision，授权不在 FlowGuard。真实 END-TO-END 等相关证据门槛具备，独立生产分发等待 GE-RELEASE。

- [x] 5.1 在 `src/cli.rs` 与 `docs/adr/cli-contract.md` 定版 discover/stage status/evidence verify/gate check 参数、明确绑定输入和逐命令输出；`tests/cli_contract.rs` 覆盖 gate 0/2/3/4、无 JSON 前绑定错误、取消及 fresh 输出。验收：退出码反映自身报告，verify 不认证授权，不凭任务 ID 猜候选，未设计的 `--report` 拒绝。对应 **Read-only interfaces and separated execution authority**、**Binding-aware error and decision transport**。
- [x] 5.2 在 `adapters/legacy/mod.rs` 消费 1.1 已核实格式，`tests/legacy_differential.rs` 执行合法/违规/未知/故障、继承/跳过/失效差分。验收：每个规则 owner 唯一，差异未裁决或 legacy 未核实则保持未支持/不切换；无自动文档迁移。对应 **Evidence-based legacy compatibility investigation**。
- [ ] 5.3 在 `adapters/{mcp,host_hooks}/` 与 `docs/adr/host-api.md` 发布首个只读宿主能力矩阵，定义认证/租户/幂等/取消及 HTTP 是否确有需求；`tests/host_interface.rs` 注入 self-approve、任意 shell、越权 artifact、取消重入。验收：全部写权限能力拒绝；每个新增宿主版本单独测试，未选 HTTP 不开放端口。对应 **Read-only interfaces and separated execution authority**、**Bounded access and auditable stage evidence**。
- [ ] 5.4 在 `tests/acceptance/protected-host.md` 记录真实测试宿主 required check 配置与执行证据；包含关闭本地 Hook、候选改弱策略、批准消费前撤销。验收：三类绕过均不能合入，实际查验对象是精确队列候选，无生产凭据暴露；仅模拟不可标 enforced。对应 **Phased validation migration and reversible rollout**、**Exact merge-queue composition without executor dependency**。
- [ ] 5.5 在 `tests/acceptance/joint-gates.md` 执行 END-TO-END：两并行需求、共享基线、M1→M2、过期/撤销、规则/分析器漂移、旧运行迟到和 CodeGuard 原生/适配旁车对照。验收：零跨满足/旧候选放行、原生 CLI/schema/退出未改变，每项记录版本/命令/输入摘要/真实结果。对应 **Phased validation migration and reversible rollout**、**Immutable attempt isolation and conditional publication**。
- [ ] 5.6 在 `docs/rollout.md` 与 `tests/acceptance/rollback.md` 实施 advisory→shadow→opt-in 的逐规则 owner 切换及回滚演练，固定生产依赖在 GE-RELEASE 后再发布兼容矩阵。验收：未解释差异阻止推广，回滚停新路由且保留审计/撤销历史，无 Git refs 回退或 CodeGuard 行为修改，N/N-1 仅对实际通过版本承诺。对应 **Phased validation migration and reversible rollout**。

## Completion Evidence

执行者逐任务归档固定输入、命令、预期/实际输出与失败项；运行工具和官方 OpenSpec validator 成功后才能分别说明对应验证，不能勾选本计划代替实现。当前所有任务均未完成；本次计划检查不构成 F0–F4 运行验收。
