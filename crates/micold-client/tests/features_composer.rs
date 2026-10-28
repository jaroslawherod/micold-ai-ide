//! The structured session's composer, in isolation (feature 941, T039–T042, data-model *Window*).
//!
//! Builds `features::composer::State` (and, for `shows_composer`, the window state that says what
//! a pane shows) and drives it as the shell does: key presses through `composer_key`, the editor's
//! text through `edited`, sends and their answers, the daemon's `StructuredInput`, and drops.

mod support;

use std::path::PathBuf;

use micold_client::app::State as Window;
use micold_client::features::composer::{
    composer_key, shows_composer, transcript_key, DroppedFiles, InputState, KeyIntent, State,
    CONNECTION_LOST, EDIT_GONE, NOT_CONNECTED, NOT_RUNNING, OVERDUE,
};
use micold_client::keymap::{Key, KeyInput, Mods, NamedKey, PhysicalKey};
use micold_core::path_insert::{plan_insertion, InsertTarget, ShellKind};
use micold_core::protocol::messages::{
    AttachmentMeta, ImageMedia, InterruptAsk, QueueEntryId, QueuedSummary, SessionProcess,
    TerminalRef, TurnId, TurnPhase,
};
use micold_core::session::{AiCli, Session, SessionId, SessionKind, SessionLocation};
use micold_core::structured::attachment::{
    admit, DropPlan, MAX_IMAGES_PER_MESSAGE, MAX_IMAGE_BYTES,
};
use micold_core::structured::transcript::Transcript;
use micold_core::structured::{SequencedEvent, StructuredEvent};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

fn key(named: NamedKey, mods: Mods) -> KeyInput {
    KeyInput {
        key: Key::Named(named),
        mods,
        text: None,
        physical: PhysicalKey::Other,
    }
}

fn plain(named: NamedKey) -> KeyInput {
    key(named, Mods::NONE)
}

fn shift(named: NamedKey) -> KeyInput {
    key(
        named,
        Mods {
            shift: true,
            ..Mods::NONE
        },
    )
}

fn busy() -> TurnPhase {
    TurnPhase::Busy {
        turn: TurnId(1),
        interrupt: InterruptAsk::None,
    }
}

fn overdue() -> TurnPhase {
    TurnPhase::Busy {
        turn: TurnId(1),
        interrupt: InterruptAsk::Overdue,
    }
}

fn input(phase: TurnPhase) -> InputState {
    InputState {
        phase,
        hold: false,
        queue: Vec::new(),
    }
}

fn queued(id: u64, text: &str, pictures: usize) -> QueuedSummary {
    QueuedSummary {
        id: QueueEntryId(id),
        text: text.into(),
        attachments: (0..pictures)
            .map(|i| AttachmentMeta {
                name: format!("p{i}.png"),
                media: ImageMedia::Png,
                size: 10,
            })
            .collect(),
    }
}

/// A composer for a fresh session that the daemon reported in `phase`.
fn composer_in(phase: TurnPhase) -> (State, SessionId) {
    let mut state = State::default();
    let session = SessionId::new();
    state.input_reported(session, input(phase));
    (state, session)
}

fn type_text(state: &mut State, session: SessionId, text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    let caret = (lines.len() - 1, lines.last().map_or(0, |l| l.len()));
    state.edited(session, text.to_string(), caret);
}

fn picture(name: &str) -> micold_core::protocol::messages::ImageAttachment {
    admit(&[], name, PNG.to_vec()).expect("a PNG is admitted")
}

fn transcript_of(messages: &[&str]) -> Transcript {
    let mut transcript = Transcript::new();
    let events: Vec<SequencedEvent> = messages
        .iter()
        .zip(0..)
        .map(|(text, seq)| SequencedEvent {
            seq,
            event: StructuredEvent::UserSent {
                text: (*text).to_string(),
                attachments: Vec::new(),
            },
        })
        .collect();
    transcript.apply(0, &events, true);
    transcript
}

fn text_of(state: &State, session: SessionId) -> String {
    state
        .draft(session)
        .map(|d| d.text.clone())
        .unwrap_or_default()
}

// ---- T039: keys ----

