//! The per-CLI launch binding to the tool server (feature 034, contracts/binding.md §4–§5; U41–U46,
//! U58).

use std::path::{Path, PathBuf};

use micold_core::mcp::binding::{plan, SkipReason, PROMPT_TOOL_TIMEOUT_MS, TOOL_TIMEOUT_MS};
use micold_core::provider::ToolServerSupport;
use serde_json::{json, Value};

const URL: &str = "http://127.0.0.1:41234/mcp";
const CREDENTIAL: &str = "0123456789abcdef0123456789abcdef";

fn file() -> PathBuf {
    Path::new("/data/micold-ai-ide/mcp").join("5f0c1e2a-0000-4000-8000-000000000001.json")
}

fn claude_entry() -> Value {
    json!({
        "type": "http",
        "url": URL,
        "headers": {"Authorization": format!("Bearer {CREDENTIAL}")},
        "timeout": 120000
    })
}

#[test]
fn claude_is_bound_by_mcp_config_and_allowed_tools_as_the_last_arguments() {
    let bound = plan(
        ToolServerSupport::McpConfigArg,
        URL,
        CREDENTIAL,
        &file(),
        TOOL_TIMEOUT_MS,
    )
    .unwrap();
    assert_eq!(
        bound.args,
        vec![
            "--mcp-config".to_string(),
            file().to_string_lossy().into_owned(),
            "--allowedTools".to_string(),
            "mcp__micold".to_string(),
        ],
        "the binding is exactly these four arguments, appended after everything else"
    );
}

#[test]
fn the_claude_file_is_exactly_one_http_server_named_micold() {
    let bound = plan(
        ToolServerSupport::McpConfigArg,
        URL,
        CREDENTIAL,
        &file(),
        TOOL_TIMEOUT_MS,
    )
    .unwrap();
    let parsed: Value = serde_json::from_str(&bound.file_contents).unwrap();
    assert_eq!(parsed, json!({"mcpServers": {"micold": claude_entry()}}));
}

#[test]
fn copilot_is_bound_by_additional_mcp_config_and_allow_tool() {
    let bound = plan(
        ToolServerSupport::AdditionalMcpConfig,
        URL,
        CREDENTIAL,
        &file(),
        TOOL_TIMEOUT_MS,
    )
    .unwrap();
    assert_eq!(
        bound.args,
        vec![
            "--additional-mcp-config".to_string(),
            format!("@{}", file().to_string_lossy()),
            "--allow-tool".to_string(),
            "micold".to_string(),
        ]
    );
}

#[test]
fn the_copilot_file_is_the_claude_entry_plus_every_tool() {
    let bound = plan(
        ToolServerSupport::AdditionalMcpConfig,
        URL,
        CREDENTIAL,
        &file(),
        TOOL_TIMEOUT_MS,
    )
    .unwrap();
    let parsed: Value = serde_json::from_str(&bound.file_contents).unwrap();
    let mut entry = claude_entry();
    entry["tools"] = json!(["*"]);
    assert_eq!(parsed, json!({"mcpServers": {"micold": entry}}));
}

#[test]
fn an_unsupported_cli_is_skipped_with_its_reason_and_no_arguments() {
    let skipped = plan(
        ToolServerSupport::Unsupported {
            reason: "Pi has no MCP support",
        },
        URL,
        CREDENTIAL,
        &file(),
        TOOL_TIMEOUT_MS,
    )
    .unwrap_err();
    assert_eq!(skipped, SkipReason::Unsupported("Pi has no MCP support"));
}

#[test]
fn no_plan_ever_passes_strict_mcp_config() {
    for support in [
        ToolServerSupport::McpConfigArg,
        ToolServerSupport::AdditionalMcpConfig,
    ] {
        let bound = plan(support, URL, CREDENTIAL, &file(), TOOL_TIMEOUT_MS).unwrap();
        assert!(
            !bound.args.iter().any(|a| a.contains("strict-mcp-config")),
            "{support:?} must never replace the user's own servers"
        );
    }
}

#[test]
fn each_skip_reason_renders_its_contract_text() {
    let cases = [
        (SkipReason::Disabled, "disabled in settings".to_string()),
        (
            SkipReason::Unsupported("Pi has no MCP support"),
            "Pi has no MCP support".to_string(),
        ),
        (
            SkipReason::NameTaken(PathBuf::from("/home/u/.claude.json")),
            format!(
                "a server named \"micold\" is already configured in {}",
                Path::new("/home/u/.claude.json").display()
            ),
        ),
        (
            SkipReason::ServerUnavailable,
            "tool server unavailable".to_string(),
        ),
        (
            SkipReason::WriteFailed("permission denied".to_string()),
            "could not write the binding: permission denied".to_string(),
        ),
        (
            SkipReason::UnboundAtStart,
            "not bound when it started".to_string(),
        ),
    ];
    for (reason, text) in cases {
        assert_eq!(reason.to_string(), text);
    }
}

/// #942 R3, FR-018: a structured session's binding waits as long as Node's largest timer for the
/// prompt tool's answer; every other binding keeps the 120 s it always had, byte for byte.
#[test]
fn a_structured_binding_waits_for_the_prompt_tools_answer_and_a_terminal_one_is_unchanged() {
    assert_eq!(PROMPT_TOOL_TIMEOUT_MS, 2_147_483_647);
    assert_eq!(TOOL_TIMEOUT_MS, 120_000);
    let structured = plan(
        ToolServerSupport::McpConfigArg,
        URL,
        CREDENTIAL,
        &file(),
        PROMPT_TOOL_TIMEOUT_MS,
    )
    .unwrap();
    let parsed: Value = serde_json::from_str(&structured.file_contents).unwrap();
    let mut entry = claude_entry();
    entry["timeout"] = json!(2_147_483_647u64);
    assert_eq!(parsed, json!({"mcpServers": {"micold": entry}}));

    let terminal = plan(
        ToolServerSupport::McpConfigArg,
        URL,
        CREDENTIAL,
        &file(),
        TOOL_TIMEOUT_MS,
    )
    .unwrap();
    let expected =
        serde_json::to_string_pretty(&json!({"mcpServers": {"micold": claude_entry()}})).unwrap();
    assert_eq!(terminal.file_contents, expected, "byte for byte as before");
    assert_eq!(terminal.args, structured.args, "the same arguments");
}
