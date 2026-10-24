//! The structured session's composer, drawn under its transcript (feature 941, FR-001, T057,
//! T058).
//!
//! Top to bottom: the queued messages (`QueueRow`, or the one being edited in place), the hold
//! notice with **Send queued** (FR-022), the notice line, the status row (`BusyIndicator`, **Stop
//! session** while an interrupt is overdue, and #942's slot, I-1), the draft's pictures
//! (`AttachmentChip`), #943's completion slot (I-4), and the text area beside #942's control,
//! **Send** and, while busy, **Stop**. Everything it shows is `features::composer`'s; every control sends a
//! `composer::Msg` for the shell to carry out.
//!
//! #943 fills two of its places: the completion slot with the command palette while a `/` is
//! typed ([`palette`]), and the status row with the session's model, which opens the model picker
//! (`Typeahead`) in its place (FR-007, FR-013, FR-014).

use iced::widget::text_editor::{Binding, KeyPress, Status};
use iced::widget::{column, container, row, Row, Space};
use iced::{Alignment, Element, Length};
use micold_core::protocol::messages::TurnPhase;
use micold_core::session::SessionId;
use micold_core::tokens::{spacing, Roles};

use crate::app::{EditorAction, Message, State};
use crate::features::command_palette::{palette_key, PaletteLine, PaletteState};
use crate::features::composer::{composer_key, KeyIntent, Msg, QueueEditDraft};
use crate::features::window::{FieldId, Msg as WindowMsg};
use crate::keymap::KeyInput;
use crate::ui::cdk::focus_within::FocusWithin;
use crate::ui::focus::TrackFocus;
use crate::ui::material::terminal_pane::{to_keymap_key, to_keymap_mods, to_keymap_physical};
use crate::ui::material::{
    AttachmentChip, BusyIndicator, Button, CommandPalette, PromptCard, QueueRow, Scrollable, Text,
    TextArea, TextField, TypeRole, Typeahead, TypeaheadRow,
};
use iced::widget::text::Wrapping;
use micold_core::session::SessionKind;

/// The height the text area stops growing at, its lines then scrolling inside it (FR-008):
/// about eight lines.
pub const MAX_HEIGHT: f32 = 220.0;

/// The height the queue's rows stop growing at, scrolling inside it past that: two rows and part
/// of a third, which says there are more.
pub const QUEUE_MAX_HEIGHT: f32 = 120.0;

/// The status row's words while a turn is open (FR-010).
pub const BUSY: &str = "Working";
/// The status row's words while the session waits for a message (FR-010).
pub const READY: &str = "Ready";
/// The status row's words while the session's CLI is not running.
pub const NOT_RUNNING: &str = "Not running";
/// The status row's words while #942 waits on the user (I-1).
pub const WAITING: &str = "Waiting for you";
/// The hold notice (FR-022).
pub const ON_HOLD: &str = "Queue on hold";
/// What the empty text area shows.
pub const PLACEHOLDER: &str = "Message Claude";

/// The switch offer's dismissing button (#943 US4 scenario 1).
pub const NOT_NOW: &str = "Not now";

/// What the model picker's field says while empty (#943 FR-014).
pub const MODEL_PLACEHOLDER: &str = "Model name";

/// The model picker's width in the status row: room for a model's full name.
pub const MODEL_PICKER_WIDTH: f32 = 280.0;

/// The model picker's field of `session`, which the shell focuses as the picker opens. One per
/// session: two panes side by side each draw their own.
pub fn model_picker_field(session: SessionId) -> iced::widget::Id {
    iced::widget::Id::from(format!("composer-model-picker-{session}"))
}

/// `session`'s text area, which the shell focuses again when its model picker closes.
pub fn composer_field(session: SessionId) -> iced::widget::Id {
    iced::widget::Id::from(format!("composer-area-{session}"))
}

/// What the composer's later issues put into it (plan I-1, I-4): empty in #941.
#[derive(Default)]
pub struct Seams<'a> {
    /// #942's element at the end of the status row.
    pub status: Option<Element<'a, Message>>,
    /// #942 waits on the user: its words replace the busy label.
    pub waiting: bool,
    /// #943's element over the text area.
    pub completion: Option<Element<'a, Message>>,
    /// #942's control beside the text area, before **Stop** and **Send**: in the input row, whose
    /// height the text area sets, so it adds no row to the composer.
    pub control: Option<Element<'a, Message>>,
}

