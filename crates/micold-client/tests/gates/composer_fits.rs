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
use micold_client::app::{EditorAction, Message, State};
use micold_client::features::composer::{DroppedFiles, InputState, OVERDUE};
use micold_client::features::window::{FieldId, Msg as WindowMsg};
use micold_client::ui::composer::{
    Seams, BUSY, MAX_HEIGHT, ON_HOLD, PLACEHOLDER, QUEUE_MAX_HEIGHT, READY,
};
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
    let (mut idle, id) = pane();
    report(&mut idle, id, TurnPhase::Idle, false, 0);
    let mut state = idle.clone();
    report(&mut state, id, busy(InterruptAsk::None), true, 12);
    let text: Vec<String> = (1..=400).map(|n| format!("line {n}")).collect();
    type_draft(&mut state, id, &text.join("\n"));
    let header = crate::session_area_fits::header_height(state.clone());
    assert!(header > 0.0, "the session area draws its header");
    let (_, roomy) = paint(idle, id);
    let (painted, transcript) = paint(state, id);
    // The crowded composer stops at its limits and takes no more from the transcript: the queue's
    // room and the hold row (a control of the shared 40dp touch target), each with its gap, and
    // the input row's growth from its idle height, at least that same 40dp **Send**, to the text
    // area's limit (FR-008); the hold row's 40dp and the idle row's cancel out. Measured against the same pane with an idle, empty composer, so the
    // bound holds whatever fixed chrome the window draws around the pane (#943's model control
    // and Open in terminal, 1124's status bar, 1126's header); an absolute floor at one window size broke each
    // time chrome was added, while a queue without its cap still fails here (941 T055 mutant).
    let gap = micold_core::tokens::spacing::SM;
    let limits = QUEUE_MAX_HEIGHT + gap + gap + MAX_HEIGHT;
    let lost = roomy.height - transcript.height;
    assert!(
        lost <= limits + TOLERANCE,
        "a crowded composer takes at most its limits ({limits}) from the transcript: \
         {roomy:?} → {transcript:?}"
    );
    assert!(
        transcript.height > 0.0,
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
        control: None,
    });
    assert!(
        filled > empty + TOLERANCE,
        "a filled completion slot takes room the empty one does not: {empty} vs {filled}"
    );
    let with_status = height(Seams {
        status: Some(iced::widget::text("mode").into()),
        waiting: false,
        completion: None,
        control: None,
    });
    assert!(
        (with_status - empty).abs() <= TOLERANCE,
        "the status slot sits in the status row, adding no row: {empty} vs {with_status}"
    );
    let with_control = height(Seams {
        control: Some(iced::widget::text("mode").into()),
        ..Seams::default()
    });
    assert!(
        (with_control - empty).abs() <= TOLERANCE,
        "the control sits in the input row, adding no row: {empty} vs {with_control}"
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

/// The fields `messages` report as having gained the keyboard.
fn focused_fields(messages: &[Message]) -> Vec<FieldId> {
    messages
        .iter()
        .filter_map(|m| match m {
            Message::Window(WindowMsg::FieldFocusChanged(field, true)) => Some(*field),
            _ => None,
        })
        .collect()
}

/// A press on the composer's text area reports it as the window's focused field, so the window
/// stops handing keys to the pane: without the report `KeyboardElsewhere` took the editor's focus
/// back on the first key and nothing typed reached the draft (T061 visual pass, step 1).
#[test]
fn a_press_on_the_composer_takes_the_keyboard_from_the_pane() {
    let (mut state, id) = pane();
    report(&mut state, id, TurnPhase::Idle, false, 0);
    assert!(
        state.terminal_focused(),
        "the pane holds the keyboard first"
    );
    let (painted, transcript) = paint(state.clone(), id);
    let send = find(&painted, "Send");
    // Inside the text area, which starts at the pane's left; #942's mode control sits between it
    // and Send.
    let at = iced::Point::new(
        transcript.x + 100.0,
        send.origin.y + send.natural_height / 2.0,
    );
    let under = StateUnderTest::new(state.clone());
    let messages = lay::messages_after(
        lay::view_of(&under),
        &lay::renderer(),
        &[lay::Input::Press(at, iced::mouse::Button::Left)],
    );
    assert_eq!(focused_fields(&messages), vec![FieldId::Composer(id)]);
    for message in messages {
        state.update(message);
    }
    assert!(
        !state.terminal_focused(),
        "a focused composer keeps its keys from the pane"
    );
}

/// The open model picker keeps the keyboard from the pane: its field takes the focus from the
/// composer, whose text area then reports none, and without this `KeyboardElsewhere` took the
/// picker's focus back on the first key, so a typed model name never reached it (#943 T060 visual
/// pass, Part B step 5).
#[test]
fn the_open_model_picker_keeps_the_keyboard_from_the_pane() {
    let (mut state, id) = pane();
    report(&mut state, id, TurnPhase::Idle, false, 0);
    let composer =
        |focused| Message::Window(WindowMsg::FieldFocusChanged(FieldId::Composer(id), focused));
    state.update(composer(true));
    state.open_model_picker(id);
    state.update(composer(false));
    assert!(
        !state.terminal_focused(),
        "the picker's field holds the keys while it is open"
    );
    state.composers.palette.close_picker(id);
    assert!(
        state.terminal_focused(),
        "closed, and nothing else focused, the pane has them back"
    );
}

/// The queued message edited in place reports its field's focus the same way (FR-020).
#[test]
fn a_press_on_a_queued_message_being_edited_reports_its_field() {
    let (mut state, id) = pane();
    report(&mut state, id, busy(InterruptAsk::None), false, 1);
    assert!(state.composers.start_queue_edit(id, QueueEntryId(1)));
    let (painted, _) = paint(state.clone(), id);
    let cancel = find(&painted, "Cancel");
    // The field sits above the Save / Cancel row: press down the column above it.
    let presses: Vec<lay::Input> = (1..40)
        .map(|n| {
            lay::Input::Press(
                iced::Point::new(cancel.origin.x - 200.0, cancel.origin.y - 4.0 * n as f32),
                iced::mouse::Button::Left,
            )
        })
        .collect();
    let under = StateUnderTest::new(state);
    let messages = lay::messages_after(lay::view_of(&under), &lay::renderer(), &presses);
    assert!(
        focused_fields(&messages).contains(&FieldId::QueuedMessage(id)),
        "the queued message's field reports its focus: {:?}",
        focused_fields(&messages)
    );
}

/// The window's tree for `state`, laid out and settled; `tree` is reused when given, as a frame
/// does after the state moved on.
fn frame<'a>(
    under: &'a StateUnderTest,
    tree: Option<iced::advanced::widget::Tree>,
    renderer: &iced::Renderer,
) -> (
    iced::Element<'a, Message>,
    iced::advanced::widget::Tree,
    iced::advanced::layout::Node,
) {
    use iced::advanced::{layout, widget::Tree};
    let mut element = lay::view_of(under);
    let mut tree = match tree {
        Some(mut tree) => {
            tree.diff(element.as_widget());
            tree
        }
        None => Tree::new(element.as_widget()),
    };
    let limits = layout::Limits::new(iced::Size::ZERO, WINDOW);
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    lay::settle(
        &mut element,
        &mut tree,
        &node,
        renderer,
        std::time::Instant::now(),
        0..lay::SETTLE_FRAMES,
        WINDOW,
    );
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    (element, tree, node)
}

