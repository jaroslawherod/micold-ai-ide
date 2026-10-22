//! The structured session composer's shell half (feature 941, T047, research R10): keeps one
//! text editor per session beside the draft `features::composer` holds, maps key presses through
//! `composer_key` / `transcript_key`, and sends what the reducer decided — messages, interrupts
//! and queue requests. It decides nothing itself; the answers come back through
//! `shell::daemon_sync`.

use iced::widget::text_editor::{Action, Edit};
use iced::Task;
use micold_client::app::{Editor, EditorAction, Message};
use micold_client::features::composer::Msg;
use micold_client::features::composer::{composer_key, transcript_key, KeyIntent, NOT_CONNECTED};
use micold_core::protocol::messages::{ClientMsg, TurnPhase};
use micold_core::session::SessionId;

use crate::shell::daemon_sync::{send_op_bound, send_op_bound_req, PendingOp};
use crate::App;

/// Run one composer message.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let session = session_of(&msg);
    match msg {
        Msg::Key {
            session,
            press,
            composing,
        } => {
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
    }
    sync_editor(app, session);
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
        | Msg::Dropped { session, .. } => *session,
        Msg::SendRequested(session)
        | Msg::StopRequested(session)
        | Msg::SendQueuedRequested(session)
        | Msg::QueueEditSaved(session)
        | Msg::QueueEditCancelled(session) => *session,
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

/// `session`'s editor, made from its draft the first time it is asked for.
fn editor(app: &mut App, session: SessionId) -> &mut Editor {
    sync_editor(app, session);
    &mut app
        .composer_editors
        .get_mut(&session)
        .expect("sync_editor made it")
        .0
}

/// Show the draft in the editor again when the composer replaced it (a send, a restore, a
/// history step, a drop, a queued message taken back); typing leaves the editor alone.
pub fn sync_editor(app: &mut App, session: SessionId) {
    let (text, caret, generation) = app
        .core
        .composers
        .composer(session)
        .map_or((String::new(), (0, 0), 0), |c| {
            (c.draft.text.clone(), c.draft.caret, c.generation)
        });
    let current = app.composer_editors.get(&session).map(|(_, g)| *g);
    if current != Some(generation) {
        app.composer_editors
            .insert(session, (Editor::at(&text, caret), generation));
    }
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