fn send(msg: Msg) -> Message {
    Message::Composer(msg)
}

/// `session`'s composer, themed by `roles`, with `seams` filled by the issues after #941.
pub fn view<'a>(
    state: &'a State,
    session: SessionId,
    roles: Roles,
    seams: Seams<'a>,
) -> Element<'a, Message> {
    let composers = &state.composers;
    let composer = composers.composer(session);
    let phase = composers.phase(session);
    let mut parts: Vec<Element<'a, Message>> = Vec::new();

    // The queue, in order (FR-017, FR-020).
    let queue = composer
        .and_then(|c| c.remote.as_ref())
        .map_or(&[][..], |r| &r.queue[..]);
    let editing = composer.and_then(|c| c.editing.as_ref());
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    let mut edited = None;
    for (index, queued) in queue.iter().enumerate() {
        let entry = queued.id;
        if let Some(edit) = editing.filter(|e| e.entry == entry) {
            edited = Some(edit);
            continue;
        }
        rows.push(
            QueueRow::new(index + 1, queued.text.clone(), roles)
                .pictures(queued.attachments.len())
                .on_edit(send(Msg::QueueEditStarted { session, entry }))
                .on_remove(send(Msg::QueueRemoveRequested { session, entry }))
                .on_move_to_composer(
                    composers
                        .can_take(session, entry)
                        .then(|| send(Msg::QueueTakeRequested { session, entry })),
                )
                .into(),
        );
    }
    if !rows.is_empty() {
        // A long queue scrolls inside its own room, so the composer never pushes the transcript
        // out of the pane (FR-008).
        parts.push(
            container(Scrollable::new(column(rows).spacing(spacing::XS), roles))
                .max_height(QUEUE_MAX_HEIGHT)
                .into(),
        );
    }
    // The message being edited sits under the queue's room, never inside it: its field and
    // **Save** / **Cancel** are taller than the room, which would hide them (T061 run 2, step 6).
    if let Some(edit) = edited {
        parts.push(queue_edit(session, edit, state.window.focused_field, roles));
    }

    // The hold notice and **Send queued** (FR-022).
    if composers.offers_send_queued(session) {
        parts.push(
            row![
                Text::new(ON_HOLD, TypeRole::Body, roles),
                Space::new().width(Length::Fill),
                Button::outlined("Send queued", roles)
                    .on_press(send(Msg::SendQueuedRequested(session))),
            ]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .into(),
        );
    }

    // The notice line: a refusal, a restore, what a drop could not attach, or why it cannot send.
    let notice = composer
        .and_then(|c| c.notice.as_deref())
        .or_else(|| composers.unavailable(session, true));
    if let Some(notice) = notice {
        parts.push(Text::new(notice, TypeRole::Caption, roles).into());
    }

    parts.push(status_row(
        state,
        session,
        phase,
        roles,
        seams.status,
        seams.waiting,
    ));

    // The draft's pictures (US5).
    if let Some(composer) = composer.filter(|c| !c.draft.images.is_empty()) {
        let chips = composer
            .draft
            .images
            .iter()
            .zip(&composer.previews)
            .enumerate()
            .map(|(index, (image, preview))| {
                AttachmentChip::new(image.name(), preview.handle(), roles)
                    .on_remove(send(Msg::AttachmentRemoved { session, index }))
                    .into()
            });
        parts.push(Row::with_children(chips).spacing(spacing::SM).wrap().into());
    }

    if let Some(completion) = seams.completion {
        parts.push(completion);
    }

    // The parts above the input row come and go; held in a column of their own, they leave the
    // input row at the same place in the tree, so the text area keeps its state (focus, caret)
    // when a queue row, a notice or a chip appears (T061 run 2, step 6).
    let input = input_row(state, session, phase, roles, seams.control);
    let composer =
        container(column![column(parts).spacing(spacing::SM), input].spacing(spacing::SM))
            .padding([spacing::SM, spacing::MD])
            .width(Length::Fill);
    // Tab from the text area to **Send** (or among the queue's actions) keeps the keyboard in the
    // window: without the region's report the area's blur handed it back to the pane, which took
    // the button's focus on the next event (T061 visual pass F5).
    FocusWithin::new(composer, move |within| {
        Message::Window(WindowMsg::FocusWithinChanged(
            FieldId::Composer(session),
            within,
        ))
    })
    .into()
}