/// What `event` publishes on `element`, with the pointer at `at`.
fn deliver(
    element: &mut iced::Element<'_, Message>,
    tree: &mut iced::advanced::widget::Tree,
    node: &iced::advanced::layout::Node,
    renderer: &iced::Renderer,
    event: iced::Event,
    at: Option<iced::Point>,
) -> Vec<Message> {
    use iced::advanced::{clipboard, Layout, Shell};
    let mut published = Vec::new();
    let mut shell = Shell::new(&mut published);
    element.as_widget_mut().update(
        tree,
        &event,
        Layout::new(node),
        at.map_or(
            iced::mouse::Cursor::Unavailable,
            iced::mouse::Cursor::Available,
        ),
        renderer,
        &mut clipboard::Null,
        &mut shell,
        &Rectangle::with_size(WINDOW),
    );
    published
}

/// A message queued while busy puts a row above the text area; the text area keeps the keyboard,
/// so the next key still edits the draft (T061 visual pass run 2, step 6: before, the row took the
/// editor's place in the tree and the editor's focus went with it).
#[test]
fn the_composer_keeps_the_keyboard_when_a_queue_row_appears_above_it() {
    use iced::keyboard;
    let renderer = lay::renderer();
    let (mut state, id) = pane();
    report(&mut state, id, busy(InterruptAsk::None), false, 0);
    let (painted, transcript) = paint(state.clone(), id);
    let send = find(&painted, "Send");
    // Inside the text area, which starts at the pane's left; #942's mode control sits between it
    // and Send.
    let at = iced::Point::new(
        transcript.x + 100.0,
        send.origin.y + send.natural_height / 2.0,
    );
    let under = StateUnderTest::new(state.clone());
    let (mut element, mut tree, node) = frame(&under, None, &renderer);
    let pressed = deliver(
        &mut element,
        &mut tree,
        &node,
        &renderer,
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        Some(at),
    );
    assert_eq!(focused_fields(&pressed), vec![FieldId::Composer(id)]);
    drop(element);
    for message in pressed {
        state.update(message);
    }
    report(&mut state, id, busy(InterruptAsk::None), false, 1);

    let under = StateUnderTest::new(state);
    let (mut element, mut tree, node) = frame(&under, Some(tree), &renderer);
    let typed = deliver(
        &mut element,
        &mut tree,
        &node,
        &renderer,
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("a".into()),
            modified_key: keyboard::Key::Character("a".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyA),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: Some("a".into()),
            repeat: false,
        }),
        None,
    );
    assert!(
        typed.iter().any(|m| matches!(
            m,
            Message::Composer(micold_client::features::composer::Msg::Edited { session, .. })
                if *session == id
        )),
        "the key edits the draft once the queue row is drawn"
    );
}

