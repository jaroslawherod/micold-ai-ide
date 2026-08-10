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
        !s.terminal_focused,
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
    let mut s = State::default();
    s.update(Message::TerminalFocused);
    assert!(s.terminal_focused);
    s.update(Message::TerminalFocusReleased);
    assert!(!s.terminal_focused);
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
    assert!(!s.terminal_focused, "precondition: starts unfocused");
    let id = Session::start_new(SessionLocation::Worktree("feat-x".to_string())).id;
    s.update(Message::SessionSelected(id));
    assert!(
        s.terminal_focused,
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
        s.terminal_focused,
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
    assert!(s.terminal_focused);
    s.update(Message::TerminalFocusReleased);
    assert!(
        !s.terminal_focused,
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
    assert!(s.terminal_focused);
    s.update(Message::SessionCloseRequested(id));
    assert!(
        !s.terminal_focused,
        "closing the displayed session leaves no terminal to focus (focus-model.md BUG-001)"
    );
    assert!(s.active_session.is_none());
}
