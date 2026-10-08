//! The shell half of Run in parallel (feature 483): turns the reducer's [`Effect`] into a
//! correlated request to the service.

use iced::Task;
use micold_client::features::runs::Msg;
use micold_client::features::runs::{Effect, SummaryRead};
use micold_core::protocol::messages::ClientMsg;
use micold_core::runs;
use micold_core::session::SessionLocation;

use crate::shell::daemon_sync::{send_op, PendingOp};
use crate::App;
use micold_client::app::Message;

/// Apply a `features::runs::Msg` to the core, then run the request it left (shape B, FR-015).
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    // `features::runs::Msg` is routed through the root so its overlay and sidebar effects apply.
    let opens_diff = matches!(msg, Msg::DiffOpened { .. });
    app.core.update(Message::Runs(msg));
    let reads = run_pending(app);
    // **Open diff** leaves the Changes view's list read for the shell (C4).
    if opens_diff {
        return Task::batch([reads, crate::shell::changes::run_pending(app)]);
    }
    reads
}

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
        Some(Effect::Send(ClientMsg::RunGroupDismiss { project, group, .. })) => {
            send_op(app, PendingOp::RunGroupDismiss, move |req| {
                ClientMsg::RunGroupDismiss {
                    req,
                    project,
                    group,
                }
            })
        }
        Some(Effect::Send(ClientMsg::RunGroupPick {
            project,
            group,
            run,
            ..
        })) => send_op(app, PendingOp::RunGroupPick { run }, move |req| {
            ClientMsg::RunGroupPick {
                req,
                project,
                group,
                run,
            }
        }),
        // The reducer sends nothing else; a new request needs a route above.
        Some(Effect::Send(_)) => {}
        Some(Effect::ReadSummaries(reads)) => {
            return Task::batch(reads.into_iter().map(|read| {
                read_summary(app, read, |seq, run, result| Msg::SummaryRead {
                    seq,
                    run,
                    result,
                })
            }));
        }
        // The cleanup offer's fresh reads (K3, K4) answer `CleanupRead`.
        Some(Effect::ReadCleanup(reads)) => {
            return Task::batch(reads.into_iter().map(|read| {
                read_summary(app, read, |seq, run, result| Msg::CleanupRead {
                    seq,
                    run,
                    result,
                })
            }));
        }
        // The offer's deletes: the existing Delete, one per loser (K4, FR-016).
        Some(Effect::SendEach(msgs)) => {
            for msg in msgs {
                if let ClientMsg::WorktreeDelete {
                    project,
                    dir_name,
                    stop_sessions,
                    delete_branch,
                    ..
                } = msg
                {
                    let dir = dir_name.clone();
                    send_op(app, PendingOp::WorktreeDelete(dir), move |req| {
                        ClientMsg::WorktreeDelete {
                            req,
                            project,
                            dir_name,
                            stop_sessions,
                            delete_branch,
                        }
                    });
                }
            }
        }
    }
    Task::none()
}

/// Read one run's counts off the UI thread (C2, T045) and answer [`Msg::SummaryRead`]. A run
/// whose directory cannot be read here (no project, or the session service runs on another
/// computer, as for `shell/changes.rs`) answers with the reason instead of counts.
fn read_summary(
    app: &App,
    read: SummaryRead,
    answer: fn(u64, u8, Result<runs::RunSummary, String>) -> Msg,
) -> Task<Message> {
    let SummaryRead {
        seq,
        run,
        dir_name,
        base_branch,
    } = read;
    let entry = SessionLocation::Worktree(dir_name);
    let dir = crate::shell::changes::entry_dir(app, &entry);
    let git = app.caps.shared_git();
    Task::perform(
        async move {
            let (Some(dir), Some(git)) = (dir, git) else {
                return Err("This run's files are not on this computer.".to_string());
            };
            tokio::task::spawn_blocking(move || {
                runs::summary::read(&*git, &dir, &base_branch).map_err(|err| err.to_string())
            })
            .await
            .unwrap_or_else(|joined| Err(joined.to_string()))
        },
        move |result| Message::Runs(answer(seq, run, result)),
    )
}
