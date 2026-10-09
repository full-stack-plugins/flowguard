## Why

FlowGuard 的原始 main `a6bd25fc38b0a323f880bedd395373d3bb672826` 只有四份 Markdown；本次规划前已复查文档分支 `de60563f9d80f5ad0ef835191ae907c469c04e38`，新增共享设计也没有带来运行时、Cargo manifest、测试或既有 OpenSpec change。阶段图、可信审批、证据组合与并行候选隔离均未实现，不能把设计中的 accepted 或 ALLOW 当作现有授权能力。

本变更把已审阅[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)及[集成契约草案](../../../docs/integration-contract.md)分解成可实施的增量要求。外部 flowguard-plugin、Python flowguard_lib、历史十阶段行为与 Hooks 均未检查，兼容性首先是调查任务，不是既有事实。

## What Changes

- 定义十阶段/docs 路径的候选兼容配置、只读来源适配、稳定需求绑定、阶段 DAG、不可变基线继承和经认证的批准引用。
- 执行前冻结义务，消费各专业守卫证据；FlowGuard 只把本域阶段义务投影成自己的 GuardFacts，使用自己的受保护规则生成独立 GuardReport。
- 明确上游报告不可变：批准后专业 REQUIRE_APPROVAL 仍保持原值；新的 FlowGuard 门禁评估可判定其待审义务满足。自身 envelope.decision 等于自身报告，动作资格在领域附件，执行授权始终外部。
- 建立前绑定诊断、完整绑定后的 error/cancelled 信封、技术决策/阶段状态/执行授权分离，以及有限只读 CLI/MCP/Hook 适配。
- 对并行需求、共享基线、晚到运行、审批过期/撤销和精确合并队列候选实现隔离、失效传播、CAS 和审计；不执行 Git 合并或发布。
- 通过源版本调查、差分样例、模拟合同、真实宿主与影子运行逐步验收；未实现或未核实能力保持阻断/未支持，不静默换权威。

## Capabilities

### New Capabilities

- `workflow-stage-baselines`: 原生来源、十阶段图、基线继承和审批引用的领域约束。
- `workflow-evidence-gates`: 冻结义务、专业证据消费、自身引擎报告、错误传输与只读接口。
- `workflow-run-isolation`: 并行隔离、失效、精确队列候选、受控审计与迁移验收。

### Modified Capabilities

无。本仓无既有 OpenSpec capability；本 change 不修改 GuardEngine 现行协议、其他守卫实现或外部旧插件。

## Impact

实施阶段拟新增 `src/{context,stage,dependencies,baseline,approvals,obligations,evidence,projection,gate,run_store,cli}.rs`、版本化领域 schema、适配器和 fixtures；这些路径当前不存在。本次只创建规划文件，不创建产品代码、安装依赖或执行实现测试。

FlowGuard 拥有阶段语义；专业解析/规则仍归对应 Guard；GuardEngine 只拥有通用 Rule/Contract/Evidence；审批身份、信任根与写执行归可信控制器。保留 `guard.partme.ai/v1alpha1`，集成信封 `guard.integration/v1alpha1` 是独立草案，不注入当前严格字段。

## Dependencies and Delivery Gates

遵循[共享路线图](../../guard-roadmap.md)，这些是阶段门槛而非整个仓库互等：

| 本变更阶段 | 依赖门槛 | 可提前进行的工作 |
|---|---|---|
| 任务组 1 调查/发现 | 无跨仓阻塞 | 固定 legacy 调查、只读 docs fixture、领域 schema |
| 任务组 2 图/基线 | SG-BASELINE 用于真实需求与义务导入 | 本地图、状态机与模拟基线先行 |
| 任务组 3 证据组合 | GE-CONTRACT、GE-ADAPTER；SG-BASELINE、GG-CANDIDATE 与策略要求的 AG-EVIDENCE/CG-ADAPTER/TG-EVIDENCE | 本域投影 fixture 可先设计，缺真实 provider 不称集成通过 |
| 任务组 4 信任/队列 | GE-TRUST + 上述所需证据，达到 FG-GATE | 受控假批准仅用于测试，无生产授权 |
| 任务组 5 接口/推广 | FG-GATE 后 END-TO-END；生产独立分发等待 GE-RELEASE | 基于固定源修订开发，不必等公开包 |

依赖方向是 GitGuard **只读** candidate/binding → FlowGuard 门禁 → 外部控制器/可选 GitGuard 写执行器；FlowGuard 从不等待后者完成才评估，禁止循环依赖。代码/测试/架构证据仅在冻结策略要求其义务时必需，不能暗中省略必需 provider。

跨仓计划：[引擎](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)、[规格](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis)、[架构](https://github.com/full-stack-plugins/archguard/tree/docs/guard-design-20261009/openspec/changes/extend-architecture-analysis-and-evidence)、[代码适配](https://github.com/full-stack-plugins/codeguard/tree/docs/guard-design-20261009/openspec/changes/add-guardengine-compatibility-adapter)、[测试](https://github.com/full-stack-plugins/testguard/tree/docs/guard-design-20261009/openspec/changes/add-test-obligation-evidence-pipeline)、[Git 候选](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance)。

## Non-goals and Open Decisions

不重建原生任务正文，不运行文档指令，不默认外部写入，不改写 CodeGuard 原生退出码，不实现专业守卫算法或审批签发。待决定 legacy 可读版本/许可证、首批原生规格适配格式、审批后端与刷新时限、领域摘要编码、CLI 参数、持久化/保留策略与首个真实宿主。默认只读、无自动跳过/继承、无自动合并；缺资料不能承诺兼容。
