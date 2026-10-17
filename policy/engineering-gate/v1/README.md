# FlowGuard 工程动作政策包 v1

对应正式事实源：CodeGuard `add-engineering-guard-core` 的 EG-K08、EG-GATE-001/005；为 EG-L02/W03 提供 FlowGuard 自有的组合策略。包版本 `1.0.0` 是候选协议版本，不表示完整产品验收或已发布。既有十阶段 Python provider、`stage status|check`、签名 evidence check 不改入口或旧 wire。

固定查询为 `data.flowguard.gate.decision`。`manifest.json` 只装载生产 `gate.rego`，测试模块不进入发布包；能力文件在执行器私有 bundle 之外。所有原始文件固定 LF，摘要为 SHA-256。**整个包身份只取 GuardEngine `PolicyPackage::digest()`**（JCS manifest 的既定公式）；这里不维护第二个包摘要公式。更新 Rego 或 capabilities 后必须更新 manifest 原件摘要，重新获得相应配置批准。

## 安全边界

Rego 不能从普通 JSON 判断签名、角色或来源是否可信。调用端必须从已批准 CompiledContract、当前 RegisteredPlan、当前运行台账、独立 Guard 结果和批准核验端口构造 facts，尤其是 `authorityContext`。这里的 verified 引用和 enforcementVerified 是**外部核验的输入端口**，不是项目文件、模型、CLI 参数或调用者自报即可取得的权限。直接运行本包可复现决策函数；不能证明输入真实，也不能取得授权。本包不签发 GateDecision/ExecutionGrant、不推进阶段、不写任务状态。

K09 的真实批准提供者和撤销来源、L01 原生阶段完整适配、W03 受保护工作流接线、真实托管平台绕过验收仍需独立落地；本包测试通过不关闭这些任务。原生文档 `accepted` 不是技术检查，也不是独立批准。

## 闭合输入

顶层为通用 `guard.policy-input/v1`：`schemaVersion`、`binding`、`facts`，不接受其他字段。binding 包含 repositoryId、taskId、subjectDigest、contractDigest、policyDigest、planDigest、action，由通用执行器依据当前计划构造。四种摘要必须为小写 `sha256:` 加 64 位十六进制。

facts 字段固定如下。所有对象拒绝未知字段；数组最多 4096 项；标识字符串非空且最多 4096 Unicode code points；时间为 0..2^53−1 范围内整数秒。原始 JSON 重复键由通用执行器拒绝（OPA 解析之后已无法恢复重复键，不能假称 Rego 本身检测）。协议层字节和资源上限由通用执行器继续执行。

| 字段 | 约束 |
| --- | --- |
| schemaVersion | 数值 `1` |
| mode | `advisory`、`cooperative`、`enforced`；与规则处置方式分离 |
| action/subjectDigest/contractDigest | 必须与 binding 精确相同 |
| nowEpochSeconds | 由控制端提供的核验时刻，禁止网络/系统时钟 builtin |
| obligations | 非空；对象为 obligationId、ruleId、verifierId、target、required(bool)、mode、nonWaivable(bool)；obligationId 唯一 |
| results | resultId、obligationId、ruleId、verifierId、target、subjectDigest、contractDigest、policyDigest、planDigest、execution、coverage、verdict、findings；resultId 唯一 |
| stages | stageId、state、approvalId；stageId 为 `01`..`10` 且唯一 |
| approvals | id、kind、actor、requestedBy、action、subjectDigest、contractDigest、scope、notBefore、expiresAt、revoked；id 唯一 |
| exceptions | id、obligationId、ruleId、target、findingId、approvalId、reason；各字符串非空、id 唯一 |
| authorityContext | verifiedApprovalIds、verifiedResultIds、verifiedContractDigest、verifiedSubjectDigest、enforcementVerified；摘要绑定本次契约/候选 |

规则 mode 是批准的 `ENFORCE`、`REVIEW`、`ADVISE`。规则及 required/nonWaivable 必须从批准契约投影，不能从待检查源码或接受文档中选择。同 ruleId 在不同目标中的 mode/nonWaivable/verifierId 冲突会拒绝求值为合法输入。结果 execution 为 SUCCEEDED/FAILED/NOT_RUN/TIMED_OUT/CANCELLED，coverage 为 COMPLETE/PARTIAL/NONE，verdict 为 SATISFIED/VIOLATED/UNKNOWN/NOT_APPLICABLE。finding 固定 `{findingId,message}`，ID 在该结果内唯一。当前最小 profile **不接受无适用性证明的 NOT_APPLICABLE 抵消必需检查**，返回缺证据；未来要支持时需有独立适用性事实扩展，不能默认放行。

只有独立 verifiedResultIds 中、全部身份与精确 target 匹配、同义务唯一的结果才可覆盖义务。调用端必须先核实 current run/sequence、生产者和工具 pin、新鲜度、源文件/日志/证据原件和摘要；不能仅验证 JSON 可解析就填 verifiedResultIds。本包不重新运行各专业检查算法。

