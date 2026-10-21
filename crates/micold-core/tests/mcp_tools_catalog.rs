//! The tool catalog and argument validation (feature 034, contracts/mcp-tools.md; U83–U89 for the
//! read-only tools of milestone M1, U90–U94 for the create tools of milestone M3, U85/U95/U96 and
//! the lifecycle tools for milestone M4).

use micold_core::mcp::errors::ErrorCategory;
use micold_core::mcp::jsonrpc::{parse, route, Route};
use micold_core::mcp::tools::{
    catalog, is_mutating_tool, parse_call, parse_operation, CreateWorktreeRequest, LineCount,
    NonEmptyText, Operation, SessionRef, WorktreeRef,
};
use micold_core::naming::ConventionalType;
use micold_core::occupancy::Additional;
use micold_core::review::comment::{CommentRef, Outcome};
use micold_core::review::prompt::REPORT_TOOL;
use micold_core::session::AiCli;
use micold_core::worktree::CreateMode;
use serde_json::{json, Value};
use uuid::Uuid;

/// The read-only tools, shipped in milestone M1.
const READ_TOOLS: [&str; 7] = [
    "whoami",
    "list_worktrees",
    "list_branches",
    "list_sessions",
    "get_session",
    "list_resumable_sessions",
    "read_session_output",
];

