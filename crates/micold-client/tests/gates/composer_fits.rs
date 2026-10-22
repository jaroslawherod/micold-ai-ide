//! The composer sits under a structured session's transcript and fits its slots (feature 941,
//! T055, FR-001, FR-008, FR-010, FR-014, FR-030).
//!
//! A structured session's pane is laid out in the whole window, in both schemes: the composer's
//! words lie below the transcript's viewport and inside the window; a terminal session's pane draws
//! none of them (FR-001). A long draft grows the composer to its limit and no further, and the
//! transcript keeps most of the pane (FR-008). Busy and idle say so in words, and only a busy
//! session offers **Stop** (FR-010, FR-012). The queue's rows, the hold notice with **Send
//! queued**, the overdue notice with **Stop session**, the notice line and the chips row all draw
//! inside the pane under the transcript. The #942 status slot and the #943 completion slot take
//! space only when filled (plan I-1, I-4). An interrupted turn closes with its notice, and a
//! message sent with pictures names them (FR-014, FR-030).
//!
//! Compiled into the `layout_snapshot` binary beside `composer_parts_fit`.

use crate::support::covered_states::{session_pane, structured_pane};
use crate::support::layout::{self as lay, Overflow, StateUnderTest, WINDOW};
use crate::transcript_jump_control_fits::viewport;
use iced::widget::text_editor::{Action, Edit};
use iced::Rectangle;
use micold_client::app::{EditorAction, State};
use micold_client::features::composer::{DroppedFiles, InputState, OVERDUE};
use micold_client::ui::composer::{Seams, BUSY, MAX_HEIGHT, ON_HOLD, PLACEHOLDER, READY};
use micold_client::ui::transcript::transcript_scroll_id;
use micold_core::path_insert::ShellKind;
use micold_core::protocol::messages::{
    InterruptAsk, QueueEntryId, QueuedSummary, TurnId, TurnPhase,
};
use micold_core::session::{SessionId, SessionKind};
use micold_core::structured::attachment::DropPlan;
use micold_core::theme::{ColorScheme, ThemePreference};

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The dropped picture's name.
const PICTURE: &str = "screenshot.png";

/// The notice line's words in the crowded pose.
const NOTICE: &str = "The session service refused the message: try again.";

fn themed(mut state: State, scheme: ColorScheme) -> State {
    state.settings.theme_pref = match scheme {
        ColorScheme::Light => ThemePreference::Light,
        ColorScheme::Dark => ThemePreference::Dark,
    };
    state
}

fn pane() -> (State, SessionId) {
    structured_pane(&crate::support::transcript::fixture_lines("tool-turn"))
}

fn report(state: &mut State, id: SessionId, phase: TurnPhase, hold: bool, queue: usize) {
    let queue = (0..queue)
        .map(|n| QueuedSummary {
            id: QueueEntryId(n as u64 + 1),
            text: format!("queued message {}", n + 1),
            attachments: Vec::new(),
        })
        .collect();
    state
        .composers
        .input_reported(id, InputState { phase, hold, queue });
}

fn busy(interrupt: InterruptAsk) -> TurnPhase {
    TurnPhase::Busy {
        turn: TurnId(1),
        interrupt,
    }
}

/// Type `text` into `id`'s composer as the shell does: through its editor, then the draft.
fn type_draft(state: &mut State, id: SessionId, text: &str) {
    let editor = state.composers.editor_mut(id);
    let shown = editor.perform(&EditorAction(Action::Edit(Edit::Paste(
        std::sync::Arc::new(text.to_owned()),
    ))));
    let caret = editor.caret();
    state.composers.edited(id, shown, caret);
}

fn drop_picture(state: &mut State, id: SessionId) {
    let bytes = include_bytes!("../../../../assets/icon/png/icon-64.png").to_vec();
    state.composers.apply_drop(
        id,
        DroppedFiles {
            plan: DropPlan {
                images: vec![PICTURE.into()],
                paths: Vec::new(),
            },
            reads: vec![Ok(bytes)],
            shell: ShellKind::Posix,
            sandbox: None,
        },
    );
}

