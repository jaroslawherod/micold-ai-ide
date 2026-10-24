//! The structured session composer's shell half (feature 941, T047, research R10): acts on the
//! text editor each composer in `features::composer` keeps beside its draft, maps key presses
//! through `composer_key` / `transcript_key`, and sends what the reducer decided — messages, interrupts
//! and queue requests. It decides nothing itself; the answers come back through
//! `shell::daemon_sync`.

use iced::widget::text_editor::{Action, Edit};
use iced::Task;
use micold_client::app::{Editor, EditorAction, Message};
use micold_client::features::command_palette::{palette_key, PaletteKey, SendIntent};
use micold_client::features::composer::Msg;
use micold_client::features::composer::{composer_key, transcript_key, KeyIntent, NOT_CONNECTED};
use micold_client::features::session::SwitchRequest;
use micold_client::ui::composer::{composer_field, model_picker_field};
use micold_core::protocol::messages::{ClientMsg, TurnPhase};
use micold_core::session::SessionId;
use micold_core::structured::model::ModelChoice;

use crate::shell::daemon_sync::{send_op_bound, send_op_bound_req, PendingOp};
use crate::App;

/// Run one composer message.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let session = session_of(&msg);
    let picker_was_open = app.core.composers.palette.picker(session).is_some();
    let mut follow = false;
    match msg {
        Msg::Key {
            session,
            press,
            composing,
        } => {
            // The open command palette takes its keys before the composer (#943 FR-008a).
            let palette = app.core.composers.palette.palette(session);
            if let Some(key) = palette_key(palette, &press).filter(|_| !composing) {
                app.core.palette_key(session, key);
                // The arrows move the highlight: scroll the list to follow it.
                return if matches!(key, PaletteKey::Up | PaletteKey::Down) {
                    micold_client::ui::picker_highlight_into_view()
                } else {
                    Task::none()
                };
            }
            let editor = editor(app, session);
            let (caret, lines) = (editor.caret(), editor.line_count());
            let phase = app.core.composers.phase(session);
            run(
                app,
                session,
                composer_key(&press, caret, lines, composing, phase),
            );
        }
        Msg::TranscriptKey { session, press } => {
            let phase = app.core.composers.phase(session);
            run(app, session, transcript_key(&press, phase));
        }
        Msg::Edited { session, action } => edited(app, session, &action),
        Msg::SendRequested(session) => send(app, session),
        Msg::StopRequested(session) => interrupt(app, session),
        Msg::StopSessionRequested(session) => {
            // Offered only once an interrupt is overdue (FR-015); never without the user asking.
            if app.core.composers.offers_stop_session(session) {
                if let Some(outbox) = app.outbox_of(app.session_binding_of(session)) {
                    outbox.send(ClientMsg::SessionStop { session });
                }
            }
        }
        Msg::SendQueuedRequested(session) => {
            if app.core.composers.offers_send_queued(session) {
                queue_request(app, session, PendingOp::QueueOp { session }, |req| {
                    ClientMsg::QueueRelease { req, session }
                });
            }
        }
        Msg::AttachmentRemoved { session, index } => {
            app.core.composers.remove_attachment(session, index);
        }
        Msg::QueueRemoveRequested { session, entry } => {
            queue_request(app, session, PendingOp::QueueOp { session }, |req| {
                ClientMsg::QueueRemove {
                    req,
                    session,
                    entry,
                }
            });
        }
        Msg::QueueTakeRequested { session, entry } => {
            if app.core.composers.can_take(session, entry) {
                queue_request(app, session, PendingOp::QueueTake { session }, |req| {
                    ClientMsg::QueueTake {
                        req,
                        session,
                        entry,
                    }
                });
            }
        }
        Msg::QueueEditStarted { session, entry } => {
            app.core.composers.start_queue_edit(session, entry);
        }
        Msg::QueueEditChanged { session, text } => {
            app.core.composers.queue_edit_text(session, text);
        }
        Msg::QueueEditAttachmentRemoved { session, index } => {
            app.core
                .composers
                .queue_edit_remove_attachment(session, index);
        }
        Msg::QueueEditSaved(session) => {
            if let Some((entry, text, keep)) = app.core.composers.queue_edit_request(session) {
                queue_request(app, session, PendingOp::QueueEdit { session }, |req| {
                    ClientMsg::QueueEdit {
                        req,
                        session,
                        entry,
                        text,
                        keep,
                    }
                });
            }
        }
        Msg::QueueEditCancelled(session) => app.core.composers.queue_edit_done(session),
        Msg::Dropped { session, files } => app.core.composers.apply_drop(session, *files),
        Msg::PaletteClicked { session, row } => app.core.palette_clicked(session, row),
        Msg::ModelPickerRequested(session) => app.core.open_model_picker(session),
        Msg::ModelPickerTyped { session, text } => app.core.model_picker_typed(session, text),
        Msg::ModelPicked { session, item } => {
            if let Some(choice) = app.core.model_picked(session, item) {
                set_model(app, session, choice);
            }
        }
        Msg::ModelPickerMoved { session, direction } => {
            app.core.model_picker_moved(session, direction);
            follow = true;
        }
        Msg::ModelPickerClosed(session) => app.core.composers.palette.close_picker(session),
        Msg::SwitchRequested { session, to } => {
            if let Some(request) = app.core.switch_requested(session, to) {
                switch_kind(app, request);
            }
        }
        Msg::SwitchConfirmed(session) => {
            if app.core.switch_offer_is(session) {
                if let Some(request) = app.core.switch_confirmed() {
                    switch_kind(app, request);
                }
            }
        }
        Msg::SwitchDismissed(session) => {
            if app.core.switch_offer_is(session) {
                app.core.switch_dismissed();
            }
        }
        // The card beside the failure is the offer itself: taking it switches (FR-022a). Not
        // over another offer still open.
        Msg::LoginOffered(session) => {
            if app.core.composers.switch_offer.is_some() {
                return Task::none();
            }
            app.core.login_offered(session);
            if let Some(request) = app.core.switch_confirmed() {
                switch_kind(app, request);
            }
        }
    }
    // The palette follows the draft, whoever changed it (#943 FR-007).
    app.core.composer_edited(session);
    // The model picker opening, from the view or by `/model`, takes the keyboard; closing it
    // gives the keyboard back to the composer, so typing goes on where it left off (FR-014).
    let picker_open = app.core.composers.palette.picker(session).is_some();
    if picker_open != picker_was_open {
        return iced::widget::operation::focus(if picker_open {
            model_picker_field(session)
        } else {
            composer_field(session)
        });
    }
    if follow {
        return micold_client::ui::picker_highlight_into_view();
    }
    Task::none()
}