#[test]
fn enter_sends_and_shift_enter_breaks_the_line() {
    assert_eq!(
        composer_key(
            &plain(NamedKey::Enter),
            (0, 0),
            1,
            false,
            false,
            TurnPhase::Idle
        ),
        KeyIntent::Send
    );
    assert_eq!(
        composer_key(
            &shift(NamedKey::Enter),
            (0, 0),
            1,
            false,
            false,
            TurnPhase::Idle
        ),
        KeyIntent::NewLine
    );
}

#[test]
fn enter_during_an_ime_composition_completes_it() {
    assert_eq!(
        composer_key(
            &plain(NamedKey::Enter),
            (0, 0),
            1,
            true,
            false,
            TurnPhase::Idle
        ),
        KeyIntent::Edit
    );
}

#[test]
fn up_steps_back_only_on_the_first_line_and_down_forward_only_on_the_last() {
    let up = plain(NamedKey::ArrowUp);
    let down = plain(NamedKey::ArrowDown);
    assert_eq!(
        composer_key(&up, (0, 3), 3, false, false, TurnPhase::Idle),
        KeyIntent::HistoryBack
    );
    assert_eq!(
        composer_key(&up, (1, 0), 3, false, false, TurnPhase::Idle),
        KeyIntent::Edit
    );
    assert_eq!(
        composer_key(&down, (2, 0), 3, false, false, TurnPhase::Idle),
        KeyIntent::HistoryForward
    );
    assert_eq!(
        composer_key(&down, (1, 0), 3, false, false, TurnPhase::Idle),
        KeyIntent::Edit
    );
}

#[test]
fn escape_interrupts_only_a_busy_session() {
    let escape = plain(NamedKey::Escape);
    assert_eq!(
        composer_key(&escape, (0, 0), 1, false, false, busy()),
        KeyIntent::Interrupt
    );
    assert_eq!(
        composer_key(&escape, (0, 0), 1, false, false, overdue()),
        KeyIntent::Interrupt
    );
    for phase in [TurnPhase::Idle, TurnPhase::NotRunning] {
        assert_eq!(
            composer_key(&escape, (0, 0), 1, false, false, phase),
            KeyIntent::Edit
        );
    }
}

fn ctrl_c(mods: Mods) -> KeyInput {
    KeyInput {
        key: Key::Char('c'),
        mods,
        text: Some("c".into()),
        physical: PhysicalKey::Other,
    }
}

const CTRL: Mods = Mods {
    ctrl: true,
    ..Mods::NONE
};

#[test]
fn ctrl_c_interrupts_a_busy_session_without_a_selection() {
    let press = ctrl_c(CTRL);
    for phase in [busy(), overdue()] {
        assert_eq!(
            composer_key(&press, (0, 0), 1, false, false, phase),
            KeyIntent::Interrupt
        );
        assert_eq!(transcript_key(&press, phase), KeyIntent::Interrupt);
    }
}

#[test]
fn ctrl_c_stays_the_editors_when_it_must_not_interrupt() {
    let press = ctrl_c(CTRL);
    // A selection: Ctrl+C copies it.
    assert_eq!(
        composer_key(&press, (0, 0), 1, false, true, busy()),
        KeyIntent::Edit
    );
    // Composing.
    assert_eq!(
        composer_key(&press, (0, 0), 1, true, false, busy()),
        KeyIntent::Edit
    );
    // Nothing to interrupt.
    for phase in [TurnPhase::Idle, TurnPhase::NotRunning] {
        assert_eq!(
            composer_key(&press, (0, 0), 1, false, false, phase),
            KeyIntent::Edit
        );
        assert_eq!(transcript_key(&press, phase), KeyIntent::Edit);
    }
    // Another modifier beside Ctrl, or none (a plain `c` is typing).
    for mods in [
        Mods {
            shift: true,
            ..CTRL
        },
        Mods { alt: true, ..CTRL },
        Mods { logo: true, ..CTRL },
        Mods {
            logo: true,
            ..Mods::NONE
        },
        Mods::NONE,
    ] {
        let press = ctrl_c(mods);
        assert_eq!(
            composer_key(&press, (0, 0), 1, false, false, busy()),
            KeyIntent::Edit,
            "{mods:?}"
        );
        assert_eq!(transcript_key(&press, busy()), KeyIntent::Edit, "{mods:?}");
    }
}

#[test]
fn an_editor_reports_whether_text_is_selected() {
    use iced::widget::text_editor::{Action, Motion};
    use micold_client::app::{Editor, EditorAction};
    let mut editor = Editor::new("hello");
    assert!(!editor.has_selection());
    editor.act(&EditorAction(Action::Select(Motion::Right)));
    assert!(editor.has_selection());
}

