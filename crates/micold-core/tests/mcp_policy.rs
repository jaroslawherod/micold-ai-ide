//! The tool server's policy table (feature 034, data-model TM7; U104–U105 for milestone M3,
//! U106–U114 for milestone M4).
//!
//! `policy::decide` is evaluated before anything changes. A Default session (one running in the
//! project root) may not rename or delete a worktree, because Principle III forbids a Default
//! session to modify or remove any git worktree (FR-015a). It may create one: since constitution
//! 1.7.0 that call is the application creating the worktree on the session's behalf. An agent may not stop,
//! delete or interrupt its own session, nor delete the worktree it runs in (FR-015). The other
//! destructive operations wait for the user's confirmation (FR-014), and a refusal is always
//! decided before any confirmation.

use std::path::PathBuf;

use micold_core::mcp::errors::ErrorCategory;
use micold_core::mcp::policy::{
    decide, Caller, ConfirmedOp, CrossSessionAccess, PolicyDecision, TargetFacts,
};
use micold_core::mcp::tools::{LineCount, NonEmptyText, Operation, SessionRef, WorktreeRef};
use micold_core::session::{AiCli, SessionId, SessionLocation};
use micold_core::worktree::CreateMode;
use uuid::Uuid;

/// The caller's own session id.
const ME: u128 = 1;
/// Another session of the caller's project.
const OTHER: u128 = 2;