/// What the window paints for `state`, and the transcript's viewport.
fn paint(state: State, id: SessionId) -> (Vec<Overflow>, Rectangle) {
    let under = StateUnderTest::new(state);
    let transcript = viewport(&under, transcript_scroll_id(id));
    let mut renderer = lay::renderer();
    let (painted, _) = lay::painted_text_settled_at(lay::view_of(&under), &mut renderer, WINDOW);
    (painted, transcript)
}

fn find<'p>(painted: &'p [Overflow], content: &str) -> &'p Overflow {
    painted
        .iter()
        .find(|p| p.content == content)
        .unwrap_or_else(|| {
            panic!(
                "`{content}` is drawn: {:?}",
                painted.iter().map(|p| &p.content).collect::<Vec<_>>()
            )
        })
}

/// `content` is drawn under the transcript and inside the window.
fn under_the_transcript(painted: &[Overflow], transcript: Rectangle, content: &str) {
    let text = find(painted, content);
    assert!(
        text.origin.y >= transcript.y + transcript.height - TOLERANCE,
        "`{content}` at {:?} is under the transcript {transcript:?}",
        text.origin
    );
    assert!(
        text.origin.x >= -TOLERANCE
            && text.origin.x + text.natural_width.min(text.allowed_width)
                <= WINDOW.width + TOLERANCE
            && text.origin.y + text.natural_height <= WINDOW.height + TOLERANCE,
        "`{content}` at {:?} is inside the window",
        text.origin
    );
}

#[test]
fn the_composer_sits_under_a_structured_transcript_in_both_schemes() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let (mut state, id) = pane();
        report(&mut state, id, TurnPhase::Idle, false, 0);
        let (painted, transcript) = paint(themed(state, scheme), id);
        // The empty area's placeholder is drawn by the editor, not as a paragraph the painter sees.
        for part in [READY, "Send"] {
            under_the_transcript(&painted, transcript, part);
        }
    }
}

#[test]
fn a_terminal_session_has_no_composer() {
    let (state, _) = session_pane(SessionKind::Terminal, &[]);
    let under = StateUnderTest::new(state);
    let mut renderer = lay::renderer();
    let (painted, _) = lay::painted_text_settled_at(lay::view_of(&under), &mut renderer, WINDOW);
    for part in [READY, PLACEHOLDER, "Send"] {
        assert!(
            !painted.iter().any(|p| p.content == part),
            "a terminal session's pane draws no `{part}`"
        );
    }
}

#[test]
fn a_long_draft_grows_the_composer_to_its_limit_and_the_transcript_stays() {
    let (mut short, id) = pane();
    report(&mut short, id, TurnPhase::Idle, false, 0);
    let mut long = short.clone();
    type_draft(&mut short, id, "one line");
    let text: Vec<String> = (1..=400).map(|n| format!("line {n}")).collect();
    type_draft(&mut long, id, &text.join("\n"));
    let (_, short_view) = paint(short, id);
    let (_, long_view) = paint(long, id);
    let lost = short_view.height - long_view.height;
    assert!(
        lost > TOLERANCE,
        "a long draft grows the composer: {short_view:?} → {long_view:?}"
    );
    assert!(
        lost <= MAX_HEIGHT + TOLERANCE,
        "the composer stops at its limit ({MAX_HEIGHT}): the transcript lost {lost}"
    );
    assert!(
        long_view.height >= WINDOW.height / 3.0,
        "the transcript keeps most of the pane: {long_view:?}"
    );
}

#[test]
fn a_long_queue_and_a_long_draft_leave_the_transcript_and_send_in_view() {
    let (mut state, id) = pane();
    report(&mut state, id, busy(InterruptAsk::None), true, 12);
    let text: Vec<String> = (1..=400).map(|n| format!("line {n}")).collect();
    type_draft(&mut state, id, &text.join("\n"));
    let (painted, transcript) = paint(state, id);
    assert!(
        transcript.height >= WINDOW.height / 4.0,
        "the transcript keeps room above a crowded composer: {transcript:?}"
    );
    under_the_transcript(&painted, transcript, "Send");
}

