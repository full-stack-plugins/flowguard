use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path, process::ExitCode};

/// 显式受信执行证据只读核验；无 Java 对应类型。
pub(crate) const TOOL: guard_install::DomainTool = guard_install::DomainTool {
    name: "flowguard_evidence_check",
    description: "Read-only signed execution verification against explicit external trust key and task policy. Never executes tests, advances stages or grants approval.",
    schema,
    call: check,
};
fn schema() -> Value {
    json!({"type":"object","properties":{"project":{"type":"string"},"receipt":{"type":"string"},"policy":{"type":"string"},"trusted_key":{"type":"string"}},"required":["project","receipt","policy","trusted_key"],"additionalProperties":false})
}
fn ids(value: &Value) -> Result<BTreeSet<&str>, String> {
    let values = value
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 8192)
        .ok_or("nonempty bounded policy test IDs required")?;
    let mut result = BTreeSet::new();
    for value in values {
        let id = value
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.len() <= 2048)
            .ok_or("invalid policy test ID")?;
        if !result.insert(id) {
            return Err("duplicate policy test ID".into());
        }
    }
    Ok(result)
}
fn verify(args: &Value) -> Result<Value, String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    if object.len() != 4 {
        return Err("exactly project, receipt, policy, trusted_key required".into());
    }
    for (key, value) in object {
        if !["project", "receipt", "policy", "trusted_key"].contains(&key.as_str())
            || value
                .as_str()
                .is_none_or(|s| s.is_empty() || s.len() > 4096 || !Path::new(s).is_absolute())
        {
            return Err(format!("invalid absolute path argument: {key}"));
        }
    }
    let policy = guard_evidence::read_json(Path::new(args["policy"].as_str().unwrap()))?;
    let fields = policy.as_object().ok_or("policy must be an object")?;
    if fields.len() != 5
        || fields.keys().any(|k| {
            ![
                "task_id",
                "plan_digest",
                "required_tests",
                "runner",
                "max_age_seconds",
            ]
            .contains(&k.as_str())
        })
    {
        return Err(
            "policy requires exactly task_id, plan_digest, required_tests, runner, max_age_seconds"
                .into(),
        );
    }
    let task = policy["task_id"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 240)
        .ok_or("bounded task_id required")?;
    let plan = policy["plan_digest"]
        .as_str()
        .ok_or("plan_digest required")?;
    let runner = policy["runner"]
        .as_str()
        .filter(|s| ["cargo", "maven"].contains(s))
        .ok_or("policy runner must be cargo or maven")?;
    let age = policy["max_age_seconds"]
        .as_u64()
        .ok_or("max_age_seconds required")?;
    let required = ids(&policy["required_tests"])?;
    let envelope = guard_evidence::read_json(Path::new(args["receipt"].as_str().unwrap()))?;
    let payload = guard_evidence::verify_execution(
        &envelope,
        Path::new(args["trusted_key"].as_str().unwrap()),
        Path::new(args["project"].as_str().unwrap()),
        plan,
        age,
    )?;
    if payload["task_id"] != task
        || payload["runner"] != runner
        || required != ids(&payload["required_tests"])?
        || !required.is_subset(&ids(&payload["observed_tests"])?)
    {
        return Err(
            "execution differs from explicit task, runner or required-test obligations".into(),
        );
    }
    Ok(
        json!({"decision":"requires_approval","approval":"required","technical_pass":true,"execution_verified":true,"task_id":task,"plan_digest":plan,"runner":runner,"required_tests":policy["required_tests"],"trust":"explicit_external_key","message":"受信执行证据已核验；仍需真实用户批准与原生阶段门禁。"}),
    )
}
fn check(args: &Value) -> Result<Value, String> {
    Ok(match verify(args) {
        Ok(value) => value,
        Err(error) => {
            json!({"decision":"unverified","approval":"required","technical_pass":false,"execution_verified":false,"error":error})
        }
    })
}
/// 只读 CLI；技术通过仍退出 3 表示待审批，未验证退出 2，参数语法错误退出 4。
pub(crate) fn cli(args: &[String]) -> ExitCode {
    let mut values = serde_json::Map::new();
    for pair in args.chunks(2) {
        if pair.len() != 2
            || !pair[0].starts_with("--")
            || values
                .insert(pair[0][2..].replace('-', "_"), json!(pair[1]))
                .is_some()
        {
            eprintln!("unique options with values required");
            return ExitCode::from(4);
        }
    }
    let value = check(&Value::Object(values)).expect("check returns a structured decision");
    println!("{value}");
    ExitCode::from(if value["technical_pass"] == true {
        3
    } else {
        2
    })
}
