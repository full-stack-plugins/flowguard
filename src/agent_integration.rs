/// 返回当前二进制真实接入能力；不替代专业领域的运行结果。
pub(crate) fn tool() -> guard_install::Tool {
    let mut tool = guard_install::Tool::new(
        "flowguard",
        env!("CARGO_PKG_VERSION"),
        "使用 evidence check / flowguard_evidence_check 显式核验外部受信公钥、当前执行证据和任务策略，技术通过仍需审批。使用 stage status/check 或 flowguard_stage_check，只读复用显式可信 flowguard-plugin 的 docs/project 与 docs/features 原生阶段服务。产物齐备仅 requires_approval，不生成 allow，不把本地 actor 或批准引用视为可信授权。仍需真实用户审批和原插件当前证据门禁；不执行阶段推进。",
        &["stage status", "stage check", "evidence check"],
    );
    tool.mcp_tools = crate::stage_check::TOOLS;
    tool
}
