## ADDED Requirements

### Requirement: Frozen domain evidence obligations

FlowGuard SHALL 在收集前从受保护策略与固定图快照生成冻结义务，包含 required stages/guards/coverage、candidate/base/group、基线与规则/分析器版本。MUST 保留失败或缺失 provider 的义务，不由候选文档、返回子集或模型声明缩减必查范围。专业领域检查 SHALL 仍由各 Guard 执行。

#### Scenario: Required provider is missing

- **WHEN** 冻结义务要求 TestGuard 但未提供有效 TestGuard 证据
- **THEN** 该义务保持缺失并阻断门禁，不从计划删除或用 CodeGuard 成功替代

#### Scenario: Candidate weakens its policy copy

- **WHEN** 候选删除强制阶段或降低必需覆盖但保护策略未变
- **THEN** 原冻结义务仍适用，候选改弱不产生放行

### Requirement: Scope-bound specialist evidence validation

FlowGuard SHALL 校验专业 envelope 的版本/能力、完整调用绑定、工件摘要、声明/所需覆盖和生产者认证；MUST 区分 unsigned 重算一致性与来源信任。当前 `guard.partme.ai/v1alpha1` 字段 SHALL 不被扩展，未支持的协议/CodeGuard 原生 profile MUST 明确拒绝或通过经验证的显式适配消费。

#### Scenario: Reject old candidate or partial evidence

- **WHEN** 报告对应旧 base/candidate/group 或必需覆盖为 partial
- **THEN** 该报告不能满足当前完整义务，门禁不得因报告可重算而放行

#### Scenario: Preserve native CodeGuard compatibility

- **WHEN** 消费 CodeGuard adapter 输出及原生报告
- **THEN** 按声明的命令/flags/schema/profile 判定能力，保留原始退出码和工件，不把原生数字 2 静默解释成通用 BLOCK

### Requirement: Independent gate report and immutable upstream verdicts

FlowGuard SHALL 通过版本化领域投影将阶段义务观察映射到自己的受保护精确关系合同，生成自己的 GuardReport；每个 engine-backed completed GuardRunEnvelope.decision MUST 等于其引用的 FlowGuard 报告 decision。上游报告 MUST 原样保留。GateDecision 动作资格 SHALL 经 `artifacts.domain` 引用，授权由外部控制器另行判断，不新增共享字段或改写专业 verdict。

#### Scenario: Approval satisfies a review obligation without rewriting evidence

- **WHEN** 上游专业报告为 REQUIRE_APPROVAL 且可信批准随后满足其确切范围，其他流程义务均有效
- **THEN** 原专业报告字节及 decision 不变，新的 FlowGuard 运行可生成自身 ALLOW 报告与匹配的 envelope，仍不授予执行权

#### Scenario: Approval cannot waive a block or an unmapped finding

- **WHEN** 上游存在 enforce/BLOCK、未完成分析，或本域强制缺口没有可靠投影
- **THEN** 批准不能将其表示为已满足；缺口须 BLOCK/partial/error 或显式未支持，不得因空 facts 产生 ALLOW

### Requirement: Binding-aware error and decision transport

FlowGuard SHALL 只在完整 binding、producer 和 frozen coverage 建立后输出 `guard.integration/v1alpha1` 信封。前绑定失败 MUST 使用独立诊断且无信封；后绑定工具错误/验证错误 SHALL 为 error/null decision，取消为 cancelled/null。有效 partial 事实 SHALL 保持 BLOCK/INDETERMINATE；完整只缺审批 SHALL REQUIRE_APPROVAL，不能将运行故障伪装为业务决策。

#### Scenario: Candidate binding fails before a run exists

- **WHEN** repo/task/candidate/base 不明确或保护策略无法读取以冻结范围
- **THEN** 返回失败诊断与 CLI 4，不伪造 OID、空 binding 或 GuardRunEnvelope

#### Scenario: Bound attempt times out or is cancelled

- **WHEN** 完整绑定后的 provider 超时或调用被主动取消
- **THEN** 分别产生 error/null 或 cancelled/null，目标 CLI 4，已收集工件只保留诊断用途；强制终止不保证输出

### Requirement: Read-only interfaces and separated execution authority

FlowGuard SHALL 为 discover/status/evidence verify/gate check 发布独立版本化输入输出合同；gate check MUST 使用 0/2/3/4 对应自身 ALLOW/BLOCK/REQUIRE_APPROVAL/error。MCP/Hook/API SHALL 复用同一只读能力和绑定校验，MUST 不暴露审批签发、任意 shell、Git 写入或发布；check 不隐式修改阶段文档。

#### Scenario: Local gate returns ALLOW

- **WHEN** 本地 gate check 输出自身 ALLOW 和退出 0
- **THEN** 输出明确其范围与证据，既不表示下游所有报告 ALLOW，也不执行或授权合并/发布

#### Scenario: Host requests an unauthorized mutation

- **WHEN** MCP/Hook 请求 self-approve、任意命令执行或隐式写阶段文档
- **THEN** 接口拒绝该能力，不能借由检查入口绕过独立授权