#[test]
fn no_composer_key_needs_a_platform_modifier() {
    let presses = [
        (plain(NamedKey::Enter), KeyIntent::Send),
        (shift(NamedKey::Enter), KeyIntent::NewLine),
        (plain(NamedKey::ArrowUp), KeyIntent::HistoryBack),
        (plain(NamedKey::ArrowDown), KeyIntent::HistoryForward),
        (plain(NamedKey::Escape), KeyIntent::Interrupt),
    ];
    for (press, intent) in presses {
        assert!(!press.mods.ctrl && !press.mods.alt && !press.mods.logo);
        assert_eq!(
            composer_key(&press, (0, 0), 1, false, false, busy()),
            intent,
            "{press:?}"
        );
    }
}

#[test]
fn other_keys_are_the_editors() {
    let typed = KeyInput {
        key: Key::Char('a'),
        mods: Mods::NONE,
        text: Some("a".into()),
        physical: PhysicalKey::Other,
    };
    assert_eq!(
        composer_key(&typed, (0, 0), 1, false, false, busy()),
        KeyIntent::Edit
    );
}

#[test]
fn escape_on_a_focused_transcript_interrupts_only_a_busy_session() {
    let escape = plain(NamedKey::Escape);
    assert_eq!(transcript_key(&escape, busy()), KeyIntent::Interrupt);
    assert_eq!(transcript_key(&escape, TurnPhase::Idle), KeyIntent::Edit);
    assert_eq!(
        transcript_key(&escape, TurnPhase::NotRunning),
        KeyIntent::Edit
    );
    assert_eq!(
        transcript_key(&plain(NamedKey::Enter), busy()),
        KeyIntent::Edit
    );
}

fn window_with(sessions: Vec<Session>) -> Window {
    let mut workspace = support::workspace_with(vec![("/fixture/project", sessions)]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    Window {
        workspace,
        ..Window::default()
    }
}

fn session_of(kind: SessionKind) -> Session {
    Session::start_new_of_kind(
        SessionLocation::Worktree("feat-00".into()),
        AiCli::ClaudeCode,
        kind,
    )
}

#[test]
fn only_a_pane_showing_a_structured_primary_shows_the_composer() {
    let (structured, terminal) = (
        session_of(SessionKind::Structured),
        session_of(SessionKind::Terminal),
    );
    let (s, t) = (structured.id, terminal.id);
    let window = window_with(vec![structured, terminal]);
    let primary = |session| TerminalRef {
        session,
        process: SessionProcess::Primary,
    };
    assert!(shows_composer(&window, Some(primary(s))));
    assert!(
        !shows_composer(&window, Some(primary(t))),
        "a terminal session"
    );
    assert!(!shows_composer(&window, None), "an empty pane");
    assert!(
        !shows_composer(
            &window,
            Some(TerminalRef {
                session: s,
                process: SessionProcess::Shell(micold_core::session::ShellInstanceId(1)),
            })
        ),
        "a structured session's shell instance stays a terminal"
    );
}

// ---- T040: history ----

#[test]
fn history_back_recalls_newest_first_down_to_the_oldest() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let transcript = transcript_of(&["first", "second", "third"]);
    for expected in ["third", "second", "first", "first"] {
        assert!(state.history_back(s, Some(&transcript)));
        assert_eq!(text_of(&state, s), expected);
    }
}

#[test]
fn stepping_forward_past_the_newest_restores_the_draft_unchanged() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let transcript = transcript_of(&["first", "second"]);
    state.edited(s, "my draft".into(), (0, 2));
    assert!(state.history_back(s, Some(&transcript)));
    assert!(state.history_back(s, Some(&transcript)));
    assert!(state.history_forward(s, Some(&transcript)));
    assert_eq!(text_of(&state, s), "second");
    assert!(state.history_forward(s, Some(&transcript)));
    let draft = state.draft(s).unwrap();
    assert_eq!((draft.text.as_str(), draft.caret), ("my draft", (0, 2)));
    assert!(
        !state.history_forward(s, Some(&transcript)),
        "past the draft, Down is a caret move again"
    );
}

