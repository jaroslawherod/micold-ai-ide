//! Focus-routing + write-gating contract tests (feature 006). Pure —
//! `cargo test --no-default-features`. See `contracts/focus-model.md`.

use micold_ai_ide::app::{route_key, should_write_to, KeyRouting, Overlay, State};
use micold_ai_ide::keymap::KeyOutput;
use micold_ai_ide::session::SessionLifecycle;

#[test]
fn base_state_defaults() {
    let s = State::default();
    assert!(
        !s.terminal_focused,
        "terminal must start unfocused (FR-010)"
    );
    assert!(s.settings_draft.is_none());
    assert_eq!(s.overlay, Overlay::None);
}

#[test]
fn unfocused_routes_every_key_to_the_app() {
    for out in [
        KeyOutput::Bytes(vec![0x03]),
        KeyOutput::Copy,
        KeyOutput::Paste,
        KeyOutput::ReleaseFocus,
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
fn write_gating_only_when_running() {
    assert!(should_write_to(SessionLifecycle::Running));
    assert!(!should_write_to(SessionLifecycle::Starting));
    assert!(!should_write_to(SessionLifecycle::Restarting {
        attempts: 1
    }));
    assert!(!should_write_to(SessionLifecycle::Failed));
    assert!(!should_write_to(SessionLifecycle::Idle));
}

#[test]
fn escape_closes_the_settings_overlay() {
    use micold_ai_ide::app::{on_escape, Message};
    let s = State {
        overlay: Overlay::Settings,
        ..State::default()
    };
    assert_eq!(on_escape(&s), Some(Message::SettingsCancelled));
}

#[test]
fn focus_toggles_via_messages() {
    use micold_ai_ide::app::Message;
    let mut s = State::default();
    s.update(Message::TerminalFocused);
    assert!(s.terminal_focused);
    s.update(Message::TerminalFocusReleased);
    assert!(!s.terminal_focused);
}
