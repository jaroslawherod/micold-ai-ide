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
use micold_client::features::structured_transcript::TranscriptMsg;
use micold_client::features::transcript_selection::SelectMsg;
use micold_client::features::window::FieldId;
use micold_client::ui::composer::{composer_field, model_picker_field};
use micold_core::protocol::messages::{ClientMsg, TurnPhase};
use micold_core::session::SessionId;
use micold_core::structured::effort::EffortChoice;
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
            let selecting = editor.has_selection();
            let phase = app.core.composers.phase(session);
            run(
                app,
                session,
                composer_key(&press, caret, lines, composing, selecting, phase),
            );
        }
        Msg::TranscriptKey { session, press } => {
            let phase = app.core.composers.phase(session);
            let selecting = app.core.transcript_selection.selecting(session);
            let focused = app.core.window.focused_field == Some(FieldId::Transcript(session));
            let intent =
                transcript_key(&press, phase, selecting, focused, cfg!(target_os = "macos"));
            // Feature 1550: the transcript's own keys are its selection's.
            let select = match intent {
                KeyIntent::CopySelection => Some(SelectMsg::CopySelection { session }),
                KeyIntent::SelectAll => Some(SelectMsg::SelectAll { session }),
                _ => None,
            };
            if let Some(select) = select {
                return crate::shell::transcript::update(app, TranscriptMsg::Select(select));
            }
            run(app, session, intent);
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
        Msg::Dropped { session, files } => {
            let can_accept = app.core.composers.accepts_input(session);
            app.core.composers.apply_drop(session, *files, can_accept);
        }
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
        Msg::EffortPickerToggled(session) => app.core.effort_picker_toggled(session),
        Msg::EffortPickerMoved { session, direction } => {
            app.core.effort_picker_moved(session, direction);
        }
        Msg::EffortPicked { session, row } => {
            if let Some(choice) = app.core.effort_picked(session, row) {
                set_effort(app, session, choice);
            }
        }
        Msg::EffortPickerClosed(session) => app.core.composers.effort.close(session),
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
        Msg::SwitchDismissed(session) => app.core.switch_dismissed_for(session),
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
        | Msg::EffortPickerMoved { session, .. }
        | Msg::EffortPicked { session, .. }
        | Msg::SwitchRequested { session, .. } => *session,
        Msg::SendRequested(session)
        | Msg::StopRequested(session)
        | Msg::StopSessionRequested(session)
        | Msg::SendQueuedRequested(session)
        | Msg::QueueEditSaved(session)
        | Msg::QueueEditCancelled(session)
        | Msg::ModelPickerRequested(session)
        | Msg::ModelPickerClosed(session)
        | Msg::EffortPickerToggled(session)
        | Msg::EffortPickerClosed(session)
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
        // The editor's own binding has it; the composer never yields the transcript's keys.
        KeyIntent::Edit | KeyIntent::CopySelection | KeyIntent::SelectAll => {}
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

/// Ask the service to run `session` with the effort `choice` (#1554 FR-004, FR-005); a refusal
/// is the composer's notice, and the choice stays what the catalog says. `false` when nothing
/// could be sent.
fn set_effort(app: &mut App, session: SessionId, choice: EffortChoice) -> bool {
    let binding = app.session_binding_of(session);
    let sent = send_op_bound_req(app, binding, PendingOp::SessionEffort { session }, |req| {
        ClientMsg::SetSessionEffort {
            req,
            session,
            effort: choice,
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

#[cfg(test)]
mod tests {
    //! #1554 T022: a pick from the effort picker asks the service once, and a refusal is said
    //! with the choice left as the catalog holds it (FR-004, FR-005; US1 scenario 7).

    use super::*;
    use crate::tests::base_app;
    use iced::futures::channel::mpsc::{unbounded, UnboundedReceiver};
    use micold_core::protocol::messages::{DaemonMsg, ErrorKind};
    use micold_core::session::{AiCli, Session, SessionKind, SessionLocation};
    use micold_core::structured::effort::EffortLevel;

    /// A connected window holding one structured Claude Code session, and the wire's far end.
    fn app_with_session() -> (App, UnboundedReceiver<ClientMsg>, SessionId) {
        let (tx, rx) = unbounded();
        let mut app = base_app();
        app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
        let session = Session::start_new_of_kind(
            SessionLocation::Default,
            AiCli::ClaudeCode,
            SessionKind::Structured,
        );
        let id = session.id;
        let path = std::path::PathBuf::from("/repo/effort");
        app.core
            .workspace
            .sessions
            .insert(path.clone(), vec![session]);
        app.core.workspace.active = Some(path);
        (app, rx, id)
    }

    fn effort_requests(rx: &mut UnboundedReceiver<ClientMsg>) -> Vec<(u64, EffortChoice)> {
        let mut sent = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let ClientMsg::SetSessionEffort { req, effort, .. } = msg {
                sent.push((req, effort));
            }
        }
        sent
    }

    #[test]
    fn a_pick_asks_the_service_once_and_a_refusal_is_said_with_the_choice_unchanged() {
        let (mut app, mut rx, s) = app_with_session();
        let _ = update(&mut app, Msg::EffortPickerToggled(s));
        assert!(app.core.composers.effort.picker(s).is_some(), "opened");
        // Default, low, medium, high: the fourth row.
        let _ = update(
            &mut app,
            Msg::EffortPicked {
                session: s,
                row: Some(3),
            },
        );
        let high = EffortChoice::Level(EffortLevel::parse("high").expect("a level"));
        let sent = effort_requests(&mut rx);
        assert_eq!(
            sent.iter().map(|(_, e)| e.clone()).collect::<Vec<_>>(),
            [high],
            "one request, for the picked level"
        );
        assert!(app.core.composers.effort.picker(s).is_none(), "closed");

        let _ = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            micold_core::daemons::DaemonId(1),
            DaemonMsg::OperationError {
                req: sent[0].0,
                kind: ErrorKind::Refused,
                message: "Claude Code did not accept the level".into(),
                detail: None,
            },
        );
        assert_eq!(
            app.core
                .composers
                .composer(s)
                .and_then(|c| c.notice.as_deref()),
            Some("Claude Code did not accept the level"),
            "the reason, in words"
        );
        assert_eq!(
            app.core.effort_choice(s),
            EffortChoice::Default,
            "unchanged"
        );
        assert_eq!(app.core.effort_header(s), "Effort: Default");
        assert!(effort_requests(&mut rx).is_empty(), "nothing more asked");
    }
}
