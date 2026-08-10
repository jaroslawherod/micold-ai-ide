//! Focus-routing contract tests (feature 006). Pure — `cargo test --no-default-features`. See
//! `contracts/focus-model.md`.
//!
//! The write-gate itself (FR-012a: discard input to a non-`Running` session) moved daemon-side
//! when feature 010 introduced the session daemon — the daemon drops input for any session not
//! present in its live registry (see `micold-daemon`'s `DaemonState::session_input`); the client
//! no longer tracks process liveness for this purpose, so there is no client-side
//! `should_write_to` left to test here.

use micold_client::app::{route_key, KeyRouting, State};
use micold_client::keymap::KeyOutput;
use micold_core::session::SessionLocation;

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

#[test]
fn base_state_defaults() {
    let s = State::default();
    assert!(
        !s.terminal_focused(),
        "terminal must start unfocused (FR-010)"
    );
    assert!(s.settings_draft.is_none());
    assert_eq!(open_dialog(&s), None);
}

#[test]
fn unfocused_routes_every_key_to_the_app() {
    for out in [
        KeyOutput::Bytes(vec![0x03]),
        KeyOutput::Copy,
        KeyOutput::Paste,
        KeyOutput::ReleaseFocus,
        KeyOutput::NewTerminalInstance,
        KeyOutput::Ignore,
    ] {
        assert_eq!(route_key(false, out), KeyRouting::App);
    }
}

#[test]
fn focused_routes_bytes_to_the_terminal() {
    assert_eq!(
        route_key(true, KeyOutput::Bytes(vec![0x03])),
        KeyRouting::Write(vec![0x03])
    );
    assert_eq!(route_key(true, KeyOutput::Copy), KeyRouting::Copy);
    assert_eq!(route_key(true, KeyOutput::Paste), KeyRouting::Paste);
    assert_eq!(route_key(true, KeyOutput::Ignore), KeyRouting::Ignore);
}

#[test]
fn release_focus_never_yields_pty_bytes() {
    match route_key(true, KeyOutput::ReleaseFocus) {
        KeyRouting::ReleaseFocus => {}
        other => panic!("expected ReleaseFocus, got {other:?}"),
    }
}

#[test]
fn new_terminal_instance_chord_never_yields_pty_bytes() {
    match route_key(true, KeyOutput::NewTerminalInstance) {
        KeyRouting::NewTerminalInstance => {}
        other => panic!("expected NewTerminalInstance, got {other:?}"),
    }
}

#[test]
fn escape_closes_the_settings_overlay() {
    use micold_client::app::{on_escape, Message};
    let s = State {
        settings_draft: Some(Default::default()),
        ..State::default()
    };
    assert_eq!(on_escape(&s), Some(Message::SettingsCancelled));
}

#[test]
fn focus_toggles_via_messages() {
    use micold_client::app::Message;
    use micold_core::session::Session;
    // A session has to be displayed first: since feature 023 the predicate names `active_session`,
    // so "focused with nothing on screen" is not a state that can be reached (FR-020).
    let mut s = State::default();
    s.update(Message::SessionStarted(Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
    )));
    s.update(Message::TerminalFocused);
    assert!(s.terminal_focused());
    s.update(Message::TerminalFocusReleased);
    assert!(!s.terminal_focused());
}

#[test]
fn context_menu_opens_at_a_point_and_dismisses() {
    use micold_client::app::Message;
    let mut s = State::default();
    assert_eq!(
        s.terminal_context_menu, None,
        "the terminal context menu starts closed (FR-013)"
    );
    s.update(Message::TerminalContextMenuOpened { x: 48, y: 16 });
    assert_eq!(
        s.terminal_context_menu,
        Some((48, 16)),
        "right-click opens the context menu anchored at the clicked point"
    );
    s.update(Message::TerminalContextMenuClosed);
    assert_eq!(
        s.terminal_context_menu, None,
        "an outside click or a chosen item closes the context menu"
    );
}

// ---- Bugfix BUG-001: auto-focus the displayed session's terminal on select/start ----