/// Busy or ready in words and the indicator (FR-010), **Stop session** while an interrupt is
/// overdue (FR-015), and #942's slot.
fn status_row<'a>(
    state: &'a State,
    session: SessionId,
    phase: TurnPhase,
    roles: Roles,
    slot: Option<Element<'a, Message>>,
    waiting: bool,
) -> Element<'a, Message> {
    let composers = &state.composers;
    let (label, busy) = match phase {
        _ if waiting => (WAITING, false),
        TurnPhase::Busy { .. } => (BUSY, true),
        TurnPhase::Idle => (READY, false),
        TurnPhase::NotRunning => (NOT_RUNNING, false),
    };
    let mut status = row![
        BusyIndicator::new(label, roles).busy(busy),
        model_control(state, session, roles)
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    if let Some(overdue) = composers.overdue_notice(session) {
        status = status
            .push(Text::new(overdue, TypeRole::Caption, roles).width(Length::Fill))
            .push(
                Button::outlined("Stop session", roles)
                    .on_press(send(Msg::StopSessionRequested(session))),
            );
    }
    if let Some(slot) = slot {
        status = status.push(slot);
    }
    status.into()
}

/// The session's model as the CLI last reported it and whose choice it is, pressed to open the
/// model picker in its place (#943 FR-013, FR-014).
fn model_control<'a>(state: &'a State, session: SessionId, roles: Roles) -> Element<'a, Message> {
    let Some(picker) = state.composers.palette.picker(session) else {
        return Button::text(state.model_header(session), roles)
            .on_press(send(Msg::ModelPickerRequested(session)))
            .into();
    };
    let rows = picker.rows();
    let items: Vec<Option<usize>> = rows.iter().map(|r| r.item).collect();
    let rows = rows
        .into_iter()
        .map(|r| row_with_details(r.label, Vec::new(), r.details))
        .collect();
    let typeahead = Typeahead::new(
        picker.typed(),
        rows,
        move |text| send(Msg::ModelPickerTyped { session, text }),
        roles,
    )
    .id(model_picker_field(session))
    .placeholder(MODEL_PLACEHOLDER)
    .open(true)
    .highlighted(picker.enter_row())
    .selected(picker.current_row())
    .on_move(move |direction| send(Msg::ModelPickerMoved { session, direction }))
    .on_pick(move |row| {
        send(Msg::ModelPicked {
            session,
            item: items.get(row).copied().flatten(),
        })
    })
    .on_dismiss(send(Msg::ModelPickerClosed(session)));
    container(typeahead)
        .width(Length::Fixed(MODEL_PICKER_WIDTH))
        .into()
}

/// A picker row labelled `label`, its `spans` emphasised, with `details` under it when there are
/// any: the one place the palette and the model picker build a row.
fn row_with_details(
    label: String,
    spans: Vec<std::ops::Range<usize>>,
    details: Option<String>,
) -> TypeaheadRow {
    let row = TypeaheadRow::new(label, spans);
    match details {
        Some(details) => row.details(details, Vec::new()),
        None => row,
    }
}

