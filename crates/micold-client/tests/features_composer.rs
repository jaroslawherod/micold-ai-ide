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
    CONNECTION_LOST,
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
        composer_key(&plain(NamedKey::Enter), (0, 0), 1, false, TurnPhase::Idle),
        KeyIntent::Send
    );
    assert_eq!(
        composer_key(&shift(NamedKey::Enter), (0, 0), 1, false, TurnPhase::Idle),
        KeyIntent::NewLine
    );
}

#[test]
fn enter_during_an_ime_composition_completes_it() {
    assert_eq!(
        composer_key(&plain(NamedKey::Enter), (0, 0), 1, true, TurnPhase::Idle),
        KeyIntent::Edit
    );
}

#[test]
fn up_steps_back_only_on_the_first_line_and_down_forward_only_on_the_last() {
    let up = plain(NamedKey::ArrowUp);
    let down = plain(NamedKey::ArrowDown);
    assert_eq!(
        composer_key(&up, (0, 3), 3, false, TurnPhase::Idle),
        KeyIntent::HistoryBack
    );
    assert_eq!(
        composer_key(&up, (1, 0), 3, false, TurnPhase::Idle),
        KeyIntent::Edit
    );
    assert_eq!(
        composer_key(&down, (2, 0), 3, false, TurnPhase::Idle),
        KeyIntent::HistoryForward
    );
    assert_eq!(
        composer_key(&down, (1, 0), 3, false, TurnPhase::Idle),
        KeyIntent::Edit
    );
}

#[test]
fn escape_interrupts_only_a_busy_session() {
    let escape = plain(NamedKey::Escape);
    assert_eq!(
        composer_key(&escape, (0, 0), 1, false, busy()),
        KeyIntent::Interrupt
    );
    assert_eq!(
        composer_key(&escape, (0, 0), 1, false, overdue()),
        KeyIntent::Interrupt
    );
    for phase in [TurnPhase::Idle, TurnPhase::NotRunning] {
        assert_eq!(
            composer_key(&escape, (0, 0), 1, false, phase),
            KeyIntent::Edit
        );
    }
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
            composer_key(&press, (0, 0), 1, false, busy()),
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
        composer_key(&typed, (0, 0), 1, false, busy()),
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
    let intent = composer_key(&plain(NamedKey::ArrowUp), caret, 1, false, TurnPhase::Idle);
    assert_eq!(intent, KeyIntent::HistoryBack);
    assert!(state.history_back(s, Some(&transcript)));
    let draft = state.draft(s).unwrap();
    let lines = draft.text.split('\n').count();
    let intent = composer_key(
        &plain(NamedKey::Enter),
        draft.caret,
        lines,
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
    assert!(
        state.unavailable(s, false).is_some(),
        "the composer says so"
    );

    state.input_reported(s, input(busy()));
    assert!(state.can_send(s, true), "a busy session queues it");
    state.input_reported(s, input(TurnPhase::NotRunning));
    assert!(!state.can_send(s, true), "not running");
    assert!(state.unavailable(s, true).is_some());
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
    state.apply_drop(s, dropped);
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
    assert!(state.overdue_notice(s).is_some());
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
    assert!(state.composer(s).unwrap().notice.is_some());
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