/// The tools whose handlers ship by milestone M6, in catalog order.
const SHIPPED: [&str; 17] = [
    "whoami",
    "list_worktrees",
    "list_branches",
    "list_sessions",
    "get_session",
    "list_resumable_sessions",
    "read_session_output",
    "create_worktree",
    "rename_worktree",
    "delete_worktree",
    "create_session",
    "start_session",
    "stop_session",
    "interrupt_session",
    "send_session_input",
    "report_review_comment",
    "delete_session",
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
    let Route::Reply(response) =
        route(message, "0", micold_core::mcp::jsonrpc::ListScope::Terminal)
    else {
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

/// Constitution 1.7.0: the catalog tells an agent in the project root (Default) what it is
/// refused, which is renaming and deleting a worktree, and no longer creating one.
#[test]
fn only_rename_and_delete_worktree_say_a_default_session_is_refused() {
    let says_refused = |name: &str| {
        let tools = tools_list();
        let tool = tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not listed"));
        let description = tool["description"].as_str().unwrap();
        description.contains("(Default) is refused")
    };
    assert!(
        !says_refused("create_worktree"),
        "a Default session may create a worktree"
    );
    assert!(says_refused("rename_worktree"));
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
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(
            description.len() >= 40 && description.trim_end().ends_with('.'),
            "{name} has a description of at least a sentence: {description:?}"
        );
        // FR-014: a tool that asks the user first says so, so an agent is not surprised by it.
        assert_eq!(
            description.contains("confirms in an app window"),
            DESTRUCTIVE.contains(&name),
            "{name}: exactly the destructive tools say the user confirms: {description:?}"
        );
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            json!(read_only),
            "{name}"
        );
        assert_eq!(
            tool["annotations"]["destructiveHint"],
            json!(DESTRUCTIVE.contains(&name)),
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
        Operation::ListWorktrees
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
fn include_hidden_is_no_longer_an_argument_of_list_worktrees() {
    invalid("list_worktrees", json!({"include_hidden": true}));
    invalid("list_worktrees", json!({"include_hidden": "yes"}));
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
    Operation::CreateWorktree(CreateWorktreeRequest::Literal {
        branch: branch.into(),
        name: name.map(str::to_string),
        mode,
        start: None,
    })
}

fn derived(type_: Option<ConventionalType>, ticket: Option<&str>, name: Option<&str>) -> Operation {
    Operation::CreateWorktree(CreateWorktreeRequest::Derived {
        type_,
        ticket: ticket.map(str::to_string),
        name: name.map(str::to_string),
        github_issue: None,
        start: None,
    })
}

#[test]
fn create_worktree_schema_lists_the_derived_inputs() {
    let listing = micold_core::mcp::tools::list_result();
    let tool = listing["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "create_worktree")
        .unwrap();
    let schema = &tool["inputSchema"];
    assert_eq!(schema["required"], json!([]));
    let props = &schema["properties"];
    let allowed: Vec<&str> = ConventionalType::ALL.iter().map(|t| t.as_str()).collect();
    assert_eq!(props["type"]["enum"], json!(allowed));
    assert_eq!(props["ticket"]["type"], "string");
    assert_eq!(props["github_issue"]["type"], "integer");
    assert_eq!(props["github_issue"]["minimum"], 1);
    assert!(props["github_issue"]["description"]
        .as_str()
        .unwrap()
        .contains("explicit values win"));
    let description = tool["description"].as_str().unwrap();
    assert!(description.contains("branch") && description.contains("type"));
    assert!(description.contains("never both"), "{description}");
}

/// FR-001, FR-002, FR-006, SC-004: the guidance names the convention, the issue-reference rule
/// and the inputs that carry it, so a session passes the ticket as its own input.
#[test]
fn create_worktree_guidance_states_the_issue_reference_convention() {
    let listing = micold_core::mcp::tools::list_result();
    let tool = listing["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "create_worktree")
        .unwrap();
    let description = tool["description"].as_str().unwrap();
    for phrase in [
        "${type}/${ticket}_${name}",
        "${type}-${ticket}_${name}",
        "include the issue reference",
        "`ticket`",
        "`github_issue`",
        "not inside a literal `branch`",
    ] {
        assert!(
            description.contains(phrase),
            "the description must say {phrase:?}: {description}"
        );
    }
    let ticket = tool["inputSchema"]["properties"]["ticket"]["description"]
        .as_str()
        .unwrap();
    for phrase in [
        "slugified as the form does",
        "blank means none",
        "issue tag",
        "issue filter",
    ] {
        assert!(
            ticket.contains(phrase),
            "the ticket input must say {phrase:?}: {ticket}"
        );
    }
}

/// Feature 837, FR-014: the tool says what a `github_issue` call's result reports
/// (contracts/tool-result.md, "Published text").
#[test]
fn create_worktree_publishes_the_issue_object() {
    let listing = micold_core::mcp::tools::list_result();
    let tool = listing["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "create_worktree")
        .unwrap();
    let description = tool["description"].as_str().unwrap();
    let published = "A github_issue call's result also carries issue: what blocks the issue and \
        what it blocks, its parent and sub-issues, labels, assignees, milestone, project fields and \
        linked pull requests (only the kinds it has; a kind that cannot be read is listed under \
        not_readable; details_error when none could be read). Being blocked is reported, never \
        enforced. Fails while GitHub issues are turned off in Settings.";
    assert!(
        description.contains(published),
        "the description must carry the published text: {description}"
    );
    let github_issue = tool["inputSchema"]["properties"]["github_issue"]["description"]
        .as_str()
        .unwrap();
    assert!(
        github_issue.contains("The result's issue object reports its relationships."),
        "{github_issue}"
    );
}

#[test]
fn derived_inputs_parse_to_a_derived_request() {
    assert_eq!(
        parse_call(
            "create_worktree",
            &json!({"type": "fix", "ticket": "#123", "name": "login crash"})
        )
        .unwrap(),
        derived(
            Some(ConventionalType::Fix),
            Some("#123"),
            Some("login crash")
        )
    );
    assert_eq!(
        parse_call("create_worktree", &json!({"name": "dark mode"})).unwrap(),
        derived(None, None, Some("dark mode"))
    );
    assert_eq!(
        parse_call("create_worktree", &json!({"branch": "feat-x", "name": "x"})).unwrap(),
        create_worktree("feat-x", Some("x"), CreateMode::NewBranch)
    );
}

#[test]
fn literal_and_derived_inputs_are_alternatives() {
    for args in [
        json!({"branch": "b", "type": "fix"}),
        json!({"branch": "b", "ticket": "1"}),
        json!({"branch": "b", "github_issue": 3}),
        json!({"branch": "b", "mode": "existing_local", "type": "fix"}),
        json!({"mode": "new_branch", "type": "fix"}),
        json!({"remote": "origin", "ticket": "1"}),
    ] {
        let message = invalid("create_worktree", args.clone());
        assert!(message.contains("alternatives"), "{args}: {message}");
    }
}

#[test]
fn mode_or_remote_without_a_branch_is_refused() {
    for args in [
        json!({"mode": "new_branch"}),
        json!({"remote": "origin"}),
        json!({"mode": "existing_local", "name": "x"}),
    ] {
        let message = invalid("create_worktree", args.clone());
        assert!(message.contains("need a branch"), "{args}: {message}");
    }
}

#[test]
fn an_empty_create_worktree_says_what_to_provide() {
    let message = invalid("create_worktree", json!({}));
    assert!(message.contains("provide a branch"), "{message}");
}

#[test]
fn an_unknown_type_names_the_allowed_values() {
    let message = invalid("create_worktree", json!({"type": "hotfix", "name": "x"}));
    for t in ConventionalType::ALL {
        assert!(message.contains(t.as_str()), "{message}");
    }
    invalid("create_worktree", json!({"type": "FIX", "name": "x"}));
    invalid("create_worktree", json!({"type": 3}));
}

/// A valid `github_issue` parses as the derived request; a malformed one is refused as malformed.
#[test]
fn github_issue_is_accepted_and_validated() {
    let Operation::CreateWorktree(CreateWorktreeRequest::Derived { github_issue, .. }) =
        parse_call("create_worktree", &json!({"github_issue": 12})).unwrap()
    else {
        panic!("not a derived request")
    };
    assert_eq!(github_issue, Some(12));
    for bad in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!("7"),
        json!(2147483648u64),
    ] {
        let message = invalid("create_worktree", json!({"github_issue": bad}));
        assert!(message.contains("whole number"), "{bad}: {message}");
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
fn create_worktree_rejects_a_malformed_branch() {
    invalid("create_worktree", json!({"branch": ""}));
    invalid("create_worktree", json!({"branch": 7}));
}

/// Feature 776, FR-014: the catalogue tells an agent that a worktree has at most one session.
#[test]
fn create_session_says_a_worktree_has_at_most_one_session() {
    let tools = tools_list();
    let description = tools
        .iter()
        .find(|t| t["name"] == "create_session")
        .and_then(|t| t["description"].as_str())
        .unwrap();
    assert!(
        description.contains("at most one session") && description.contains("naming the existing"),
        "{description:?}"
    );
}

/// Feature 776, FR-015: `allow_additional_session` is an optional boolean, default false, and the
/// description states that the limit is soft.
#[test]
fn create_session_holds_the_optional_allow_additional_session_flag() {
    let tools = tools_list();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "create_session")
        .unwrap();
    let flag = &tool["inputSchema"]["properties"]["allow_additional_session"];
    assert_eq!(flag["type"], "boolean", "{flag}");
    assert_eq!(flag["default"], json!(false));
    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(!required.iter().any(|r| r == "allow_additional_session"));
    let description = tool["description"].as_str().unwrap();
    assert!(
        description.contains("allow_additional_session: true") && description.contains("soft"),
        "{description:?}"
    );

    let parsed = |args: Value| match parse_call("create_session", &args).unwrap() {
        Operation::CreateSession { additional, .. } => additional,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        parsed(json!({"worktree": "feat-x", "allow_additional_session": true})),
        Additional::Yes
    );
    assert_eq!(
        parsed(json!({"worktree": "feat-x", "allow_additional_session": false})),
        Additional::No
    );
    assert_eq!(parsed(json!({"worktree": "feat-x"})), Additional::No);
}

/// Feature 776, FR-014 and FR-015: `start_session` states the rule and its refusal, and holds the
/// same optional flag.
#[test]
fn start_session_states_the_rule_and_holds_the_optional_flag() {
    let tools = tools_list();
    let tool = tools.iter().find(|t| t["name"] == "start_session").unwrap();
    let flag = &tool["inputSchema"]["properties"]["allow_additional_session"];
    assert_eq!(flag["type"], "boolean", "{flag}");
    assert_eq!(flag["default"], json!(false));
    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(!required.iter().any(|r| r == "allow_additional_session"));
    let description = tool["description"].as_str().unwrap();
    assert!(
        description.contains("allow_additional_session: true")
            && description.contains("Conflict")
            && description.contains("naming the running one"),
        "{description:?}"
    );

    let parsed = |args: Value| match parse_call("start_session", &args).unwrap() {
        Operation::StartSession { additional, .. } => additional,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        parsed(json!({"session": S, "allow_additional_session": true})),
        Additional::Yes
    );
    assert_eq!(parsed(json!({"session": S})), Additional::No);
    invalid_any(
        "start_session",
        json!({"session": S, "allow_additional_session": "yes"}),
    );
}

#[test]
fn create_session_accepts_exactly_the_five_ai_clis() {
    for (name, cli) in [
        ("claude_code", AiCli::ClaudeCode),
        ("copilot", AiCli::Copilot),
        ("pi", AiCli::Pi),
        ("codex", AiCli::Codex),
        ("opencode", AiCli::OpenCode),
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
                additional: Additional::No,
            }
        );
    }
    assert_eq!(
        parse_call("create_session", &json!({"worktree": "default"})).unwrap(),
        Operation::CreateSession {
            worktree: WorktreeRef::Default,
            ai_cli: None,
            prompt: None,
            additional: Additional::No,
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
    let message = invalid_any(
        "create_session",
        json!({"worktree": "feat-x", "ai_cli": "vim"}),
    );
    assert!(
        message.contains("codex") && message.contains("opencode"),
        "the refusal names every provider: {message}"
    );
    invalid(
        "create_session",
        json!({"worktree": "feat-x", "allow_additional_session": "yes"}),
    );
    invalid(
        "create_session",
        json!({"worktree": "feat-x", "allow_additional_session": 1}),
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
    let (destructive_tools, other_tools): (Vec<&Value>, Vec<&Value>) = tools
        .iter()
        .partition(|t| t["annotations"]["destructiveHint"] == json!(true));
    assert_eq!(destructive_tools.len(), DESTRUCTIVE.len());
    for tool in destructive_tools {
        assert_eq!(
            tool["annotations"]["readOnlyHint"],
            json!(false),
            "{}: a destructive tool is not read-only",
            tool["name"]
        );
    }
    for tool in other_tools {
        assert_eq!(
            tool["annotations"]["destructiveHint"],
            json!(false),
            "{}: only the destructive tools carry the hint",
            tool["name"]
        );
    }
}

#[test]
fn the_destructive_tools_are_callable_from_m5() {
    for name in DESTRUCTIVE {
        let message = invalid(name, json!({}));
        assert!(!message.contains("unknown tool"), "{name}: {message}");
    }
    assert_eq!(
        parse_call("stop_session", &json!({"session": S})).unwrap(),
        Operation::StopSession { session: s7() },
    );
}

#[test]
fn the_session_lifecycle_tools_take_a_session_id() {
    for (name, expected) in [
        (
            "start_session",
            Operation::StartSession {
                session: s7(),
                additional: Additional::No,
            },
        ),
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
        Operation::StartSession {
            session: s7(),
            additional: Additional::No,
        },
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

// --- Milestone M6: the cross-session tools (FR-012, FR-012a; U97–U103) ---

fn read_lines(arguments: Value) -> u16 {
    match parse_call("read_session_output", &arguments).unwrap() {
        Operation::ReadSessionOutput { session, lines } => {
            assert_eq!(session, s7());
            lines.get()
        }
        other => panic!("expected ReadSessionOutput, got {other:?}"),
    }
}

/// U97
#[test]
fn read_session_output_without_lines_means_200() {
    assert_eq!(read_lines(json!({"session": S})), 200);
    assert_eq!(read_lines(json!({"session": S, "lines": null})), 200);
    assert_eq!(LineCount::DEFAULT.get(), 200);
}

/// U98
#[test]
fn a_line_count_below_one_is_invalid_input() {
    for lines in [json!(0), json!(-1), json!(-5000)] {
        let message = invalid("read_session_output", json!({"session": S, "lines": lines}));
        assert!(message.contains("lines"), "{message}");
    }
}

/// U99, U100: the bounds themselves are accepted as they are.
#[test]
fn a_line_count_of_1_and_of_2000_are_accepted_unchanged() {
    assert_eq!(read_lines(json!({"session": S, "lines": 1})), 1);
    assert_eq!(read_lines(json!({"session": S, "lines": 2000})), 2000);
}

/// U101 (EC-16): more than the maximum is clamped, never refused.
#[test]
fn a_line_count_above_2000_is_clamped_to_2000() {
    assert_eq!(read_lines(json!({"session": S, "lines": 2001})), 2000);
    assert_eq!(
        read_lines(json!({"session": S, "lines": u64::MAX})),
        2000,
        "a count too large for the handler's own integer is still only too large"
    );
    assert_eq!(LineCount::MAX.get(), 2000);
}

#[test]
fn a_line_count_that_is_not_a_whole_number_is_invalid_input() {
    for lines in [json!("200"), json!(1.5), json!(true)] {
        invalid("read_session_output", json!({"session": S, "lines": lines}));
    }
}

/// U102 (FR-012a)
#[test]
fn send_session_input_with_empty_text_is_invalid_input() {
    let message = invalid("send_session_input", json!({"session": S, "text": ""}));
    assert!(message.contains("text"), "{message}");
    invalid("send_session_input", json!({"session": S}));
    invalid("send_session_input", json!({"session": S, "text": 7}));
}

/// U103: one character is text, and so is whitespace (assumption A-4).
#[test]
fn send_session_input_accepts_any_non_empty_text() {
    for text in ["y", " ", "line one\nline two"] {
        assert_eq!(
            parse_call("send_session_input", &json!({"session": S, "text": text})).unwrap(),
            Operation::SendSessionInput {
                session: s7(),
                text: NonEmptyText::new(text).unwrap(),
            }
        );
    }
    assert!(NonEmptyText::new("").is_none());
    assert_eq!(NonEmptyText::new("y").unwrap().as_str(), "y");
}

/// FR-012a: text made only of line breaks types nothing but the closing Enter, so it is empty.
/// A space before the line break is text (assumption A-4).
#[test]
fn send_session_input_with_only_line_breaks_is_invalid_input() {
    for text in ["\n", "\r\n", "\n\n\r"] {
        let message = invalid("send_session_input", json!({"session": S, "text": text}));
        assert!(message.contains("empty"), "{text:?}: {message}");
        assert!(NonEmptyText::new(text).is_none(), "{text:?}");
    }
    assert!(NonEmptyText::new(" \n").is_some());
}

/// A control character is a keystroke, not text: Ctrl-C would interrupt the other session without
/// the confirmation `interrupt_session` needs (FR-014), and Escape would drive its menus. Line
/// breaks and tabs are text.
#[test]
fn send_session_input_with_a_control_character_is_invalid_input() {
    for text in [
        "\u{3}",
        "stop\u{3}",
        "\u{4}",
        "up\u{1b}[A",
        "x\u{7f}",
        "a\u{0}b",
        "\u{9b}A",
    ] {
        let message = invalid("send_session_input", json!({"session": S, "text": text}));
        assert!(message.contains("control character"), "{text:?}: {message}");
        assert!(NonEmptyText::new(text).is_none(), "{text:?}");
    }
    for text in ["a\tb", "one\r\ntwo", "żółć 日本"] {
        assert!(NonEmptyText::new(text).is_some(), "{text:?}");
    }
}

#[test]
fn the_cross_session_tools_need_a_session_id() {
    invalid("read_session_output", json!({}));
    invalid("send_session_input", json!({"text": "hello"}));
    invalid("read_session_output", json!({"session": "not-a-uuid"}));
}

/// FR-018: reading changes nothing; typing does, and is audited by its target, never its text.
#[test]
fn only_send_session_input_is_a_mutating_cross_session_tool() {
    assert!(!is_mutating_tool("read_session_output"));
    assert!(is_mutating_tool("send_session_input"));
    let operation = parse_call(
        "send_session_input",
        &json!({"session": S, "text": "secret-text"}),
    )
    .unwrap();
    assert_eq!(operation.audit_target(), S);
    assert!(
        !format!("{operation:?}").contains("secret-text"),
        "input text must not be printable through Debug (FR-018)"
    );
}

/// Feature 552 (T022): attaching a worktree is gone, so neither the catalog nor the parser knows it.
#[test]
fn attach_worktree_is_not_in_the_catalog_and_does_not_parse() {
    assert!(
        tools_list().iter().all(|t| t["name"] != "attach_worktree"),
        "the catalog must not advertise attach_worktree"
    );
    assert!(!is_mutating_tool("attach_worktree"));
    assert!(parse_call("attach_worktree", &json!({"worktree": "wt-a"})).is_err());
}

#[test]
fn list_resumable_sessions_pages_with_a_limit_and_an_offset() {
    assert_eq!(
        parse_call("list_resumable_sessions", &json!({})).unwrap(),
        Operation::ListResumableSessions {
            limit: 50,
            offset: 0,
            worktree: None
        }
    );
    assert_eq!(
        parse_call(
            "list_resumable_sessions",
            &json!({"limit": 200, "offset": 7, "worktree": "wt-a"})
        )
        .unwrap(),
        Operation::ListResumableSessions {
            limit: 200,
            offset: 7,
            worktree: Some(WorktreeRef::Named("wt-a".into()))
        }
    );
    invalid("list_resumable_sessions", json!({"limit": 0}));
    invalid("list_resumable_sessions", json!({"limit": 201}));
    invalid("list_resumable_sessions", json!({"limit": -1}));
    invalid("list_resumable_sessions", json!({"offset": -1}));
    invalid("list_resumable_sessions", json!({"offset": "1"}));
    assert!(!is_mutating_tool("list_resumable_sessions"));
}

#[test]
fn the_session_tools_name_the_origin_row_fields() {
    // Feature 551, FR-013 (contracts/mcp-origin.md): an agent reading the catalog learns that rows
    // can say which session started a session.
    for name in ["list_sessions", "get_session"] {
        let tool = tools_list()
            .into_iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name} is shipped"));
        let description = tool["description"].as_str().unwrap();
        assert!(
            description.contains("origin") && description.contains("origin_state"),
            "{name} names the origin fields: {description:?}"
        );
    }
}

/// Feature 776, FR-006: `create_worktree` takes the session's definition at start. `ai_cli` is an
/// enum of every CLI's tool name, `prompt` a string, and neither is required.
#[test]
fn create_worktree_schema_offers_the_session_definition() {
    let listing = micold_core::mcp::tools::list_result();
    let tool = listing["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "create_worktree")
        .unwrap();
    let props = &tool["inputSchema"]["properties"];
    let names: Vec<&str> = AiCli::ALL.iter().map(|c| c.tool_name()).collect();
    let offered: Vec<&str> = props["ai_cli"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(offered, names);
    assert_eq!(props["prompt"]["type"], "string");
    assert_eq!(
        tool["inputSchema"]["required"],
        json!([]),
        "neither is required"
    );
}

#[test]
fn create_worktree_parses_the_definition_with_either_form() {
    use micold_core::mcp::tools::StartSession;
    let literal = parse_call(
        "create_worktree",
        &json!({"branch": "feat-x", "ai_cli": "claude_code", "prompt": "go"}),
    )
    .unwrap();
    let Operation::CreateWorktree(request) = literal else {
        panic!("a create")
    };
    assert_eq!(
        request.start(),
        Some(&StartSession {
            ai_cli: Some(AiCli::ClaudeCode),
            prompt: Some("go".into()),
        })
    );
    // A prompt alone is a definition with the default CLI.
    let derived = parse_call("create_worktree", &json!({"name": "x", "prompt": "go"})).unwrap();
    let Operation::CreateWorktree(request) = derived else {
        panic!("a create")
    };
    assert_eq!(
        request.start(),
        Some(&StartSession {
            ai_cli: None,
            prompt: Some("go".into()),
        })
    );
    // Neither: the 550 call.
    let plain = parse_call("create_worktree", &json!({"branch": "feat-x"})).unwrap();
    let Operation::CreateWorktree(request) = plain else {
        panic!("a create")
    };
    assert_eq!(request.start(), None);
}

#[test]
fn create_worktree_refuses_an_unknown_ai_cli() {
    invalid(
        "create_worktree",
        json!({"branch": "feat-x", "ai_cli": "emacs"}),
    );
}

// Feature 946: `report_review_comment` (contracts/agent-tool.md).

fn report(arguments: Value) -> Operation {
    parse_call("report_review_comment", &arguments).expect("a valid report")
}

#[test]
fn report_review_comment_is_listed_after_send_session_input_as_the_contract_gives() {
    let tools = tools_list();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let at = names
        .iter()
        .position(|n| *n == "send_session_input")
        .unwrap();
    assert_eq!(names[at + 1], "report_review_comment");
    assert_eq!(
        REPORT_TOOL, "report_review_comment",
        "the batch names the tool (P9)"
    );
    let tool = &tools[at + 1];
    assert_eq!(
        tool["description"],
        "Report the outcome of one review comment the user sent you, by its reference: applied \
         when you made the change, or answered with a reply explaining why not. The comment's \
         status and your reply show next to it in the user's Changes view. Only comments of your \
         own worktree (or of the project root, for a session running there) can be reported."
    );
    assert_eq!(
        tool["annotations"],
        json!({"readOnlyHint": false, "destructiveHint": false})
    );
    assert_eq!(
        tool["inputSchema"],
        json!({
            "type": "object",
            "properties": {
                "comment": {"type": "string", "minLength": 1,
                    "description": "The comment's reference from the review message, such as C-1a2b3c4d."},
                "outcome": {"type": "string", "enum": ["applied", "answered"],
                    "description": "applied: you made the change. answered: you did not, and the reply says why."},
                "reply": {"type": "string",
                    "description": "What you did, or why not. Required for answered; at most 20000 characters."},
            },
            "required": ["comment", "outcome"],
            "additionalProperties": false,
        })
    );
    assert!(is_mutating_tool("report_review_comment"), "audited");
}

#[test]
fn report_review_comment_parses_to_an_operation_with_the_reference_and_outcome() {
    let operation =
        report(json!({"comment": " c-1A2B3C4D ", "outcome": "answered", "reply": "Not needed"}));
    let comment = CommentRef::parse("C-1a2b3c4d").unwrap();
    assert_eq!(
        operation,
        Operation::ReportReviewComment {
            comment,
            outcome: Outcome::Answered,
            reply: "Not needed".into(),
        }
    );
    assert_eq!(operation.tool_name(), "report_review_comment");
    assert_eq!(operation.audit_target(), "C-1a2b3c4d");
    assert!(!operation.audit_target().contains("Not needed"));
    assert_eq!(
        report(json!({"comment": "C-1a2b3c4d", "outcome": "applied"})),
        Operation::ReportReviewComment {
            comment,
            outcome: Outcome::Applied,
            reply: String::new(),
        },
        "applied needs no reply"
    );
}

#[test]
fn report_review_comment_refuses_checks_one_to_four_in_order() {
    let name = "report_review_comment";
    for bad in [
        json!({"outcome": "applied"}),
        json!({"comment": "C-12", "outcome": "applied"}),
        json!({"comment": 7, "outcome": "applied"}),
    ] {
        assert!(
            invalid(name, bad.clone()).contains("references look like C-1a2b3c4d"),
            "{bad}"
        );
    }
    let message = invalid(name, json!({"comment": "C-1a2b3c4d", "outcome": "fixed"}));
    assert!(
        message.contains("applied") && message.contains("answered"),
        "{message}"
    );
    for bad in [
        json!({"comment": "C-1a2b3c4d", "outcome": "answered"}),
        json!({"comment": "C-1a2b3c4d", "outcome": "answered", "reply": " \n "}),
    ] {
        assert!(
            invalid(name, bad.clone())
                .contains("an answered comment needs an explanation in reply"),
            "{bad}"
        );
    }
    let over = "x".repeat(20_001);
    assert!(invalid(
        name,
        json!({"comment": "C-1a2b3c4d", "outcome": "applied", "reply": over})
    )
    .contains("over the 20000-character limit"));
    report(json!({"comment": "C-1a2b3c4d", "outcome": "applied", "reply": "x".repeat(20_000)}));
}

/// Feature 802, FR-005 and FR-006: both descriptions say a person is asked to confirm, an agent is
/// refused unless it sets `allow_additional_session: true`, and why.
#[test]
fn session_start_tools_state_the_person_confirms_agent_refused_rule() {
    let tools = tools_list();
    for name in ["create_session", "start_session"] {
        let description = tools
            .iter()
            .find(|t| t["name"] == name)
            .and_then(|t| t["description"].as_str())
            .unwrap();
        assert!(
            description.contains("person is asked to confirm")
                && description.contains("agent is refused unless it sets")
                && description.contains("nobody is present to confirm"),
            "{name}: {description:?}"
        );
    }
}

/// #942 FR-001: the prompt tool is not in the catalog, so a terminal caller's `tools/call` of it
/// fails as any unlisted tool does; only a structured caller's call reaches the prompt handler.
#[test]
fn the_prompt_tool_is_not_a_catalog_tool() {
    let message = invalid(
        "permission_prompt",
        json!({"tool_name": "Bash", "input": {"command": "ls"}}),
    );
    assert_eq!(message, "unknown tool \"permission_prompt\"");
    assert!(!tools_list()
        .iter()
        .any(|tool| tool["name"] == "permission_prompt"));
    assert!(!catalog()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "permission_prompt"));
}
