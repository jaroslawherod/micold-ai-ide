//! Which session the window reports having in view, read off the application's own state (feature
//! 039, FR-001, FR-002, W1.1).
//!
//! `tests/features_attention.rs` hands the attention feature its facts as values. This file asks
//! where the facts come from: `State::view_facts` fills them from what the window shows, and
//! `State::view_report` is what the shell calls after every message. Whether the window has
//! keyboard focus is the binary's fact, so each test passes it in, as the shell does.

use micold_client::app::{Message, State, WindowFacts};
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_core::attention::ViewFacts;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WindowView;
use micold_core::session::{AiCli, Session, SessionId, SessionLocation, ShellInstanceId};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

/// The active project with one worktree, and one session started in it and selected.
fn state_with_selected_session() -> (State, SessionId) {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.workspace.record_user_created(&path, "feat-x");
    state.worktree.worktrees.push(Worktree {
        dir_name: "feat-x".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        branch: Some("feat/x".to_string()),
        status: WorktreeStatus::Valid,
        included: false,
    });
    let session = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    state.update(Message::Session(SessionMsg::Started(session)));
    (state, id)
}

/// Open a regular terminal tab on `session` without showing it.
fn open_terminal_tab(state: &mut State, session: SessionId) -> ShellInstanceId {
    state
        .workspace
        .find_session_mut(session)
        .expect("the selected session is in the workspace")
        .1
        .open_shell_instance()
}

#[test]
fn view_facts_names_the_active_projects_selected_session_and_passes_focus_through() {
    // U176 (US1 scenario 1, FR-001).
    let (state, session) = state_with_selected_session();

    assert_eq!(
        state.view_facts(true),
        ViewFacts {
            window_focused: true,
            main_area_taken: false,
            selected: Some(session),
        }
    );
    assert_eq!(
        state.view_facts(false),
        ViewFacts {
            window_focused: false,
            main_area_taken: false,
            selected: Some(session),
        },
        "an unfocused window still has a selected session; `in_view` is what reads the focus"
    );
}

#[test]
fn view_facts_has_the_main_area_taken_while_settings_is_open() {
    // U177 (US1 scenario 9, US2 scenario 11).
    let (mut state, session) = state_with_selected_session();

    state.update(Message::Settings(SettingsMsg::Opened));

    assert_eq!(
        state.view_facts(true),
        ViewFacts {
            window_focused: true,
            main_area_taken: true,
            selected: Some(session),
        },
        "Settings fills the main area in place of the session, which stays selected"
    );
}

#[test]
fn view_facts_has_the_main_area_taken_while_the_changes_view_is_open() {
    // Feature 482 V1: the Changes view replaces the terminal pane, as Settings does, so the
    // selected session is not in view behind it.
    use micold_client::features::changes;
    let (mut state, session) = state_with_selected_session();

    changes::update(
        &mut state.changes,
        changes::Msg::Opened {
            project: "/repo".into(),
            entry: SessionLocation::Worktree("feat-x".to_string()),
        },
    );

    assert_eq!(
        state.view_facts(true),
        ViewFacts {
            window_focused: true,
            main_area_taken: true,
            selected: Some(session),
        },
        "the Changes view fills the main area in place of the session, which stays selected"
    );
}

#[test]
fn view_facts_has_the_main_area_taken_while_the_window_does_not_hold_its_project() {
    // 039 BUG-568: displaced from the project, or refused it, the window shows the takeover banner
    // and no session.
    let (state, session) = state_with_selected_session();

    assert_eq!(
        state.view_facts(WindowFacts {
            focused: true,
            holds_project: false,
        }),
        ViewFacts {
            window_focused: true,
            main_area_taken: true,
            selected: Some(session),
        }
    );
}

#[test]
fn showing_another_tab_of_the_selected_session_leaves_view_facts_equal() {
    // U178 (US1 scenario 10, US2 scenario 12).
    let (mut state, session) = state_with_selected_session();
    let tab = open_terminal_tab(&mut state, session);
    let on_the_ai_tab = state.view_facts(true);
    assert_eq!(
        on_the_ai_tab.selected,
        Some(session),
        "the precondition: there is a selected session for a tab to belong to"
    );

    state.update(Message::Session(SessionMsg::ShellInstanceSelected(
        session, tab,
    )));

    assert_eq!(
        state.view_facts(true),
        on_the_ai_tab,
        "a regular terminal tab of the selected session is that session, still"
    );
}

#[test]
fn with_settings_filling_the_main_area_the_window_reports_nothing_in_view() {
    // A9 (US1 scenario 9, FR-001): the service then counts the selected session's change, because
    // no window has it in view.
    let (mut state, session) = state_with_selected_session();
    assert_eq!(
        state.view_report(true),
        Some(WindowView {
            focused: true,
            in_view: Some(session),
        }),
        "before Settings opens the focused window has its selected session in view"
    );

    state.update(Message::Settings(SettingsMsg::Opened));

    assert_eq!(
        state.view_report(true),
        Some(WindowView {
            focused: true,
            in_view: None,
        })
    );
}

#[test]
fn a_selected_session_stays_reported_in_view_whichever_of_its_tabs_is_shown() {
    // A10 (US1 scenario 10, FR-002): the service keeps counting nothing for it, because the last
    // report it holds still names the session and no other follows.
    let (mut state, session) = state_with_selected_session();
    let tab = open_terminal_tab(&mut state, session);
    let in_view = WindowView {
        focused: true,
        in_view: Some(session),
    };
    assert_eq!(state.view_report(true), Some(in_view));

    state.update(Message::Session(SessionMsg::ShellInstanceSelected(
        session, tab,
    )));

    assert_eq!(
        state.view_report(true),
        None,
        "showing another tab is not a change of what the window has in view"
    );
    assert_eq!(
        state.attention.sent_view,
        Some(in_view),
        "what the service was last told still names the session"
    );
}
