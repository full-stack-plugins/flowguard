# ADR: fixed-revision legacy survey; migration unsupported

Inspected https://github.com/full-stack-plugins/flowguard-plugin at **13b52b054c31f614dc272b18c195b8fa929aa595**, manifest version **0.4.2**, on 2026-10-09. The read-only cloud clone remains in `/workspace/guard-implementation-ledger/flowguard-legacy`; no changes, notifications or pushes were made. Source license is Apache-2.0 (`LICENSE`); no source code is copied into this crate.

All following citations resolve within that fixed commit, not mutable main:

- [`plugin.json`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/plugin.json) declares name/version/license.
- [`scripts/flowguard_lib/registry.py`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/scripts/flowguard_lib/registry.py) `ARTIFACTS` contains the ten names and project ownership of 02/07/10. It defines dependency edges and FIFO ready-node traversal.
- [`scripts/flowguard_lib/stage_docs.py`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/scripts/flowguard_lib/stage_docs.py) `path_for` uses `docs/project/` and `docs/features/<task>/`; `_metadata` and `_rows` use Chinese Markdown table fields. `LEGAL` allows direct pending/in_progress/invalidated→accepted. `_content_hash` omits selected metadata and evidence sections.
- [`scripts/flowguard_state.py`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/scripts/flowguard_state.py) exposes `init`, `discover`, `context`, `evidence`, `governance`, `stage`, `validate`. Its help was actually executed successfully with Python 3.12. Not all commands are read-only: init/advance/approve/record are not imported or executed by the Rust implementation.
- [`hooks/hooks.json`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/hooks/hooks.json) and [`hooks/__protocol__.md`](https://github.com/full-stack-plugins/flowguard-plugin/blob/13b52b054c31f614dc272b18c195b8fa929aa595/hooks/__protocol__.md) define Python hooks: UserPromptSubmit, PreToolUse, PostToolUse and summaries. Source protocol documents gate exit 0/2; artifact check exit 0. These are source observations, not real-host enforcement evidence.

Executed `test_registry.py`: 4 passed. Executed `test_stage_transitions.py`: 8 passed after explicitly setting PYTHONPATH and FLOWGUARD_STATE_HOME to isolated cloud scratch. Initial import failed with missing flowguard_lib; initial corrected-import run failed because the default home state directory was read-only. Both failures are recorded, not hidden. Full legacy suite and real host were not run. No legacy/new differential acceptance was performed.

Compatibility decisions: ten path names are source-confirmed and reused as a versioned layout. New graph uses lexicographic ready order; new state transition requirements prohibit invalidated→accepted without resubmission; discovery hashes every source byte, unlike legacy metadata exclusions. These are known differences, not parity. Legacy status-table parsing, mutation commands, hook payload compatibility, identity authentication, and automatic migration remain UNSUPPORTED. No rule-owner transfer is authorized by this survey.

## Task 5.2: fixed-revision declaration differential

`adapters/legacy/mod.rs` now observes the surveyed two-column Chinese stage/status rows through the bounded `AllowedRoot`, with an explicit exact legacy revision argument and one of the ten surveyed paths. It returns `declared` state and the complete raw-byte digest. This is not legacy `read()`'s effective state, authenticated task ownership, approval, inheritance validation, migration, or gate eligibility. `migration_supported()` is always false and no mutator or migration command is exposed. The caller's immutable-checkout condition still applies.

`tests/support/capture_legacy.py <existing-pinned-checkout>` refuses another commit or tracked modifications, performs 15 actual old-library reads in disposable directories, and records the old 49-pair LEGAL table in `fixtures/legacy/differential.json`. It neither fetches nor changes legacy source. `tests/legacy_differential.rs` consumes those frozen observations without requiring Python or an external checkout for normal Rust tests. Sources are byte-pinned in the fixture. Execution is local differential evidence, not a claim of real-host compatibility.

| Rule | Sole authority in this profile | Known difference / disposition |
|---|---|---|
| Legacy effective status and excluded-content fingerprint | Fixed legacy `stage_docs.py` | New adapter only reports the literal declaration. A legacy accepted declaration can be effectively invalidated; unsupported migration. |
| Legacy approval, inheritance and skip metadata | Legacy native implementation | Old accepted/inherited/skipped text cannot authorize new evidence-qualified transitions. |
| New transition requirements | Existing FlowGuard `stage_transition::transition` | Exact baseline, protected skip and technical evidence require their existing approval paths; no legacy override. |
| Source byte identity | FlowGuard bounded reader/digest | Full bytes hashed; no legacy metadata exclusion. |
| Duplicate or fenced example metadata | FlowGuard declaration parser | Duplicate recognized fields reject; fenced examples ignored. Legacy regex accepts fenced rows and keeps last duplicate. |
| Missing, directory, invalid UTF-8, unknown status/stage | Bounded read/parser | New adapter rejects; legacy missing/directory reads yield pending, invalid UTF-8 throws. No assumed pending authority. |

Fingerprint recomputation, upstream/project release resolution, real-host hooks, changing ownership, CLI migration and broad old/new behavioral parity remain unsupported. Documented differences are not silently adjudicated into compatibility; every observation remains ineligible for migration. No user documents are changed.

## 5.6 待裁决材料：规则 owner 与迁移条件

此表是可审阅建议，不代表已选定兼容语义或允许 owner 切换。旧版本固定为
`13b52b054c31f614dc272b18c195b8fa929aa595`；当前实现观察点为
`1b4ce598940922f76984f3a921603f8c6696ed28`。后者的 legacy adapter 只读声明，
`migration_supported()` 仍为 false。下面的 owner 指该规则的唯一判定实现，
不表示给文档声明授予批准或执行权限。

证据分级：

- **实际 differential**：[15 个旧读取结果和 LEGAL 表](../../fixtures/legacy/differential.json)，旧 `stage_docs.py` SHA256 为 `1df0f26b26a754dc1854457b561ecce4db63860aae62c63043643aa544266a00`；[当前逐向量/49 状态对照](../../tests/legacy_differential.rs)。49 对覆盖的是旧 LEGAL 表与当前 transition 返回要求的比较，不是执行 49 次旧 mutator。
- **源码观察**：固定旧 `_content_hash`、`_upstream_fingerprint`、`_advance_unlocked`、registry 遍历，与当前 [声明解析](../../adapters/legacy/mod.rs)、[transition](../../src/stage_transition.rs)、[依赖图](../../src/dependencies.rs) 和既有资格接口对照。这些行没有新增真实迁移演练。
- 本次仅整理上述材料，未重跑已有矩阵，未更改源文档、旧实现或当前规则。表内建议均待明确选择；解释差异机制不等于裁决其可接受性。

| 待裁决项 / 证据 | 固定旧行为 | 当前行为 | 迁移影响 | 建议唯一 owner | 可允许后续迁移的明确决策 |
| --- | --- | --- | --- | --- | --- |
| D1 有效状态与声明分离；`body-invalidated` 实际向量 | 文本仍是 accepted，但正文改变后 `read()` 返回 invalidated | adapter 返回 `declared=accepted` 和全字节摘要；不计算旧有效状态，不产生资格 | 把 declared 当 status 会向用户展示错误的满足状态 | 旧有效状态继续归固定 legacy；新证据资格归现有 FG 核心，两字段分别命名 | 选择“保留声明/旧有效状态/新资格三个含义，旧 accepted 不导入为新资格”，并指定需要展示哪些字段；如必须保持单一旧 status，则先保留 legacy owner，并补有效状态兼容读取后再迁移 |
| D2 内容指纹排除范围；源码观察 | 排除证据区以及状态、批准依据、前置/验收指纹行 | 新 reader 对全部原始字节取摘要 | 仅追加证据或改元数据也会改变新摘要，可能触发重验 | 新证据身份与失效由 FG 全字节摘要判定；旧显示指纹仍由 legacy 判定 | 明确接受“元数据变化也重验”的不兼容行为，或明确列出需保留的旧排除集合并要求独立兼容摘要；不得把两种摘要互认。选择后补元数据/证据区的同输入专项对照 |
| D3 重复字段；`duplicate-state` 实际向量 | 最后一个状态覆盖前值，得到 in_progress | 任一重复受识别字段拒绝 | 已有重复元数据文档需修复，不能静默迁移成功 | 建议 FG 严格声明 parser | 明确接受重复字段为错误，并选择“由文档 owner 修复后重验”；若必须保留 last-wins，则在另行批准的兼容读取范围内保留旧 owner，不能自动清理用户文档 |
| D4 围栏示例；`fenced-example` 实际向量 | 正则读取代码围栏内的阶段/状态行，得到 pending | 忽略围栏内容；该向量没有真实元数据，拒绝 | 示例可能被旧逻辑当事实，现有只有示例的文档不再可读 | 建议 FG parser，仅正文规范位置是事实 | 明确确认“围栏是示例、不授事实地位”；现有受影响文档须补正文元数据再迁移。若要求旧行为，保留 legacy owner，不能将示例解释为新批准 |
| D5 缺失路径；`missing` 实际向量 | 返回 pending、missing=true | 返回 source unavailable，不合成声明 | 自动看板初始状态和缺失输入诊断不同 | FG reader 判可用性；看板若需 pending，只能是独立显示标签 | 明确允许显示“未创建/pending”但资格仍 unavailable，或选择统一显示 unavailable；两种选择都不能据显示 pending 声明来源完整 |
| D6 路径为目录；`directory` 实际向量 | 同样返回 pending、missing=true | 拒绝读取 | 把配置错误当未开始会掩盖坏路径 | 建议 FG reader | 明确将目录占位视为配置错误并阻止迁移；由文档 owner 修复路径。若保留旧显示，仍不得放宽新读取/资格边界 |
| D7 直接验收路径；旧 LEGAL 与当前 transition 实际对照 | LEGAL 允许 pending/in_progress/invalidated→accepted；旧 mutator 另检查批准引用非空、正文/阶段校验与前置满足 | 三条直达 accepted 都 Illegal；仅 pending_acceptance→accepted 返回“当前技术证据与批准”要求 | 旧脚本的一步验收会失败；状态表相同不代表批准含义相同 | 建议 FG transition/qualification 是新验收唯一 owner | 明确采用 pending_acceptance 提交与重新验收流程，并指定旧调用如何显式重提交；若必须保持一步调用，需另审适配流程，不能把旧字符串引用转换成合格批准 |
| D8 返工与重新提交；同一 49 对比较 | pending_acceptance→in_progress；invalidated→in_progress；accepted/inherited/skipped→in_progress 可走 | 上述返回 Illegal；invalidated→pending_acceptance 及 accepted/inherited/skipped→invalidated 是新允许声明路径 | 旧“退回编辑”命令不能直接沿用；新 invalidated 是可声明状态，旧注释要求它由指纹派生 | 建议 FG transition 负责新返工路径，旧工作流暂由 legacy owner | 明确选定“满足态先失效，再重提交”的新状态路径及文档操作含义；如保留旧返工体验，需另审调用映射，不能默改 transition 或隐藏中间失效 |
| D9 继承/跳过；LEGAL 比较及旧 mutator 源码 | LEGAL 允许 pending/in_progress/pending_acceptance/invalidated→inherited/skipped；mutator 要非空批准引用、理由及前置指纹可用 | 前三个源状态分别要求精确基线批准/受保护跳过批准；invalidated→这两态 Illegal；声明 adapter 不验证旧引用 | 旧理由/引用不足以构造新不可变基线、范围或当前批准；直接导入会越权 | 新基线继承/跳过由现有 FG typed qualification 和外部批准端口唯一判定 | 明确旧 inherited/skipped 仅保留历史显示，新资格必须重新按精确基线或受保护 skip 取得批准；若要迁移旧批准，必须先指定可验证引用来源及范围映射，不能默认承认 |
| D10 共享项目/发布依赖；源码观察 | 02/07 等项目节点使用 project 标记；10 从发布 Scope 表取功能集合并读取07/各09指纹；其他功能阶段读取此前所有阶段 | 新核心绑定显式图边、不可变基线/09引用及当前发布集合；legacy adapter 不导入旧 Scope 或上游指纹 | 仅状态/摘要搬运不能证明同一依赖闭包；可能漏失效或错继承 | 建议新 FG graph/baseline/release 接口负责新闭包；旧 release 仍保留原 owner 直到映射验收 | 明确每个旧发布 Scope 行到新 feature/requirement/09引用的映射与缺项处理（建议拒绝），之后补一条真实旧→新发布闭包专项。现有联合共享源测试不是该迁移证据 |
| D11 同时就绪节点顺序；registry/graph 源码观察 | 声明顺序 FIFO | 新图按排序后的节点集合选择就绪节点 | 自定义图有多个就绪节点时，展示/读取次序可能不同；未证明旧十阶段样本已有可见分歧 | 建议执行依赖约束归 FG graph，展示顺序由明确的呈现规则处理 | 明确顺序是否只是展示；若是，可接受确定性新排序；若外部调用依赖 FIFO，先固定目标图及其顺序对照，不能宣称现有来源证明完全相同 |

已解释且不应列为待放行差异的边界：七个合法状态名在未触发指纹失效的固定向量中一致；`unknown-state`、`wrong-stage` 都拒绝。`invalid-utf8` 两者都失败，但旧异常名与新有限诊断不是同一个协议。建议新宿主诊断以新端口为 owner；若有消费者依赖旧异常/退出格式，须先给出该消费者和协议，再做定点兼容测试。没有这些消费者证据时，不宣称旧 CLI/hook 输出兼容，也不新增异常模拟层。

裁决记录可以直接按 `D编号 → 选择上述新语义 / 保留旧owner及所需兼容行为 → 适用规则与文档范围` 填写；D7–D10 还需明确批准引用来源和调用映射。建议优先裁决 D1、D7–D10，因为它们决定状态、批准、依赖是否可互认；D3–D6 可独立选择拒绝歧义/坏输入。全部仍是建议，当前没有裁决记录。

即使某项差异得到裁决，也只解除该规则的语义前置条件。5.6 尚缺逐规则调用路由、未裁决项停用行为、停新路由并保留审计/撤销历史的实际回滚演练；本次文档没有实现这些行为。受控 Linux GE 分发子片已验收，不能概括为“GE 全部阻塞”；正式生产依赖渠道和 N/N-1 兼容只能对另行实际测试的组合承诺。未裁决规则不进行 opt-in，且不修改 CodeGuard 或回退 Git refs。
