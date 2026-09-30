//! The tool catalog and argument validation (feature 034, contracts/mcp-tools.md; U83–U89 for the
//! read-only tools of milestone M1).

use micold_core::mcp::errors::ErrorCategory;
use micold_core::mcp::jsonrpc::{parse, route, Route};
use micold_core::mcp::tools::{parse_call, Operation, SessionRef, WorktreeRef};
use serde_json::{json, Value};
use uuid::Uuid;

/// The tools whose handlers ship in milestone M1, in catalog order.
const SHIPPED: [&str; 5] = [
    "whoami",
    "list_worktrees",
    "list_branches",
    "list_sessions",
    "get_session",
];

fn tools_list() -> Vec<Value> {
    let message = parse(br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).unwrap();
    let Route::Reply(response) = route(message, "0") else {
        panic!("tools/list is answered directly");
    };
    response["result"]["tools"].as_array().unwrap().clone()
}

fn invalid(name: &str, arguments: Value) -> String {
    let error = parse_call(name, &arguments).expect_err("must be rejected");
    assert_eq!(error.category, ErrorCategory::InvalidInput, "{name} {arguments}");
    error.message
}

#[test]
fn tools_list_names_exactly_the_shipped_tools() {
    let names: Vec<String> = tools_list()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, SHIPPED);
}

#[test]
fn every_tool_has_an_object_input_schema_and_the_read_tools_are_read_only() {
    for tool in tools_list() {
        let name = tool["name"].as_str().unwrap();
        assert_eq!(tool["inputSchema"]["type"], "object", "{name}");
        assert_eq!(
            tool["inputSchema"]["additionalProperties"],
            json!(false),
            "{name} rejects unknown arguments"
        );
        assert!(
            tool["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{name} has a description"
        );
        assert_eq!(tool["annotations"]["readOnlyHint"], json!(true), "{name}");
        assert_eq!(tool["annotations"]["destructiveHint"], json!(false), "{name}");
    }
}

#[test]
fn an_unknown_tool_is_invalid_input() {
    let message = invalid("delete_everything", json!({}));
    assert!(message.contains("delete_everything"), "{message}");
}

#[test]
fn the_read_tools_parse_their_arguments() {
    let id = Uuid::from_u128(7);
    assert_eq!(parse_call("whoami", &json!({})).unwrap(), Operation::Whoami);
    assert_eq!(
        parse_call("list_worktrees", &json!({})).unwrap(),
        Operation::ListWorktrees {
            include_hidden: false
        }
    );
    assert_eq!(
        parse_call("list_worktrees", &json!({"include_hidden": true})).unwrap(),
        Operation::ListWorktrees {
            include_hidden: true
        }
    );
    assert_eq!(
        parse_call("list_branches", &json!({})).unwrap(),
        Operation::ListBranches
    );
    assert_eq!(
        parse_call("list_sessions", &json!({})).unwrap(),
        Operation::ListSessions { worktree: None }
    );
    assert_eq!(
        parse_call("list_sessions", &json!({"worktree": "feat-x"})).unwrap(),
        Operation::ListSessions {
            worktree: Some(WorktreeRef::Named("feat-x".into()))
        }
    );
    assert_eq!(
        parse_call("get_session", &json!({"session": id.to_string()})).unwrap(),
        Operation::GetSession {
            session: SessionRef(id)
        }
    );
}

#[test]
fn a_non_boolean_include_hidden_is_invalid_input() {
    invalid("list_worktrees", json!({"include_hidden": "yes"}));
    invalid("list_worktrees", json!({"include_hidden": 1}));
}

#[test]
fn a_session_that_is_not_a_uuid_string_is_invalid_input() {
    invalid("get_session", json!({"session": "not-a-uuid"}));
    invalid("get_session", json!({"session": 42}));
    invalid("get_session", json!({}));
}

#[test]
fn an_unknown_argument_or_a_non_string_worktree_is_invalid_input() {
    invalid("whoami", json!({"extra": 1}));
    invalid("list_sessions", json!({"worktree": 3}));
    invalid("list_sessions", json!({"worktree": ""}));
}

#[test]
fn worktree_ref_default_is_the_project_root_and_anything_else_is_named() {
    assert_eq!(WorktreeRef::parse("default"), WorktreeRef::Default);
    assert_eq!(
        WorktreeRef::parse("feat-x"),
        WorktreeRef::Named("feat-x".into())
    );
    assert_eq!(WorktreeRef::Default.as_str(), "default");
    assert_eq!(WorktreeRef::Named("a".into()).as_str(), "a");
}
