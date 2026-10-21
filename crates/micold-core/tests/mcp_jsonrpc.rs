//! The tool server's JSON-RPC envelope and MCP method routing (feature 034, contracts/binding.md §3,
//! contracts/mcp-tools.md §Results; U6–U16).

use micold_core::mcp::errors::{tool_failure, tool_success, ErrorCategory, OpError};
use micold_core::mcp::jsonrpc::{
    parse, route, ListScope, Message, Route, LATEST_PROTOCOL_VERSION, METHOD_NOT_FOUND, PARSE_ERROR,
};
use serde_json::{json, Value};

/// The service version the router reports in `serverInfo.version`.
const VERSION: &str = "9.9.9";

/// Route a JSON value and expect a direct reply.
fn reply(request: Value) -> Value {
    let message = parse(request.to_string().as_bytes()).expect("well-formed request");
    match route(message, VERSION, ListScope::Terminal) {
        Route::Reply(response) => response,
        other => panic!("expected a reply, got {other:?}"),
    }
}

fn initialize(protocol_version: &str) -> Value {
    reply(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": protocol_version,
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}
        }
    }))
}

#[test]
fn a_message_with_an_id_parses_as_a_request() {
    let message = parse(br#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#).unwrap();
    match message {
        Message::Request { id, method, .. } => {
            assert_eq!(id, json!(7));
            assert_eq!(method, "ping");
        }
        other => panic!("expected a request, got {other:?}"),
    }
}

#[test]
fn a_message_without_an_id_parses_as_a_notification() {
    let message = parse(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).unwrap();
    match message {
        Message::Notification { method, .. } => assert_eq!(method, "notifications/initialized"),
        other => panic!("expected a notification, got {other:?}"),
    }
}

#[test]
fn a_notification_is_accepted_without_a_reply() {
    let message = parse(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).unwrap();
    assert!(matches!(
        route(message, VERSION, ListScope::Terminal),
        Route::Accepted
    ));
}

#[test]
fn malformed_json_answers_a_parse_error_with_a_null_id() {
    let response = parse(b"{not json").expect_err("malformed JSON must not parse");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], json!(PARSE_ERROR));
    assert_eq!(PARSE_ERROR, -32700);
}

#[test]
fn an_unknown_method_answers_method_not_found() {
    let response = reply(json!({"jsonrpc":"2.0","id":"a","method":"resources/list"}));
    assert_eq!(response["id"], "a");
    assert_eq!(response["error"]["code"], json!(METHOD_NOT_FOUND));
    assert_eq!(METHOD_NOT_FOUND, -32601);
}

#[test]
fn initialize_echoes_each_supported_protocol_version() {
    for version in ["2025-03-26", "2025-06-18", "2025-11-25", "2026-07-28"] {
        let response = initialize(version);
        assert_eq!(
            response["result"]["protocolVersion"], version,
            "{version} is supported and must be echoed"
        );
    }
}

#[test]
fn initialize_answers_the_latest_version_for_any_other() {
    for version in ["2024-11-05", "1999-01-01", ""] {
        let response = initialize(version);
        assert_eq!(
            response["result"]["protocolVersion"],
            LATEST_PROTOCOL_VERSION
        );
    }
    assert_eq!(LATEST_PROTOCOL_VERSION, "2026-07-28");
}

#[test]
fn initialize_declares_tools_without_list_changes_and_names_the_server() {
    let result = &initialize("2025-06-18")["result"];
    assert_eq!(result["capabilities"]["tools"]["listChanged"], json!(false));
    assert_eq!(result["serverInfo"]["name"], "micold");
    assert_eq!(result["serverInfo"]["version"], VERSION);
    // contracts/binding.md: the paragraph names the scope (own project) and the confirmation policy.
    let instructions = result["instructions"].as_str().unwrap_or_default();
    for phrase in [
        "this session belongs to, and only that project",
        "another project's sessions and worktrees are reported as not found",
        "ask the user in the app first",
        "needs_confirmation",
        // feature 532: the issue-reference convention.
        "include the issue reference",
        "${type}/${ticket}_${name}",
    ] {
        assert!(
            instructions.contains(phrase),
            "the instructions must say {phrase:?}: {instructions}"
        );
    }
}

#[test]
fn ping_answers_an_empty_object() {
    let response = reply(json!({"jsonrpc":"2.0","id":3,"method":"ping"}));
    assert_eq!(response["id"], 3);
    assert_eq!(response["result"], json!({}));
}

