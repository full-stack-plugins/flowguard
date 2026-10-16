# FlowGuard 只读阶段产物预检

本增量对应 `guardengine/openspec/changes/professional-guard-checks`。阶段含义与依赖仍以现有 `flowguard-plugin` 的 `flowguard_lib.stage_docs` 为事实源，不另建十阶段矩阵、审批系统或项目状态目录。

独立 Rust CLI 沿用原有 `install/uninstall/context/instructions/doctor/serve` 安装接口，新增：

```sh
flowguard stage status --project /repo --flowguard-root /trusted/flowguard-plugin --python /usr/bin/python3 --task-id refund
flowguard stage check --project /repo --flowguard-root /trusted/flowguard-plugin --python /usr/bin/python3 --task-id refund --action code_write
flowguard stage check --project /repo --flowguard-root /trusted/flowguard-plugin --python /usr/bin/python3 --task-id refund --action git_commit
flowguard stage check --project /repo --flowguard-root /trusted/flowguard-plugin --python /usr/bin/python3 --task-id refund --action release
```

路径必须是绝对路径。调用者显式选择已信任插件源码与现有 Python 可执行文件，不从项目收据或项目配置静默执行脚本。该源码属于可执行信任边界，不能指向待检查项目随意提供的脚本。

`status` 原样呈现 `stage_docs.snapshot` 的阶段状态、原生引用、批准引用与指纹；批准引用只是观测到的本地记录，不能证明真实用户授权。`check` 通过原 `missing_before` 查询所选动作的前置阶段，保留原生正文指纹和上游指纹失效逻辑。

适配层先通过原注册表和 `path_for` 定位阶段文件，拒绝文件或其祖先中的符号链接、Windows reparse point（包括 junction）、项目外路径与非普通文件。每个来源不超过 1MiB，去重后总量不超过 16MiB；动态读取的跨功能发布依赖使用相同限制。原生服务基于一次有界输入缓存读取，结束后重读 SHA256；来源变化则丢弃阶段结果并返回 `unverified`。这些是本地只读检查的边界，不构成操作系统级原子文件系统快照。

| decision | 含义 | CLI 退出码 |
|---|---|---|
| observed | 阶段状态已读取 | 0 |
| deny | 原生前置产物不满足 | 2 |
| requires_approval | 产物预检齐备，仍需要真实审批与当前证据检查 | 3 |
| unverified | 文档缺失、非法、适配器不可用或执行异常 | 3 |

参数非法时退出 4。`approval` 始终为 `required`，`quality_decision` 始终为 `not_evaluated`。此工具从不生成 `allow`，不执行 `stage advance`，不接受 `actor`、`approval_ref`、`apply` 参数。

产物就绪不能替代原插件治理服务的上下文、当前测试/静态分析/语义审查证据、发布依赖或真实用户审批。该接口是阶段产物预检，不是完整提交/发布门禁。

## MCP

启动 `flowguard serve --mcp` 后，调用 `flowguard_stage_check`：

```json
{"project":"/repo","flowguard_root":"/trusted/flowguard-plugin","python":"/usr/bin/python3","task_id":"refund","mode":"check","action":"git_commit"}
```

`mode=status` 不接受 `action`；`mode=check`（默认）要求 `action=code_write|git_commit|release`。CLI 与 MCP 共用同一验证、进程调用和结果对象。显式插件参数必须随每次调用传入；原安装接口不新增项目路径配置，也不改变已有安装记录。

## 验证

`tests/stage_check.py` 使用原插件 API 在隔离临时目录构造夹具，验证缺失→补齐→复验、非法元信息修复、正文改变后指纹失效、CLI/MCP 完全一致，以及检查前后文件哈希不变。夹具中的本地批准文本用于证明其不能提升成真实授权。

测试另覆盖有效元信息文档追加超过 1MiB 正文与外部 docs 链接（Windows 实测 junction；其他平台为符号链接）：拒绝后修复为项目内有界普通文件，CLI/MCP 均重新观察成功。

构建后设置 `FLOWGUARD_BIN` 为二进制绝对路径，运行 `python -B tests/stage_check.py`。Windows 开发将 `CARGO_HOME/CARGO_TARGET_DIR/TEMP/TMP` 指向 D/E 盘。需要现有兄弟目录 `flowguard-plugin`；测试和 CLI 不安装插件、不初始化 SDD、不推进真实阶段。

Portable package: root plugin.json and mcp.json register the already-built flowguard executable from PATH; skills/ contains the package-local usage skill. The package does not bundle or install the executable. Structure and stdio discovery are checked by guardengine/scripts/verify_guard_packages.py; actual host loading remains unverified.

## Signed execution evidence

The `guardengine/openspec/changes/execution-trace-evidence` increment adds a read-only consumer of TestGuard Ed25519 receipts:

```sh
flowguard evidence check --project /candidate --receipt /ci/execution.json --policy /controller/policy.json --trusted-key /controller/testguard-public.hex
```

All paths must be absolute. The explicit trust key must be outside the candidate project. Policy is an independent obligation, with exactly these fields:

```json
{"task_id":"refund","plan_digest":"sha256:64_lowercase_hex_characters","required_tests":["refund::accepts"],"runner":"cargo","max_age_seconds":3600}
```

Use the actual frozen execution plan SHA256, not the illustrative digest above. The signed payload must include matching `task_id`, plan digest, runner and required-test set. Observed tests must cover the independent required set. Time budget is 1..604800 seconds. A trusted controller must derive policy from approved specifications/plan; copying receipt claims into policy establishes no independent obligation.

The shared verifier checks the pinned Ed25519 key, producer/schema, PASS with exit 0, execution_verified, current source snapshot, executable/report hashes and timestamps. Unsigned imports, signature tampering, wrong keys, missing reports and source drift are unverified. Success returns `technical_pass=true`, `decision=requires_approval`, `approval=required` and CLI exit 3; failure returns `unverified`, false and exit 2. CLI syntax errors exit 4. This does not combine or replace the existing stage precheck and never advances a stage.

MCP `flowguard_evidence_check` accepts `{project,receipt,policy,trusted_key}` and uses the same verifier. Both entry points are read-only and never execute tests. Signature authenticity is relative to the explicitly trusted producer key; it is neither human approval, sandboxing nor remote attestation. Protect signing keys and runner/workspace in controlled CI. See `skills/flowguard-evidence-check/SKILL.md` and `tests/evidence_check.rs` for use and negative/recovery tests.
