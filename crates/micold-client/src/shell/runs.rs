//! The shell half of Run in parallel (feature 483): turns the reducer's [`Effect`] into a
//! correlated request to the service.

use iced::Task;
use micold_client::features::runs::Effect;
use micold_core::protocol::messages::ClientMsg;

use crate::shell::daemon_sync::{send_op, PendingOp};
use crate::App;
use micold_client::app::Message;

/// Run the request the reducer (or the root, while it opened the dialog) left pending.
pub fn run_pending(app: &mut App) -> Task<Message> {
    match app.core.runs.pending.take() {
        None | Some(Effect::None) => {}
        Some(Effect::Send(ClientMsg::BranchList { project, .. })) => {
            let asked_for = project.clone();
            send_op(
                app,
                PendingOp::RunBranchList { project: asked_for },
                move |req| ClientMsg::BranchList { req, project },
            );
        }
        Some(Effect::Send(ClientMsg::RunGroupCreate {
            project,
            naming,
            prompt,
            base_branch,
            providers,
            ..
        })) => send_op(app, PendingOp::RunGroupCreate, move |req| {
            ClientMsg::RunGroupCreate {
                req,
                project,
                naming,
                prompt,
                base_branch,
                providers,
            }
        }),
        // The reducer sends nothing else; a new request needs a route above.
        Some(Effect::Send(_)) => {}
    }
    Task::none()
}