#[test]
fn selecting_a_session_focuses_its_terminal() {
    use micold_client::app::Message;
    use micold_core::session::Session;
    let mut s = State::default();
    assert!(!s.terminal_focused(), "precondition: starts unfocused");
    let id = Session::start_new(SessionLocation::Worktree("feat-x".to_string())).id;
    s.update(Message::SessionSelected(id));
    assert!(
        s.terminal_focused(),
        "selecting a session must auto-focus its terminal (BUG-001, FR-010/FR-010a)"
    );
    assert_eq!(s.active_session, Some(id));
}

#[test]
fn starting_a_session_focuses_its_terminal() {
    use micold_client::app::Message;
    use micold_core::session::Session;
    let mut s = State::default();
    s.update(Message::SessionStarted(Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
    )));
    assert!(
        s.terminal_focused(),
        "starting a session must auto-focus its terminal (BUG-001, FR-010/FR-010a)"
    );
}

#[test]
fn releasing_focus_after_auto_focus_still_works() {
    use micold_client::app::Message;
    use micold_core::session::Session;
    let mut s = State::default();
    s.update(Message::SessionSelected(
        Session::start_new(SessionLocation::Worktree("feat-x".to_string())).id,
    ));
    assert!(s.terminal_focused());
    s.update(Message::TerminalFocusReleased);
    assert!(
        !s.terminal_focused(),
        "release must still return focus to the app after auto-focus (FR-011)"
    );
}

#[test]
fn closing_the_displayed_session_clears_focus() {
    use micold_client::app::Message;
    use micold_core::session::Session;
    let mut s = State::default();
    let session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    let id = session.id;
    s.update(Message::SessionStarted(session));
    assert!(s.terminal_focused());
    s.update(Message::SessionCloseRequested(id));
    assert!(
        !s.terminal_focused(),
        "closing the displayed session leaves no terminal to focus (focus-model.md BUG-001)"
    );
    assert!(s.active_session.is_none());
}

// --- Feature 024: the current-session mark is not a focus indicator ---------------------------
//
// FR-014 and FR-015, contract §4.4. Being the session in front of you, having typing focus, and
// still running are three different facts, and the panel says the first one.

/// A project with one worktree, holding one session that is current.
fn state_with_current_session() -> State {
    use micold_core::project::{Availability, Project};
    use micold_core::session::Session;
    use micold_core::worktree::{Worktree, WorktreeStatus};
    use std::path::PathBuf;

    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.worktrees = vec![Worktree {
        dir_name: "feat-a".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
        branch: Some("feat/a".to_string()),
        status: WorktreeStatus::Valid,
    }];
    let session = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.active_session = Some(id);
    state
}

#[test]
fn the_current_session_is_marked_whether_or_not_its_terminal_has_focus() {
    let mut state = state_with_current_session();
    let marked = state.active_session;

    for focused in [true, false] {
        state.terminal_focused = focused;
        assert_eq!(
            state.active_session, marked,
            "the mark says which session the main area is showing, not where the keyboard is \
             going. A project switch deliberately does not carry focus across, so tying the two \
             together would leave the panel unmarked in exactly the case this feature exists for \
             (FR-014)"
        );
        assert!(
            state.location_open(&SessionLocation::Worktree("feat-a".to_string())),
            "and its row stays open either way"
        );
    }
}

#[test]
fn a_stopped_or_failed_session_that_is_current_is_still_marked() {
    use micold_core::session::RestartDecision;

    for drive in [0usize, 1, 8] {
        let mut state = state_with_current_session();
        let id = state.active_session.unwrap();
        {
            let session = state
                .workspace
                .sessions
                .values_mut()
                .flatten()
                .find(|s| s.id == id)
                .unwrap();
            session.mark_running();
            for _ in 0..drive {
                if session.on_unexpected_exit() == RestartDecision::GiveUp {
                    break;
                }
            }
        }

        assert_eq!(
            state.active_session,
            Some(id),
            "run state and being current are independent: a session you are looking at is the one \
             you are looking at, whether it is running, stopped, interrupted or failed (FR-015)"
        );
        assert!(
            state.location_open(&SessionLocation::Worktree("feat-a".to_string())),
            "and a failing session does not lose you the row it is in — that is when you most \
             need to find it"
        );
    }
// ---- Feature 023: the keyboard holder is derived, not stored ----
//
// `terminal_focused` is a question now, not a field. These cover the terms the predicate is a
// conjunction of, one at a time — the point of deriving it is that no reducer arm has to remember
// them, so what is tested is the answer rather than seven assignments agreeing.
// See `specs/023-terminal-focus-flow/contracts/focus-model.md` (v2).

use micold_client::app::{FieldId, Message};
use micold_core::session::Session;

/// A state showing one session's terminal, with nothing else claiming the keyboard.
fn showing_a_terminal() -> State {
    let mut s = State::default();
    s.update(Message::SessionStarted(Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
    )));
    s
}