/// Over `session`'s composer, the offer to open it in the other kind (#943 FR-010, US4 scenario
/// 1): why, **Open in terminal** and **Not now**; or, after an authentication failure, the offer of
/// **Open in terminal** with `/login` beside the CLI's error (FR-022a). The confirmation the
/// service may ask for is a dialog (`ui/confirm_switch_kind.rs`).
pub fn switch_offer<'a>(
    state: &'a State,
    session: SessionId,
    roles: Roles,
) -> Option<Element<'a, Message>> {
    use crate::features::session::{switch_label, LOGIN_REASON, LOGIN_TITLE};
    match state.composers.switch_offer.as_ref() {
        Some(offer) if offer.session == session => {
            if offer.confirm.is_some() {
                return None;
            }
            Some(
                PromptCard::new(offer.title(), roles)
                    .leading(crate::ui::switch_icon(offer.to))
                    .body(
                        Text::new(offer.reason.clone(), TypeRole::Body, roles)
                            .wrapping(Wrapping::WordOrGlyph)
                            .width(Length::Fill),
                    )
                    .action(
                        Button::filled(switch_label(offer.to), roles)
                            .on_press(send(Msg::SwitchConfirmed(session))),
                    )
                    .action(
                        Button::text(NOT_NOW, roles).on_press(send(Msg::SwitchDismissed(session))),
                    )
                    .note(offer.note())
                    .into(),
            )
        }
        _ if state.offers_login(session) => Some(
            PromptCard::new(LOGIN_TITLE, roles)
                .leading(crate::ui::switch_icon(SessionKind::Terminal))
                .body(
                    Text::new(LOGIN_REASON, TypeRole::Body, roles)
                        .wrapping(Wrapping::WordOrGlyph)
                        .width(Length::Fill),
                )
                .action(
                    Button::filled(switch_label(SessionKind::Terminal), roles)
                        .on_press(send(Msg::LoginOffered(session))),
                )
                .into(),
        ),
        _ => None,
    }
}

/// `session`'s command palette while a `/` is typed (#943 FR-007, FR-008): the composer's
/// completion slot.
pub fn palette<'a>(
    state: &'a State,
    session: SessionId,
    roles: Roles,
) -> Option<Element<'a, Message>> {
    let open = state.composers.palette.palette(session)?;
    Some(palette_of(open, roles, move |row| {
        send(Msg::PaletteClicked { session, row })
    }))
}

/// `open` drawn as a [`CommandPalette`], a click on row `n` sending `on_pick(n)`.
pub fn palette_of<'a, M: Clone + 'a>(
    open: &PaletteState,
    roles: Roles,
    on_pick: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    let mut palette = CommandPalette::new(roles)
        .highlighted(open.highlight())
        .on_pick(on_pick);
    if let Some(note) = open.source_note() {
        palette = palette.note(note);
    }
    if let Some(none) = open.no_match_note() {
        palette = palette.empty_message(none);
    }
    for line in open.lines() {
        palette = match line {
            PaletteLine::Group(title) => palette.group(title),
            PaletteLine::Command {
                label,
                matched,
                details,
            } => palette.command(row_with_details(
                label,
                matched.into_iter().collect(),
                details,
            )),
        };
    }
    palette.into()
}

/// The text area beside #942's control, **Send** and, while busy, **Stop** (FR-004, FR-012).
fn input_row<'a>(
    state: &'a State,
    session: SessionId,
    phase: TurnPhase,
    roles: Roles,
    control: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let composers = &state.composers;
    let editor = composers.editor(session);
    let (caret, lines) = (editor.caret(), editor.line_count());
    let palette = composers.palette.palette(session);
    let area = TextArea::new(editor.content(), roles)
        .placeholder(PLACEHOLDER)
        .id(composer_field(session))
        .max_height(MAX_HEIGHT)
        .on_action(move |action| {
            send(Msg::Edited {
                session,
                action: EditorAction(action),
            })
        })
        .key_binding(move |press| key_binding(session, press, caret, lines, phase, palette))
        .track_focus(FieldId::Composer(session), None);
    let mut controls = row![container(area).width(Length::Fill)]
        .spacing(spacing::SM)
        .align_y(Alignment::End);
    if let Some(control) = control {
        controls = controls.push(control);
    }
    if composers.offers_stop(session) {
        controls = controls
            .push(Button::outlined("Stop", roles).on_press(send(Msg::StopRequested(session))));
    }
    controls
        .push(
            Button::filled("Send", roles).on_press_maybe(
                composers
                    .can_send(session, true)
                    .then(|| send(Msg::SendRequested(session))),
            ),
        )
        .into()
}

