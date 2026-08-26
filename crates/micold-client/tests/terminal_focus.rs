//! Focus-routing and keyboard-holder contract tests.
//!
//! The contract is `specs/023-terminal-focus-flow/contracts/focus-model.md` (v2), which supersedes
//! feature 006's file of the same name, including its BUG-001 amendment. What 006 wrote and 023
//! kept verbatim is the **routing rule** — a focused terminal takes the keys and the app's
//! shortcuts stand down; an unfocused one lets no key reach any PTY — and the **write gate**. What
//! 023 replaced is *when* the terminal is focused: `terminal_focused` is a derived question now,
//! not a stored bool, so the tests below that once set a field drive messages instead.
//!
//! The write-gate itself (FR-012a: discard input to a non-`Running` session) moved daemon-side
//! when feature 010 introduced the session daemon — the daemon drops input for any session not
//! present in its live registry (see `micold-daemon`'s `DaemonState::session_input`); the client
//! no longer tracks process liveness for this purpose, so there is no client-side
//! `should_write_to` left to test here.

use micold_client::app::{route_key, KeyRouting, State};
use micold_client::keymap::KeyOutput;
use micold_core::session::{AiCli, SessionLocation};

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

/// Settings stopped being a floating surface with feature 027 (FR-026), so the registry no longer
/// answers for it — Escape has to leave the view by a route of its own, and this is that route.
#[test]
fn escape_leaves_the_settings_view() {
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
        AiCli::ClaudeCode,
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
    let id = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    )
    .id;
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
        AiCli::ClaudeCode,
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
        Session::start_new(
            SessionLocation::Worktree("feat-x".to_string()),
            AiCli::ClaudeCode,
        )
        .id,
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
    let session = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    );
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
        included: false,
    }];
    let session = Session::start_new(
        SessionLocation::Worktree("feat-a".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.active_session = Some(id);
    state
}

#[test]
fn the_current_session_is_marked_whether_or_not_its_terminal_has_focus() {
    let mut state = state_with_current_session();
    let marked = state.active_session;

    // Feature 023 made `terminal_focused` a derived question, so the two states are reached by
    // driving the messages rather than by assigning a field.
    for focused in [true, false] {
        state.update(if focused {
            Message::TerminalFocused
        } else {
            Message::TerminalFocusReleased
        });
        assert_eq!(
            state.terminal_focused(),
            focused,
            "precondition: the keyboard is where this iteration says it is"
        );
        assert_eq!(
            state.active_session, marked,
            "the mark says which session the main area is showing, not where the keyboard is \
             going. The two are independent facts: a released terminal is still the session you \
             are looking at, and tying them together would leave the panel unmarked in exactly \
             the case this feature exists for (FR-014)"
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
}

// ---- Feature 023: the keyboard holder is derived, not stored ----
//
// `terminal_focused` is a question now, not a field. These cover the terms the predicate is a
// conjunction of, one at a time — the point of deriving it is that no reducer arm has to remember
// them, so what is tested is the answer rather than seven assignments agreeing.
// See `specs/023-terminal-focus-flow/contracts/focus-model.md` (v2).

use micold_client::app::Message;
use micold_client::features::window::FieldId;
use micold_core::session::Session;

/// A state showing one session's terminal, with nothing else claiming the keyboard.
fn showing_a_terminal() -> State {
    let mut s = State::default();
    s.update(Message::SessionStarted(Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
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
    let first = Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    );
    let second = Session::start_new(
        SessionLocation::Worktree("feat-y".to_string()),
        AiCli::ClaudeCode,
    );
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

// ---- US2: leaving the window and coming back changes nothing (FR-013–FR-015) ----

#[test]
fn window_focus_changes_no_focus_term() {
    // This story is satisfied by writing *nothing*. There is no suspended holder to restore
    // because nothing was suspended: none of the predicate's terms is touched by the window
    // gaining or losing OS focus, so the answer on return is the answer from before by
    // construction. The test exists so that a future "helpful" restore fails loudly.
    for released in [false, true] {
        let mut s = showing_a_terminal();
        if released {
            s.update(Message::TerminalFocusReleased);
        }
        let before = (
            s.terminal_focused(),
            s.terminal_released,
            s.focused_field,
            s.active_session,
        );

        s.update(Message::WindowFocusChanged(false));
        s.update(Message::WindowFocusChanged(true));

        assert_eq!(
            (
                s.terminal_focused(),
                s.terminal_released,
                s.focused_field,
                s.active_session
            ),
            before,
            "a window focus round trip must leave the keyboard exactly where it was \
             (released={released}; FR-013–FR-015)"
        );
    }
}

#[test]
fn a_release_survives_leaving_the_window() {
    // The half that a "restore the terminal on return" rule would get wrong: the user handed the
    // keyboard back on purpose, and coming back is not a request to undo that (FR-015, FR-021).
    let mut s = showing_a_terminal();
    s.update(Message::TerminalFocusReleased);
    s.update(Message::WindowFocusChanged(false));
    s.update(Message::WindowFocusChanged(true));
    assert!(
        !s.terminal_focused(),
        "returning to the window must not hand the keyboard back to a terminal the user released"
    );
}

#[test]
fn a_field_still_holds_the_keyboard_after_a_window_round_trip() {
    // A half-typed dialog field has to survive an alt-tab (spec US2 scenario 3).
    let mut s = showing_a_terminal();
    s.update(Message::FieldFocusChanged(FieldId::AddWorktreeName, true));
    s.update(Message::WindowFocusChanged(false));
    s.update(Message::WindowFocusChanged(true));
    assert_eq!(s.focused_field, Some(FieldId::AddWorktreeName));
    assert!(
        !s.terminal_focused(),
        "and the terminal has not taken it back"
    );
}

// ---- US3: every navigation that displays a terminal lands ready to type (FR-011, FR-021a) ----

#[test]
fn every_navigation_to_a_terminal_clears_a_release() {
    // The gap this closes: selecting and starting already focused, so the user learned to expect
    // it — and then the pane switches, the instance controls and a project switch each left them
    // looking at a terminal that ignored the keyboard. An explicit release is about the present
    // moment, not a property a session carries, so deliberately going to a terminal ends it.
    let shell = micold_core::session::ShellInstanceId(1);
    /// A navigation, built from the state it is applied to (some name the displayed session).
    type Navigation = (&'static str, Box<dyn Fn(&State) -> Message>);

    let navigations: Vec<Navigation> = vec![
        (
            "SessionStarted",
            Box::new(|_: &State| {
                Message::SessionStarted(Session::start_new(
                    SessionLocation::Worktree("feat-y".to_string()),
                    AiCli::ClaudeCode,
                ))
            }),
        ),
        (
            "SessionSelected",
            Box::new(|s: &State| Message::SessionSelected(s.active_session.expect("displayed"))),
        ),
        (
            "TerminalAiCliSelected",
            Box::new(|s: &State| {
                Message::TerminalAiCliSelected(s.active_session.expect("displayed"))
            }),
        ),
        (
            "ShellInstanceOpenRequested",
            Box::new(|_: &State| Message::ShellInstanceOpenRequested),
        ),
        (
            "ShellInstanceSelected",
            Box::new(move |s: &State| {
                Message::ShellInstanceSelected(s.active_session.expect("displayed"), shell)
            }),
        ),
        (
            "ShellInstanceCloseRequested",
            Box::new(move |s: &State| {
                Message::ShellInstanceCloseRequested(s.active_session.expect("displayed"), shell)
            }),
        ),
    ];

    for (name, build) in navigations {
        let mut s = showing_a_terminal();
        s.update(Message::TerminalFocusReleased);
        assert!(!s.terminal_focused(), "{name}: precondition — released");

        let message = build(&s);
        s.update(message);

        assert!(
            !s.terminal_released,
            "{name} puts a terminal in front of the user, so it must clear the release (FR-011, \
             FR-021a)"
        );
        assert!(
            s.terminal_focused(),
            "{name} must leave the newly displayed terminal holding the keyboard"
        );
    }
}

#[test]
fn a_restored_session_holds_the_keyboard_at_launch() {
    // Nothing is carried over from the previous run; launch simply applies the default-holder rule
    // to whatever is displayed (FR-012a). `Default` is `terminal_released: false`, so this is a
    // property of the default rather than a step somebody has to remember on the startup path.
    assert!(
        !State::default().terminal_released,
        "the application starts with the terminal not released, so a restored session is focused"
    );
    let mut s = State::default();
    let session = Session::start_new(
        SessionLocation::Worktree("restored".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    s.workspace
        .sessions
        .entry(std::path::PathBuf::from("/p"))
        .or_default()
        .push(session);
    s.active_session = Some(id);
    assert!(
        s.terminal_focused(),
        "a restored, displayed session's terminal holds the keyboard at launch (FR-012a)"
    );
}

// ---- US4: the keyboard is never taken while you are typing somewhere else ----

/// Each surface that must take the keyboard while it is open, as (name, open, close).
fn keyboard_taking_surfaces() -> Vec<(&'static str, Message, Message)> {
    vec![
        ("about dialog", Message::AboutOpened, Message::AboutClosed),
        (
            "help menu",
            Message::HelpMenuToggled,
            Message::HelpMenuToggled,
        ),
        (
            "project switcher",
            Message::ProjectSwitcherToggled,
            Message::ProjectSwitcherToggled,
        ),
        (
            "sidebar filter panel",
            Message::SidebarFilterMenuToggled,
            Message::SidebarFilterMenuToggled,
        ),
    ]
}

#[test]
fn an_open_surface_takes_the_keyboard_and_gives_it_back() {
    // FR-004/FR-017 in one shape: while it is open it types (arrows, Escape, a filter query), so
    // the terminal must not also be receiving keys; when it closes the keyboard comes back with no
    // restore stack — the predicate simply reads true again (FR-010).
    for (name, open, close) in keyboard_taking_surfaces() {
        let mut s = showing_a_terminal();
        assert!(s.terminal_focused(), "{name}: precondition");

        s.update(open);
        assert!(
            !s.terminal_focused(),
            "{name} holds the keyboard while it is open (FR-004, FR-017)"
        );

        s.update(close);
        assert!(
            s.terminal_focused(),
            "{name} closing returns the keyboard to the terminal (FR-010)"
        );
    }
}

#[test]
fn a_release_outranks_a_closing_surface() {
    // The user handed the keyboard back on purpose before opening the dialog. Closing it must not
    // quietly undo that (FR-010's exception, FR-021).
    for (name, open, close) in keyboard_taking_surfaces() {
        let mut s = showing_a_terminal();
        s.update(Message::TerminalFocusReleased);
        s.update(open);
        s.update(close);
        assert!(
            !s.terminal_focused(),
            "{name}: an explicit release survives a surface opening and closing over it"
        );
    }
}

#[test]
fn the_terminals_own_context_menu_is_furniture() {
    // FR-007 files the pane's right-click menu with its scrollbar and status bar: it is drawn
    // inside the pane and offers the pane's own Copy and Paste, so taking the keyboard to open it
    // would mean a right-click stops the user typing. Deliberately *not* a term of the predicate
    // (research R4) — and the one exclusion, which is why it is asserted rather than assumed.
    let mut s = showing_a_terminal();
    s.update(Message::TerminalContextMenuOpened { x: 10, y: 4 });
    assert!(
        s.terminal_focused(),
        "the terminal keeps the keyboard while its own context menu is open (FR-007)"
    );
    s.update(Message::TerminalContextMenuClosed);
    assert!(s.terminal_focused());
}

#[test]
fn output_and_lifecycle_never_change_the_holder() {
    // FR-019. The failure this forbids is the worst one available: a keystroke meant for a form
    // field delivered to a shell because a background session happened to finish starting.
    let mut s = showing_a_terminal();
    s.update(Message::FieldFocusChanged(FieldId::AddWorktreeName, true));
    let before = s.terminal_focused();

    let other = Session::start_new(
        SessionLocation::Worktree("noisy".to_string()),
        AiCli::ClaudeCode,
    );
    let other_id = other.id;
    s.update(Message::SessionStarted(other));
    s.update(Message::FieldFocusChanged(FieldId::AddWorktreeName, true));
    s.update(Message::SessionRunning(other_id));
    s.update(Message::TerminalTick);

    assert_eq!(
        s.terminal_focused(),
        before,
        "output, a session reaching Running, and a tick must not move the keyboard (FR-019)"
    );
    assert_eq!(
        s.focused_field,
        Some(FieldId::AddWorktreeName),
        "the field the user is typing into still holds it (FR-018)"
    );
}

// --- Feature 025: a launch restores a session ready to type in --------------------------------
//
// Feature 023 made focus derived: a terminal holds the keyboard because a session is displayed and
// the user has not given it away. So restoring at launch focuses by construction, and that is the
// behaviour we want — the spec's first answer was the opposite, and reversing it is recorded in its
// Clarifications and in research R5.
//
// Kept as a test rather than left implicit because it is the reason `boot()` may reuse the switch
// path at all. If focus ever stops following the displayed session, this fails and says so.

#[test]
fn restoring_a_session_leaves_its_terminal_ready_to_type() {
    let mut state = state_with_current_session();
    let path = state.workspace.active.clone().unwrap();
    state.record_foreground();
    // The shape after a restart: the memory survived, the pointer did not.
    state.active_session = None;

    let outcomes = state.restore_after_activation(&path);
    micold_client::app::drain(outcomes, |o| micold_client::app::interpret(&mut state, o));

    assert!(
        state.active_session.is_some(),
        "precondition: the memory was honoured"
    );
    assert!(
        state.terminal_focused(),
        "you reopen on the session you left and can type into it. Withholding the keyboard here \
         would need a writer of `terminal_released` that no navigation has, and would make the \
         launch the one special case in a model built to remove them (FR-013, research R5)"
    );
}

#[test]
fn a_launch_that_restores_nothing_focuses_nothing() {
    let mut state = state_with_current_session();
    let path = state.workspace.active.clone().unwrap();
    // No memory, and every session closed: there is nothing to land on.
    for id in state
        .active_sessions()
        .iter()
        .map(|s| s.id)
        .collect::<Vec<_>>()
    {
        if let Some((_, session)) = state.workspace.find_session_mut(id) {
            session.archive();
        }
    }
    state.active_session = None;

    let outcomes = state.restore_after_activation(&path);
    micold_client::app::drain(outcomes, |o| micold_client::app::interpret(&mut state, o));

    assert!(state.active_session.is_none());
    assert!(
        !state.terminal_focused(),
        "focus follows a displayed session, so with none displayed there is nothing to focus — no \
         separate rule needed for the empty case"
    );
}