#[test]
fn busy_and_idle_say_so_and_only_busy_offers_stop() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let (mut state, id) = pane();
        report(&mut state, id, busy(InterruptAsk::None), false, 0);
        let (painted, transcript) = paint(themed(state, scheme), id);
        under_the_transcript(&painted, transcript, BUSY);
        under_the_transcript(&painted, transcript, "Stop");

        let (mut state, id) = pane();
        report(&mut state, id, TurnPhase::Idle, false, 0);
        let (painted, transcript) = paint(themed(state, scheme), id);
        under_the_transcript(&painted, transcript, READY);
        assert!(
            !painted
                .iter()
                .any(|p| p.content == "Stop" || p.content == BUSY),
            "an idle session offers no Stop and says no `{BUSY}`"
        );
    }
}

#[test]
fn the_queue_hold_overdue_notice_and_chips_fit_under_the_transcript() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let (mut state, id) = pane();
        report(&mut state, id, busy(InterruptAsk::Overdue), true, 2);
        state.composers.notice(id, NOTICE.to_owned());
        drop_picture(&mut state, id);
        let (painted, transcript) = paint(themed(state, scheme), id);
        for part in [
            "1",
            "queued message 1",
            "2",
            "queued message 2",
            ON_HOLD,
            "Send queued",
            NOTICE,
            BUSY,
            OVERDUE,
            "Stop session",
            PICTURE,
            "Stop",
            "Send",
        ] {
            under_the_transcript(&painted, transcript, part);
        }
    }
}

#[test]
fn the_status_and_completion_slots_take_space_only_when_filled() {
    let (mut state, id) = pane();
    report(&mut state, id, TurnPhase::Idle, false, 0);
    let roles = micold_core::tokens::roles(ColorScheme::Light);
    let height = |seams: Seams<'_>| {
        let renderer = lay::renderer();
        let records = lay::resolve(
            micold_client::ui::composer::view(&state, id, roles, seams),
            &renderer,
        );
        records
            .iter()
            .find(|r| r.path.is_empty())
            .expect("a root record")
            .height
    };
    let empty = height(Seams::default());
    let filled = height(Seams {
        status: None,
        waiting: false,
        completion: Some(iced::widget::text("completion").into()),
    });
    assert!(
        filled > empty + TOLERANCE,
        "a filled completion slot takes room the empty one does not: {empty} vs {filled}"
    );
    let with_status = height(Seams {
        status: Some(iced::widget::text("mode").into()),
        waiting: false,
        completion: None,
    });
    assert!(
        (with_status - empty).abs() <= TOLERANCE,
        "the status slot sits in the status row, adding no row: {empty} vs {with_status}"
    );
}

#[test]
fn an_interrupted_turn_closes_with_its_notice_and_a_message_names_its_pictures() {
    use micold_core::structured::{SequencedEvent, StructuredEvent};
    let (mut state, id) = structured_pane(&[]);
    let notes = [
        StructuredEvent::UserSent {
            text: "Describe this picture".to_owned(),
            attachments: vec![PICTURE.to_owned()],
        },
        StructuredEvent::InterruptAsked,
    ];
    let lines = crate::support::transcript::fixture_lines("interrupt-turn");
    let events: Vec<SequencedEvent> = notes
        .into_iter()
        .chain(
            crate::support::transcript::sequenced(&lines, 0)
                .into_iter()
                .map(|e| e.event),
        )
        .zip(1..)
        .map(|(event, seq)| SequencedEvent { seq, event })
        .collect();
    let _jobs = state.structured_events(id, 0, &events, false, &micold_client::ui::syntax::GRAMMAR);
    let (painted, _) = paint(state, id);
    find(&painted, micold_client::ui::transcript::INTERRUPTED);
    find(
        &painted,
        &format!("{}: {PICTURE}", micold_client::ui::transcript::ATTACHED),
    );
}