#[test]
fn tools_call_is_handed_to_the_caller_with_its_name_and_arguments() {
    let message = parse(
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
               "params":{"name":"whoami","arguments":{"x":1}}})
        .to_string()
        .as_bytes(),
    )
    .unwrap();
    match route(message, VERSION, ListScope::Terminal) {
        Route::CallTool {
            id,
            name,
            arguments,
        } => {
            assert_eq!(id, json!(4));
            assert_eq!(name, "whoami");
            assert_eq!(arguments, json!({"x":1}));
        }
        other => panic!("expected a tool call, got {other:?}"),
    }
}

#[test]
fn a_success_result_carries_text_structured_content_and_is_not_an_error() {
    let output = json!({"session": "abc", "ai_cli": "claude_code"});
    let result = tool_success(&output);
    assert_eq!(result["isError"], json!(false));
    assert_eq!(result["structuredContent"], output);
    assert_eq!(result["content"][0]["type"], "text");
    let text = result["content"][0]["text"].as_str().unwrap();
    let reparsed: Value = serde_json::from_str(text).unwrap();
    assert_eq!(reparsed, output, "the text is the output's JSON");
}

#[test]
fn a_failure_result_names_its_category_and_message() {
    let error = OpError::new(ErrorCategory::NotFound, "no such session");
    let result = tool_failure(&error);
    assert_eq!(result["isError"], json!(true));
    assert_eq!(result["content"][0]["text"], "not_found: no such session");
    assert_eq!(
        result["structuredContent"]["error"],
        json!({"category": "not_found", "message": "no such session"})
    );
}

#[test]
fn every_category_serialises_as_its_snake_case_name() {
    let expected = [
        (ErrorCategory::NotFound, "not_found"),
        (ErrorCategory::InvalidInput, "invalid_input"),
        (ErrorCategory::Conflict, "conflict"),
        (ErrorCategory::RefusedByPolicy, "refused_by_policy"),
        (ErrorCategory::NeedsConfirmation, "needs_confirmation"),
        (ErrorCategory::ServiceError, "service_error"),
    ];
    assert_eq!(ErrorCategory::ALL.len(), expected.len());
    for (category, name) in expected {
        assert_eq!(category.as_str(), name);
        assert_eq!(serde_json::to_value(category).unwrap(), json!(name));
    }
}

fn tool_names(scope: ListScope) -> Vec<String> {
    let message = parse(br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).unwrap();
    let Route::Reply(response) = route(message, VERSION, scope) else {
        panic!("tools/list is answered directly");
    };
    response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

/// #942 FR-001, contracts/claude-permission.md: the prompt tool is listed to a structured caller
/// only, with the contract's schema, after every tool a terminal caller sees.
#[test]
fn the_prompt_tool_is_listed_for_a_structured_caller_only() {
    let terminal = tool_names(ListScope::Terminal);
    let structured = tool_names(ListScope::Structured);
    assert!(!terminal.iter().any(|n| n == "permission_prompt"));
    assert_eq!(
        structured.last().map(String::as_str),
        Some("permission_prompt")
    );
    assert_eq!(structured[..structured.len() - 1], terminal[..]);

    let message = parse(br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).unwrap();
    let Route::Reply(response) = route(message, VERSION, ListScope::Structured) else {
        panic!("tools/list is answered directly");
    };
    let tool = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "permission_prompt")
        .unwrap()
        .clone();
    assert_eq!(
        tool,
        json!({"name": "permission_prompt",
            "description": "Asks the user of the Micold window whether a tool use may run.",
            "inputSchema": {"type": "object",
                "properties": {"tool_name": {"type": "string"}, "input": {"type": "object"},
                               "tool_use_id": {"type": "string"}},
                "required": ["tool_name", "input"]}})
    );
}

fn prompt_call() -> Message {
    parse(
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call",
               "params":{"name":"permission_prompt",
                         "arguments":{"tool_name":"Bash","input":{"command":"ls"}},
                         "_meta":{"claudecode/toolUseId":"toolu_1"}}})
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}

/// #942 R1: a structured caller's prompt-tool call is handed over as a prompt, with its `_meta`;
/// a terminal caller's is an ordinary tool call, which the catalog refuses as an unknown tool.
#[test]
fn a_prompt_tool_call_is_a_prompt_for_a_structured_caller_only() {
    match route(prompt_call(), VERSION, ListScope::Structured) {
        Route::Prompt {
            id,
            arguments,
            meta,
        } => {
            assert_eq!(id, json!(5));
            assert_eq!(arguments["tool_name"], "Bash");
            assert_eq!(meta, Some(json!({"claudecode/toolUseId":"toolu_1"})));
        }
        other => panic!("expected a prompt, got {other:?}"),
    }
    match route(prompt_call(), VERSION, ListScope::Terminal) {
        Route::CallTool { name, .. } => assert_eq!(name, "permission_prompt"),
        other => panic!("expected an ordinary tool call, got {other:?}"),
    }
}