技术完整性要求 SUCCEEDED + COMPLETE，以及 SATISFIED 且无 finding，或 VIOLATED 且有 finding。其他组合保留已确认阳性并报告缺口。多个当前结果不能让旧 PASS 覆盖新 FAIL：当前最小 profile 保守返回 incomplete，由上游台账选择唯一 current 结果。

**这里的 `execution=SUCCEEDED` 表示独立领域适配器已经证明验证完成，不表示 OS exitCode=0，也不是公共 GuardResult.ExecutionStatus 的直接重命名。** 例如 ArchGuard 本身可能正常完成检查并以 exit2 表示违规；其公共 execution 仍可能为 FAILED。未来 L02 的正式 normalizer 只能在核验已 pin 生产者的本次 raw 报告 `completeness=complete`、明确声明的正常违规退出码、typed VIOLATION/COMPLETE、阳性 finding、当前运行原件及完整身份后投影 SUCCEEDED；仍须保留原 FAILED/exit2 原件，不能改写公共结果。单凭 FAILED+exit2、coverage=COMPLETE 或模型声称完成，均不得推导成功。真正工具失败继续投影 FAILED，哪怕保留阳性和完整覆盖，仍缺必需技术完成证据；有效例外也不能消除该缺口。

本仓本批**没有**生产 GuardResult→facts normalizer，因此测试只从显式独立核验 facts 开始；没有用受控 raw 结构冒充实际本次工具原件，也不声明已打通公共 GuardResult 消费闭环。该映射、真实 exit2 端到端例外和真正工具失败的对照仍是 L02 的具体验收义务。

## 阶段、批准和精确例外

阶段状态保持既有 pending/in_progress/pending_acceptance/accepted/inherited/skipped/invalidated。COMMIT、PUSH、MERGE 需要 01—09；RELEASE、DEPLOY 需要 01—10；分支创建/切换不额外要求阶段，但仍执行契约义务。原 provider 的 code_write/安全读取/TDD 前置动作不在当前核心七种 Action 范围，本包不暗中重定义它们。

accepted/inherited/skipped 均需要独立、未撤销且时间有效的 stage 批准。stage scope 精确为 `{stageId,state}`，subjectDigest 为 null，绑定批准契约与动作，可在需求范围未变且重新取得新鲜技术结果时复用。继承/跳过本身不是批准；原生来源链、继承关系和失效传播应在 L01 适配端核验后才提供 stage 批准引用。本包不以文档字符串代替来源核验。

MERGE、RELEASE、DEPLOY 还需要 candidate-specific action 批准，scope 精确为 `{action}`，subjectDigest 必须为当前候选。异常批准 kind=exception，scope 精确为 `{obligationId,ruleId,target,findingId}`，并绑定当前候选、契约、动作、批准人、原因、有效起止和撤销。批准 actor 不得等于 requestedBy；真实角色独立性仍由 K09 核验。notBefore 包含边界、expiresAt 不包含边界。

不可豁免规则的任何阳性始终 DENY，不能由 ADVISE、例外或缺技术覆盖降低。允许例外仅改变对应 finding 的处置，原 finding 始终留在 activeViolations 并标注 waived=true。不能用豁免消掉 PARTIAL、未执行、超时、UNKNOWN 或缺必需 provider；不能以策略/测试清单自改降低批准基线。

## 确定性输出和优先级

输出固定六字段：`schemaVersion:1`、decision、reasons[]、requiredActions[]、missingObligations[]、activeViolations[]。集合排序保证稳定输出；activeViolations 每项为 obligationId/ruleId/target/findingId/message/mode/nonWaivable/waived。原件日志和完整 finding 由调用端保留，这里是组合投影。

优先级：已确认 ENFORCE 未豁免或不可豁免违规 → DENY；否则无效输入、技术缺口、缺阶段或 enforced 缺宿主证明 → INCOMPLETE；否则 REVIEW 违规或待批准 → REVIEW_REQUIRED；否则有有效精确例外 → ALLOW_WITH_EXCEPTIONS；否则 ALLOW。即使优先 DENY，也保留 missingObligations 和全部原因。ADVISE finding 可伴随 ALLOW，但不能改写成专业检查通过。运行模式仅说明部署能力，不抹去规范性违规；ALLOW 不表示本地调用已经强制执行动作。

## 真实验证

使用已安装 OPA **1.21.2**；没有自动下载、安装或全局配置。

```text
python -B tests/policy_gate.py --opa <OPA绝对路径> --scratch <项目外现有临时目录>
```

该命令校验版本、manifest 原件摘要、LF、纯 builtin 能力限制，再运行真实 `opa test` 和独立决策表的真实 `opa eval`。独立表在 `tests/policy_gate_cases.py`，期望显式维护，未以当前输出反推。能力包来自官方该版本 `opa capabilities --current`，只允许固定 39 个纯 builtin，allow_net=[]，禁止 http.send、time、rand、uuid、io、外部读取。测试辅助 object.union 等不进入生产 manifest，不扩运行能力。

本机 Windows/amd64 已实际执行 39 个 Rego 测试和 28 个独立表 eval；Unix 与受保护 CI/真实平台集成不能由此冒认。无定义/工具失败/超时的收尾与原始输出保存在通用 guard-policy 执行器测试，完整组合接线需双方再验。