/// A queued message edited in a long queue shows its **Save** and **Cancel** whole, outside the
/// queue's scrolled room (FR-020; T061 visual pass run 2, step 6: they were cut off below it).
#[test]
fn a_queued_message_edited_in_a_long_queue_shows_save_and_cancel() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let (mut state, id) = pane();
        report(&mut state, id, busy(InterruptAsk::None), false, 12);
        assert!(state.composers.start_queue_edit(id, QueueEntryId(12)));
        let (painted, transcript) = paint(themed(state, scheme), id);
        for part in ["Save", "Cancel"] {
            under_the_transcript(&painted, transcript, part);
            let text = find(&painted, part);
            assert!(
                text.clip.y <= text.origin.y + TOLERANCE
                    && text.origin.y + text.natural_height
                        <= text.clip.y + text.clip.height + TOLERANCE,
                "`{part}` at {:?} is drawn whole, not cut by {:?}",
                text.origin,
                text.clip
            );
        }
    }
}

// ---- #942 M9 (T071, T066): the mode control and *Waiting for you* in the composer ------------

/// `id`'s session in `mode`, as its summary says (#942 FR-015).
fn in_mode(state: &mut State, id: SessionId, mode: micold_core::session::PermissionMode) {
    state
        .workspace
        .find_session_mut(id)
        .expect("the pane's session")
        .1
        .permission_mode = mode;
}

/// A tool request of `id` pending in this window (#942 FR-009).
fn pending_request(state: &mut State, id: SessionId) {
    use micold_core::protocol::messages::{PermissionRequest, PromptId};
    use micold_core::structured::permission::request::RequestKind;
    state.permission_prompts.requested(PermissionRequest {
        id: PromptId(1),
        session: id,
        tool_use_id: Some("toolu_waiting".into()),
        tool_name: "Bash".into(),
        input_preview: r#"{"command":"echo hi > out.txt"}"#.into(),
        input_bytes: 31,
        kind: RequestKind::Tool,
        always_allow: None,
    });
}

#[test]
fn the_mode_control_offers_the_four_modes_and_shows_the_sessions_mode() {
    use micold_client::ui::mode_control::{MODE_CHOICES, MODE_LABEL};
    use micold_core::session::PermissionMode;
    assert_eq!(
        MODE_CHOICES.map(|c| c.0),
        PermissionMode::ALL,
        "the four modes, in the order a mode control lists them"
    );
    assert_eq!(
        MODE_CHOICES.map(|c| c.to_string()),
        PermissionMode::ALL.map(|m| m.label().to_owned()),
        "each by the name the user reads"
    );
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        for mode in PermissionMode::ALL {
            let (mut state, id) = pane();
            report(&mut state, id, TurnPhase::Idle, false, 0);
            in_mode(&mut state, id, mode);
            let (painted, transcript) = paint(themed(state, scheme), id);
            under_the_transcript(&painted, transcript, MODE_LABEL);
            under_the_transcript(&painted, transcript, mode.label());
            let (ready, send) = (find(&painted, READY), find(&painted, "Send"));
            let shown = find(&painted, mode.label());
            assert!(
                shown.origin.y > ready.origin.y && shown.origin.x < send.origin.x,
                "the mode sits in the input row, before Send: {:?} vs {:?}, {:?}",
                shown.origin,
                ready.origin,
                send.origin
            );
        }
    }
}

/// The path of the deepest node that contains what drew `content` — where a press lands on the
/// control a test found by its glyph (`Overflow::origin`'s own advice).
fn press_path(painted: &[Overflow], boxes: &[lay::LayoutRecord], content: &str) -> Vec<usize> {
    let at = find(painted, content).origin;
    boxes
        .iter()
        .filter(|b| {
            b.layer == lay::Layer::Base
                && b.width > 0.0
                && b.height > 0.0
                && at.x >= b.x
                && at.x <= b.x + b.width
                && at.y >= b.y
                && at.y <= b.y + b.height
        })
        .max_by_key(|b| b.path.len())
        .unwrap_or_else(|| panic!("a node contains the `{content}` it drew"))
        .path
        .clone()
}

