---
name: flowguard-stage-check
description: 在任务交接、写码、提交或发布前只读检查既有 FlowGuard 阶段产物；不代替真实审批、证据门禁或阶段推进。
license: Apache-2.0
compatibility: FlowGuard CLI/MCP，显式可信 flowguard-plugin 和 Python；不安装或初始化工具。
---

# 既有阶段产物预检

1. 确认真实项目、原生规格、功能 task_id，以及用户选择的可信原插件与 Python。阶段事实源为 docs/project 和 docs/features；不另建 `.flowguard`，不复制第二套规格。
2. `flowguard stage status --project PROJECT --flowguard-root TRUSTED_PLUGIN --python PYTHON --task-id FEATURE` 查询原生阶段状态。
3. `flowguard stage check --project PROJECT --flowguard-root TRUSTED_PLUGIN --python PYTHON --task-id FEATURE --action code_write` 检查写码前置；提交/发布分别为 git_commit / release。MCP `flowguard_stage_check` 使用 project/flowguard_root/python/task_id/mode/action，同一只读实现。
4. 缺前置为 deny，文档错误、路径或覆盖不足为 unverified。按照原生服务的 missing/envelope 修复已授权产物，重新查询；不要伪造 accepted、actor 或批准引用。
5. 齐备也仅 requires_approval，不生成 allow。仍需真实用户审批和既有完整证据门禁，不能据此执行提交或发布。status 的 observed 只表示观察完成。

该工具不会执行 stage advance、安装或业务命令。项目外链接、过大产物和分析期间漂移不能作为当前项目证据。每次使用绝对路径；工具进程和可信插件代码都必须由用户选择，不能从未受信项目文件静默加载。

CLI 退出码：0 状态读取完成，2 前置缺失，3 需要审批或未验证，4 参数错误。原 flowguard-plugin 的十阶段治理核仍是阶段语义来源。