#[test]
fn editing_a_recalled_message_leaves_the_history_unchanged() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let transcript = transcript_of(&["first", "second"]);
    assert!(state.history_back(s, Some(&transcript)));
    type_text(&mut state, s, "second, changed");
    assert!(state.history_back(s, Some(&transcript)));
    assert_eq!(text_of(&state, s), "first");
    assert!(state.history_forward(s, Some(&transcript)));
    assert_eq!(text_of(&state, s), "second");
    assert_eq!(
        transcript.user_messages().collect::<Vec<_>>(),
        ["first", "second"]
    );
}

#[test]
fn a_session_without_messages_keeps_up_and_down_as_caret_moves() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    type_text(&mut state, s, "draft");
    assert!(!state.history_back(s, None));
    assert!(!state.history_back(s, Some(&Transcript::new())));
    assert!(!state.history_forward(s, Some(&Transcript::new())));
    assert_eq!(text_of(&state, s), "draft");
}

#[test]
fn up_then_enter_resends_the_previous_message() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let transcript = transcript_of(&["older", "previous"]);
    let caret = state.draft(s).map(|d| d.caret).unwrap_or_default();
    let intent = composer_key(
        &plain(NamedKey::ArrowUp),
        caret,
        1,
        false,
        false,
        TurnPhase::Idle,
    );
    assert_eq!(intent, KeyIntent::HistoryBack);
    assert!(state.history_back(s, Some(&transcript)));
    let draft = state.draft(s).unwrap();
    let lines = draft.text.split('\n').count();
    let intent = composer_key(
        &plain(NamedKey::Enter),
        draft.caret,
        lines,
        false,
        false,
        TurnPhase::Idle,
    );
    assert_eq!(intent, KeyIntent::Send);
    let sent = state.take_send(s, true).expect("sendable");
    assert_eq!(sent.text, "previous");
}

// ---- T041: sends and the queue ----

#[test]
fn can_send_needs_a_sendable_draft_a_running_session_a_connection_and_no_pending_send() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    assert!(!state.can_send(s, true), "an empty draft");
    type_text(&mut state, s, "  \n\t ");
    assert!(!state.can_send(s, true), "whitespace only");
    type_text(&mut state, s, "hello");
    assert!(state.can_send(s, true));
    assert!(!state.can_send(s, false), "no connection");
    assert_eq!(
        state.unavailable(s, false),
        Some(NOT_CONNECTED),
        "the composer says so"
    );

    state.input_reported(s, input(busy()));
    assert!(state.can_send(s, true), "a busy session queues it");
    state.input_reported(s, input(TurnPhase::NotRunning));
    assert!(!state.can_send(s, true), "not running");
    assert_eq!(state.unavailable(s, true), Some(NOT_RUNNING));
    assert_eq!(text_of(&state, s), "hello", "the draft is kept");

    state.input_reported(s, input(TurnPhase::Idle));
    let draft = state.take_send(s, true).unwrap();
    state.sending(s, 7, draft);
    type_text(&mut state, s, "next");
    assert!(!state.can_send(s, true), "a send is pending");
}

#[test]
fn a_draft_of_pictures_alone_is_sendable() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let dropped = DroppedFiles {
        plan: DropPlan {
            images: vec![PathBuf::from("/d/shot.png")],
            paths: Vec::new(),
        },
        reads: vec![Ok(PNG.to_vec())],
        shell: ShellKind::Posix,
        sandbox: None,
    };
    state.apply_drop(s, dropped, true);
    assert!(state.can_send(s, true));
}

#[test]
fn a_send_empties_the_draft_and_keeps_it_pending_until_ack() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    type_text(&mut state, s, "hello");
    let draft = state.take_send(s, true).unwrap();
    assert_eq!(draft.text, "hello");
    assert_eq!(text_of(&state, s), "", "the composer is emptied");
    state.sending(s, 7, draft);
    assert!(state.composer(s).unwrap().pending.is_some());
    state.send_acked(7);
    assert!(state.composer(s).unwrap().pending.is_none());
    assert_eq!(text_of(&state, s), "");
}

#[test]
fn a_refused_send_comes_back_unchanged_with_the_reason() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    type_text(&mut state, s, "line one\nline two");
    let mut draft = state.take_send(s, true).unwrap();
    draft.images.push(picture("a.png"));
    let sent = draft.clone();
    state.sending(s, 7, draft);
    assert!(state.send_failed(7, "The session is not running."));
    let composer = state.composer(s).unwrap();
    assert_eq!(composer.draft, sent, "text, pictures and caret unchanged");
    assert!(composer.pending.is_none());
    assert_eq!(
        composer.notice.as_deref(),
        Some("The session is not running.")
    );
    assert!(!state.send_failed(7, "again"), "answered once");
}

