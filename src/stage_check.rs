use serde_json::{Value, json};
use std::{
    io::Read,
    path::Path,
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant},
};

/// 真实只读 MCP 阶段查询与预检；无 Java 对应类型。
pub(crate) const TOOLS: &[guard_install::DomainTool] = &[
    crate::evidence_check::TOOL,
    guard_install::DomainTool {
        name: "flowguard_stage_check",
        description: "Read-only native FlowGuard stage snapshot or prerequisite artifact precheck. Explicit trusted plugin/Python paths required; never grants approval or executes advance.",
        schema,
        call: check,
    },
];

fn schema() -> Value {
    json!({"type":"object","properties":{
        "project":{"type":"string"},"flowguard_root":{"type":"string"},"python":{"type":"string"},
        "task_id":{"type":"string"},"mode":{"type":"string","enum":["status","check"]},
        "action":{"type":"string","enum":["code_write","git_commit","release"]}
    },"required":["project","flowguard_root","python","task_id"],"additionalProperties":false})
}

fn unverified(code: &str) -> Value {
    json!({"decision":"unverified","approval":"required","quality_decision":"not_evaluated",
           "envelope":{"severity":"ERROR","code":code,"message":"可信阶段适配器缺失或执行失败。","fix":"核验显式插件与 Python 路径，修复后复验。"}})
}

fn check(args: &Value) -> Result<Value, String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    for (key, value) in object {
        if ![
            "project",
            "flowguard_root",
            "python",
            "task_id",
            "mode",
            "action",
        ]
        .contains(&key.as_str())
            || !value.is_string()
            || value.as_str().unwrap().len() > 4096
        {
            return Err(format!("invalid argument: {key}"));
        }
    }
    let mode = args["mode"].as_str().unwrap_or("check");
    if !["status", "check"].contains(&mode) {
        return Err("mode must be status or check".into());
    }
    if mode == "check"
        && !["code_write", "git_commit", "release"].contains(&args["action"].as_str().unwrap_or(""))
    {
        return Err("action required: code_write, git_commit or release".into());
    }
    if mode == "status" && args.get("action").is_some() {
        return Err("status does not take action".into());
    }
    let task = args["task_id"].as_str().ok_or("task_id required")?;
    if task.is_empty() || task.len() > 240 {
        return Err("task_id must be nonempty and bounded".into());
    }
    let mut paths = Vec::new();
    for key in ["flowguard_root", "python", "project"] {
        let path = Path::new(args[key].as_str().ok_or(format!("{key} required"))?);
        if !path.is_absolute() {
            return Err(format!("{key} must be absolute"));
        }
        paths.push(path);
    }
    let (root, python, project) = (paths[0], paths[1], paths[2]);
    if !root.join("scripts/flowguard_lib/stage_docs.py").is_file()
        || !python.is_file()
        || !project.is_dir()
    {
        return Ok(unverified("stage_adapter_missing"));
    }
    let mut child = match Command::new(python)
        .args([
            "-I",
            "-B",
            "-X",
            "utf8",
            "-c",
            include_str!("stage_reader.py"),
        ])
        .arg(root)
        .arg(project)
        .arg(args.to_string())
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Ok(unverified("stage_adapter_spawn_failed")),
    };
    let output = child.stdout.take().ok_or("missing adapter stdout")?;
    let errors = child.stderr.take().ok_or("missing adapter stderr")?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        output
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let error_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        errors.take(65537).read_to_end(&mut bytes).map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(20));
    };
    let bytes = reader
        .join()
        .map_err(|_| "adapter reader failed")?
        .map_err(|e| e.to_string())?;
    let stderr = error_reader
        .join()
        .map_err(|_| "adapter error reader failed")?
        .map_err(|e| e.to_string())?;
    if !status.is_some_and(|s| s.success()) || bytes.len() > 1024 * 1024 || stderr.len() > 65536 {
        return Ok(unverified("stage_adapter_execution_failed"));
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid adapter JSON")?;
    Ok(value)
}

/// 运行只读 CLI 命令；0 是状态读取成功，2 为前置拒绝，3 为需审批/未验证，4 为非法请求。
pub(crate) fn cli(mode: &str, args: &[String]) -> ExitCode {
    let result = (|| {
        let mut values = serde_json::Map::new();
        values.insert("mode".into(), json!(mode));
        for pair in args.chunks(2) {
            if pair.len() != 2 || !pair[0].starts_with("--") {
                return Err("options require values".into());
            }
            let key = pair[0][2..].replace('-', "_");
            if values.insert(key, json!(pair[1])).is_some() {
                return Err("duplicate option".into());
            }
        }
        check(&Value::Object(values))
    })();
    match result {
        Ok(value) => {
            println!("{value}");
            ExitCode::from(match value["decision"].as_str() {
                Some("observed") => 0,
                Some("deny") => 2,
                _ => 3,
            })
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(4)
        }
    }
}