fn caller(location: SessionLocation) -> Caller {
    Caller {
        session: SessionId::from_uuid(Uuid::from_u128(ME)),
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

/// U104 (constitution 1.7.0, Principle III's exception): a Default session may create a worktree
/// through the tool server, in every create mode.
#[test]
fn a_default_caller_may_create_a_worktree() {
    for mode in [
        CreateMode::NewBranch,
        CreateMode::ReuseLocal,
        CreateMode::TrackRemote {
            remote: "origin".into(),
        },
    ] {
        let operation = Operation::CreateWorktree {
            branch: "feat-x".into(),
            name: None,
            mode: mode.clone(),
        };
        assert_eq!(
            decide_for(SessionLocation::Default, &operation),
            PolicyDecision::Proceed,
            "{mode:?}"
        );
    }
}

#[test]
fn a_worktree_caller_may_create_a_worktree() {
    assert_eq!(
        decide_for(SessionLocation::Worktree("b".into()), &create_worktree()),
        PolicyDecision::Proceed
    );
}

/// The exception is one verb wide: the refusal a Default caller still gets names what it may not
/// do, rename and delete, and no longer says it may not create.
#[test]
fn the_default_refusal_names_rename_and_delete_and_not_create() {
    for operation in [rename_worktree("b"), delete_worktree("b")] {
        let error = refusal(decide_for(SessionLocation::Default, &operation));
        assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
        assert!(error.message.contains("Principle III"), "{}", error.message);
        assert!(error.message.contains("rename"), "{}", error.message);
        assert!(error.message.contains("delete"), "{}", error.message);
        assert!(
            !error.message.contains("create"),
            "a Default session may create a worktree: {}",
            error.message
        );
    }
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

/// FR-015: the read-only operations proceed, absolutely, for a worktree caller and a Default one.
#[test]
fn the_read_only_operations_proceed_for_a_worktree_caller_and_a_default_caller() {
    for operation in [
        Operation::Whoami,
        Operation::ListWorktrees {
            include_hidden: true,
        },
        Operation::ListBranches,
        Operation::ListSessions { worktree: None },
        Operation::GetSession {
            session: session(ME),
        },
        Operation::GetSession {
            session: session(OTHER),
        },
    ] {
        for location in [in_worktree("b"), SessionLocation::Default] {
            assert_eq!(
                decide_for(location.clone(), &operation),
                PolicyDecision::Proceed,
                "a read-only operation needs no confirmation: {operation:?} from {location:?}"
            );
        }
    }
}

fn session(n: u128) -> SessionRef {
    SessionRef(Uuid::from_u128(n))
}

fn in_worktree(name: &str) -> SessionLocation {
    SessionLocation::Worktree(name.into())
}

fn rename_worktree(worktree: &str) -> Operation {
    Operation::RenameWorktree {
        worktree: WorktreeRef::Named(worktree.into()),
        display_name: "Renamed".into(),
    }
}

fn delete_worktree(worktree: &str) -> Operation {
    Operation::DeleteWorktree {
        worktree: WorktreeRef::Named(worktree.into()),
        stop_sessions: true,
        delete_branch: false,
    }
}

fn refusal(decision: PolicyDecision) -> micold_core::mcp::errors::OpError {
    match decision {
        PolicyDecision::Refuse(error) => error,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// U106
#[test]
fn a_default_caller_is_refused_rename_worktree_and_a_worktree_caller_may_rename() {
    let error = refusal(decide_for(SessionLocation::Default, &rename_worktree("b")));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert!(error.message.contains("Principle III"), "{}", error.message);
    assert_eq!(
        decide_for(in_worktree("a"), &rename_worktree("b")),
        PolicyDecision::Proceed
    );
    assert_eq!(
        decide_for(in_worktree("b"), &rename_worktree("b")),
        PolicyDecision::Proceed,
        "renaming the caller's own worktree changes only its display name"
    );
}

/// U107: refused, not confirmed — the user is never asked about a request policy refuses.
#[test]
fn a_default_caller_is_refused_delete_worktree_before_any_confirmation() {
    let error = refusal(decide_for(SessionLocation::Default, &delete_worktree("b")));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert!(error.message.contains("Principle III"), "{}", error.message);
}

/// U108: the session operations are decided the same from the project root as from a worktree.
#[test]
fn a_default_caller_gets_the_same_decision_as_a_worktree_caller_for_session_operations() {
    for operation in [
        Operation::StartSession {
            session: session(OTHER),
        },
        Operation::StopSession {
            session: session(OTHER),
        },
        Operation::StopSession {
            session: session(ME),
        },
        Operation::InterruptSession {
            session: session(OTHER),
        },
        Operation::InterruptSession {
            session: session(ME),
        },
        Operation::DeleteSession {
            session: session(OTHER),
        },
        Operation::DeleteSession {
            session: session(ME),
        },
        Operation::GetSession {
            session: session(OTHER),
        },
    ] {
        assert_eq!(
            decide_for(SessionLocation::Default, &operation),
            decide_for(in_worktree("b"), &operation),
            "FR-015a restricts only worktree mutations: {operation:?}"
        );
    }
}

/// U109
#[test]
fn deleting_the_callers_own_worktree_is_refused_by_policy() {
    let error = refusal(decide_for(in_worktree("b"), &delete_worktree("b")));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert!(
        error.message.contains("runs in"),
        "the refusal says why: {}",
        error.message
    );
}

/// U110: the confirmation carries the options the user is asked to allow.
#[test]
fn deleting_another_worktree_waits_for_confirmation() {
    assert_eq!(
        decide_for(in_worktree("a"), &delete_worktree("b")),
        PolicyDecision::Confirm(ConfirmedOp::DeleteWorktree {
            stop_sessions: true,
            delete_branch: false,
        })
    );
}

/// U111
#[test]
fn stop_session_on_self_is_refused_and_on_another_session_confirmed() {
    let error = refusal(decide_for(
        in_worktree("b"),
        &Operation::StopSession {
            session: session(ME),
        },
    ));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert_eq!(
        decide_for(
            in_worktree("b"),
            &Operation::StopSession {
                session: session(OTHER)
            }
        ),
        PolicyDecision::Confirm(ConfirmedOp::StopSession)
    );
}

/// U112
#[test]
fn delete_session_on_self_is_refused_and_on_another_session_confirmed() {
    let error = refusal(decide_for(
        in_worktree("b"),
        &Operation::DeleteSession {
            session: session(ME),
        },
    ));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert_eq!(
        decide_for(
            in_worktree("b"),
            &Operation::DeleteSession {
                session: session(OTHER)
            }
        ),
        PolicyDecision::Confirm(ConfirmedOp::DeleteSession)
    );
}

/// U113: interrupting its own turn would abort the call awaiting the result (decision D2).
#[test]
fn interrupt_session_on_self_is_invalid_input_and_on_another_session_confirmed() {
    let error = refusal(decide_for(
        in_worktree("b"),
        &Operation::InterruptSession {
            session: session(ME),
        },
    ));
    assert_eq!(error.category, ErrorCategory::InvalidInput);
    assert_eq!(
        decide_for(
            in_worktree("b"),
            &Operation::InterruptSession {
                session: session(OTHER)
            }
        ),
        PolicyDecision::Confirm(ConfirmedOp::InterruptSession)
    );
}

/// U114
#[test]
fn start_session_proceeds_on_any_session() {
    for target in [ME, OTHER] {
        assert_eq!(
            decide_for(
                in_worktree("b"),
                &Operation::StartSession {
                    session: session(target)
                }
            ),
            PolicyDecision::Proceed
        );
    }
}

// --- Milestone M6: the cross-session tools under the user's option (FR-015, FR-016) ---

const EVERY_ACCESS: [CrossSessionAccess; 3] = [
    CrossSessionAccess::Auto,
    CrossSessionAccess::ConfirmEachSend,
    CrossSessionAccess::Off,
];

fn read_output(target: u128) -> Operation {
    Operation::ReadSessionOutput {
        session: session(target),
        lines: LineCount::DEFAULT,
    }
}

fn send_input(target: u128) -> Operation {
    Operation::SendSessionInput {
        session: session(target),
        text: NonEmptyText::new("carry on").expect("non-empty"),
    }
}

fn decide_at(access: CrossSessionAccess, operation: &Operation) -> PolicyDecision {
    decide(
        &caller(in_worktree("b")),
        operation,
        &TargetFacts::default(),
        access,
    )
}

/// U115: reading is never confirmed; only Off closes it.
#[test]
fn read_session_output_proceeds_at_auto_and_confirm_each_send_and_is_refused_at_off() {
    assert_eq!(
        decide_at(CrossSessionAccess::Auto, &read_output(OTHER)),
        PolicyDecision::Proceed
    );
    assert_eq!(
        decide_at(CrossSessionAccess::ConfirmEachSend, &read_output(OTHER)),
        PolicyDecision::Proceed,
        "Confirm each send asks about sends only (FR-016)"
    );
    let error = refusal(decide_at(CrossSessionAccess::Off, &read_output(OTHER)));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert!(
        error.message.contains("Settings"),
        "the refusal says where the user turned it off: {}",
        error.message
    );
}

/// U116
#[test]
fn send_session_input_proceeds_at_auto_is_confirmed_at_confirm_each_send_and_refused_at_off() {
    assert_eq!(
        decide_at(CrossSessionAccess::Auto, &send_input(OTHER)),
        PolicyDecision::Proceed
    );
    assert_eq!(
        decide_at(CrossSessionAccess::ConfirmEachSend, &send_input(OTHER)),
        PolicyDecision::Confirm(ConfirmedOp::SendInput)
    );
    let error = refusal(decide_at(CrossSessionAccess::Off, &send_input(OTHER)));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert!(error.message.contains("Settings"), "{}", error.message);
}

/// U117: validation comes before policy, so the caller's own session is invalid input even when
/// the option is Off (contracts/mcp-tools.md *Order of checks*).
#[test]
fn the_cross_session_tools_on_the_callers_own_session_are_invalid_input_at_every_option_value() {
    for access in EVERY_ACCESS {
        for operation in [read_output(ME), send_input(ME)] {
            let error = refusal(decide_at(access, &operation));
            assert_eq!(
                error.category,
                ErrorCategory::InvalidInput,
                "{access:?} {operation:?}"
            );
        }
    }
}

/// FR-016 is a separate option from every other row: it changes no other decision.
#[test]
fn the_option_changes_no_decision_but_the_cross_session_tools() {
    for operation in [
        Operation::Whoami,
        Operation::GetSession {
            session: session(OTHER),
        },
        Operation::StartSession {
            session: session(OTHER),
        },
        Operation::StopSession {
            session: session(OTHER),
        },
        create_worktree(),
    ] {
        for access in EVERY_ACCESS {
            assert_eq!(
                decide_at(access, &operation),
                decide_at(CrossSessionAccess::Auto, &operation),
                "{access:?} {operation:?}"
            );
        }
    }
}

/// FR-015a restricts worktree mutations only: a Default caller reads and types like any other.
#[test]
fn a_default_caller_gets_the_same_cross_session_decisions_as_a_worktree_caller() {
    for access in EVERY_ACCESS {
        for operation in [read_output(OTHER), send_input(OTHER)] {
            assert_eq!(
                decide(
                    &caller(SessionLocation::Default),
                    &operation,
                    &TargetFacts::default(),
                    access
                ),
                decide_at(access, &operation),
                "{access:?} {operation:?}"
            );
        }
    }
}

/// Feature 582, FR-015: attaching changes a worktree's ownership, so a Default caller is refused
/// outright with the Principle III text; a worktree caller may attach, and listing resumable
/// sessions is open to every caller.
#[test]
fn a_default_caller_is_refused_attach_worktree_and_may_list_resumable_sessions() {
    let attach = Operation::AttachWorktree {
        worktree: "wt-a".into(),
    };
    let error = refusal(decide_for(SessionLocation::Default, &attach));
    assert_eq!(error.category, ErrorCategory::RefusedByPolicy);
    assert!(error.message.contains("Principle III"), "{}", error.message);
    assert!(error.message.contains("attach"), "{}", error.message);
    assert_eq!(
        decide_for(in_worktree("a"), &attach),
        PolicyDecision::Proceed
    );
    let list = Operation::ListResumableSessions {
        limit: 50,
        offset: 0,
        worktree: None,
    };
    for location in [SessionLocation::Default, in_worktree("a")] {
        assert_eq!(decide_for(location, &list), PolicyDecision::Proceed);
    }
}