#[test]
fn a_displayed_terminal_holds_the_keyboard_by_default() {
    assert!(
        showing_a_terminal().terminal_focused(),
        "the displayed terminal is the default keyboard holder (FR-009)"
    );
}

#[test]
fn no_displayed_session_means_no_terminal_holds_the_keyboard() {
    let s = State::default();
    assert!(s.active_session.is_none(), "precondition");
    assert!(
        !s.terminal_focused(),
        "with nothing displayed there is no terminal to hold the keyboard (FR-012, FR-016)"
    );
}

#[test]
fn an_explicit_release_outranks_the_default() {
    let mut s = showing_a_terminal();
    s.update(Message::TerminalFocusReleased);
    assert!(
        !s.terminal_focused(),
        "an explicit release holds until given back or navigated away from (FR-021)"
    );
}

#[test]
fn a_focused_text_field_takes_the_keyboard_and_gives_it_back() {
    let mut s = showing_a_terminal();
    s.update(Message::FieldFocusChanged(FieldId::AddWorktreeName, true));
    assert!(
        !s.terminal_focused(),
        "a field that types holds the keyboard while it does (FR-004, FR-018)"
    );
    s.update(Message::FieldFocusChanged(FieldId::AddWorktreeName, false));
    assert!(
        s.terminal_focused(),
        "when the field finishes the keyboard returns, with no restore stack (FR-010)"
    );
}

#[test]
fn only_the_displayed_sessions_terminal_is_eligible() {
    // Two sessions exist; neither is displayed. FR-020 is structural — `active_session` is the
    // only session the predicate names — and this is what would notice a second one creeping in.
    let mut s = State::default();
    let first = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    let second = Session::start_new(SessionLocation::Worktree("feat-y".to_string()));
    s.update(Message::SessionStarted(first));
    s.update(Message::SessionStarted(second));
    assert!(
        s.terminal_focused(),
        "precondition: the second one is displayed"
    );

    s.active_session = None;
    assert!(
        !s.terminal_focused(),
        "background sessions are never eligible to hold the keyboard (FR-020)"
    );
}

#[test]
fn pressing_the_pane_wins_over_a_field_that_held_the_keyboard() {
    // The mirror of the reported bug: a press into the pane must take the keyboard *on that press*
    // (FR-008b), which FR-018 permits precisely because it is a user press. If `focus_terminal()`
    // cleared only the release, this would depend on iced's blur arriving first.
    let mut s = showing_a_terminal();
    s.update(Message::FieldFocusChanged(FieldId::RenameProjectName, true));
    assert!(!s.terminal_focused(), "precondition: the field has it");

    s.update(Message::TerminalFocused);
    assert!(
        s.terminal_focused(),
        "a press on the pane takes the keyboard from a field on that press (FR-008b)"
    );
    assert_eq!(
        s.focused_field, None,
        "and the field must not still believe it holds it"
    );
}

#[test]
fn a_late_blur_after_a_pane_press_is_a_no_op() {
    // `FieldFocusChanged(_, false)` is guarded on the field still being the focused one, so a blur
    // arriving after the press cannot undo it.
    let mut s = showing_a_terminal();
    s.update(Message::FieldFocusChanged(FieldId::RenameProjectName, true));
    s.update(Message::TerminalFocused);
    s.update(Message::FieldFocusChanged(
        FieldId::RenameProjectName,
        false,
    ));
    assert!(
        s.terminal_focused(),
        "a stale blur must not disturb the holder the press already decided (FR-008a)"
    );
}
