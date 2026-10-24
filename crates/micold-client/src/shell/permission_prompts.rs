//! The permission prompts' shell half (#942, contracts/wire.md): sends what
//! `features::permission_prompts` builds — an answer, a request for the whole input, a mode
//! change — with its `req`, to the daemon running the session. It decides nothing itself; the
//! replies come back through `shell::daemon_sync`.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::permission_prompts::Msg;
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::{PermissionMode, SessionId};

use crate::shell::daemon_sync::{send_op_bound_req, PendingOp};
use crate::App;

/// Run one permission prompt message.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let prompts = &mut app.core.permission_prompts;
    match msg {
        Msg::DenyMessageEdited { session, id, text } => {
            prompts.edit_deny_message(session, id, text);
        }
        Msg::NoteEdited { session, id, text } => prompts.edit_note(session, id, text),
        Msg::OptionToggled {
            session,
            id,
            question,
            option,
        } => prompts.toggle_option(session, id, question, option),
        Msg::OtherChosen {
            session,
            id,
            question,
        } => prompts.choose_other(session, id, question),
        Msg::OtherEdited {
            session,
            id,
            question,
            text,
        } => prompts.edit_other(session, id, question, text),
        Msg::Submitted {
            session,
            id,
            action,
        } => {
            if let Some(choice) = prompts.choice(session, id, action) {
                let binding = app.session_binding_of(session);
                let op = PendingOp::PermissionDecision { session, id };
                let sent =
                    send_op_bound_req(app, binding, op, |req| ClientMsg::PermissionDecision {
                        req,
                        session,
                        id,
                        choice,
                    });
                if sent.is_some() {
                    app.core.permission_prompts.sending(session, id);
                }
            }
        }
        Msg::ShowAllRequested { session, id } => {
            if prompts.shown(session, id).is_some() {
                let binding = app.session_binding_of(session);
                let op = PendingOp::PermissionInput { session, id };
                let _ = send_op_bound_req(app, binding, op, |req| ClientMsg::PermissionInput {
                    req,
                    session,
                    id,
                });
            }
        }
        Msg::ModeRequested { session, mode } => {
            // Bypass: its confirmation opens as every dialog does, over nothing else and with no
            // field holding the keyboard; cleared first, since clearing cancels open dialogs.
            if mode == PermissionMode::Bypass {
                app.core.clear_for_dialog();
            }
            if let Some(mode) = app.core.permission_prompts.request_mode(session, mode) {
                send_mode(app, session, mode);
            }
        }
        // Closed through the core, as `ModeCancelled` is, so its after-message work runs too: an
        // agent prompt held behind the dialog shows (M8 review A r3).
        Msg::ModeConfirmed => {
            let confirmed = prompts.mode_confirmed_change();
            app.core
                .update(Message::PermissionPrompts(Msg::ModeConfirmed));
            if let Some((session, mode)) = confirmed {
                send_mode(app, session, mode);
            }
        }
        // Through the core, as the registry's closes go, so its after-message work runs too.
        Msg::ModeCancelled => app
            .core
            .update(Message::PermissionPrompts(Msg::ModeCancelled)),
    }
    Task::none()
}

/// Send a `SetPermissionMode` (FR-017); its reply is routed to `mode_applied` / `mode_failed`.
fn send_mode(app: &mut App, session: SessionId, mode: PermissionMode) {
    let binding = app.session_binding_of(session);
    let op = PendingOp::SetPermissionMode { session };
    let sent = send_op_bound_req(app, binding, op, |req| ClientMsg::SetPermissionMode {
        req,
        session,
        mode,
    });
    if sent.is_some() {
        app.core.permission_prompts.mode_sending(session, mode);
    }
}
