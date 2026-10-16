# FlowGuard 阶段产物预检验证记录

日期：2026-10-16。统一增量规格：`guardengine/openspec/changes/professional-guard-checks`。原生阶段语义依据 `flowguard-plugin/docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md`。

## 交付内容

- `stage status`：只读原 `stage_docs.snapshot`，返回观测到的阶段状态、原生引用和指纹。
- `stage check --action code_write|git_commit|release`：复用原 `missing_before`，不复制阶段名、阶段依赖矩阵或审批规则。
- MCP `flowguard_stage_check` 与 CLI 共用 Rust 参数校验和同一嵌入适配脚本，适配器插件/Python 路径必须每次显式提供。
- 原生服务不满足时 `deny`，文档缺失/非法或适配失败时 `unverified`。产物齐备仅 `requires_approval`；`status` 为 `observed`。没有 `allow` 结果。
- 始终标明 `approval=required`、`quality_decision=not_evaluated`。不接受 actor/approval_ref/apply，不执行阶段推进或写入。
- 已补 `plugin/README.md`、双语入口和 CI 检查步骤。未改原插件，原插件工作树仍干净；无发版、分支切换或公共 crate 修改。

## 实际验证

本地 Windows，现有 Cargo/Rust 与 Python。所有开发输出均位于 E 盘：

```powershell
$env:CARGO_HOME='E:/workspaces/workspace-agent-skills/.guard-build/cargo'
$env:CARGO_TARGET_DIR='E:/workspaces/workspace-agent-skills/.guard-build/targets/flowguard-agent'
$env:TEMP='E:/workspaces/workspace-agent-skills/.guard-build/tmp'
$env:TMP=$env:TEMP
cargo build --locked
$env:FLOWGUARD_BIN="$env:CARGO_TARGET_DIR/debug/flowguard.exe"
python -B tests/stage_check.py
cargo fmt --check
cargo clippy --locked -- -D warnings
git diff --check
```

- RED：实现前两测试均因 `unsupported command`、CLI 退出 4 失败。
- GREEN：`tests/stage_check.py` 两测试通过，最终执行 **1.699s**。包含缺文档→生成原生夹具→缺阶段拒绝→仅夹具本地记录齐备→仍需真实审批；code_write 前置齐备不放行 git_commit/release；正文变更导致原生指纹失效；非法元信息→修复→状态复验；CLI/MCP 全 JSON 相同；每次检查前后文件内容 SHA256 相同；无项目私有状态目录；actor 参数拒绝。
- `cargo build --locked`、`cargo fmt --check`、`cargo clippy --locked -- -D warnings`、`git diff --check` 均通过。
- 原插件 `python -B -m unittest discover -s ../flowguard-plugin/tests -p test_stage_transitions.py`：**8 测试通过，1.734s**；PYTHONPATH 指向原插件 scripts，FLOWGUARD_STATE_HOME 指向 E 盘隔离测试缓存。
- 原安装回归 `verify_guard_installation.py --bin-dir <flowguard-agent>/debug --tools flowguard`：**6 通过、1 失败、3 跳过，1.142s**。唯一失败是公共脚本的 MCP inventory 预期尚未登记 `flowguard_stage_check`；已通知主代理更新并统一复验。跳过项需要 GitFlow、六个工具或 GitGuard，本轮仅选择 FlowGuard。其余共存安装、冲突原子性、能力边界、受链接宿主配置拒绝、卸载/二进制归属均通过。

## 限制

### 来源边界补强（2026-10-16）

按主代理审查增加原生阶段读取前置限制，仅修改独立 FlowGuard，未改原插件：通过原 `registry.ARTIFACTS` 与 `stage_docs.path_for` 定位全部十阶段文件，检查祖先路径，拒绝 symlink、Windows reparse point/junction、项目外路径和非普通文件。单文件 1MiB、所有不同来源累计 16MiB；原生发布阶段可能读取的其他功能依赖文件也经相同受限读取。首次读取缓存内容，评估结束再次检查路径并重读 SHA256，变化时返回 `unverified` 且移除快照。

- RED：新增两个真实 CLI/MCP 测试，带有效阶段元信息的文档追加大于 1MiB 正文、docs junction 指向另一临时目录中的真实阶段文档，原实现均返回 `observed`；两测试按预期失败，0.489s。
- GREEN：`cargo build --locked` 后，`python -B tests/stage_check.py` **4 测试全部通过，3.460s**。新增测试验证超限/外部链接返回 `unverified`，还原文件/目录后返回 `observed`；CLI/MCP 相同，检查不改文件。Windows junction 已实际创建并执行；非 Windows 分支使用 symlink，尚未在本机执行。
- 缺失文档仍经原生服务返回未验证；没有把边界失败提升为通过或审批。

前后校验与缓存不承诺操作系统级原子快照，不替代沙箱权限隔离。

本轮是原生阶段产物预检，不证明当前测试、静态分析、语义审查、发布依赖或真实用户审批满足。检查返回的批准引用只是本地数据，不是可信授权；必须由宿主真实确认机制和原插件当前证据门禁继续执行。未观察远程 CI 与真实宿主加载。公共 MCP inventory 回归等待主代理更新和复验。
