//! The tool catalog and argument validation (feature 034, contracts/mcp-tools.md; U83–U89 for the
//! read-only tools of milestone M1, U90–U94 for the create tools of milestone M3, U85/U95/U96 and
//! the lifecycle tools for milestone M4).

use micold_core::mcp::errors::ErrorCategory;
use micold_core::mcp::jsonrpc::{parse, route, Route};
use micold_core::mcp::tools::{
    catalog, parse_call, parse_operation, Operation, SessionRef, WorktreeRef,
};
use micold_core::session::AiCli;
use micold_core::worktree::CreateMode;
use serde_json::{json, Value};
use uuid::Uuid;

/// The read-only tools, shipped in milestone M1.
const READ_TOOLS: [&str; 5] = [
    "whoami",
    "list_worktrees",
    "list_branches",
    "list_sessions",
    "get_session",
];

/// The tools whose handlers ship by milestone M4, in catalog order.
const SHIPPED: [&str; 9] = [
    "whoami",
    "list_worktrees",
    "list_branches",
    "list_sessions",
    "get_session",
    "create_worktree",
    "rename_worktree",
    "create_session",
    "start_session",
];

/// The tools FR-014 classifies as destructive (U85).
const DESTRUCTIVE: [&str; 4] = [
    "delete_worktree",
    "stop_session",
    "interrupt_session",
    "delete_session",
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
    assert_eq!(
        error.category,
        ErrorCategory::InvalidInput,
        "{name} {arguments}"
    );
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
        let read_only = READ_TOOLS.contains(&name);
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
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            json!(read_only),
            "{name}"
        );
        assert_eq!(
            tool["annotations"]["destructiveHint"],
            json!(false),
            "{name}"
        );
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

fn create_worktree(branch: &str, name: Option<&str>, mode: CreateMode) -> Operation {
    Operation::CreateWorktree {
        branch: branch.into(),
        name: name.map(str::to_string),
        mode,
    }
}

#[test]
fn create_worktree_without_a_mode_starts_a_new_branch() {
    assert_eq!(
        parse_call("create_worktree", &json!({"branch": "feat-x"})).unwrap(),
        create_worktree("feat-x", None, CreateMode::NewBranch)
    );
    assert_eq!(
        parse_call(
            "create_worktree",
            &json!({"branch": "feat-x", "name": "x", "mode": "existing_local"})
        )
        .unwrap(),
        create_worktree("feat-x", Some("x"), CreateMode::ReuseLocal)
    );
}

#[test]
fn track_remote_needs_the_remote_to_track() {
    let message = invalid(
        "create_worktree",
        json!({"branch": "feat-x", "mode": "track_remote"}),
    );
    assert!(message.contains("remote"), "{message}");
    assert_eq!(
        parse_call(
            "create_worktree",
            &json!({"branch": "feat-x", "mode": "track_remote", "remote": "origin"})
        )
        .unwrap(),
        create_worktree(
            "feat-x",
            None,
            CreateMode::TrackRemote {
                remote: "origin".into()
            }
        )
    );
}

#[test]
fn create_worktree_offers_no_way_to_overwrite_a_branch() {
    invalid(
        "create_worktree",
        json!({"branch": "feat-x", "overwrite": true}),
    );
    invalid(
        "create_worktree",
        json!({"branch": "feat-x", "mode": "overwrite"}),
    );
}

#[test]
fn create_worktree_needs_a_branch() {
    invalid("create_worktree", json!({}));
    invalid("create_worktree", json!({"branch": ""}));
    invalid("create_worktree", json!({"branch": 7}));
}

#[test]
fn create_session_accepts_exactly_the_three_ai_clis() {
    for (name, cli) in [
        ("claude_code", AiCli::ClaudeCode),
        ("copilot", AiCli::Copilot),
        ("pi", AiCli::Pi),
    ] {
        assert_eq!(
            parse_call(
                "create_session",
                &json!({"worktree": "feat-x", "ai_cli": name, "prompt": "go"})
            )
            .unwrap(),
            Operation::CreateSession {
                worktree: WorktreeRef::Named("feat-x".into()),
                ai_cli: Some(cli),
                prompt: Some("go".into()),
            }
        );
    }
    assert_eq!(
        parse_call("create_session", &json!({"worktree": "default"})).unwrap(),
        Operation::CreateSession {
            worktree: WorktreeRef::Default,
            ai_cli: None,
            prompt: None,
        }
    );
    invalid(
        "create_session",
        json!({"worktree": "feat-x", "ai_cli": "regular_terminal"}),
    );
    invalid(
        "create_session",
        json!({"worktree": "feat-x", "ai_cli": "vim"}),
    );
    invalid("create_session", json!({"ai_cli": "pi"}));
    invalid("create_session", json!({"worktree": "feat-x", "prompt": 3}));
}