#[test]
fn a_refused_send_keeps_what_was_typed_meanwhile_after_it() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    type_text(&mut state, s, "sent");
    let draft = state.take_send(s, true).unwrap();
    state.sending(s, 7, draft);
    type_text(&mut state, s, "typed");
    state.send_failed(7, "refused");
    let draft = state.draft(s).unwrap();
    assert_eq!(draft.text, "sent\ntyped");
    assert_eq!(draft.caret, (1, 5), "the caret stays in the typed text");
}

#[test]
fn a_lost_connection_restores_every_pending_draft() {
    let mut state = State::default();
    let (a, b) = (SessionId::new(), SessionId::new());
    for (session, req, text) in [(a, 1, "to a"), (b, 2, "to b")] {
        state.input_reported(session, input(TurnPhase::Idle));
        type_text(&mut state, session, text);
        let draft = state.take_send(session, true).unwrap();
        state.sending(session, req, draft);
    }
    state.connection_lost(&[1, 2]);
    for (session, text) in [(a, "to a"), (b, "to b")] {
        let composer = state.composer(session).unwrap();
        assert_eq!(composer.draft.text, text);
        assert!(composer.pending.is_none());
        assert_eq!(composer.notice.as_deref(), Some(CONNECTION_LOST));
    }
}

#[test]
fn stop_is_offered_only_while_busy_and_stop_session_only_while_overdue() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    assert!(!state.offers_stop(s));
    state.input_reported(s, input(busy()));
    assert!(state.offers_stop(s));
    assert!(!state.offers_stop_session(s));
    assert!(state.overdue_notice(s).is_none());
    state.input_reported(s, input(overdue()));
    assert!(state.offers_stop(s), "the user may interrupt again");
    assert!(state.offers_stop_session(s));
    assert_eq!(state.overdue_notice(s), Some(OVERDUE));
    state.input_reported(s, input(TurnPhase::Idle));
    assert!(!state.offers_stop(s));
    assert!(!state.offers_stop_session(s));
    assert!(state.overdue_notice(s).is_none(), "gone when the turn ends");
    state.input_reported(s, input(TurnPhase::NotRunning));
    assert!(!state.offers_stop(s));
}

#[test]
fn send_queued_is_offered_only_for_a_held_queue_of_a_running_session() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let held = |phase| InputState {
        phase,
        hold: true,
        queue: vec![queued(1, "later", 0)],
    };
    assert!(!state.offers_send_queued(s));
    state.input_reported(s, held(TurnPhase::Idle));
    assert!(state.offers_send_queued(s));
    state.input_reported(s, held(TurnPhase::NotRunning));
    assert!(!state.offers_send_queued(s));
}

#[test]
fn a_queued_message_moves_back_only_into_an_empty_draft() {
    let (mut state, s) = composer_in(busy());
    state.input_reported(
        s,
        InputState {
            phase: busy(),
            hold: false,
            queue: vec![queued(1, "queued", 1)],
        },
    );
    assert!(state.can_take(s, QueueEntryId(1)));
    assert!(!state.can_take(s, QueueEntryId(9)), "no such entry");
    type_text(&mut state, s, "x");
    assert!(!state.can_take(s, QueueEntryId(1)));
    type_text(&mut state, s, "");
    state.taken(s, "queued".into(), vec![picture("p0.png")]);
    let draft = state.draft(s).unwrap();
    assert_eq!(draft.text, "queued");
    assert_eq!(draft.images, vec![picture("p0.png")]);
}

#[test]
fn a_queue_edit_draft_holds_the_text_and_the_kept_pictures() {
    let (mut state, s) = composer_in(busy());
    state.input_reported(
        s,
        InputState {
            phase: busy(),
            hold: false,
            queue: vec![queued(1, "queued", 3)],
        },
    );
    assert!(state.start_queue_edit(s, QueueEntryId(1)));
    state.queue_edit_text(s, "edited".into());
    state.queue_edit_remove_attachment(s, 1);
    assert_eq!(
        state.queue_edit_request(s),
        Some((QueueEntryId(1), "edited".to_string(), vec![0, 2]))
    );
    let names: Vec<_> = state
        .composer(s)
        .and_then(|c| c.editing.as_ref())
        .map(|e| e.attachments.iter().map(|a| a.name.clone()).collect())
        .unwrap_or_default();
    assert_eq!(names, ["p0.png", "p2.png"]);
    state.queue_edit_done(s);
    assert!(state.queue_edit_request(s).is_none());
}

