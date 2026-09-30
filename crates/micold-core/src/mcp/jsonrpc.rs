//! The JSON-RPC 2.0 envelope and MCP method routing (contracts/binding.md §3).
//!
//! The server is stateless: each `POST /mcp` carries one message, and every method except
//! `tools/call` is answered here from pure data. `tools/call` is handed back to the caller as
//! [`Route::CallTool`], because answering it needs the daemon's state.

use serde_json::{json, Map, Value};

/// JSON-RPC: the body is not valid JSON.
pub const PARSE_ERROR: i64 = -32700;
/// JSON-RPC: valid JSON but not a request object.
pub const INVALID_REQUEST: i64 = -32600;
/// JSON-RPC: the method is not one this server offers.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC: the method's parameters are malformed.
pub const INVALID_PARAMS: i64 = -32602;

/// The MCP protocol revisions this server speaks; a client asking for one of them gets it back.
pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-03-26", "2025-06-18", "2025-11-25", "2026-07-28"];

/// The revision answered to a client asking for any other one.
pub const LATEST_PROTOCOL_VERSION: &str = "2026-07-28";

/// The server's name in `serverInfo`, and the MCP server name every binding uses.
pub const SERVER_NAME: &str = "micold";

/// The `instructions` paragraph `initialize` returns: the scope and the confirmation policy.
pub const INSTRUCTIONS: &str = "These tools read and manage the worktrees and sessions of the Micold \
project this session belongs to, and only that project: another project's sessions and worktrees \
are reported as not found. Changes made here are the same operations the user performs in the app \
and appear in its windows at once. Operations that stop, interrupt or delete another session or a \
worktree ask the user in the app first and fail with needs_confirmation when nobody answers.";

/// One parsed JSON-RPC message.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// A request: it has an `id` and expects a response.
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    /// A notification: no `id`, no response.
    Notification { method: String, params: Value },
}

/// What to do with a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    /// Answer with this JSON-RPC response (`200`).
    Reply(Value),
    /// A notification: answer `202` with an empty body.
    Accepted,
    /// A `tools/call`: the caller runs the tool and answers with [`result`] of its outcome.
    CallTool {
        id: Value,
        name: String,
        arguments: Value,
    },
}

/// Parse one request body. On failure, the JSON-RPC error response to send.
pub fn parse(body: &[u8]) -> Result<Message, Value> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|_| error(Value::Null, PARSE_ERROR, "Parse error"))?;
    let Value::Object(mut object) = value else {
        return Err(error(Value::Null, INVALID_REQUEST, "Invalid Request"));
    };
    let id = object.remove("id");
    let Some(Value::String(method)) = object.remove("method") else {
        return Err(error(
            id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "Invalid Request",
        ));
    };
    let params = object.remove("params").unwrap_or(Value::Null);
    Ok(match id {
        Some(id) => Message::Request { id, method, params },
        None => Message::Notification { method, params },
    })
}

/// Route one message. `server_version` is reported in `serverInfo.version`.
pub fn route(message: Message, server_version: &str) -> Route {
    let (id, method, params) = match message {
        Message::Notification { .. } => return Route::Accepted,
        Message::Request { id, method, params } => (id, method, params),
    };
    match method.as_str() {
        "initialize" => Route::Reply(result(id, initialize_result(&params, server_version))),
        "ping" => Route::Reply(result(id, json!({}))),
        "tools/list" => Route::Reply(result(id, super::tools::list_result())),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Route::Reply(error(id, INVALID_PARAMS, "tools/call needs a tool name"));
            };
            let arguments = match params.get("arguments") {
                None | Some(Value::Null) => Value::Object(Map::new()),
                Some(arguments) => arguments.clone(),
            };
            Route::CallTool {
                id,
                name: name.to_string(),
                arguments,
            }
        }
        _ => Route::Reply(error(id, METHOD_NOT_FOUND, "Method not found")),
    }
}

/// The `initialize` result: the negotiated revision, the capabilities and the server's identity.
fn initialize_result(params: &Value, server_version: &str) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let protocol_version = requested
        .filter(|v| SUPPORTED_PROTOCOL_VERSIONS.contains(v))
        .unwrap_or(LATEST_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {"name": SERVER_NAME, "version": server_version},
        "instructions": INSTRUCTIONS,
    })
}

/// A JSON-RPC success response.
pub fn result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// A JSON-RPC error response.
pub fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}
