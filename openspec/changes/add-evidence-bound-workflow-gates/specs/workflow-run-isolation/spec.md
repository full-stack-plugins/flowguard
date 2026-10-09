## ADDED Requirements

### Requirement: Immutable attempt isolation and conditional publication

FlowGuard SHALL 按 repo/task/worktree/requirementIds/candidate/base/group/source/baseline/contract/analyzer/config/coverage/action 固定工作绑定，给每次重试新的 attempt/run 身份，幂等键与 runId 分离。结果 MUST 追加且当前指针按完整绑定和 generation 条件更新；不同需求不共享可覆盖的 latest 结果。

#### Scenario: Late result arrives after a newer candidate

- **WHEN** A/head1 的 ALLOW 在 A/head2 的 BLOCK 之后到达，B 同时独立运行
- **THEN** 旧结果只写原绑定历史，不覆盖 A/head2 或 B 的状态

#### Scenario: Duplicate request conflicts with a changed payload

- **WHEN** 相同幂等键被用于不同输入摘要
- **THEN** 系统拒绝冲突；相同请求的重交可去重，真正重新执行创建新 runId

### Requirement: Dependency-aware eligibility invalidation

FlowGuard SHALL 在 candidate/base/group、源内容、规则、分析器/配置/覆盖、基线、需求依赖或批准过期/撤销变化时重评资格，并沿显式依赖边传播。MUST 保留原始工件和终态，不修改旧报告。每次消费 SHALL 重新核验批准新鲜度，不能由缓存豁免撤销。

#### Scenario: Shared project baseline changes

- **WHEN** 02/07 的固定基线被新修订替代且 A/B 都依赖旧版本
- **THEN** 两需求相关继承及下游资格失效，新版本必须显式绑定与重验，不能读取 latest 后自动继承

#### Scenario: Release dependency or approval changes

- **WHEN** 发布 10 引用的功能 09 内容变化，或其必要批准被撤销
- **THEN** 该发布资格失效并记录因果路径，不影响无依赖的其他发布记录

### Requirement: Exact merge-queue composition without executor dependency

FlowGuard SHALL 消费可信控制器及 GitGuard 只读 GG-CANDIDATE 的精确候选/基底/合并组，并对该对象冻结、验证所需专业证据。MUST 拒绝用 PR head 证据替代合并组义务；队列重组要求新运行。FG-GATE SHALL 不依赖 GitGuard privileged executor 的完成；可选写端仅在外部授权后消费 FlowGuard 输出。

#### Scenario: Queue membership or base changes

- **WHEN** 分支 head 不变但合并组成员重排或 base 推进产生候选 M2
- **THEN** M1/分支证据不能满足 M2 门禁，可信控制器安排精确 M2 重算

#### Scenario: Read-only candidate is available without write executor

- **WHEN** GG-CANDIDATE 与必需专业证据已具备而 Git 写端未部署
- **THEN** FlowGuard 可输出其局部技术报告；不等待写端或执行写操作，外部动作保持未授权/未执行

### Requirement: Bounded access and auditable stage evidence

FlowGuard SHALL 限制授权根、symlink/路径遍历、输入大小、图规模/深度、解析时间与允许工件存储，MUST 不执行源文档指令或不可信策略脚本。审计 SHALL 记录绑定、摘要、规则/分析器、外部批准引用、失效原因和控制器时间，脱敏并实施租户权限与明确保留策略；候选不得持有批准或合并凭据。

#### Scenario: Input escapes the allowed scope

- **WHEN** 文档或工件引用使用越界 symlink、任意可执行 URI，或超出资源预算
- **THEN** 读取被拒绝/中止并留下有限脱敏诊断，不把被跳过的来源声明为 complete

#### Scenario: Audit storage expires a required artifact

- **WHEN** 保留策略删除了门禁仍需核验的工件或记录不可读取
- **THEN** 该证据失去消费资格并要求重验，不能仅凭历史绿色摘要放行

### Requirement: Phased validation migration and reversible rollout

FlowGuard SHALL 按调查/图基线、证据投影、信任并发、真实宿主顺序记录验收；模拟合同 MUST 与真实跨守卫/宿主证据区分。强制推广 SHALL 通过合法/违规/partial/故障、两并行需求、精确队列、过期/撤销、漂移、晚结果、CodeGuard 原生兼容和回滚演练。MUST 保持单一规则 owner，回滚保留历史且不改写 Git refs。

#### Scenario: Shadow comparison contains unresolved differences

- **WHEN** 旧新 provider 或模拟与真实宿主结果存在未裁决差异
- **THEN** 保持 advisory/shadow 或禁用能力，不宣布迁移完成或服务端门禁已强制

#### Scenario: Roll back the new gate integration

- **WHEN** opt-in rollout 触发回滚
- **THEN** 控制器停用新适配路由并恢复已验证旧 owner 或阻断能力，保留不可变证据/撤销历史，不改 CodeGuard 原生命令或回退 Git refs