#[test]
fn an_edit_of_an_entry_that_left_the_queue_ends_with_a_notice() {
    let (mut state, s) = composer_in(busy());
    state.input_reported(
        s,
        InputState {
            phase: busy(),
            hold: false,
            queue: vec![queued(1, "queued", 0)],
        },
    );
    assert!(state.start_queue_edit(s, QueueEntryId(1)));
    state.input_reported(s, input(busy()));
    assert!(state.queue_edit_request(s).is_none());
    assert_eq!(
        state.composer(s).unwrap().notice.as_deref(),
        Some(EDIT_GONE)
    );
}

#[test]
fn drafts_pictures_and_carets_are_kept_per_session() {
    let mut state = State::default();
    let (a, b) = (SessionId::new(), SessionId::new());
    state.edited(a, "for a".into(), (0, 3));
    state.edited(b, "for b".into(), (0, 1));
    state.apply_drop(
        a,
        DroppedFiles {
            plan: DropPlan {
                images: vec![PathBuf::from("/d/a.png")],
                paths: Vec::new(),
            },
            reads: vec![Ok(PNG.to_vec())],
            shell: ShellKind::Posix,
            sandbox: None,
        },
        true,
    );
    let (da, db) = (state.draft(a).unwrap(), state.draft(b).unwrap());
    assert_eq!(
        (da.text.as_str(), da.caret, da.images.len()),
        ("for a", (0, 3), 1)
    );
    assert_eq!(
        (db.text.as_str(), db.caret, db.images.len()),
        ("for b", (0, 1), 0)
    );
}

#[test]
fn closing_a_session_forgets_its_composer() {
    let mut window = Window::default();
    let (a, b) = (SessionId::new(), SessionId::new());
    window.composers.edited(a, "for a".into(), (0, 5));
    window.composers.edited(b, "for b".into(), (0, 5));
    window.forget_structured(a);
    assert!(window.composers.composer(a).is_none());
    assert_eq!(text_of(&window.composers, b), "for b");
}

// ---- T042: drops (reducer half) ----

#[test]
fn dropped_paths_are_inserted_at_the_caret_as_a_terminal_drop_plans_them() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    state.edited(s, "see  please".into(), (0, 4));
    let paths = vec![PathBuf::from("/d/my file.rs"), PathBuf::from("/d/folder")];
    let expected = plan_insertion(&paths, ShellKind::Posix, InsertTarget::Host)
        .text()
        .expect("insertable");
    state.apply_drop(
        s,
        DroppedFiles {
            plan: DropPlan {
                images: Vec::new(),
                paths,
            },
            reads: Vec::new(),
            shell: ShellKind::Posix,
            sandbox: None,
        },
        true,
    );
    let draft = state.draft(s).unwrap();
    assert_eq!(draft.text, format!("see {expected} please"));
    assert_eq!(draft.caret, (0, 4 + expected.len()), "after the insertion");
    assert!(draft.images.is_empty(), "nothing attached");
}

#[test]
fn twenty_two_pictures_attach_the_first_twenty_and_name_the_rest() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let images: Vec<PathBuf> = (0..22)
        .map(|i| PathBuf::from(format!("/d/{i}.png")))
        .collect();
    state.apply_drop(
        s,
        DroppedFiles {
            reads: images.iter().map(|_| Ok(PNG.to_vec())).collect(),
            plan: DropPlan {
                images,
                paths: Vec::new(),
            },
            shell: ShellKind::Posix,
            sandbox: None,
        },
        true,
    );
    let composer = state.composer(s).unwrap();
    assert_eq!(composer.draft.images.len(), MAX_IMAGES_PER_MESSAGE);
    assert_eq!(composer.draft.images[19].name(), "19.png");
    let notice = composer.notice.as_deref().unwrap_or_default();
    assert!(
        notice.contains("20.png") && notice.contains("21.png"),
        "{notice}"
    );
    assert!(!notice.contains("19.png"), "{notice}");
}

