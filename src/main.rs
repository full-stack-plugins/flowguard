mod agent_integration;
mod evidence_check;
mod stage_check;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("stage")
        && matches!(args.get(1).map(String::as_str), Some("status" | "check"))
    {
        return stage_check::cli(&args[1], &args[2..]);
    }
    if args.first().map(String::as_str) == Some("evidence")
        && args.get(1).map(String::as_str) == Some("check")
    {
        return evidence_check::cli(&args[2..]);
    }
    if let Some(code) = guard_install::dispatch(&agent_integration::tool(), &args) {
        return code;
    }
    match args.first().map(String::as_str) {
        Some("--version") if args.len() == 1 => {
            println!("flowguard {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help" | "-h") | None => {
            println!(
                "flowguard: evidence check --project ABS --receipt ABS --policy ABS --trusted-key EXTERNAL_ABS; stage status|check --project ABS --flowguard-root TRUSTED_ABS --python ABS --task-id ID [--action code_write|git_commit|release]; install; uninstall; context; instructions render|sync; doctor --integration; serve --mcp\n阶段产物预检只读，不能替代真实用户批准或当前证据门禁。"
            );
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("unsupported command; use --help for implemented capabilities");
            ExitCode::from(4)
        }
    }
}
