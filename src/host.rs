//! Controller-frozen local read-only jobs. This is not remote authentication.
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};
pub const VERSION: &str = "flowguard.host/v1";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: String,
    tenant: String,
    jobs: BTreeMap<String, Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Request {
    job: String,
    request_key: String,
}
#[derive(Debug)]
pub enum HostError {
    InvalidConfig,
}
pub struct ReadOnlySession {
    config: Config,
    keys: BTreeMap<String, String>,
}
pub fn error(code: &str) -> Value {
    json!({"version":VERSION,"profile":"local-readonly","authorization":"not_evaluated","exitCode":4,"bundle":null,"diagnostic":code})
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}
impl ReadOnlySession {
    pub fn freeze(bytes: &[u8]) -> Result<Self, HostError> {
        if bytes.len() > 256 * 1024 {
            return Err(HostError::InvalidConfig);
        }
        let config: Config = unique_json(bytes)
            .and_then(serde_json::from_value)
            .map_err(|_| HostError::InvalidConfig)?;
        if config.version != VERSION
            || !name(&config.tenant)
            || config.jobs.is_empty()
            || config.jobs.len() > 16
        {
            return Err(HostError::InvalidConfig);
        }
        for (key, args) in &config.jobs {
            let allowed = matches!(
                (
                    args.first().map(String::as_str),
                    args.get(1).map(String::as_str)
                ),
                (Some("discover"), _)
                    | (Some("stage"), Some("status"))
                    | (Some("evidence"), Some("verify"))
                    | (Some("gate"), Some("check"))
            );
            if !name(key)
                || !allowed
                || args.len() > 32
                || args.iter().map(String::len).sum::<usize>() > 16384
            {
                return Err(HostError::InvalidConfig);
            }
        }
        Ok(Self {
            config,
            keys: BTreeMap::new(),
        })
    }
    pub fn call(&mut self, bytes: &[u8], cancelled: &AtomicBool) -> Value {
        if bytes.len() > 8192 || cancelled.load(Ordering::SeqCst) {
            return error("unavailable");
        }
        let request: Request = match unique_json(bytes).and_then(serde_json::from_value) {
            Ok(v) => v,
            Err(_) => return error("invalid_request"),
        };
        if !name(&request.job) || !name(&request.request_key) {
            return error("invalid_request");
        }
        let Some(args) = self.config.jobs.get(&request.job) else {
            return error("unknown_job");
        };
        if let Some(job) = self.keys.get(&request.request_key) {
            if job != &request.job {
                return error("key_rebound");
            }
        } else {
            if self.keys.len() >= 256 {
                return error("capacity");
            };
            self.keys.insert(request.request_key, request.job);
        }
        let result = crate::cli::run(args.clone());
        if cancelled.load(Ordering::SeqCst) || result.stdout.len() > 8 * 1024 * 1024 {
            return error("unavailable");
        }
        let bundle = serde_json::from_slice::<Value>(&result.stdout).unwrap_or(Value::Null);
        json!({"version":VERSION,"profile":"local-readonly","authorization":"not_evaluated","exitCode":result.code,"bundle":bundle})
    }
}
pub(crate) fn unique_json(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    struct Unique(Value);
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Unique;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("unambiguous JSON")
                }
                fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                    Ok(Unique(json!(v)))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                    Ok(Unique(Value::Null))
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    let mut values = vec![];
                    while let Some(Unique(v)) = a.next_element()? {
                        values.push(v);
                    }
                    Ok(Unique(Value::Array(values)))
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    let mut values = serde_json::Map::new();
                    while let Some(key) = a.next_key::<String>()? {
                        if values.contains_key(&key) {
                            return Err(serde::de::Error::custom("duplicate member"));
                        }
                        let Unique(v) = a.next_value()?;
                        values.insert(key, v);
                    }
                    Ok(Unique(Value::Object(values)))
                }
            }
            d.deserialize_any(Visitor)
        }
    }
    serde_json::from_slice::<Unique>(bytes).map(|v| v.0)
}
