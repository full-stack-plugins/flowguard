//! MCP 2025-11-25 local stdio profile; one controller-frozen session per process.
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    sync::atomic::AtomicBool,
};
pub const PROTOCOL_VERSION: &str = "2025-11-25";
const MAX_FRAME: usize = 8192;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}
pub struct Protocol {
    session: crate::host::ReadOnlySession,
    initialized: bool,
    ready: bool,
}
fn failure(id: Value, code: i32) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":"request unavailable"}})
}
impl Protocol {
    pub fn new(session: crate::host::ReadOnlySession) -> Self {
        Self {
            session,
            initialized: false,
            ready: false,
        }
    }
    pub fn handle(&mut self, bytes: &[u8], cancel: &AtomicBool) -> Option<Value> {
        if bytes.len() > MAX_FRAME {
            return Some(failure(Value::Null, -32600));
        }
        let msg: Message = match crate::host::unique_json(bytes).and_then(serde_json::from_value) {
            Ok(v) => v,
            Err(_) => return Some(failure(Value::Null, -32600)),
        };
        if msg.jsonrpc != "2.0"
            || msg
                .id
                .as_ref()
                .is_some_and(|v| !v.is_string() && !v.is_i64() && !v.is_u64())
        {
            return Some(failure(Value::Null, -32600));
        }
        let Some(id) = msg.id else {
            if msg.method == "notifications/initialized" && self.initialized {
                self.ready = true
            }
            return None;
        };
        let result = match msg.method.as_str() {
            "initialize" if !self.initialized => {
                if !msg.params["protocolVersion"].is_string()
                    || !msg.params["capabilities"].is_object()
                    || !msg.params["clientInfo"]["name"].is_string()
                    || !msg.params["clientInfo"]["version"].is_string()
                {
                    return Some(failure(id, -32602));
                }
                self.initialized = true;
                json!({"protocolVersion":PROTOCOL_VERSION,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"flowguard-local-readonly","version":env!("CARGO_PKG_VERSION")},"instructions":"Controller-frozen local reads only; results confer no execution or approval authority."})
            }
            "ping" => json!({}),
            _ if !self.ready => return Some(failure(id, -32002)),
            "tools/list" => {
                json!({"tools":[{"name":"flowguard_read","description":"Read a controller-frozen job; repeat reads recompute source state. No execution authority.","inputSchema":{"type":"object","properties":{"job":{"type":"string"},"requestKey":{"type":"string"}},"required":["job","requestKey"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false},"execution":{"taskSupport":"forbidden"}}]})
            }
            "tools/call" => {
                let Some(params) = msg.params.as_object() else {
                    return Some(failure(id, -32602));
                };
                if params
                    .keys()
                    .any(|k| !matches!(k.as_str(), "name" | "arguments" | "_meta"))
                    || msg.params["name"] != "flowguard_read"
                {
                    return Some(failure(id, -32602));
                }
                let response = self.session.call(
                    &serde_json::to_vec(&msg.params["arguments"]).unwrap_or_default(),
                    cancel,
                );
                json!({"content":[{"type":"text","text":response.to_string()}],"structuredContent":response,"isError":response["exitCode"]!=0})
            }
            _ => return Some(failure(id, -32601)),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
}
/// Sequential framing, bounded before allocation. Cancellation is checked at call boundaries.
pub fn serve(
    session: crate::host::ReadOnlySession,
    mut input: impl BufRead,
    mut output: impl Write,
    cancel: &AtomicBool,
) -> std::io::Result<()> {
    let mut protocol = Protocol::new(session);
    loop {
        let mut frame = Vec::new();
        let mut too_long = false;
        let mut seen = false;
        loop {
            let bytes = input.fill_buf()?;
            if bytes.is_empty() {
                break;
            }
            seen = true;
            let newline = bytes.iter().position(|b| *b == b'\n');
            let len = newline.map_or(bytes.len(), |p| p + 1);
            if frame.len() + len > MAX_FRAME {
                too_long = true
            } else if !too_long {
                frame.extend_from_slice(&bytes[..len])
            }
            input.consume(len);
            if newline.is_some() {
                break;
            }
        }
        if !seen {
            return Ok(());
        }
        let response = if too_long {
            Some(failure(Value::Null, -32600))
        } else {
            protocol.handle(&frame, cancel)
        };
        if let Some(value) = response {
            serde_json::to_writer(&mut output, &value)?;
            output.write_all(b"\n")?;
            output.flush()?
        }
    }
}
