use std::io::Write;
fn main() -> std::process::ExitCode {
    let args: Result<Vec<String>, _> = std::env::args_os()
        .skip(1)
        .map(|a| a.into_string())
        .collect();
    let output = match args {
        Ok(args) => flowguard::cli::run(args),
        Err(_) => flowguard::cli::CliOutput {
            code: 4,
            stdout: vec![],
            stderr: "flowguard: invalid arguments\n".into(),
        },
    };
    if std::io::stdout().write_all(&output.stdout).is_err()
        || std::io::stderr()
            .write_all(output.stderr.as_bytes())
            .is_err()
    {
        return std::process::ExitCode::from(4);
    }
    std::process::ExitCode::from(output.code)
}
