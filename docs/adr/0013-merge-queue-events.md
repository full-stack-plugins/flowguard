# Local read-only merge-queue event adapter

The adapter consumes a `ControllerEventPort` and real GitGuard `CandidateSnapshot`; it does not compose commits, run a merge executor, change refs/index/source, issue grants or release anything. `ProtectedQueue` freezes the expected principal, queue/repo/task/worktree identity, native GitGuard policy digest and canonical allowed paths before evidence arrives.

Only `ControllerProfile::Fixture` is implemented. The port owns its current event registry independently of caller-supplied candidate data. A caller's matching text/hash is not production authentication. A deployment requiring real issuer authentication remains unsupported; the fixture cannot be promoted by changing an enum or trusting an event JSON file. No production identity platform is added.

Each event binds exact candidate/base/group, ordered distinct member OIDs and the canonical union of each member's requirement IDs. The adapter first bounds input and checks independent issuer/scope/reference, then checks event sequence/validity and compares the actual native candidate. GitGuard validates real objects and immutable source bytes, while FlowGuard bind also rejects dirty candidate/worktree mismatch. The adapter does not reproduce GitGuard merge logic. Event composition correctness remains the controller's responsibility.

A `QueueSession` is exclusive mutable process-local state. Protected time advances even on subsequent authority/input failure. Correctly scoped higher event sequences retire earlier events before expiry/candidate errors; a failed new event cannot revive an old observation. Equal-sequence event equivocation poisons that sequence until a strictly newer event arrives. `revalidate` refreshes the port, clock and native candidate, then compares the old exact event/binding; old observations cannot silently become a new queue run. No durability, multi-process replay prevention or authority across a session restart is claimed.

Bounds: 64 members, 4096 member requirement entries in aggregate, 256-byte textual fields, 64KiB aggregate text/path input, 256 allowed paths each at most 4096 bytes. Existing native candidate budgets apply before object reads. Provider allocations and latency remain the port's responsibility.

The tests construct real base/A/B commits and synthetic M1, then advance the base and reverse member order to form M2 in external fixture setup. Each read-only adapter window compares refs, HEAD, index and tracked source/status. Exact M2 passes; head/M1/wrong member order/requirements and old M1 specialist evidence fail. The specialist evidence used to isolate the queue binding contract is explicitly a local test producer, not a newly generated five-provider production matrix. Separate accepted DAG tests cover the captured real five-provider corpus.

The original uncompiled API draft remains under docs/drafts without being presented as validation. The local fixture slice has passed independent review, including real Git M1/M2 checks. Task 4.5 remains unchecked because the enum accepts only Fixture/Unsupported and no authenticated external-controller adapter is implemented. This is a concrete code and input-source gap, not a requirement to deploy production before testing. Task 5.4 separately requires actual protected-host evidence.


## 接入一个真实只读控制器所需的输入与剩余代码

已接受的本地能力包括：冻结 queue/repo/task/worktree/policy/path，精确候选与成员需求并集比较，sequence 高水位、同序歧义、时钟及失效重查。联合验收另已生成 M2 对应的真实五守卫工件；它没有给 fixture 事件增加身份认证。

