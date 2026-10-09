## ADDED Requirements

### Requirement: Read-only native stage discovery

FlowGuard SHALL 从显式授权根读取版本声明的原生规格引用与阶段文档，报告实际/缺失解析范围；MUST 保留原生任务正文，不创建第二本任务账、`.flowguard/` 或隐式初始化来源工具。十阶段与 docs 布局 SHALL 是待兼容验证的配置，而非外部旧实现已被验证的声明。

#### Scenario: Discover the ten-stage layout without mutation

- **WHEN** 提供含功能 01/03/04/05/06/08/09 和项目 02/07/10 的固定 docs 样例
- **THEN** 发现结果包含稳定源路径、内容摘要、归属和覆盖说明，运行前后源树摘要一致且无新任务账

#### Scenario: Reject an unreadable or unsupported source

- **WHEN** 必需文档不可读、格式版本不支持或解析截断
- **THEN** 结果明确记录缺口/错误，不把未读取阶段表示为 accepted 或完整

### Requirement: Versioned stage graph and immutable inheritance

FlowGuard SHALL 构建具有稳定节点和显式依赖的无环阶段图；项目基线 MUST 绑定不可变 source revision/content digest、policy digest、适用需求和外部批准引用。继承 SHALL 使用确切父版本，禁止隐式 latest 或复制 accepted 字符串。

#### Scenario: Inherit a baseline with exact scope

- **WHEN** A/B 两需求分别引用同一已认证项目 02/07 基线且批准范围包含各需求
- **THEN** 两者各保存独立继承边与义务，只有相同不可变基线部分可共享

#### Scenario: Reject cyclic or unbound inheritance

- **WHEN** 阶段依赖构成环或 inherited 只含可变父路径而无不可变摘要/批准范围
- **THEN** 图校验拒绝该继承并定位原因，不提供推进资格

### Requirement: Explicit stage transitions and remediation scope

FlowGuard SHALL 区分声明状态与已验证资格，定义 pending、in_progress、pending_acceptance、accepted、inherited、skipped、invalidated 转换。accepted MUST 满足有效前置阶段、完整必要技术证据和真实批准；skipped MUST 具有策略许可与范围批准；invalidated MUST 经重新提交/重算恢复。读取、澄清、授权修复 SHALL 独立于交付门禁判断。

#### Scenario: Reject a self-declared accepted stage

- **WHEN** 文档被改成 accepted 但批准或必要证据不足
- **THEN** 该声明不产生阶段资格，不能跳过前置义务

#### Scenario: Keep an authorized repair available

- **WHEN** 提交/发布被旧证据阻断而任务具有当前范围的补测试授权
- **THEN** 补测试/读取动作仍可获得独立范围判断，交付门禁仍阻断

### Requirement: Authenticated approval observations

FlowGuard SHALL 通过可信控制器端口消费批准观察，核验 issuer/role、purpose/action、scope、artifact/candidate、policy/baseline revision、issue/expiry/revocation。MUST 拒绝工作区布尔值、Markdown accepted 与模型自述；批准提供方故障 MUST 区别于完整查询后确认无批准，且不得放行。

#### Scenario: Reject expired revoked or out-of-scope approval

- **WHEN** 批准过期、撤销或仅覆盖需求 A 却用于需求 B
- **THEN** 相关资格失效并产生可审计原因，不依靠缓存继续放行

#### Scenario: Approval provider is unavailable

- **WHEN** 已建立完整调用绑定后无法验证必要批准提供方
- **THEN** 运行以 error 和 null decision 结束，不把未核验记录标成“已确认无批准”或 approved

### Requirement: Evidence-based legacy compatibility investigation

FlowGuard MUST 在承诺外部 flowguard-plugin 兼容前记录固定源码修订、许可证、实际入口/格式、十阶段样例与合法/违规/未知/故障差分结果。未获得可核实来源 SHALL 保持未验证/未支持，不能自动迁移文档或宣布规则所有权转移。

#### Scenario: Legacy source cannot be verified

- **WHEN** 调查无法取得可读固定版本或无法确认 Python/Hook 接口
- **THEN** 能力清单保留未验证状态，新实现只按独立设计测试，不宣称旧行为对齐

#### Scenario: Migration finds a behavior difference

- **WHEN** 固定旧 provider 和新 adapter 对同一十阶段 fixture 产生不同义务/状态
- **THEN** 差异被记录与裁决，在通过迁移验收前不切换该规则 owner