const S: &str = "00000000-0000-0000-0000-000000000007";

fn s7() -> SessionRef {
    SessionRef(Uuid::parse_str(S).unwrap())
}

fn invalid_any(name: &str, arguments: Value) -> String {
    let error = parse_operation(name, &arguments).expect_err("must be rejected");
    assert_eq!(
        error.category,
        ErrorCategory::InvalidInput,
        "{name} {arguments}"
    );
    error.message
}

/// U85: over the whole catalog, including the tools a later milestone ships.
#[test]
fn destructive_hint_is_set_on_exactly_the_destructive_tools() {
    let tools = catalog()["tools"].as_array().unwrap().clone();
    let destructive: Vec<&str> = tools
        .iter()
        .filter(|t| t["annotations"]["destructiveHint"] == json!(true))
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(destructive, DESTRUCTIVE);
    for tool in &tools {
        let name = tool["name"].as_str().unwrap();
        if DESTRUCTIVE.contains(&name) {
            assert_eq!(tool["annotations"]["readOnlyHint"], json!(false), "{name}");
        }
    }
}

#[test]
fn a_tool_whose_handler_has_not_shipped_is_unknown() {
    for name in DESTRUCTIVE {
        let message = invalid(name, json!({}));
        assert!(message.contains("unknown tool"), "{name}: {message}");
    }
}

#[test]
fn the_session_lifecycle_tools_take_a_session_id() {
    for (name, expected) in [
        ("start_session", Operation::StartSession { session: s7() }),
        ("stop_session", Operation::StopSession { session: s7() }),
        (
            "interrupt_session",
            Operation::InterruptSession { session: s7() },
        ),
        ("delete_session", Operation::DeleteSession { session: s7() }),
    ] {
        assert_eq!(
            parse_operation(name, &json!({"session": S})).unwrap(),
            expected
        );
        invalid_any(name, json!({}));
        invalid_any(name, json!({"session": "not-a-uuid"}));
    }
    assert_eq!(
        parse_call("start_session", &json!({"session": S})).unwrap(),
        Operation::StartSession { session: s7() },
        "start_session ships in M4"
    );
}

#[test]
fn rename_worktree_takes_a_worktree_and_a_trimmed_display_name() {
    assert_eq!(
        parse_call(
            "rename_worktree",
            &json!({"worktree": "b", "display_name": "  Login fix "})
        )
        .unwrap(),
        Operation::RenameWorktree {
            worktree: WorktreeRef::Named("b".into()),
            display_name: "Login fix".into(),
        }
    );
    invalid("rename_worktree", json!({"worktree": "b"}));
    invalid("rename_worktree", json!({"display_name": "x"}));
    let message = invalid(
        "rename_worktree",
        json!({"worktree": "b", "display_name": ""}),
    );
    assert!(message.contains("empty"), "the dialog's wording: {message}");
    let message = invalid(
        "rename_worktree",
        json!({"worktree": "b", "display_name": "   "}),
    );
    assert!(
        message.contains("whitespace"),
        "the dialog's wording: {message}"
    );
}

/// U95
#[test]
fn default_is_not_a_worktree_to_rename_or_delete() {
    let message = invalid(
        "rename_worktree",
        json!({"worktree": "default", "display_name": "x"}),
    );
    assert!(message.contains("default"), "{message}");
    let message = invalid_any("delete_worktree", json!({"worktree": "default"}));
    assert!(message.contains("default"), "{message}");
}

/// U96
#[test]
fn delete_worktree_keeps_live_sessions_and_deletes_the_branch_by_default() {
    assert_eq!(
        parse_operation("delete_worktree", &json!({"worktree": "b"})).unwrap(),
        Operation::DeleteWorktree {
            worktree: WorktreeRef::Named("b".into()),
            stop_sessions: false,
            delete_branch: true,
        }
    );
    assert_eq!(
        parse_operation(
            "delete_worktree",
            &json!({"worktree": "b", "stop_sessions": true, "delete_branch": false})
        )
        .unwrap(),
        Operation::DeleteWorktree {
            worktree: WorktreeRef::Named("b".into()),
            stop_sessions: true,
            delete_branch: false,
        }
    );
    invalid_any(
        "delete_worktree",
        json!({"worktree": "b", "stop_sessions": "yes"}),
    );
}