/// The open list is the one part of the mode control a still window never shows: at rest the
/// control paints its own mode and nothing else, so this is what says the four modes are actually
/// *offered* there, and that picking one asks the shell for it (T071; the visual pass could not
/// capture the open list).
#[test]
fn pressing_the_mode_control_opens_the_four_modes_and_picking_one_asks_for_it() {
    use micold_client::features::permission_prompts::Msg as PromptsMsg;
    use micold_client::ui::mode_control::MODE_LABEL;
    use micold_core::session::PermissionMode;

    let (mut state, id) = pane();
    report(&mut state, id, TurnPhase::Idle, false, 0);
    in_mode(&mut state, id, PermissionMode::Default);
    let under = StateUnderTest::new(state);

    let mut renderer = lay::renderer();
    let (resting, boxes) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut renderer, WINDOW);
    let control = press_path(&resting, &boxes, MODE_LABEL);

    let mut renderer = lay::renderer();
    let opened = lay::painted_text_pressing(lay::view_of(&under), &mut renderer, &control);
    let rows: Vec<&Overflow> = PermissionMode::ALL
        .iter()
        .map(|mode| {
            opened
                .iter()
                .find(|p| p.content == mode.label() && p.layer == lay::Layer::Overlay)
                .unwrap_or_else(|| {
                    panic!(
                        "pressing `{MODE_LABEL}` opens a list offering `{}`: the overlay painted {:?}",
                        mode.label(),
                        opened
                            .iter()
                            .filter(|p| p.layer == lay::Layer::Overlay)
                            .map(|p| &p.content)
                            .collect::<Vec<_>>()
                    )
                })
        })
        .collect();
    for (mode, row) in PermissionMode::ALL.iter().zip(&rows) {
        assert!(
            row.excess() <= TOLERANCE
                && row.on_screen.x >= -TOLERANCE
                && row.on_screen.y + row.natural_height <= WINDOW.height + TOLERANCE,
            "`{}` is offered whole and inside the window: {row:?}",
            mode.label()
        );
    }

    for (mode, row) in PermissionMode::ALL.iter().zip(&rows) {
        let renderer = lay::renderer();
        let published =
            lay::messages_from_open_list(lay::view_of(&under), &renderer, &control, row.on_screen);
        assert!(
            published.contains(&Message::PermissionPrompts(PromptsMsg::ModeRequested {
                session: id,
                mode: *mode,
            })),
            "picking `{}` in the open list asks the shell for it, which sends \
             `SetPermissionMode` (Bypass after its confirmation): {published:?}",
            mode.label()
        );
    }
}

#[test]
fn choosing_bypass_sends_nothing_before_its_confirmation() {
    // What each pick asks for is pinned through the painted list by
    // `pressing_the_mode_control_opens_the_four_modes_and_picking_one_asks_for_it`.
    use micold_core::session::PermissionMode;
    let (mut state, id) = pane();
    assert_eq!(
        state
            .permission_prompts
            .request_mode(id, PermissionMode::Bypass),
        None,
        "Bypass sends nothing before its confirmation"
    );
    assert!(
        micold_client::overlay::registry::open_dialog(&state).is_some(),
        "its confirmation is the open dialog"
    );
}

#[test]
fn a_refused_mode_change_says_why_in_the_status_row() {
    const WHY: &str = "Claude did not answer the mode change.";
    let (mut state, id) = pane();
    report(&mut state, id, TurnPhase::Idle, false, 0);
    state.permission_prompts.mode_failed(id, WHY.to_owned());
    let (painted, transcript) = paint(state, id);
    under_the_transcript(&painted, transcript, WHY);
}

#[test]
fn a_session_with_a_pending_request_says_waiting_for_you() {
    use micold_client::ui::composer::WAITING;
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let (mut state, id) = pane();
        report(&mut state, id, busy(InterruptAsk::None), false, 0);
        pending_request(&mut state, id);
        let (painted, transcript) = paint(themed(state, scheme), id);
        under_the_transcript(&painted, transcript, WAITING);
        assert!(
            !painted.iter().any(|p| p.content == BUSY),
            "*{WAITING}* replaces `{BUSY}`"
        );

        let (mut state, id) = pane();
        report(&mut state, id, busy(InterruptAsk::None), false, 0);
        let (painted, _) = paint(themed(state, scheme), id);
        assert!(
            !painted.iter().any(|p| p.content == WAITING),
            "no request pending: no *{WAITING}*"
        );
    }
}
