//! The tool server's policy table (feature 034, data-model TM7; U104–U105 for milestone M3).
//!
//! `policy::decide` is evaluated before anything changes. Milestone M3 ships its first row: a
//! Default session (one running in the project root) may not create a worktree, because
//! Principle III forbids a Default session to create, modify or remove any git worktree (FR-015a).

use std::path::PathBuf;

use micold_core::mcp::errors::ErrorCategory;
use micold_core::mcp::policy::{decide, Caller, CrossSessionAccess, PolicyDecision, TargetFacts};
use micold_core::mcp::tools::{Operation, WorktreeRef};
use micold_core::session::{AiCli, SessionId, SessionLocation};
use micold_core::worktree::CreateMode;
use uuid::Uuid;

fn caller(location: SessionLocation) -> Caller {
    Caller {
        session: SessionId::from_uuid(Uuid::from_u128(1)),
        project: PathBuf::from("/work/project"),
        location,
        provider: AiCli::ClaudeCode,
    }
}

fn create_worktree() -> Operation {
    Operation::CreateWorktree {
        branch: "feat-x".into(),
        name: None,
        mode: CreateMode::NewBranch,
    }
}

fn decide_for(location: SessionLocation, operation: &Operation) -> PolicyDecision {
    decide(
        &caller(location),
        operation,
        &TargetFacts::default(),
        CrossSessionAccess::default(),
    )
}

#[test]
fn a_default_caller_is_refused_create_worktree_naming_principle_iii() {
    let decision = decide_for(SessionLocation::Default, &create_worktree());
    let PolicyDecision::Refuse(error) = decision else {
        panic!("a Default session must not create a worktree (FR-015a): {decision:?}");
    };
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert!(
        error.message.contains("Principle III"),
        "the refusal names the rule the user can look up: {}",
        error.message
    );
}

#[test]
fn a_worktree_caller_may_create_a_worktree() {
    assert_eq!(
        decide_for(SessionLocation::Worktree("b".into()), &create_worktree()),
        PolicyDecision::Proceed,
        "only the Default session is bound by Principle III"
    );
}

#[test]
fn a_default_caller_keeps_every_operation_that_touches_no_worktree() {
    for operation in [
        Operation::Whoami,
        Operation::ListWorktrees {
            include_hidden: false,
        },
        Operation::CreateSession {
            worktree: WorktreeRef::Named("b".into()),
            ai_cli: None,
            prompt: None,
        },
    ] {
        assert_eq!(
            decide_for(SessionLocation::Default, &operation),
            PolicyDecision::Proceed,
            "FR-015a: every other operation stays available to a Default session: {operation:?}"
        );
    }
}