#[test]
fn a_too_large_picture_and_a_failed_read_are_not_attached_and_named() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let mut large = PNG.to_vec();
    large.resize(MAX_IMAGE_BYTES + 1, 0);
    state.apply_drop(
        s,
        DroppedFiles {
            plan: DropPlan {
                images: vec![
                    PathBuf::from("/d/large.png"),
                    PathBuf::from("/d/gone.png"),
                    PathBuf::from("/d/fine.png"),
                ],
                paths: Vec::new(),
            },
            reads: vec![
                Ok(large),
                Err("No such file or directory".into()),
                Ok(PNG.to_vec()),
            ],
            shell: ShellKind::Posix,
            sandbox: None,
        },
        true,
    );
    let composer = state.composer(s).unwrap();
    let names: Vec<_> = composer.draft.images.iter().map(|i| i.name()).collect();
    assert_eq!(names, ["fine.png"]);
    let notice = composer.notice.as_deref().unwrap_or_default();
    assert!(
        notice.contains("large.png") && notice.contains("bytes"),
        "{notice}"
    );
    assert!(
        notice.contains("gone.png") && notice.contains("No such file or directory"),
        "{notice}"
    );
}

/// A drop of `images` (each a PNG that reads fine) and `paths` (inserted as quoted paths).
fn drop_of(images: &[&str], paths: &[&str]) -> DroppedFiles {
    let images: Vec<PathBuf> = images.iter().map(PathBuf::from).collect();
    DroppedFiles {
        reads: images.iter().map(|_| Ok(PNG.to_vec())).collect(),
        plan: DropPlan {
            images,
            paths: paths.iter().map(PathBuf::from).collect(),
        },
        shell: ShellKind::Posix,
        sandbox: None,
    }
}

// --- #1464 M1: the drop admission guard (FR-011, FR-016) and the spec 941 behaviour kept -----------

#[test]
fn a_drop_on_a_session_that_cannot_accept_input_changes_only_the_notice() {
    let (mut state, s) = composer_in(TurnPhase::NotRunning);
    state.edited(s, "draft".into(), (0, 5));
    let before = state.draft(s).unwrap().clone();
    state.apply_drop(s, drop_of(&["/d/a.png"], &["/d/b.txt"]), false);
    assert_eq!(state.draft(s), Some(&before), "draft untouched");
    assert_eq!(
        state.composer(s).unwrap().notice.as_deref(),
        Some("2 files not attached: the session is not accepting input")
    );
    state.apply_drop(s, drop_of(&["/d/a.png"], &[]), false);
    assert_eq!(
        state.composer(s).unwrap().notice.as_deref(),
        Some("1 file not attached: the session is not accepting input")
    );
}

#[test]
fn an_empty_drop_leaves_the_draft_and_the_notice_untouched() {
    for can_accept in [true, false] {
        let (mut state, s) = composer_in(TurnPhase::Idle);
        state.edited(s, "draft".into(), (0, 2));
        state.notice(s, "earlier".into());
        let before = state.draft(s).unwrap().clone();
        state.apply_drop(s, drop_of(&[], &[]), can_accept);
        assert_eq!(state.draft(s), Some(&before));
        assert_eq!(
            state.composer(s).unwrap().notice.as_deref(),
            Some("earlier")
        );
    }
}

#[test]
fn a_session_accepts_input_while_it_runs_and_not_after_it_ended() {
    let (state, s) = composer_in(TurnPhase::Idle);
    assert!(state.accepts_input(s));
    let (state, s) = composer_in(TurnPhase::NotRunning);
    assert!(!state.accepts_input(s));
    assert!(!State::default().accepts_input(SessionId::new()));
}

#[test]
fn several_pictures_dropped_at_once_attach_in_drop_order() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    state.apply_drop(s, drop_of(&["/d/c.png", "/d/a.png", "/d/b.png"], &[]), true);
    let names: Vec<_> = state
        .draft(s)
        .unwrap()
        .images
        .iter()
        .map(|i| i.name())
        .collect();
    assert_eq!(names, ["c.png", "a.png", "b.png"]);
}

#[test]
fn pictures_and_other_files_dropped_together_give_chips_and_quoted_paths() {
    let (mut state, s) = composer_in(TurnPhase::Idle);
    let files = drop_of(&["/d/shot.png"], &["/d/my notes.txt"]);
    let expected = plan_insertion(&files.plan.paths, ShellKind::Posix, InsertTarget::Host)
        .text()
        .expect("insertable");
    state.apply_drop(s, files, true);
    let draft = state.draft(s).unwrap();
    assert_eq!(draft.images.len(), 1);
    assert_eq!(draft.images[0].name(), "shot.png");
    assert_eq!(draft.text, expected);
    assert_eq!(state.composer(s).unwrap().notice, None);
}

