use std::sync::atomic::AtomicBool;
fn start() -> Result<(), ()> {
    let args: Result<Vec<_>, _> = std::env::args_os()
        .take(6)
        .map(|v| v.into_string())
        .collect();
    let args = args.map_err(|_| ())?;
    if args.len() != 5 || args[1] != "--local-readonly-v1" {
        return Err(());
    }
    let root =
        flowguard::input_limits::AllowedRoot::new(std::path::Path::new(&args[2]), 256 * 1024)
            .map_err(|_| ())?;
    let bytes = root.read(&args[3]).map_err(|_| ())?;
    if flowguard::digest(&bytes) != args[4] {
        return Err(());
    }
    let session = flowguard::host::ReadOnlySession::freeze(&bytes).map_err(|_| ())?;
    flowguard::mcp::serve(
        session,
        std::io::stdin().lock(),
        std::io::stdout().lock(),
        &AtomicBool::new(false),
    )
    .map_err(|_| ())
}
fn main() -> std::process::ExitCode {
    match start() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(()) => {
            eprintln!("flowguard MCP: local profile or transport unavailable");
            std::process::ExitCode::from(4)
        }
    }
}