fn session_of(msg: &Msg) -> SessionId {
    match msg {
        Msg::Key { session, .. }
        | Msg::TranscriptKey { session, .. }
        | Msg::Edited { session, .. }
        | Msg::AttachmentRemoved { session, .. }
        | Msg::QueueRemoveRequested { session, .. }
        | Msg::QueueTakeRequested { session, .. }
        | Msg::QueueEditStarted { session, .. }
        | Msg::QueueEditChanged { session, .. }
        | Msg::QueueEditAttachmentRemoved { session, .. }
        | Msg::Dropped { session, .. }
        | Msg::PaletteClicked { session, .. }
        | Msg::ModelPickerTyped { session, .. }
        | Msg::ModelPickerMoved { session, .. }
        | Msg::ModelPicked { session, .. }
        | Msg::SwitchRequested { session, .. } => *session,
        Msg::SendRequested(session)
        | Msg::StopRequested(session)
        | Msg::StopSessionRequested(session)
        | Msg::SendQueuedRequested(session)
        | Msg::QueueEditSaved(session)
        | Msg::QueueEditCancelled(session)
        | Msg::ModelPickerRequested(session)
        | Msg::ModelPickerClosed(session)
        | Msg::SwitchConfirmed(session)
        | Msg::SwitchDismissed(session)
        | Msg::LoginOffered(session) => *session,
    }
}

/// Carry out what a key meant.
fn run(app: &mut App, session: SessionId, intent: KeyIntent) {
    match intent {
        KeyIntent::Send => send(app, session),
        KeyIntent::NewLine => edited(app, session, &EditorAction(Action::Edit(Edit::Enter))),
        KeyIntent::HistoryBack | KeyIntent::HistoryForward => {
            let back = intent == KeyIntent::HistoryBack;
            let transcript = app.core.structured_transcript.transcript(session);
            let stepped = if back {
                app.core.composers.history_back(session, transcript)
            } else {
                app.core.composers.history_forward(session, transcript)
            };
            if !stepped {
                // No history to step: the key is a caret move after all (FR-023).
                editor(app, session).move_line(back);
                caret_moved(app, session);
            }
        }
        KeyIntent::Interrupt => interrupt(app, session),
        // The editor's own binding has it.
        KeyIntent::Edit => {}
    }
}