#[test]
fn a_drop_on_one_session_leaves_another_sessions_composer_alone() {
    let (mut state, a) = composer_in(TurnPhase::Idle);
    let b = SessionId::new();
    state.input_reported(b, input(TurnPhase::Idle));
    state.apply_drop(a, drop_of(&["/d/a.png"], &["/d/x"]), true);
    let other = state.draft(b).unwrap();
    assert!(other.images.is_empty() && other.text.is_empty());
    assert_eq!(state.composer(b).unwrap().notice, None);
}

// --- #1274 T035: the shared text area's save chord and `.on_press` -------------------------------

mod text_area_keys {
    use iced::advanced::widget::Tree;
    use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
    use iced::keyboard::{self, key, Modifiers};
    use iced::widget::text_editor::Content;
    use iced::{Element, Event, Point, Rectangle, Size};
    use micold_material::TextArea;

    #[derive(Debug, Clone, PartialEq)]
    enum M {
        Edited,
        Saved,
        Claimed(&'static str),
    }

    const ROOM: Size = Size {
        width: 400.0,
        height: 200.0,
    };

    /// The messages a text area over "note" sends for a click in it, then `presses`, as the
    /// runtime delivers them; `claim` is its `.on_press`, when set.
    fn messages(
        presses: &[Event],
        claim: Option<fn(&iced::widget::text_editor::KeyPress) -> Option<M>>,
    ) -> Vec<M> {
        let content = Content::with_text("note");
        let renderer = super::support::layout::renderer();
        let mut area = TextArea::new(&content, micold_core::tokens::LIGHT)
            .on_action(|_| M::Edited)
            .on_submit(M::Saved);
        if let Some(claim) = claim {
            area = area.on_press(claim);
        }
        let mut element: Element<'_, M> = area.into();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, ROOM),
        );
        let inside = Point::new(20.0, 20.0);
        let click = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let mut out = Vec::new();
        for event in std::iter::once(&click).chain(presses) {
            let mut shell = Shell::new(&mut out);
            element.as_widget_mut().update(
                &mut tree,
                event,
                Layout::new(&node),
                mouse::Cursor::Available(inside),
                &renderer,
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(ROOM),
            );
        }
        out.retain(|m| *m != M::Edited);
        out
    }

    fn enter(modifiers: Modifiers) -> Event {
        let key = keyboard::Key::Named(key::Named::Enter);
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(key::Code::Enter),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    fn letter(c: &str) -> Event {
        let key = keyboard::Key::Character(c.into());
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(key::Code::KeyX),
            location: keyboard::Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some(c.into()),
            repeat: false,
        })
    }

    /// The comment box's save chord: Ctrl/Cmd+Enter saves; with Shift also held (the review's send
    /// chord) it does not (#1274 US4 s4, research R7).
    #[test]
    fn primary_enter_saves_and_primary_shift_enter_does_not() {
        assert_eq!(messages(&[enter(Modifiers::COMMAND)], None), [M::Saved]);
        assert_eq!(
            messages(&[enter(Modifiers::COMMAND | Modifiers::SHIFT)], None),
            Vec::<M>::new()
        );
    }

    /// `.on_press(..)` hears each press first: a press it claims sends its message (and never
    /// the save), one it leaves goes on to the editor's own bindings.
    #[test]
    fn on_press_sends_its_message_for_each_press_it_claims() {
        fn claim(press: &iced::widget::text_editor::KeyPress) -> Option<M> {
            match press.key.as_ref() {
                keyboard::Key::Named(key::Named::Enter) if press.modifiers.shift() => {
                    Some(M::Claimed("chord"))
                }
                keyboard::Key::Character("x") => Some(M::Claimed("x")),
                _ => None,
            }
        }
        let chord = enter(Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(
            messages(&[chord.clone(), letter("x"), chord], Some(claim)),
            [M::Claimed("chord"), M::Claimed("x"), M::Claimed("chord")]
        );
        assert_eq!(
            messages(&[letter("y"), enter(Modifiers::COMMAND)], Some(claim)),
            [M::Saved],
            "unclaimed presses keep the editor's keys and the save"
        );
    }
}
