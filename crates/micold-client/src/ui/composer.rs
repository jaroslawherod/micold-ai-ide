//! The structured session's composer, drawn under its transcript (feature 941, FR-001, T057,
//! T058).
//!
//! Top to bottom: the queued messages (`QueueRow`, or the one being edited in place), the hold
//! notice with **Send queued** (FR-022), the notice line, the status row (`BusyIndicator`, **Stop
//! session** while an interrupt is overdue, and #942's slot, I-1), the draft's pictures
//! (`AttachmentChip`), #943's completion slot (I-4), and the text area beside **Send** and, while
//! busy, **Stop**. Everything it shows is `features::composer`'s; every control sends a
//! `composer::Msg` for the shell to carry out.

use iced::widget::text_editor::{Binding, KeyPress, Status};
use iced::widget::{column, container, row, Row, Space};
use iced::{Alignment, Element, Length};
use micold_core::protocol::messages::TurnPhase;
use micold_core::session::SessionId;
use micold_core::tokens::{spacing, Roles};

use crate::app::{EditorAction, Message, State};
use crate::features::composer::{composer_key, KeyIntent, Msg, QueueEditDraft};
use crate::features::window::FieldId;
use crate::keymap::KeyInput;
use crate::ui::focus::TrackFocus;
use crate::ui::material::terminal_pane::{to_keymap_key, to_keymap_mods, to_keymap_physical};
use crate::ui::material::{
    AttachmentChip, BusyIndicator, Button, QueueRow, Scrollable, Text, TextArea, TextField,
    TypeRole,
};

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

/// What the composer's later issues put into it (plan I-1, I-4): empty in #941.
#[derive(Default)]
pub struct Seams<'a> {
    /// #942's element at the end of the status row.
    pub status: Option<Element<'a, Message>>,
    /// #942 waits on the user: its words replace the busy label.
    pub waiting: bool,
    /// #943's element over the text area.
    pub completion: Option<Element<'a, Message>>,
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
    for (index, queued) in queue.iter().enumerate() {
        let entry = queued.id;
        rows.push(match editing.filter(|e| e.entry == entry) {
            Some(edit) => queue_edit(session, edit, state.window.focused_field, roles),
            None => QueueRow::new(index + 1, queued.text.clone(), roles)
                .pictures(queued.attachments.len())
                .on_edit(send(Msg::QueueEditStarted { session, entry }))
                .on_remove(send(Msg::QueueRemoveRequested { session, entry }))
                .on_move_to_composer(
                    composers
                        .can_take(session, entry)
                        .then(|| send(Msg::QueueTakeRequested { session, entry })),
                )
                .into(),
        });
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

    parts.push(input_row(state, session, phase, roles));

    container(column(parts).spacing(spacing::SM))
        .padding([spacing::SM, spacing::MD])
        .width(Length::Fill)
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
    let mut status = row![BusyIndicator::new(label, roles).busy(busy)]
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

/// The text area beside **Send** and, while busy, **Stop** (FR-004, FR-012).
fn input_row<'a>(
    state: &'a State,
    session: SessionId,
    phase: TurnPhase,
    roles: Roles,
) -> Element<'a, Message> {
    let composers = &state.composers;
    let editor = composers.editor(session);
    let (caret, lines) = (editor.caret(), editor.line_count());
    let area = TextArea::new(editor.content(), roles)
        .placeholder(PLACEHOLDER)
        .max_height(MAX_HEIGHT)
        .on_action(move |action| {
            send(Msg::Edited {
                session,
                action: EditorAction(action),
            })
        })
        .key_binding(move |press| key_binding(session, press, caret, lines, phase))
        .track_focus(FieldId::Composer(session), None);
    let mut controls = row![container(area).width(Length::Fill)]
        .spacing(spacing::SM)
        .align_y(Alignment::End);
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
/// other key is the editor's default (FR-002, FR-012, FR-023). An IME composition reaches the
/// editor as input-method events, never as key presses, so a press is never composing.
pub fn key_binding(
    session: SessionId,
    press: KeyPress,
    caret: (usize, usize),
    lines: usize,
    phase: TurnPhase,
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
    match composer_key(&input, caret, lines, false, phase) {
        KeyIntent::Edit => Binding::from_key_press(press),
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