/// `session`'s editor: the one its composer draws.
fn editor(app: &mut App, session: SessionId) -> &mut Editor {
    app.core.composers.editor_mut(session)
}

fn edited(app: &mut App, session: SessionId, action: &EditorAction) {
    if !action.0.is_edit() {
        editor(app, session).act(action);
        return caret_moved(app, session);
    }
    let editor = editor(app, session);
    let text = editor.perform(action);
    let caret = editor.caret();
    app.core.composers.edited(session, text, caret);
}

fn caret_moved(app: &mut App, session: SessionId) {
    let editor = editor(app, session);
    let (text, caret) = (editor.text(), editor.caret());
    app.core.composers.edited(session, text, caret);
}

/// Whether the service that runs `session` is connected (FR-006).
fn connected(app: &App, session: SessionId) -> bool {
    app.outbox_of(app.session_binding_of(session)).is_some()
}

/// Send the draft: delivered, or queued while busy (FR-004 to FR-006, FR-017).
fn send(app: &mut App, session: SessionId) {
    // A command the app handles never reaches the CLI (#943 FR-010, FR-018).
    match app.core.send_intent(session) {
        SendIntent::Message => {}
        SendIntent::Nothing => return,
        SendIntent::SetModel(choice) => {
            if set_model(app, session, choice) {
                app.core
                    .composers
                    .replace_text(session, String::new(), (0, 0));
            }
            return;
        }
    }
    let connected = connected(app, session);
    let Some(draft) = app.core.composers.take_send(session, connected) else {
        return;
    };
    let (text, images) = (draft.text.clone(), draft.images.clone());
    let binding = app.session_binding_of(session);
    let sent = send_op_bound_req(app, binding, PendingOp::ComposerSend { session }, |req| {
        ClientMsg::SessionUserMessage {
            req,
            session,
            text,
            images,
        }
    });
    match sent {
        Some(req) => app.core.composers.sending(session, req, draft),
        None => app.core.composers.returned(session, draft, NOT_CONNECTED),
    }
}

/// Ask the service to interrupt the turn the window sees busy (FR-012, FR-013); no answer.
fn interrupt(app: &mut App, session: SessionId) {
    let TurnPhase::Busy { turn, .. } = app.core.composers.phase(session) else {
        return;
    };
    if let Some(outbox) = app.outbox_of(app.session_binding_of(session)) {
        outbox.send(ClientMsg::TurnInterrupt { session, turn });
    }
}

fn queue_request(
    app: &mut App,
    session: SessionId,
    op: PendingOp,
    build: impl FnOnce(u64) -> ClientMsg,
) {
    let binding = app.session_binding_of(session);
    send_op_bound(app, binding, op, build);
}

/// Ask the service to run `session` with `choice` (#943 FR-014); a refusal is the composer's
/// notice. `false` when nothing could be sent.
fn set_model(app: &mut App, session: SessionId, choice: ModelChoice) -> bool {
    let binding = app.session_binding_of(session);
    let sent = send_op_bound_req(app, binding, PendingOp::SessionModel { session }, |req| {
        ClientMsg::SetSessionModel {
            req,
            session,
            model: choice,
        }
    });
    if sent.is_none() {
        app.core
            .composers
            .notice(session, NOT_CONNECTED.to_string());
    }
    sent.is_some()
}

/// Ask the service to switch a session's kind (#943 FR-019–FR-022); a `NeedsConfirmation`
/// answer asks the user (`shell::daemon_sync`).
fn switch_kind(app: &mut App, request: SwitchRequest) {
    let session = request.session;
    let binding = app.session_binding_of(session);
    let op = PendingOp::SwitchKind {
        request: request.clone(),
    };
    let sent = send_op_bound_req(app, binding, op, |req| ClientMsg::SessionSwitchKind {
        req,
        session,
        to: request.to,
        command: request.wire_command(),
        confirmed: request.confirmed,
    });
    if sent.is_none() {
        app.core
            .composers
            .notice(session, NOT_CONNECTED.to_string());
    }
}