/// A queued message edited in place (FR-020): its text, its pictures each with a remove control,
/// and **Save** / **Cancel**.
fn queue_edit<'a>(
    session: SessionId,
    edit: &'a QueueEditDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let field = TextField::new("", &edit.text, roles)
        .on_input(move |text| send(Msg::QueueEditChanged { session, text }))
        .on_submit(send(Msg::QueueEditSaved(session)))
        .track_focus(FieldId::QueuedMessage(session), focused);
    let pictures = edit.attachments.iter().enumerate().map(|(index, meta)| {
        Button::text(format!("Remove {}", meta.name), roles)
            .on_press(send(Msg::QueueEditAttachmentRemoved { session, index }))
            .into()
    });
    column![
        field,
        Row::with_children(pictures).spacing(spacing::SM).wrap(),
        row![
            Space::new().width(Length::Fill),
            Button::text("Cancel", roles).on_press(send(Msg::QueueEditCancelled(session))),
            Button::filled("Save", roles).on_press(send(Msg::QueueEditSaved(session))),
        ]
        .spacing(spacing::SM),
    ]
    .spacing(spacing::SM)
    .into()
}

/// What a key press in `session`'s composer does: the composer's keys (send, new line, history,
/// interrupt) go to the shell as `Msg::Key`, decided again there on the editor's own caret; every
/// other key is the editor's default (FR-002, FR-012, FR-023). While the command palette is open
/// (`palette`), the keys it takes (arrows, Tab, Enter, Escape) go to the shell too, which
/// gives them to the palette first (#943 FR-008a). An IME composition reaches the editor as
/// input-method events, never as key presses, so a press is never composing.
pub fn key_binding(
    session: SessionId,
    press: KeyPress,
    caret: (usize, usize),
    lines: usize,
    phase: TurnPhase,
    palette: Option<&PaletteState>,
) -> Option<Binding<Message>> {
    if !matches!(press.status, Status::Focused { .. }) {
        return Binding::from_key_press(press);
    }
    let Some(input) = key_input(
        &press.key,
        &press.physical_key,
        press.modifiers,
        press.text.as_deref(),
    ) else {
        return Binding::from_key_press(press);
    };
    let to_palette = palette_key(palette, &input).is_some();
    match composer_key(&input, caret, lines, false, phase) {
        KeyIntent::Edit if !to_palette => Binding::from_key_press(press),
        _ => Some(Binding::Custom(send(Msg::Key {
            session,
            press: input,
            composing: false,
        }))),
    }
}

/// A press as the keymap reads it; `None` for a key it does not know. Shared by the composer's
/// binding and the transcript's listener.
pub(crate) fn key_input(
    key: &iced::keyboard::Key,
    physical: &iced::keyboard::key::Physical,
    modifiers: iced::keyboard::Modifiers,
    text: Option<&str>,
) -> Option<KeyInput> {
    Some(KeyInput {
        key: to_keymap_key(key)?,
        mods: to_keymap_mods(modifiers),
        text: text.map(str::to_owned),
        physical: to_keymap_physical(physical),
    })
}

/// The keys of the active session's transcript while it shows one and its turn can be interrupted
/// (feature 941, FR-012): `None` otherwise, so Escape stays as inert as it was.
pub(crate) fn transcript_interrupt(state: &State) -> Option<iced::Subscription<Message>> {
    let session = state.session.active?;
    (crate::ui::terminal::shows_transcript(state, session) && state.composers.offers_stop(session))
        .then(|| {
            iced::keyboard::listen()
                .with(session)
                .filter_map(transcript_key_message)
        })
}

/// A key press on `session`'s transcript, for the composer to decide (`transcript_key`).
fn transcript_key_message(
    (session, event): (micold_core::session::SessionId, iced::keyboard::Event),
) -> Option<Message> {
    let iced::keyboard::Event::KeyPressed {
        key,
        physical_key,
        modifiers,
        text,
        ..
    } = event
    else {
        return None;
    };
    let press = crate::ui::composer::key_input(&key, &physical_key, modifiers, text.as_deref())?;
    Some(Message::Composer(
        crate::features::composer::Msg::TranscriptKey { session, press },
    ))
}