| 需要给定的外部输入 | 最小可用内容 | 缺失时仍不能完成的具体代码/验证 |
| --- | --- | --- |
| 控制器种类与只读契约 | 指定一个事件/API版本、端点或进程通道；说明 opaque reference 如何取“当前事件”，给出脱敏的M1/M2真实消息样本 | 无法实现具体 response decoder、current(reference) 获取及错误映射；目前没有该 adapter，不能以读取任意事件 JSON 代替 |
| 信任根与会话归属 | 已认证传输/session或issuer验证材料的提供接口，以及明确的预期 principal；测试环境即可，候选侧不得提供这些材料 | 无法把返回事件与独立认证记录绑定；新增 Authenticated 枚举/布尔值不构成认证。不能由 FG 发明 signer、凭据或身份平台 |
| 事件内容绑定规则 | reference、issuer身份、完整event digest与当前返回body的绑定方法；说明认证覆盖哪些字段 | 无法证明认证的事件就是被消费的候选/base/group/members；仅核对principal字符串或一个候选自报digest不足 |
| 受保护范围映射 | tenant/repo/queue允许值；事件到task/worktree及每个member requirementIds的可信映射；policy/path配置由谁提供 | 无法区分合法事件与跨租户/跨仓/跨任务事件，也无法从真实成员生成精确需求并集；现有 fixture 映射不是默认生产映射 |
| 时序与撤销来源 | sequence是否单调、同序更新语义、可信UTC时钟、有效期、当前撤销查询/不可用状态；重启是否必须保持高水位 | 无法决定真实序号/过期/撤销字段映射或重启行为；已有 process-local 高水位不是持久多进程防重放承诺 |
| 一次可授权的只读集成条件 | 可访问的测试通道、精确M1/M2对象/refs只读权限，以及错误身份、旧事件、撤销/不可用的可重现观察 | 无法验证非fixture认证路径；纯fixture测试不能证明该外部issuer接线正确。这里不要求 merge/write token 或生产上线 |

最小实现范围是一个所选 controller adapter 和其受保护配置，不是通用身份平台：

1. `ControllerEventPort::current` 从选定可信来源自行取得事件及独立认证观察；不接受候选提供的事件 body 或“已验证”字段作为替代。复用已有 CallerPort/GE 外部观察边界时，仍必须绑定实际 reference/body，而非仅复用一个身份字符串。
2. 在 `QueueSession::observe` 的 fixture-only 分支之外接入这个已认证端口；先核验真实issuer、内容绑定与受保护scope，再允许推进 high_water。保留 fixture 的显式用途及不支持来源的拒绝行为。
3. 复用已有候选比较及 `revalidate`，每次消费重新取当前事件/撤销状态；在新适配器上验证错误issuer、替换body/reference、跨scope、旧/同序事件、过期、撤销和来源不可用。没有认证结果时保持拒绝，不能仅将错误转换为 fixture 记录。

上述三项当前没有实现；现有 trait 是扩展点，不能被描述为已有真实 connector。选型/信任材料尚未给定使该具体接线无法确定，不代表所有本地代码都属于外部阻塞。确认某个只读测试来源后，可以只实现和验证这一种适配器；宿主 required-check 的关闭hook/弱化策略/撤销后合入测试仍属于独立的5.4范围。

## 2026-10-09 GitHub 只读前提核验

根线程于 `2026-10-09T16:38:24.954659+00:00` 对
`full-stack-plugins/flowguard` 完成一次只读检查，原始结果保存在
`/workspace/guard-implementation-ledger/flowguard-host-readonly-prerequisites.json`：

- `gh api repos/full-stack-plugins/flowguard` 成功：default_branch 为 `main`，`allow_auto_merge=false`；返回的 repository permissions 包含 admin/pull 等 true。这不证明当前 integration 能读取所有保护配置，也不构成远端写授权。
- `gh api repos/full-stack-plugins/flowguard/rulesets` 成功并返回 `[]`。
- `gh api repos/full-stack-plugins/flowguard/branches/main/protection` 返回 HTTP 403：`Resource not accessible by integration`。

因此已确认仓库读取可用，但 main 分支保护配置不可读；不能由空 rulesets
断言 main 没有分支保护，也不能由 `allow_auto_merge=false` 推断队列配置或
required-check 的实际强制效果。本次未取得真实 authenticated M1/M2 事件，
未执行保护绕过实验，也未修改远端任何配置、分支或检查。

后续具体需要补充：让所用 integration 能只读查看目标分支保护及 queue 配置的
授权，以及选定测试 controller、事件版本和事件到 repo/queue/task/worktree/
requirements 的受保护映射。可通过连接权限或受控会话提供，无需发送原始 token。
在这些条件具备前，真实事件适配与保护宿主验证仍未完成；当前禁止远端写的范围
保持不变，不自动启用队列、修改保护规则或尝试合入。
