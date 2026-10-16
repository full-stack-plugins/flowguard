---
name: flowguard-evidence-check
description: 以显式外部受信公钥和任务策略只读核验当前 TestGuard 执行证据；技术通过仍需真实审批。
license: Apache-2.0
compatibility: FlowGuard CLI/MCP；不执行测试、不推进阶段、不安装工具。
---

# 受信执行证据核验

1. 读取正式规格、执行计划和真实必需测试义务；由调用者明确选择项目外受控生产者的公钥。不能从候选项目、receipt.signature.public_key 或模型声明自动建立信任。
2. 准备显式 policy JSON，恰好包含 `task_id`、`plan_digest`、`required_tests`、`runner`、`max_age_seconds`。必需用例来自独立规格/冻结计划，不能仅复制回执自报集合；runner 为 cargo 或 maven，时效为 1..604800 秒。
3. `flowguard evidence check --project PROJECT_ABS --receipt RECEIPT_ABS --policy POLICY_ABS --trusted-key EXTERNAL_PUBKEY_ABS`。MCP `flowguard_evidence_check` 使用 project/receipt/policy/trusted_key，同一只读实现。
4. 失败 `unverified` 时按 error 修复源代码、真实测试、计划或证据，重新真实执行并签署，随后复验；不得重写 PASS、时间戳或签名掩盖失败。该入口不运行测试。
5. `technical_pass=true` 仅表示指定密钥签署的当前执行声明满足外部义务；decision 始终 `requires_approval`，仍需原生阶段检查和真实用户审批，不能据此发布或推进阶段。

签名需要受控 CI/生产者保护密钥、程序和工作区。本地签名不提供远程证明或操作系统沙箱；验签不等于人工批准。检查会重算当前源码快照、执行程序和报告 SHA256，拒绝旧源码、删除/改变报告、过期/未来时间、错公钥、篡改、未签名导入、伪生产者以及 PASS/非零退出码矛盾。policy.task_id 和 runner 必须匹配，required_tests 集合必须与签名声明相等且被 observed_tests 覆盖。

退出码：3 技术通过但待审批，2 未验证，4 CLI 选项语法错误。结果不生成 allow，不创建项目状态目录，不执行原插件阶段写入。
