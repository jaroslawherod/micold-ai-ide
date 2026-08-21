//! Talking to the session daemon (feature 021, T052 — FR-019a).
//!
//! The external system here is **the daemon process**: everything below either puts a `ClientMsg`
//! on the wire or folds a `CatalogSnapshot` back into the core's state. FR-019a asks the shell to
//! be divided by the system each part addresses, and the daemon is the largest of them — it owns
//! sessions, projects and worktrees, so the client's picture of all three is downstream of what
//! arrives here.
//!
//! # Three things the task named, and three that had to come with them
//!
//! T052 names `send_op`, `switch_daemon_attachment`, `reconcile_catalog` and [`PendingOp`].
//! `PendingOp::describe` came with its type; `wire_to_lifecycle` and `wire_to_worktree_status` came
//! with `reconcile_catalog`, their only caller — both translate a daemon wire enum and nothing
//! else in the client has a reason to (FR-001a).
//!
//! Those three have since moved to the **library**, as [`micold_client::catalog_sync`], and are
//! called from here. The grouping T052 argued for was right and is unchanged; what was wrong was
//! the crate. A binary-crate function cannot be reached from `tests/`, and the fold from the
//! daemon's snapshot into client state is exactly the seam three bugs lived in (`010` BUG-011,
//! `012` BUG-003 and BUG-004) — each with both halves tested and the join untestable. The module
//! docs over there carry the full account.
//!
//! # `App` is the argument, and that is the plan rather than a shortcut
//!
//! `send_op` and `switch_daemon_attachment` both take `&mut App` (the latter since BUG-002, which
//! made the switch resume the session it restores), which `shell/mod.rs` said
//! this module would when it was written at T050: these operate on the binary's own type. That is
//! *not* what T051 did with `persist_settings`, and the difference is what each function needs.
//! `persist_settings` needed one capability out of seven, so narrowing it to that was FR-016's
//! argument and it bought two tests. `send_op` needs four fields spanning two concerns — the
//! outbox, the correlation counter, the in-flight table, and the notification queue — and spelling
//! those out at eleven call sites would trade a readable call for a parameter list. What it costs
//! in testability is nothing: `App` has a neutral-default builder in `main.rs`'s test module, and
//! the tests below use it.
//!
//! # The fixtures live here
//!
//! `snapshot_with`, `summary` and `summary_at` build daemon wire types, so they moved with the
//! reconcile tests and `main.rs`'s remaining users import them back. `base_app` went the other
//! way for the same reason — `App` is the binary's type. Each fixture sits with what it is *of*,
//! not with who happens to call it.

use std::path::{Path, PathBuf};

use iced::Task;

use micold_client::app::Message;
// Moved to the library (see `micold_client::catalog_sync` for why): the fold has to be reachable
// from `tests/`, and a binary-crate function is not.
use micold_client::catalog_sync::{attach_log_line, reconcile_catalog, wire_to_worktree_status};
use micold_client::features::worktree_form::{
    BranchSource, Msg as FormMsg, ResolutionState, WorktreeForm, WorktreeFormStatus,
};
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, OperationResult, SessionProcess,
};
use micold_core::session::{Session, SessionId, SessionLocation, ShellInstanceId, TerminalMode};
use micold_core::worktree::{BranchOrigin, CreateMode};

use crate::shell::env_include::{default_resolution_cwd, refresh_env_include};
use crate::App;
use crate::{
    active_project_displaced, session_cwd_for_location, session_cwd_mode_and_active_shell,
};

/// A mutating RPC the client has sent to the daemon and is awaiting a reply for (T055). Tracked per
/// correlation `req` so the reply can be matched, a duplicate submission avoided, and — if the
/// connection drops before the reply arrives — the user can be told the outcome is unknown (FR-031/035).
#[derive(Debug, Clone)]
pub enum PendingOp {
    /// A `SessionCreate`; on success the daemon-assigned session is selected + viewed. Further
    /// variants (project/session-delete) are added as each mutation domain is migrated.
    CreateSession,
    DeleteSession,
    WorktreeCreate(String),
    WorktreeDelete(String),
    /// A read-only `BranchPreflight` (feature 016). Carries what the reply needs to continue:
    /// the project it was asked about, the derived names, whether the branch came from the
    /// picker, and the remote the user named by picking that row — so nothing is recomputed when
    /// the answer lands. `project` is what makes a reply that outlived its form (cancelled, or
    /// the user switched project) detectable instead of acted upon.
    BranchPreflight {
        project: PathBuf,
        names: micold_core::naming::DerivedNames,
        picked: bool,
        preferred_remote: Option<String>,
    },
    /// A read-only `BranchList` for the existing-branch picker (feature 016).
    BranchList {
        project: PathBuf,
    },
    /// A `WorktreeInclude` / `WorktreeExclude` (016 BUG-002). Both carry the path so a failure can
    /// name what it was about, and so the reply is recognisable without re-deriving it.
    WorktreeInclude(PathBuf),
    WorktreeExclude(PathBuf),
    WorktreeRename(String),
    ProjectAdd,
    ProjectRemove,
    ProjectRename,
    /// A `SettingsSet` (FR-012a/FR-012b, BUG-003/T100): the service echoes the persisted result
    /// back as `SettingsChanged` to every connected client (including this one), which is what
    /// actually applies it — this variant exists only so a failure reaches the user and a
    /// disconnect-before-reply resolves to "unknown" like every other mutating RPC (T055).
    SettingsSet,
}

impl PendingOp {
    /// A short verb phrase for an error / unknown-outcome notification ("create the session …").
    pub fn describe(&self) -> String {
        match self {
            PendingOp::CreateSession => "create the session".into(),
            PendingOp::DeleteSession => "delete the session".into(),
            PendingOp::WorktreeCreate(d) => format!("create the worktree \"{d}\""),
            PendingOp::BranchPreflight { .. } => "check the branch".into(),
            PendingOp::BranchList { .. } => "list the branches".into(),
            PendingOp::WorktreeDelete(d) => format!("delete the worktree \"{d}\""),
            PendingOp::WorktreeRename(d) => format!("rename the worktree \"{d}\""),
            PendingOp::WorktreeInclude(p) => format!("include the worktree at {}", p.display()),
            PendingOp::WorktreeExclude(p) => {
                format!("stop showing the worktree at {}", p.display())
            }
            PendingOp::ProjectAdd => "add the project".into(),
            PendingOp::ProjectRemove => "remove the project".into(),
            PendingOp::ProjectRename => "rename the project".into(),
            PendingOp::SettingsSet => "update the settings".into(),
        }
    }
}

/// Send a correlated mutating RPC to the daemon: allocate a `req`, record the pending op (so the
/// reply can be matched and a disconnect can resolve it as unknown), and send the message `build`s.
/// A no-op that notifies the user when there is no daemon connection (T055).
pub fn send_op(app: &mut App, op: PendingOp, build: impl FnOnce(u64) -> ClientMsg) {
    let Some(daemon) = &app.daemon else {
        app.core.notify_error(format!(
            "Not connected to the session service — can't {} right now.",
            op.describe()
        ));
        return;
    };
    let req = app.next_req;
    app.next_req += 1;
    daemon.send(build(req));
    app.pending_ops.insert(req, op);
}

/// Move this client's daemon attachment from `old` to `new` on a project switch: release the old
/// (so another window can take it), attach the new, and set the viewed session — so the daemon
/// streams grid frames and discovers worktrees for the project now in focus (T055). A no-op when
/// disconnected; the initial attach on connect is handled by `DaemonConnected`.
pub fn switch_daemon_attachment(app: &mut App, old: Option<PathBuf>, new: &Path) {
    let Some(daemon) = app.daemon.clone() else {
        return;
    };
    if let Some(old) = old {
        if old != new {
            daemon.send(ClientMsg::Detach { project: old });
        }
    }
    daemon.send(ClientMsg::Attach {
        project: new.to_path_buf(),
        force: false,
    });
    // The session this switch restored is *started*, not merely viewed (BUG-002, FR-004a). Within a
    // run this usually changes nothing — the session you switch to is normally still running, and a
    // start naming a running session is a no-op — but a remembered session that has **stopped** is
    // one the restore honours (feature 008's BUG-001), and it reached the same dead end a launch did:
    // current, with no process the daemon could stream.
    //
    // `&mut App` for `crate::view_and_start`, which resets the selection and scroll offset. A switch
    // wants both reset anyway — they belong to the session being left, not the one being shown.
    match app.core.active_session {
        Some(session) => view_and_start(app, session),
        None => daemon.send(ClientMsg::SetViewedSession {
            project: new.to_path_buf(),
            session: None,
        }),
    }
}

// ---------------------------------------------------------------------------------------------
// The arms (feature 021, T055)
//
// Everything below was an arm of `update_inner`. They are moved verbatim: the point of T055 is
// that the arm addressing an external system lives with that system, not that its logic changes,
// so a diff that rewrote a body while relocating it would be untestable against the old one.
// ---------------------------------------------------------------------------------------------

pub fn on_grid_frame(
    app: &mut App,
    frame: micold_core::protocol::grid::GridFrame,
) -> Task<Message> {
    // Feed the frame into the session's grid cache; the pane renders from it (T042).
    let session = frame.session;
    let (old_top, new_top, oldest) = {
        let cache = app.grids.entry(session).or_default();
        let old = cache.viewport_top().0;
        cache.apply(&frame);
        (old, cache.viewport_top().0, cache.oldest_available().0)
    };
    // Hold a scrolled-back view in place as new output advances the viewport: without this,
    // `line_at_row = viewport_top - display_offset + row` would slide the shown lines toward
    // the live bottom on every output tick (FR-016). Only the displayed session, only while
    // scrolled up; clamp to the retained history.
    if app.core.active_session == Some(session) && app.display_offset > 0 && new_top > old_top {
        let advanced = (new_top - old_top) as usize;
        let history = (new_top - oldest).max(0) as usize;
        app.display_offset = (app.display_offset + advanced).min(history);
    }
    Task::none()
}

pub fn on_disconnected(app: &mut App) -> Task<Message> {
    crate::log_line("attach: disconnected");
    app.daemon = None;
    // Content on screen is now stale; the banner says so (FR-027). The subscription is
    // already auto-reconnecting with backoff.
    app.disconnected = true;
    // Any request still in flight will never get a reply on this connection (`req`s are
    // per-connection). Resolve each to an explicit *unknown* outcome — never a silent
    // success or failure — and reconcile against authoritative state on reconnect
    // (FR-031/035). The daemon applied its mutation atomically before replying, so the fresh
    // welcome catalog is the source of truth for whether it actually took effect.
    for (_req, op) in app.pending_ops.drain() {
        app.core.notify_error(format!(
            "The session service disconnected before confirming the request to {} — \
             it may or may not have taken effect; reconnecting will show the current state.",
            op.describe()
        ));
    }
    Task::none()
}

pub fn on_connect_failed(app: &mut App, reason: String) -> Task<Message> {
    // The other half of `attach_log_line`'s job (`010` BUG-013): "never attached" and "attached and
    // got nothing" are different bugs, so the log has to be able to say the first one too.
    crate::log_line(&format!("attach: failed reason={reason}"));
    app.disconnected = true;
    app.core
        .notify_error(format!("Could not connect to the session daemon: {reason}"));
    Task::none()
}

/// The user chose to take the active project back after being displaced (FR-024): re-attach
/// with force, which displaces the current holder, and re-view its active session.
pub fn on_takeover_requested(app: &mut App) -> Task<Message> {
    if let (Some(project), Some(d)) = (app.core.workspace.active.clone(), app.daemon.clone()) {
        app.displaced.remove(&project);
        d.send(ClientMsg::Attach {
            project: project.clone(),
            force: true,
        });
        d.send(ClientMsg::SetViewedSession {
            project,
            session: app.core.active_session,
        });
    }
    Task::none()
}

/// Ask the daemon where it logs and for its recent errors (Phase 10, FR-046). The replies
/// arrive as `LogLocation`/`RecentErrors` events, shown as notices. Uncorrelated: only the
/// latest answer matters, so no pending-op bookkeeping is needed.
pub fn on_diagnostics_requested(app: &mut App) -> Task<Message> {
    if let Some(d) = &app.daemon {
        let req = app.next_req;
        app.next_req += 2;
        d.send(ClientMsg::LogLocationRequest { req });
        d.send(ClientMsg::RecentErrorsRequest {
            req: req + 1,
            limit: 20,
        });
    } else {
        app.core
            .notify_error("Not connected to the session service — no diagnostics to show.");
    }
    Task::none()
}

pub fn on_daemon_event(app: &mut App, event: DaemonMsg) -> Task<Message> {
    match event {
        DaemonMsg::CatalogChanged { catalog } => {
            reconcile_catalog(&mut app.core, &catalog, true);
            // Sessions can appear after connect — created in another window, or resumed —
            // so seed here too (T111). Absent-only, so this never disturbs a counter this
            // client is already driving.
            app.stamper.seed_from_catalog(&catalog);
            app.daemon_catalog = Some(catalog);
        }
        // A settings mutation reached the service — this client's own `SettingsSet` echoed
        // back, or another window's (FR-011). Sync every service-owned field and re-source
        // env-include, exactly like the local-save path below does for its own change
        // (T100): the enabled/path/timeout settings may have changed, so every previously
        // cached directory's snapshot is stale.
        DaemonMsg::SettingsChanged { settings } => {
            app.scrollback_lines = settings.scrollback_lines;
            app.env_include_enabled = settings.env_include_enabled;
            app.env_include_script_path = settings.env_include_script_path;
            app.env_include_timeout_secs = settings.env_include_timeout_secs;
            app.env_include_cache.clear();
            let cwd = default_resolution_cwd(&app.core);
            refresh_env_include(app, &cwd);
        }
        // Fetched scrollback: resolve + insert into the session's grid cache (FR-016/017).
        DaemonMsg::ScrollbackResponse {
            session,
            lines,
            styles,
            hyperlinks,
            ..
        } => {
            if let Some(grid) = app.grids.get_mut(&session) {
                grid.apply_scrollback(&lines, &styles, &hyperlinks);
            }
        }
        // A mutating request we correlated resolved. For most ops the resulting state has
        // already arrived via the `CatalogChanged` push (reconcile_catalog), so there is
        // nothing to do; a `SessionCreate` additionally names the daemon-assigned id so we
        // select + view it.
        DaemonMsg::OperationOk { req, result } => match app.pending_ops.remove(&req) {
            Some(PendingOp::CreateSession) => {
                if let OperationResult::SessionCreated { session } = result {
                    app.core.update(Message::SessionSelected(session));
                    view_and_start(app, session);
                    // No follow-up focus message: `SessionSelected` focuses the terminal in
                    // the reducer and nothing releases it on the same click any more
                    // (feature 023). The re-assertion this replaced existed to win a race
                    // against a `TerminalFocusReleased` published by the very click that
                    // selected — the shape FR-008a forbids, since it puts the keyboard
                    // somewhere the user did not ask for, however briefly.
                }
            }
            // A worktree create succeeded: close the form. The worktree itself arrives via
            // the `CatalogChanged` push (reconcile), so the constructed value here is only to
            // reuse `WorktreeCreated`'s form-closing logic (it dedups by dir_name).
            Some(PendingOp::WorktreeCreate(dir_name)) => {
                if let Some(repo) = app.core.workspace.active.clone() {
                    let path = repo.join(".claude/worktrees").join(&dir_name);
                    app.core.update(Message::WorktreeForm(FormMsg::Created(
                        micold_core::worktree::Worktree {
                            dir_name,
                            path,
                            branch: None,
                            status: micold_core::worktree::WorktreeStatus::Valid,
                            // The app made this one, so it is not an inclusion (016 BUG-002).
                            included: false,
                        },
                    )));
                }
            }
            // Feature 016: the pre-flight answer decides what happens next. A free name
            // creates straight away (FR-025 — no extra prompt); anything else either
            // resolves itself (the user already named the branch by picking it) or opens
            // the reuse/overwrite prompt.
            //
            // The answer is only acted on while the form that asked for it is still open,
            // still editing, and still pointed at the same project: cancelling the form
            // (or switching project) while the RPC is in flight must not go on to create
            // a worktree the user backed out of.
            Some(PendingOp::BranchPreflight {
                project: asked_for,
                names,
                picked,
                preferred_remote,
            }) => {
                if let OperationResult::BranchPreflight { situation } = result {
                    let form_open = app
                        .core
                        .worktree_form
                        .as_ref()
                        .is_some_and(|f| f.status == WorktreeFormStatus::Editing);
                    if let Some(project) = app
                        .core
                        .workspace
                        .active
                        .clone()
                        .filter(|p| form_open && *p == asked_for)
                    {
                        match &situation {
                            micold_core::worktree::BranchSituation::Free => {
                                send_worktree_create(app, project, names, CreateMode::NewBranch);
                            }
                            // Picking a branch IS the intent to use it, so an available
                            // candidate needs no prompt (contract branch-picker.md §5). It can
                            // never mean overwrite.
                            _ if picked => {
                                match WorktreeForm::mode_for(
                                    &situation,
                                    preferred_remote.as_deref(),
                                ) {
                                    Some(mode) => send_worktree_create(app, project, names, mode),
                                    None => app.core.update(Message::WorktreeForm(
                                        FormMsg::ConflictDetected(situation),
                                    )),
                                }
                            }
                            _ => app
                                .core
                                .update(Message::WorktreeForm(FormMsg::ConflictDetected(
                                    situation,
                                ))),
                        }
                    }
                }
            }
            // 016 BUG-002: the daemon answers with the row its own discovery produced, so
            // the worktree appears at the moment the user asked for it rather than at the
            // next catalog push — which also arrives, and agrees.
            Some(PendingOp::WorktreeInclude(_)) => {
                if let OperationResult::WorktreeIncluded { worktree } = result {
                    app.core
                        .update(Message::WorktreeIncluded(micold_core::worktree::Worktree {
                            dir_name: worktree.dir_name,
                            path: worktree.path,
                            branch: worktree.branch,
                            status: wire_to_worktree_status(worktree.status),
                            included: worktree.included,
                        }));
                }
            }
            Some(PendingOp::WorktreeExclude(_)) => {
                if let OperationResult::WorktreeExcluded { path } = result {
                    app.core.update(Message::WorktreeExcluded(path));
                }
            }
            // Same staleness guard: a listing for a project that is no longer the active
            // one must not populate the picker of a form opened on a different repo.
            Some(PendingOp::BranchList { project: asked_for }) => {
                if let OperationResult::BranchList { candidates } = result {
                    if app.core.workspace.active.as_deref() == Some(asked_for.as_path()) {
                        app.core
                            .update(Message::WorktreeForm(FormMsg::BranchesListed(candidates)));
                    }
                }
            }
            // Feature 013 (FR-015): the worktree directory and its sessions are already
            // gone by this point (that half always succeeds here) — a failed branch
            // deletion is reported as a distinct, non-blocking notice rather than
            // silently discarded, so choosing "delete the branch" that git then refuses
            // (e.g. unreachable commits) doesn't look like it silently kept the branch.
            Some(PendingOp::WorktreeDelete(dir)) => {
                if let OperationResult::WorktreeDeleted {
                    branch_delete_failed,
                    leftovers,
                } = result
                {
                    if branch_delete_failed {
                        app.core.notify_error(format!(
                            "The worktree \"{dir}\" was removed, but its branch could not \
                             be deleted (it may hold commits not present elsewhere)."
                        ));
                    }
                    // FR-023c/FR-023d: partial success. Lead with what *did* happen —
                    // the worktree is gone — so this does not read as a failed delete,
                    // then name the paths and their owner, which is the only part the
                    // user can act on. A bare error code named nothing and left them
                    // with a tree of tens of thousands of files to search (BUG-002).
                    if !leftovers.is_empty() {
                        app.core.notify_error(format!(
                            "The worktree \"{dir}\" was removed, but {}. You can delete \
                             {} yourself once you have permission to.",
                            describe_leftovers(&leftovers),
                            if leftovers.len() == 1 { "it" } else { "them" },
                        ));
                    }
                }
            }
            _ => {}
        },
        // FR-024: a stage push names the step in flight. Peeked, not removed — the
        // operation is still running and its terminal reply still needs the pending op.
        DaemonMsg::OperationProgress { req, stage, detail } => {
            if matches!(
                app.pending_ops.get(&req),
                Some(PendingOp::WorktreeCreate(_))
            ) {
                app.core
                    .update(Message::WorktreeForm(FormMsg::CreateStageChanged(
                        stage, detail,
                    )));
            }
        }
        DaemonMsg::OperationError {
            req,
            message,
            detail,
            ..
        } => {
            match app.pending_ops.remove(&req) {
                // A failed worktree create shows in the form (keeps it open to retry), not a
                // toast — mirroring the old local-create failure path. `detail` carries git's
                // own stderr verbatim (feature 010, FR-006/SC-003): for a submodule fetch
                // failure this is normally the only place that names which submodule failed
                // and why (auth/network/unreachable commit) — `message` alone is the generic
                // "git failed to create the worktree".
                Some(PendingOp::WorktreeCreate(_)) => {
                    app.core.update(Message::WorktreeForm(FormMsg::CreateFailed(
                        worktree_create_error_text(message, detail),
                    )));
                }
                // Feature 016: both branch queries back the open form, so their failures
                // belong on its own error line. A notification would be raised into the
                // surface the modal's scrim covers — invisible — and for the listing the
                // empty picker would then wrongly claim the repository has no branches.
                Some(PendingOp::BranchPreflight { .. }) => {
                    app.core.worktree_error =
                        Some(format!("Could not check the branch: {message}"));
                }
                Some(PendingOp::BranchList { .. }) => {
                    app.core.worktree_error = Some(format!("Could not list branches: {message}"));
                }
                Some(op) => app
                    .core
                    .notify_error(format!("Couldn't {}: {message}", op.describe())),
                None => {}
            }
        }
        // Diagnostics replies (Phase 10, FR-046): surface as notices.
        DaemonMsg::LogLocation { path, sink, .. } => {
            let where_ = match path {
                Some(p) => format!("a file at {}", p.display()),
                None => format!("{sink:?}"),
            };
            app.core
                .notify_info(format!("The session service logs to {where_}."));
        }
        DaemonMsg::RecentErrors { entries, .. } => {
            if entries.is_empty() {
                app.core
                    .notify_info("The session service reports no recent errors.");
            } else {
                let latest = entries.last().unwrap();
                app.core.notify_error(format!(
                    "The session service reported {} recent issue(s); most recent: [{}] {}",
                    entries.len(),
                    latest.level,
                    latest.message
                ));
            }
        }
        // Another window took over a project we held (US5, FR-024). Mark it read-only here —
        // input is suppressed and a "take over" banner is shown — but never terminate.
        DaemonMsg::Displaced { project, by } => {
            app.displaced.insert(project, by);
        }
        // A (re)attach was refused. `ProjectBusy` means another window holds it: surface the
        // same take-over banner as a live displacement, naming the current holder.
        DaemonMsg::Refused {
            reason:
                micold_core::protocol::messages::RefusalReason::ProjectBusy {
                    project, holder, ..
                },
        } => {
            app.displaced.insert(project, holder);
        }
        // An attach this window asked for was accepted (FR-024a). This is the fact that
        // falsifies a recorded displacement: the daemon decides who holds a project, and it
        // has just confirmed we do. Clearing here — rather than only on a full reconnect or
        // the banner's "Take over" button — is what lets a window that was once refused go
        // back to a project after the holder released it and simply type into it. Without
        // it, `displaced` is a latch: written by every refusal, cleared by almost nothing,
        // so the window renders a project it owns while suppressing its own input above a
        // banner naming a window that may have exited (BUG-007).
        //
        // `sessions` is deliberately ignored. It is built from `DaemonState::sessions_for`,
        // the raw durable projection with **no** live overlay, so its `activity` is always
        // `Unknown`, its labels lag the terminal title, and its `input_serial` is `0` even
        // for a session the daemon has been driving for hours — adopting it would re-create
        // BUG-006. The authoritative view arrives immediately after, as the `CatalogChanged`
        // that `refresh_worktrees_and_send` sends on the heels of every `Attached`, and that
        // one *is* overlaid.
        DaemonMsg::Attached { project, .. } => {
            app.displaced.remove(&project);
        }
        // Other control messages (Pong) are consumed as their flows land.
        _ => {}
    }
    Task::none()
}

pub fn on_connected(
    app: &mut App,
    outbox: micold_client::daemon::Outbox,
    catalog: CatalogSnapshot,
    settings: micold_core::protocol::messages::DaemonSettings,
) -> Task<Message> {
    // A fresh connection resyncs from authoritative state (FR-028): clear the transient
    // disconnected/displaced flags. If a project is still held by another window, the
    // re-attach below is refused and the displaced state is re-established from that reply.
    app.disconnected = false;
    app.displaced.clear();
    app.version_mismatch = None;
    app.build_mismatch = None;
    // The daemon is the single writer of settings + sessions; adopt what it reports
    // (FR-012a/FR-012b) — including environment-include, which this client's own
    // boot-time local read may predate (e.g. another window changed it while this one was
    // still starting up). Re-source env-include under the now-authoritative values.
    app.scrollback_lines = settings.scrollback_lines;
    app.env_include_enabled = settings.env_include_enabled;
    app.env_include_script_path = settings.env_include_script_path;
    app.env_include_timeout_secs = settings.env_include_timeout_secs;
    app.env_include_cache.clear();
    let cwd = default_resolution_cwd(&app.core);
    refresh_env_include(app, &cwd);
    reconcile_catalog(&mut app.core, &catalog, false);
    // The boot-time foreground resolve ran before this catalog existed, so for a client that has
    // just started it answered `NoSessionsForKey` against a project whose sessions were still on
    // the wire. Ask again now that they are here (`010` BUG-013) — before `active_session` is read
    // below, so the attach that follows views the restored session rather than the overview.
    // Guarded to that one case; a mid-session reconnect changes nothing.
    if let Some(outcomes) = app.core.resolve_foreground_after_catalog() {
        micold_client::app::drain(outcomes, |o| {
            micold_client::app::interpret(&mut app.core, o)
        });
    }
    // Record what this attach actually produced (`010` BUG-013). Written after the fold and the
    // re-resolve, so the counts describe the state the window is about to render from.
    crate::log_line(&attach_log_line(
        &catalog,
        app.core.workspace.active.as_deref(),
    ));
    // Adopt the daemon's per-session input position (FR-028a, T111). This process may be a
    // *new* client attached to sessions it did not start — after a package upgrade, or a
    // plain quit-and-reopen — in which case its stamper is empty and starting those counters
    // at 0 would put them behind the daemon, which then discards every keystroke as stale
    // (BUG-006). Part of the same resync as the flags and settings above: the daemon's
    // position is authoritative state, so re-read it rather than assume continuity.
    app.stamper.seed_from_catalog(&catalog);
    app.daemon_catalog = Some(catalog);
    // Attach to the active project and view its active session so the daemon starts
    // streaming grid frames for it (FR-011/FR-016).
    //
    // `app.daemon` is assigned before the sends, not after, because `view_and_start` below
    // reads it (BUG-002). Nothing between here and there can observe the difference: no
    // message is dispatched and no frame is drawn inside this arm.
    let project = app.core.workspace.active_project().map(|p| p.path.clone());
    app.daemon = Some(outbox);
    if let (Some(project), Some(daemon)) = (project, app.daemon.clone()) {
        daemon.send(ClientMsg::Attach {
            project: project.clone(),
            force: false,
        });
        match app.core.active_session {
            // Feature 025 restored a session at boot. Displaying it means *starting* it,
            // exactly as selecting it by hand does (FR-004a, contract §3.3a) — BUG-002.
            //
            // Viewing alone was not enough: `SetViewedSession` opens a view stream only for
            // a session the daemon is hosting, and after a restart it is hosting none, so
            // the restore produced a current session with no process and no way to get one
            // but the `restart` control. BUG-001 made that screen honest; this makes it
            // unnecessary.
            //
            // Through `view_and_start` rather than a third copy of its sends: it already
            // orders the pane size before the start (BUG-003, `006` FR-014a) and the view
            // after, and a start naming an already-running session is a no-op on the daemon
            // (`Session::start`), so a reconnect onto a live session costs nothing.
            Some(session) => view_and_start(app, session),
            // Landing on the project overview. FR-007 forbids choosing a session here, and
            // starting one would be that choice made twice — so only the view goes out.
            None => daemon.send(ClientMsg::SetViewedSession {
                project,
                session: None,
            }),
        }
    }
    Task::none()
}

/// Project rename (feature 001, FR-017): the daemon is the single writer, so route it through
/// the `ProjectRename` RPC (T055). The pure-core update applies it in memory — instant feedback,
/// validation, and closing the overlay — while the daemon persists it and reconciles other
/// windows. No local `persist()` happens here.
pub fn on_rename_confirmed(app: &mut App) -> Task<Message> {
    let draft = app
        .core
        .rename_draft
        .as_ref()
        .map(|d| (d.path.clone(), d.text.trim().to_string()));
    app.core.update(Message::RenameConfirmed);
    // Only send if the pure update accepted it (a rejected name leaves the draft in place).
    if app.core.rename_draft.is_none() {
        if let Some((path, display_name)) = draft {
            if !display_name.is_empty() {
                send_op(app, PendingOp::ProjectRename, move |req| {
                    ClientMsg::ProjectRename {
                        req,
                        path,
                        display_name,
                    }
                });
            }
        }
    }
    Task::none()
}

/// Forget a project (feature 014): route through the daemon's `ProjectRemove`, which stops the
/// project's sessions, drops its records, and deletes its per-project state file (FR-005/010),
/// then broadcasts the pruned catalog (T055). The pure reducer drops the record + clears the
/// active pointer in memory for instant feedback; nothing inside the project folder is touched.
pub fn on_project_forget_confirmed(app: &mut App) -> Task<Message> {
    if let Some(path) = app.core.forget_target.clone() {
        app.grids.retain(|id, _| {
            !app.core
                .workspace
                .session_ids_of_project(&path)
                .contains(id)
        });
        let remove_path = path.clone();
        send_op(app, PendingOp::ProjectRemove, move |req| {
            ClientMsg::ProjectRemove {
                req,
                path: remove_path,
            }
        });
        // Release this client's attachment on the project it is forgetting.
        if let Some(d) = &app.daemon {
            d.send(ClientMsg::Detach { project: path });
        }
    }
    app.core.update(Message::ProjectForgetConfirmed);
    Task::none()
}

/// Worktree rename (feature 008, FR-014/FR-015): the daemon is the single writer of the
/// display-name override, so route it through the `WorktreeRename` RPC (T055). The pure-core
/// update still applies it in memory for instant feedback (validated + closes the overlay);
/// the daemon persists it and reconciles a second window via `CatalogChanged`. No local
/// `persist()` — the daemon owns the durable file now.
pub fn on_worktree_rename_confirmed(app: &mut App) -> Task<Message> {
    let draft = app
        .core
        .worktree_rename_draft
        .as_ref()
        .map(|d| (d.dir_name.clone(), d.text.trim().to_string()));
    let project = app.core.workspace.active.clone();
    app.core.update(Message::WorktreeRenameConfirmed);
    // Only send if the pure update accepted it (a rejected name leaves the draft in place).
    if app.core.worktree_rename_draft.is_none() {
        if let (Some((dir_name, display_name)), Some(project)) = (draft, project) {
            if !display_name.is_empty() {
                send_op(
                    app,
                    PendingOp::WorktreeRename(dir_name.clone()),
                    move |req| ClientMsg::WorktreeRename {
                        req,
                        project,
                        dir_name,
                        display_name,
                    },
                );
            }
        }
    }
    Task::none()
}

/// Validate the form, then create the worktree (incl. any submodule fetch) via git,
/// off the update() thread so a slow fetch doesn't freeze the UI (feature 010,
/// research R4). AddWorktreeSubmitted/WorktreeCreated/WorktreeCreateFailed keep their
/// existing meaning; WorktreeCreateStarted is dispatched first so the form can show it.
/// Submitting classifies the target branch first (feature 016, FR-001). A free name
/// creates immediately, exactly as before; anything else becomes a decision for the user
/// rather than the dead-end "a branch with that name already exists" error.
pub fn on_add_worktree_submitted(app: &mut App) -> Task<Message> {
    app.core.update(Message::WorktreeForm(FormMsg::Submitted));
    let Some(form) = app.core.worktree_form.clone() else {
        return Task::none();
    };
    if form.status != WorktreeFormStatus::Editing || form.resolution.is_prompting() {
        return Task::none(); // create in flight, or a prompt is already open.
    }
    // The form stays `Editing` while the pre-flight RPC is in flight (there is nothing to
    // show yet and the answer may be a prompt, not a create), so `status` alone does not
    // stop a second submit. Without this a double-click sends two pre-flights, both come
    // back `Free`, and two `WorktreeCreate`s race for the same directory.
    if app
        .pending_ops
        .values()
        .any(|op| matches!(op, PendingOp::BranchPreflight { .. }))
    {
        return Task::none();
    }
    let Ok(names) = form.preview() else {
        return Task::none(); // validation error already recorded by the reducer
    };
    let Some(project) = app.core.workspace.active.clone() else {
        return Task::none();
    };
    // Feature 016: classify the name before creating anything. Git lives on the daemon
    // now, so pre-flight is an RPC — the reply decides whether this becomes a create or a
    // prompt. `PendingOp` carries what the answer needs so nothing is recomputed.
    let picked = form.source == BranchSource::Existing;
    // The remote the user named by picking that specific row, so a branch that exists on
    // several remotes tracks the one they chose (spec Edge Cases).
    let preferred_remote = form.selected_branch.as_ref().and_then(|c| match &c.origin {
        BranchOrigin::Remote { remote } => Some(remote.clone()),
        BranchOrigin::Local => None,
    });
    let (branch, dir_name) = (names.branch.clone(), names.dir_name.clone());
    let asked_for = project.clone();
    send_op(
        app,
        PendingOp::BranchPreflight {
            project: asked_for,
            names,
            picked,
            preferred_remote,
        },
        move |req| ClientMsg::BranchPreflight {
            req,
            project,
            branch,
            dir_name,
        },
    );
    Task::none()
}

/// The user answered the prompt: create under the mode they chose. Overwrite cannot arrive
/// here — it only ever comes through the confirmation below (FR-005).
///
/// Both arms check the state the reducer requires BEFORE letting it clear the prompt: the
/// reducer refuses transitions it considers illegal, and acting anyway would run the
/// create the reducer just declined to acknowledge — an `Overwrite` that never passed the
/// destructive confirmation, in the worst case.
pub fn on_add_worktree_resolution_chosen(app: &mut App, mode: CreateMode) -> Task<Message> {
    let answering = app.core.worktree_form.as_ref().is_some_and(|f| {
        matches!(f.resolution, ResolutionState::Choosing { .. })
            && !matches!(mode, CreateMode::Overwrite)
    });
    app.core
        .update(Message::WorktreeForm(FormMsg::ResolutionChosen(
            mode.clone(),
        )));
    if !answering {
        return Task::none();
    }
    start_resolved_create(app, mode)
}

pub fn on_add_worktree_overwrite_confirmed(app: &mut App) -> Task<Message> {
    let confirmed = app
        .core
        .worktree_form
        .as_ref()
        .is_some_and(|f| matches!(f.resolution, ResolutionState::ConfirmingOverwrite { .. }));
    app.core
        .update(Message::WorktreeForm(FormMsg::OverwriteConfirmed));
    if !confirmed {
        return Task::none();
    }
    start_resolved_create(app, CreateMode::Overwrite)
}

/// Switching to the existing-branch picker lists what the repository already has
/// (feature 016, FR-011). The daemon reads local ref storage only — nothing is fetched.
pub fn on_add_worktree_source_changed(app: &mut App, source: BranchSource) -> Task<Message> {
    app.core
        .update(Message::WorktreeForm(FormMsg::SourceChanged(source)));
    if source != BranchSource::Existing {
        return Task::none();
    }
    let Some(project) = app.core.workspace.active.clone() else {
        return Task::none();
    };
    let asked_for = project.clone();
    send_op(
        app,
        PendingOp::BranchList { project: asked_for },
        move |req| ClientMsg::BranchList { req, project },
    );
    Task::none()
}

/// Start a new session at a location — a worktree or the project root ("Default",
/// feature 010): spawn `claude` and stream it (FR-010/012/013). A `Default` location
/// never creates, modifies, or removes a worktree (FR-002) — it simply runs in `repo`
/// itself, so this arm never calls into `micold_core::worktree`.
pub fn on_session_start_requested(app: &mut App, location: SessionLocation) -> Task<Message> {
    // Correlated create: the daemon owns the id + catalog. The new session arrives via the
    // `CatalogChanged` push (reconciled into the core) and is selected + focused when the
    // `OperationOk { SessionCreated }` reply names its id.
    if let Some(project) = app.core.workspace.active.clone() {
        let worktree_dir = match &location {
            SessionLocation::Worktree(dir) => dir.clone(),
            SessionLocation::Default => String::new(),
        };
        send_op(app, PendingOp::CreateSession, |req| {
            ClientMsg::SessionCreate {
                req,
                project,
                worktree_dir,
            }
        });
    }
    Task::none()
}

/// Selecting a session reattaches/resumes whichever process its persisted mode selects
/// (FR-005, FR-011) — an Idle AI CLI session resumes via `claude --resume` (FR-023a); a
/// session last left in Regular mode gets a fresh shell instead.
pub fn on_session_selected(app: &mut App, id: SessionId) -> Task<Message> {
    app.core.update(Message::SessionSelected(id));
    // View the selected session (the daemon streams its grid), resuming it if idle — the
    // same sequence `view_and_start` performs for every other path that displays a session,
    // called rather than repeated so the pane size that now precedes the start (BUG-003,
    // FR-014a) cannot be added to one copy and not the other.
    view_and_start(app, id);
    // Nothing further: the reducer's `SessionSelected` arm focuses the terminal (FR-011),
    // and selecting from the sidebar no longer releases it, so there is no race left to
    // win with a follow-up message (feature 023, research R3).
    Task::none()
}

/// Close a session: kill both its processes (AI CLI and shell, feature 010 FR-014) and
/// drop the runtime handles. The pure core archives (not deletes) the record (FR-015a,
/// bugfix BUG-003); here we additionally record the durable, provider-side suppression
/// marker (FR-020c) so a still-existing `claude` transcript is never reconstructed by
/// reconciliation on a later project open.
/// Close (archive) a session: route through the daemon's `SessionDelete`, which archives it
/// durably (anti-resurrection marker) and stops its process (T055). The pure-core update
/// archives the record in memory for instant feedback; the daemon reconciles other windows.
pub fn on_session_close_requested(app: &mut App, id: SessionId) -> Task<Message> {
    app.grids.remove(&id);
    // Release the input counter too (T114): ids are unique UUIDs so it can never be reused,
    // and a session being archived will take no more input. Never on a mere detach — the
    // counter must survive a reconnect for loss detection to hold.
    app.stamper.forget(id);
    send_op(app, PendingOp::DeleteSession, move |req| {
        ClientMsg::SessionDelete { req, session: id }
    });
    app.core.update(Message::SessionCloseRequested(id));
    Task::none()
}

/// Permanently remove a session (bugfix BUG-003, FR-015c): the same daemon `SessionDelete` —
/// the daemon has no hard-delete, so a remove is an archive with a durable tombstone, which
/// also suppresses any future reconciliation (FR-020c). The pure core drops the record.
pub fn on_session_remove_confirmed(app: &mut App) -> Task<Message> {
    if let Some(id) = app.core.session_remove_target {
        app.grids.remove(&id);
        app.stamper.forget(id); // T114, as in the close path above.
        send_op(app, PendingOp::DeleteSession, move |req| {
            ClientMsg::SessionDelete { req, session: id }
        });
    }
    app.core.update(Message::SessionRemoveConfirmed);
    Task::none()
}

/// The AI tab was pressed — display the session's AI CLI and attach its process (feature 027,
/// FR-002; feature 010 FR-001–FR-004 for what "attach" means).
///
/// # Why this arm has to exist
///
/// It is the deleted mode toggle's other half. `Message::TerminalAiCliSelected`'s reducer sets the
/// mode and nothing more (feature 026 FR-006), and until feature 027 the message had **no arm in
/// `main.rs` at all** — it fell through to the catch-all, which runs the reducer and stops. So the
/// AI tab moved the mark while the daemon went on streaming and driving whichever shell instance
/// was attached: the strip said AI CLI and the keys went to bash.
///
/// That was invisible while a mode toggle existed to do the attach, and it only opens after a trip
/// through Regular mode — which is exactly the trip a tab strip invites. `tests` below hold it.
///
/// Unlike the toggle it replaces this never opens a shell instance: a session has exactly one AI
/// CLI process and it is not created here. Neither process is killed as a side effect (010 FR-006)
/// — the previously-attached one stops being displayed and keeps running (research R6).
pub fn on_terminal_ai_cli_selected(app: &mut App, id: SessionId) -> Task<Message> {
    app.core.update(Message::TerminalAiCliSelected(id));
    attach_current_process(app, id);
    Task::none()
}

/// Manually restart the active session's currently-attached, not-running process
/// (FR-013) — the shell never auto-restarts, so this is its only path back; also covers
/// an Idle/Failed AI CLI, which previously had no explicit affordance. Also re-sources the
/// environment-include script fresh (feature 011, FR-007) — the spec's Clarifications name
/// this restart control as a manual-retry path for a previously-failed script, alongside
/// the Settings-save refresh trigger. Unlike the passive reattach callers below, this is
/// also a direct user restart request, so it must cover a Regular Terminal instance that
/// has already `Exited` — `explicit_restart = true` lets `ensure_attached_process`'s
/// `Regular` branch spawn it, the same case `Message::ShellInstanceRestartRequested`
/// handles for a background instance.
pub fn on_terminal_restart_requested(app: &mut App) -> Task<Message> {
    if let Some(id) = app.core.active_session {
        // Re-source fresh for this session's own directory only (BUG-002) — other
        // cached directories are untouched, since only this one needs a new attempt.
        if let Some((cwd, _, _)) = session_cwd_mode_and_active_shell(&app.core, id) {
            refresh_env_include(app, &cwd);
        }
        view_and_start(app, id);
    }
    Task::none()
}

/// Manually restart one specific Regular Terminal instance (feature 011, FR-010) —
/// independent of `active_shell`, so a background instance can be restarted without first
/// switching to it. A no-op if that instance's process is already running (idempotent,
/// mirrors `ensure_attached_process`'s reattach-for-free check). Addressed by the
/// originating `SessionId` (not `app.core.active_session`) so this can't misapply to a
/// same-numbered instance of a different session if the active session changed in the
/// same message batch.
pub fn on_shell_instance_restart_requested(
    app: &mut App,
    id: SessionId,
    shell_id: ShellInstanceId,
) -> Task<Message> {
    if let Some(d) = &app.daemon {
        d.send(ClientMsg::SessionRestartShell {
            session: id,
            instance: shell_id,
        });
    }
    app.core
        .update(Message::ShellInstanceRestartRequested(id, shell_id));
    Task::none()
}

/// Open an additional Regular Terminal instance for the active session (feature 011,
/// FR-001–FR-003, FR-007; contracts/shell-instance-lifecycle.md) — the "+" bottom-bar
/// control or the Ctrl+Shift+T/Cmd+Shift+T shortcut. A no-op outside Regular mode (FR-019
/// edge case: the control/shortcut does nothing, and does not switch modes). Unlike
/// `ensure_attached_process` (spawn-if-absent/reattach), this always opens a brand-new
/// instance, even if one is already running.
pub fn on_shell_instance_open_requested(app: &mut App) -> Task<Message> {
    if let Some(id) = app.core.active_session {
        let shell_id = {
            let Some((_, session)) = app.core.workspace.find_session_mut(id) else {
                return Task::none();
            };
            // Feature 027 FR-004: the "+" is shown on the AI tab too now, so it has to *take* the
            // user to Regular rather than assume they are already there. Feature 011's FR-019 made
            // both this control and its chord a no-op outside Regular, which was coherent only
            // while a mode toggle existed and the "+" was hidden there — the no-op was unreachable.
            // Reachable, it is a control that reports success and changes nothing the user can see,
            // and a session sitting on its AI tab with no instances would have no way to make one.
            session.set_mode(TerminalMode::Regular);
            session.open_shell_instance()
        };
        if let Some(d) = &app.daemon {
            d.send(ClientMsg::SessionOpenShell {
                session: id,
                instance: shell_id,
            });
        }
        attach_current_process(app, id);
        // The new instance is what the user is now looking at, so it holds the keyboard (023
        // FR-011) and is the newly marked tab (026 FR-002d). This reducer used to be unreachable
        // from here: `update_inner` routes the message to this handler *instead of* the core, so
        // nothing ran it. Harmless while the "+" could not change which pane was displayed; not
        // harmless now that it can.
        app.core.update(Message::ShellInstanceOpenRequested);
    }
    Task::none()
}

/// Close an individual Regular Terminal instance (feature 011, FR-011–FR-013,
/// FR-018-consistent teardown) — kills and removes only that one `RuntimeTerminal`,
/// leaving sibling instances and the AI CLI process untouched. If this was the session's
/// last instance, the pure reducer flips `mode` back to `AiCli` (FR-013); reattach the AI
/// CLI process via the same shared path the primary toggle already uses (a no-op if it's
/// already attached). Addressed by the originating `SessionId` (not
/// `app.core.active_session`) — see `Message::ShellInstanceSelected`'s doc comment.
pub fn on_shell_instance_close_requested(
    app: &mut App,
    id: SessionId,
    shell_id: ShellInstanceId,
) -> Task<Message> {
    if let Some(d) = &app.daemon {
        d.send(ClientMsg::SessionCloseShell {
            session: id,
            instance: shell_id,
        });
    }
    // Core close reassigns active_shell / reverts mode to AiCli when the last one closes.
    app.core
        .update(Message::ShellInstanceCloseRequested(id, shell_id));
    // Re-attach whatever process the session now shows (a sibling instance, or the primary).
    attach_current_process(app, id);
    Task::none()
}

/// Switch which Regular-terminal instance is shown (feature 011 FR-004): select it in the
/// core, then attach that process on the daemon so its grid streams.
pub fn on_shell_instance_selected(
    app: &mut App,
    id: SessionId,
    shell_id: ShellInstanceId,
) -> Task<Message> {
    app.core
        .update(Message::ShellInstanceSelected(id, shell_id));
    attach_current_process(app, id);
    Task::none()
}

/// Stream live keystrokes/paste to the displayed session's currently-ATTACHED process
/// (FR-007/FR-008), but only while that process is Running (FR-012a, feature 010 extends
/// the write-gate to the shell): input to a non-running process is discarded, not
/// buffered.
pub fn on_terminal_bytes(app: &mut App, bytes: Vec<u8>) -> Task<Message> {
    // A window displaced from the active project is read-only: it MUST send zero further
    // input (FR-024). Bail before stamping so no serial is consumed (a consumed-but-unsent
    // serial would be an unrecoverable gap in the input log, G2).
    if active_project_displaced(app) {
        return Task::none();
    }
    if let Some(id) = app.core.active_session {
        // The daemon owns process liveness: it routes input to the session's attached
        // process and drops it harmlessly if that process isn't running. Gating on a
        // client-side lifecycle field is wrong now (the client no longer tracks process
        // state, and the daemon never marks the catalog session Running), so we send
        // whenever connected. Input is stamped with a monotonic per-session serial (G2).
        let msg = app.stamper.stamp(id, bytes);
        if let Some(d) = &app.daemon {
            d.send(msg);
        }
        // Any live keystroke means the view is at the live bottom again.
        app.display_offset = 0;
    }
    Task::none()
}

/// Reflow the displayed session's daemon PTY + grid to the visible size (FR-014/FR-015).
pub fn on_terminal_resized(app: &mut App, cols: u16, rows: u16) -> Task<Message> {
    // Remember the pane's live size so the next started session starts at it too.
    app.last_grid = Some((cols, rows));
    if let (Some(id), Some(d)) = (app.core.active_session, &app.daemon) {
        d.send(ClientMsg::SessionResize {
            session: id,
            cols,
            rows,
        });
    }
    Task::none()
}

/// Confirmed worktree delete (feature 008, FR-020): terminate the worktree's session
/// processes, remove its git worktree + branch + directory, then drop the records and
/// persist. Ordered per `CleanupStep`; every git step is idempotent (FR-023).
/// Worktree delete (feature 008/013): route through the daemon, which removes the worktree
/// (git off its runtime), and — gated on the removal actually succeeding — archives the
/// worktree's sessions and broadcasts the new catalog (T055). A failed delete surfaces as an
/// `OperationError` and the worktree reappears on the next reconcile, so the optimistic local
/// drop below self-heals. `stop_sessions: true` mirrors the old behaviour (it always stopped
/// the worktree's sessions first).
///
/// NOTE: the daemon keeps the branch (no keep/delete wire flag yet), so the confirm dialog's
/// "delete branch" choice is currently a no-op — branch deletion needs a wire field (deferred).
/// 016 BUG-002 (FR-027/FR-030). Both are settings-only on the daemon side: no git command runs, and
/// the worktree stays exactly where it is. The reply carries the row as the daemon's own discovery
/// sees it, so the client renders that rather than deriving a second answer to the same question.
pub fn on_worktree_include_requested(app: &mut App, path: PathBuf) -> Task<Message> {
    if let Some(project) = app.core.workspace.active.clone() {
        let p = path.clone();
        send_op(app, PendingOp::WorktreeInclude(path), move |req| {
            ClientMsg::WorktreeInclude {
                req,
                project,
                path: p,
            }
        });
    }
    Task::none()
}

pub fn on_worktree_exclude_requested(app: &mut App, dir: String) -> Task<Message> {
    let path = app
        .core
        .worktrees
        .iter()
        .find(|w| w.dir_name == dir && w.included)
        .map(|w| w.path.clone());
    if let (Some(path), Some(project)) = (path, app.core.workspace.active.clone()) {
        let p = path.clone();
        send_op(app, PendingOp::WorktreeExclude(path), move |req| {
            ClientMsg::WorktreeExclude {
                req,
                project,
                path: p,
            }
        });
    }
    app.core.update(Message::WorktreeExcludeRequested(dir));
    Task::none()
}

pub fn on_worktree_delete_confirmed(app: &mut App) -> Task<Message> {
    let target = app.core.worktree_delete_target.clone();
    if let (Some(dir), Some(project)) = (target, app.core.workspace.active.clone()) {
        // Drop this path's cached env-include snapshot (BUG-002): a worktree recreated for
        // the same branch reuses the exact path, and a stale snapshot would linger forever.
        let cwd = session_cwd_for_location(&project, &SessionLocation::Worktree(dir.clone()));
        app.env_include_cache.remove(&cwd);
        let (p, d) = (project, dir.clone());
        // Feature 013 (FR-011/FR-012): the user's explicit keep/delete choice from the
        // confirm dialog, defaulting to "delete the branch" (`worktree_delete_keep_branch`
        // defaults to `false`).
        let delete_branch = !app.core.worktree_delete_keep_branch;
        send_op(app, PendingOp::WorktreeDelete(dir), move |req| {
            ClientMsg::WorktreeDelete {
                req,
                project: p,
                dir_name: d,
                stop_sessions: true,
                delete_branch,
            }
        });
    }
    // Optimistically drop the records + dismiss the dialog; the daemon's `CatalogChanged`
    // reconciles the truth (re-adding the worktree on a failed delete).
    app.core.update(Message::WorktreeDeleteConfirmed);
    Task::none()
}

// ---------------------------------------------------------------------------------------------
// Helpers the arms above share (feature 021, T055)
//
// These were free functions in `main.rs` with no caller outside the daemon arms — the shape
// FR-001a asks about. Moved verbatim with them.
// ---------------------------------------------------------------------------------------------

/// View a session on the daemon — start/resume it and stream its grid — resetting the local
/// selection and scroll for the newly-displayed session.
pub fn view_and_start(app: &mut App, id: SessionId) {
    app.selection = None;
    app.display_offset = 0;
    if let (Some(project), Some(d)) = (app.core.workspace.active.clone(), &app.daemon) {
        send_pane_size(app, id);
        d.send(ClientMsg::SessionStart { session: id });
        d.send(ClientMsg::SetViewedSession {
            project,
            session: Some(id),
        });
    }
}

/// Tell the daemon what size to start `id` at, **before** its `SessionStart` (BUG-003, FR-014a).
///
/// The pane widget only publishes `Message::TerminalResized` when its own size *changes*, so a
/// session started into a window the user is not resizing is never told anything — it used to come
/// up at the daemon's 100×30 spawn seed and stay there until the next window resize. `App::last_grid`
/// is the last size the pane published; stating it here is what makes it a size the *next* session
/// starts at rather than only one the current session was corrected to. Ordered before the start so
/// the daemon has it recorded when the spawn reads it (the daemon also honours it if it arrives
/// afterwards — `010` FR-020a — but only the ordering makes the spawn itself right).
///
/// A no-op before the first frame has laid out a pane (`last_grid` is `None`), where the daemon's
/// own default correctly applies.
pub fn send_pane_size(app: &App, id: SessionId) {
    if let (Some((cols, rows)), Some(d)) = (app.last_grid, &app.daemon) {
        d.send(ClientMsg::SessionResize {
            session: id,
            cols,
            rows,
        });
    }
}

/// Phrase the surviving paths for a partial-success delete notice (FR-023d, BUG-002).
///
/// Names the owner when the platform reported one, because that is what tells the user *why* the
/// app could not remove it and what they need in order to: "owned by another user (uid 0)" points
/// straight at a container that wrote build output as root, where a bare path alone would read as
/// an unexplained failure. Long lists are truncated — the report is already capped, and naming a
/// couple of blockers plus a count is what a person can act on.
pub fn describe_leftovers(leftovers: &[micold_core::worktree::Leftover]) -> String {
    const NAMED: usize = 2;
    let named = leftovers
        .iter()
        .take(NAMED)
        .map(|l| match l.foreign_uid {
            Some(uid) => format!("{} (owned by another user, uid {uid})", l.path.display()),
            None => l.path.display().to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    match leftovers.len().saturating_sub(NAMED) {
        0 => format!("these paths could not be removed: {named}"),
        rest => format!("these paths could not be removed: {named}, and {rest} more"),
    }
}

/// Which daemon process the session currently shows (feature 011): its `Primary` (the AI CLI, or a
/// persisted Regular session's own shell) unless a specific Regular-terminal instance is selected.
pub fn session_process(session: &Session) -> SessionProcess {
    match (session.mode, session.active_shell) {
        (TerminalMode::Regular, Some(sid)) if !session.shells.is_empty() => {
            SessionProcess::Shell(sid)
        }
        _ => SessionProcess::Primary,
    }
}

/// Tell the daemon which of a session's processes to attach (stream + drive), based on its current
/// mode + active shell, and reset the local view (selection + scroll) for the switch. Called
/// whenever the attached process changes: mode toggle, instance select/open/close.
pub fn attach_current_process(app: &mut App, id: SessionId) {
    let process = app
        .core
        .workspace
        .find_session(id)
        .map(|(_, s)| session_process(s));
    app.selection = None;
    app.display_offset = 0;
    if let (Some(process), Some(d)) = (process, &app.daemon) {
        d.send(ClientMsg::SessionAttachProcess {
            session: id,
            process,
        });
    }
}

/// Discover the active project's worktrees from git + the filesystem (FR-018/018a). Delegates to the
/// shared `micold_core::worktree::discover` so the client and daemon can never diverge in how a
/// worktree is discovered.
/// Send the create for a resolved mode, re-deriving the names from the form (feature 016).
///
/// Shared by both resolution answers so `Overwrite` and the non-destructive modes take exactly one
/// path to the daemon. The daemon re-verifies the mode against a fresh pre-flight before touching
/// anything (FR-009), so a branch that changed while the prompt was open fails cleanly rather than
/// acting on a stale answer.
pub fn start_resolved_create(app: &mut App, mode: CreateMode) -> Task<Message> {
    let Some(form) = app.core.worktree_form.clone() else {
        return Task::none();
    };
    // Same double-submit guard `AddWorktreeSubmitted` applies: the answer buttons stop being
    // rendered once the prompt resolves, but two clicks can queue two messages before the next
    // render, and the reducer's second pass is a no-op — only this check stops the second one
    // from launching a concurrent create of the same worktree.
    if form.status != WorktreeFormStatus::Editing {
        return Task::none();
    }
    let Ok(names) = form.preview() else {
        return Task::none();
    };
    let Some(project) = app.core.workspace.active.clone() else {
        return Task::none();
    };
    send_worktree_create(app, project, names, mode);
    Task::none()
}

/// Hand a fully-resolved create to the daemon and put the form into its in-progress state.
pub fn send_worktree_create(
    app: &mut App,
    project: PathBuf,
    names: micold_core::naming::DerivedNames,
    mode: CreateMode,
) {
    app.core
        .update(Message::WorktreeForm(FormMsg::CreateStarted(mode.clone())));
    let (branch, dir_name) = (names.branch, names.dir_name);
    // The mode is not duplicated here: `WorktreeCreateStarted` above already put it on the form,
    // which is where the stage label reads it from (FR-024).
    send_op(
        app,
        PendingOp::WorktreeCreate(dir_name.clone()),
        move |req| ClientMsg::WorktreeCreate {
            req,
            project,
            branch,
            dir_name,
            mode,
        },
    );
}

/// The worktree-creation failure text shown in the form (feature 010, FR-006/SC-003): appends
/// `detail` (the daemon's `OperationError.detail`, git's own stderr verbatim) to `message` when
/// present and non-blank. For a submodule fetch failure, `message` alone is the generic "git
/// failed to create the worktree" — `detail` is normally the only place that names which
/// submodule failed and why (auth/network/unreachable commit).
pub fn worktree_create_error_text(message: String, detail: Option<String>) -> String {
    match detail {
        Some(detail) if !detail.trim().is_empty() => format!("{message}: {}", detail.trim()),
        _ => message,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    // The wire/domain types the fixtures below build. They were in scope via the module's own
    // imports until the fold moved to `micold_client::catalog_sync`; the production code here no
    // longer names them, so the tests import them for themselves rather than the module carrying
    // imports only its tests use.
    use micold_client::app::State;
    use micold_core::protocol::messages::WireLifecycle;
    use micold_core::session::{SessionLabel, SessionLifecycle};

    // Convergence fix (retrofit session, 2026-07-27): the daemon's OperationError.detail (git's
    // own stderr, e.g. naming which submodule failed and why) was destructured with `..` and
    // silently discarded — the worktree-creation form only ever showed the generic
    // "git failed to create the worktree" message, never the diagnostic FR-006/SC-003 requires.
    #[test]
    fn worktree_create_error_appends_a_non_blank_detail() {
        assert_eq!(
            worktree_create_error_text(
                "git failed to create the worktree".to_string(),
                Some(
                    "fatal: could not read Username for 'https://example.com': terminal prompts disabled"
                        .to_string()
                ),
            ),
            "git failed to create the worktree: fatal: could not read Username for \
             'https://example.com': terminal prompts disabled"
        );
    }

    #[test]
    fn worktree_create_error_falls_back_to_message_when_detail_is_absent_or_blank() {
        assert_eq!(
            worktree_create_error_text("git failed to create the worktree".to_string(), None),
            "git failed to create the worktree"
        );
        assert_eq!(
            worktree_create_error_text(
                "git failed to create the worktree".to_string(),
                Some("   ".to_string())
            ),
            "git failed to create the worktree"
        );
    }
    use crate::tests::base_app;
    use micold_core::protocol::messages::{ActivitySignal, ProjectSnapshot, SessionSummary};

    pub(crate) fn summary(id: SessionId, title: &str, lifecycle: WireLifecycle) -> SessionSummary {
        summary_at(id, title, lifecycle, 0)
    }

    pub(crate) fn summary_at(
        id: SessionId,
        title: &str,
        lifecycle: WireLifecycle,
        input_serial: u64,
    ) -> SessionSummary {
        SessionSummary {
            id,
            worktree_dir: None,
            title: SessionLabel::Named(title.into()),
            lifecycle,
            activity: ActivitySignal::Unknown,
            input_serial,
            live_shells: Vec::new(),
        }
    }

    /// `012` BUG-003: a session summary that names live shell instances.
    pub(crate) fn summary_with_live_shells(
        id: SessionId,
        title: &str,
        lifecycle: WireLifecycle,
        live_shells: Vec<micold_core::session::ShellInstanceId>,
    ) -> SessionSummary {
        SessionSummary {
            live_shells,
            ..summary(id, title, lifecycle)
        }
    }

    /// An `App` with a live outbox, plus the receiving end of it.
    ///
    /// `Outbox::new` is `pub` precisely so a test can hold both ends without a running daemon —
    /// `daemon.rs` records why. Nothing here starts a connection; what is asserted is what the
    /// client *would* have put on the wire.
    fn connected_app() -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        (app, rx)
    }

    /// BUG-002 (FR-004a, contract §3.3a): a project switch **starts** the session it restores.
    ///
    /// The switch and the launch reach the same dead end for the same reason — `SetViewedSession`
    /// opens a view stream only for a session the daemon is hosting — but the switch hides it,
    /// because a session you switch to within a run is normally still running. It surfaces when the
    /// remembered session has stopped, which since feature 008's BUG-001 is a session the restore
    /// honours rather than skips.
    #[test]
    fn switching_projects_starts_the_session_it_restores() {
        let old = PathBuf::from("/repo/old");
        let new = PathBuf::from("/repo/new");
        let id = SessionId::new();
        let (mut app, mut rx) = connected_app();
        // `restore_after_activation` has already run: the target is active and its remembered
        // session is current.
        app.core.workspace.active = Some(new.clone());
        app.core.active_session = Some(id);

        switch_daemon_attachment(&mut app, Some(old.clone()), &new);

        let mut sent = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            sent.push(msg);
        }
        assert!(
            sent.iter()
                .any(|m| matches!(m, ClientMsg::Detach { project: p } if *p == old)),
            "the outgoing project is released"
        );
        assert!(
            sent.iter()
                .any(|m| matches!(m, ClientMsg::Attach { project: p, .. } if *p == new)),
            "the incoming project is attached"
        );
        assert!(
            sent.iter()
                .any(|m| matches!(m, ClientMsg::SessionStart { session } if *session == id)),
            "and the restored session is started, exactly as selecting it by hand would"
        );
    }

    /// The bound, at this seam: a switch that restores nothing starts nothing, and still tells the
    /// daemon so (FR-007, SC-005a).
    #[test]
    fn switching_to_a_project_with_no_memory_starts_nothing() {
        let new = PathBuf::from("/repo/new");
        let (mut app, mut rx) = connected_app();
        app.core.workspace.active = Some(new.clone());
        assert_eq!(app.core.active_session, None);

        switch_daemon_attachment(&mut app, None, &new);

        let mut sent = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            sent.push(msg);
        }
        assert!(
            !sent
                .iter()
                .any(|m| matches!(m, ClientMsg::SessionStart { .. })),
            "arriving at a project overview runs nothing"
        );
        assert!(
            sent.iter()
                .any(|m| matches!(m, ClientMsg::SetViewedSession { session: None, .. })),
            "the daemon is still told that no session is viewed"
        );
    }

    /// The disconnected path says *which* thing did not happen.
    ///
    /// `send_op` is the only place a mutating action learns there is no daemon, and it is reached
    /// from eleven call sites that have already told the user something is under way — a bare
    /// "not connected" would leave them guessing which of them was dropped. `describe` exists for
    /// this sentence and nothing else, so nothing else would notice if it stopped being used.
    #[test]
    fn an_op_attempted_while_disconnected_names_what_it_could_not_do() {
        let mut app = base_app();
        assert!(app.daemon.is_none());

        send_op(
            &mut app,
            PendingOp::WorktreeCreate("feat-x".into()),
            |req| ClientMsg::ProjectAdd {
                req,
                path: PathBuf::from("/unused"),
            },
        );

        let notice = app
            .core
            .notify
            .visible()
            .expect("a dropped op must be reported");
        assert!(
            notice.message.contains(r#"create the worktree "feat-x""#),
            "the notice does not name the op: {:?}",
            notice.message
        );
        assert!(
            app.pending_ops.is_empty() && app.next_req == 0,
            "nothing was sent, so nothing may be recorded in flight"
        );
    }

    /// Every op gets its own correlation id, and the id on the wire is the id in the table.
    ///
    /// The reply-matching in `update_inner` looks the `req` up in `pending_ops`; if the recorded
    /// key and the sent one ever diverged, every reply would resolve to "unknown op" and the two
    /// halves would still look correct read separately. Asserted together for that reason.
    #[test]
    fn each_op_takes_the_next_correlation_id_and_is_recorded_under_it() {
        let (mut app, mut rx) = connected_app();

        send_op(&mut app, PendingOp::ProjectAdd, |req| {
            ClientMsg::ProjectAdd {
                req,
                path: PathBuf::from("/a"),
            }
        });
        send_op(&mut app, PendingOp::ProjectRemove, |req| {
            ClientMsg::ProjectRemove {
                req,
                path: PathBuf::from("/a"),
            }
        });

        let sent: Vec<u64> = (0..2)
            .map(|_| match rx.try_recv() {
                Ok(ClientMsg::ProjectAdd { req, .. })
                | Ok(ClientMsg::ProjectRemove { req, .. }) => req,
                other => panic!("expected a correlated op, got {other:?}"),
            })
            .collect();
        assert_eq!(sent, vec![0, 1], "each op allocates the next id");
        assert_eq!(app.next_req, 2);

        assert!(matches!(
            app.pending_ops.get(&sent[0]),
            Some(PendingOp::ProjectAdd)
        ));
        assert!(matches!(
            app.pending_ops.get(&sent[1]),
            Some(PendingOp::ProjectRemove)
        ));
    }

    /// A project switch releases the old attachment before taking the new one.
    ///
    /// Order is the assertion. The daemon allows one attached window per project, so an `Attach`
    /// that arrives before the `Detach` of a project this same window still holds is a window
    /// racing itself — and `SetViewedSession` must follow the `Attach` it refers to, or it names a
    /// project this client is not yet attached to.
    #[test]
    fn switching_projects_releases_the_old_attachment_before_taking_the_new() {
        let (mut app, mut rx) = connected_app();
        let viewed = SessionId::new();
        app.core.active_session = Some(viewed);
        // The production precondition, now load-bearing (BUG-002): both callers activate the target
        // before calling this, and the start goes through `view_and_start`, which reads the project
        // from `workspace.active` rather than from `new`. Without it the switch would send nothing
        // about the session at all.
        app.core.workspace.active = Some(PathBuf::from("/b"));

        switch_daemon_attachment(&mut app, Some(PathBuf::from("/a")), Path::new("/b"));

        assert!(matches!(
            rx.try_recv(),
            Ok(ClientMsg::Detach { project }) if project == Path::new("/a")
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(ClientMsg::Attach { project, force: false }) if project == Path::new("/b")
        ));
        // The start now sits between the attach and the view (BUG-002): the session is resumed, and
        // the daemon must already hold this client's attachment when it is told to.
        assert!(matches!(
            rx.try_recv(),
            Ok(ClientMsg::SessionStart { session }) if session == viewed
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(ClientMsg::SetViewedSession { project, session })
                if project == Path::new("/b") && session == Some(viewed)
        ));
    }

    /// Re-entering the project already attached must not detach it.
    ///
    /// The switch runs on every project entry, including the one that lands where it already was
    /// (a sidebar click on the active project). Detaching and immediately re-attaching would drop
    /// the daemon's grid stream for a moment and hand another window the chance to take the
    /// project in between.
    #[test]
    fn re_entering_the_attached_project_does_not_release_it() {
        let (mut app, mut rx) = connected_app();

        switch_daemon_attachment(&mut app, Some(PathBuf::from("/a")), Path::new("/a"));

        assert!(
            matches!(rx.try_recv(), Ok(ClientMsg::Attach { .. })),
            "the first message must be the attach, not a detach"
        );
    }

    /// Disconnected, the switch is silent — and silence here is the deliberate half.
    ///
    /// A first version of this test asserted that nothing reached the wire, and a probe showed it
    /// could not fail: with `daemon == None` there is no send side to reach, so "sends nothing" is
    /// a fact about the type rather than about this function. What *is* a decision is that it says
    /// nothing to the *user* either — unlike `send_op`, which reports every op it drops. Reaching
    /// the user from here means taking `&mut App` instead of `&App`, which every call site would
    /// accept without complaint, so the signature is a habit rather than a wall. The two disconnect
    /// arms differ on purpose. A
    /// project switch is not something the user asked the daemon for, it happens on every sidebar
    /// click, and `DaemonConnected` redoes the attach when a connection returns; notifying here
    /// would put a notice on screen for each click through a reconnect the user need not know
    /// about.
    #[test]
    fn a_switch_while_disconnected_says_nothing_to_the_user() {
        let mut app = base_app();
        assert!(app.daemon.is_none());

        switch_daemon_attachment(&mut app, Some(PathBuf::from("/a")), Path::new("/b"));
        assert!(
            app.core.notify.visible().is_none(),
            "an ordinary project switch must not report the connection"
        );

        // The contrast, in the same test so the two arms cannot silently converge: an op the user
        // *did* ask for is reported when it is dropped.
        send_op(&mut app, PendingOp::ProjectAdd, |req| {
            ClientMsg::ProjectAdd {
                req,
                path: PathBuf::from("/b"),
            }
        });
        assert!(
            app.core.notify.visible().is_some(),
            "a dropped op is still reported"
        );
    }

    pub(crate) fn snapshot_with(path: &str, sessions: Vec<SessionSummary>) -> CatalogSnapshot {
        CatalogSnapshot {
            schema_version: 1,
            last_active: Some(PathBuf::from(path)),
            projects: vec![ProjectSnapshot {
                path: PathBuf::from(path),
                display_name: "demo".into(),
                is_git_repo: true,
                available: true,
                worktrees: Vec::new(),
                sessions,
            }],
        }
    }

    /// `012` BUG-003 / FR-008. Before this, `mark_shell_running` and `mark_shell_exited` were
    /// reachable only from `Message::ShellInstanceRunning`/`ShellInstanceExited`, which nothing in
    /// the client emits, so every instance sat at `Starting` for its whole life and the bar read
    /// `starting…` beside a shell the user was typing into.
    ///
    /// Asserted here rather than by driving those messages — `tests/app_state.rs` already does
    /// that, and proving the transitions correct is exactly what failed to notice nothing invoked
    /// them.
    #[test]
    fn reconcile_marks_shell_instances_running_and_exited_from_the_snapshot() {
        use micold_core::session::{ShellInstanceId, ShellLifecycle};

        let path = "/repo/demo";
        let mut core = State::default();
        let id = SessionId::new();

        // The client owns the instances: it allocates the ids and creates them locally.
        reconcile_catalog(
            &mut core,
            &snapshot_with(path, vec![summary(id, "S", WireLifecycle::Running)]),
            false,
        );
        let (a, b) = {
            let (_, s) = core.workspace.find_session_mut(id).expect("session");
            (s.open_shell_instance(), s.open_shell_instance())
        };
        let lifecycle_of = |core: &State, inst: ShellInstanceId| {
            core.workspace
                .sessions
                .values()
                .flatten()
                .find(|s| s.id == id)
                .expect("session")
                .shells
                .iter()
                .find(|i| i.id == inst)
                .expect("instance")
                .lifecycle
        };

        // The daemon reports `a` live and says nothing about `b`.
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                path,
                vec![summary_with_live_shells(
                    id,
                    "S",
                    WireLifecycle::Running,
                    vec![a],
                )],
            ),
            false,
        );
        assert_eq!(lifecycle_of(&core, a), ShellLifecycle::Running);
        assert_eq!(
            lifecycle_of(&core, b),
            ShellLifecycle::Starting,
            "an instance the daemon has not (yet) spawned stays where `open_shell_instance` left \
             it — a spawn in flight must not read as an absence"
        );

        // `a` stops being reported: it exited. This is the transition no client-side inference can
        // make, because no frames is indistinguishable from a quiet shell.
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                path,
                vec![summary_with_live_shells(
                    id,
                    "S",
                    WireLifecycle::Running,
                    vec![],
                )],
            ),
            false,
        );
        assert_eq!(lifecycle_of(&core, a), ShellLifecycle::Exited);
        assert_eq!(
            lifecycle_of(&core, b),
            ShellLifecycle::Starting,
            "an instance never seen live stays starting rather than being reported as exited"
        );

        // The client stays the allocator: a snapshot naming an instance the client does not have
        // creates nothing.
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                path,
                vec![summary_with_live_shells(
                    id,
                    "S",
                    WireLifecycle::Running,
                    vec![ShellInstanceId(99)],
                )],
            ),
            false,
        );
        let count = core
            .workspace
            .sessions
            .values()
            .flatten()
            .find(|s| s.id == id)
            .expect("session")
            .shells
            .len();
        assert_eq!(
            count, 2,
            "the snapshot reports liveness; it never adds instances"
        );
    }

    #[test]
    fn reconcile_adds_updates_and_drops_sessions_from_the_snapshot() {
        let path = "/repo/demo";
        let mut core = State::default();

        // First snapshot: one Running session — added to the core.
        let a = SessionId::new();
        reconcile_catalog(
            &mut core,
            &snapshot_with(path, vec![summary(a, "A", WireLifecycle::Running)]),
            false,
        );
        let list = core.workspace.sessions.get(&PathBuf::from(path)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, a);
        assert_eq!(list[0].lifecycle, SessionLifecycle::Running);

        // Second snapshot: A is now Idle, and a new session B appears — A updated, B added.
        let b = SessionId::new();
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                path,
                vec![
                    summary(a, "A", WireLifecycle::Idle),
                    summary(b, "B", WireLifecycle::Running),
                ],
            ),
            false,
        );
        let list = core.workspace.sessions.get(&PathBuf::from(path)).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(
            list.iter().find(|s| s.id == a).unwrap().lifecycle,
            SessionLifecycle::Idle,
            "existing session's lifecycle is reconciled"
        );

        // Third snapshot: only B remains (A archived/removed on the daemon) — A is dropped, and a
        // dangling active pointer to A is cleared.
        core.active_session = Some(a);
        reconcile_catalog(
            &mut core,
            &snapshot_with(path, vec![summary(b, "B", WireLifecycle::Running)]),
            false,
        );
        let list = core.workspace.sessions.get(&PathBuf::from(path)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, b);
        assert_eq!(core.active_session, None, "dangling active pointer cleared");
    }

    // Convergence fix (retrofit session, 2026-07-27): a session transitioning to `Restarting` in
    // a background (inactive) project's snapshot must raise the FR-011/SC-007 return notice.
    // `State::note_background_restart` existed and was unit-tested in isolation
    // (`tests/background_restart.rs`), but nothing called it from the daemon-driven reconcile
    // path after feature 010 moved supervision into the daemon — so no background restart was
    // ever actually detected or notified.
    #[test]
    fn reconcile_detects_a_background_restart_and_arms_the_return_notice() {
        let mut core = State::default();
        core.workspace.active = Some(PathBuf::from("/b")); // /a is the background project

        let a = SessionId::new();
        reconcile_catalog(
            &mut core,
            &snapshot_with("/a", vec![summary(a, "A", WireLifecycle::Running)]),
            false,
        );
        assert!(core.restarted_while_inactive.is_empty());

        // /a's session crashes and the daemon starts restarting it, while /a is still inactive.
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                "/a",
                vec![summary(a, "A", WireLifecycle::Restarting { attempts: 1 })],
            ),
            false,
        );
        assert!(
            core.restarted_while_inactive.contains(&a),
            "a background session's transition into Restarting must be detected and marked"
        );

        // A further Restarting snapshot (still retrying) must not re-mark or duplicate anything.
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                "/a",
                vec![summary(a, "A", WireLifecycle::Restarting { attempts: 2 })],
            ),
            false,
        );
        assert_eq!(core.restarted_while_inactive.len(), 1);

        // Returning to /a fires the return notice (mirrors `background_restart.rs`).
        core.record_foreground();
        // Drained the way the root does: the return notice is an outcome now (T067a-9), so
        // producing it is not the same as raising it.
        let arrival = core
            .switch_active(Path::new("/a"))
            .expect("the switch happened");
        micold_client::app::drain(arrival, |o| micold_client::app::interpret(&mut core, o));
        let visible = core
            .notify
            .visible()
            .expect("the return notice reached the queue");
        assert_eq!(visible.level, micold_core::notify::Level::Info);
        assert_eq!(
            visible.message,
            "A background session was restarted while you were away."
        );
    }

    // ---- feature 027: the tab strip is the only route between the panes --------------------

    /// A connected app displaying a session in Regular mode with one open shell instance.
    fn app_showing_a_shell() -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
        SessionId,
    ) {
        let (mut app, rx) = connected_app();
        let project = PathBuf::from("/repo");
        let mut session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
        let id = session.id;
        session.set_mode(TerminalMode::Regular);
        session.open_shell_instance();
        app.core
            .workspace
            .sessions
            .insert(project.clone(), vec![session]);
        app.core.workspace.active = Some(project);
        app.core.active_session = Some(id);
        (app, rx, id)
    }

    /// Everything the client put on the wire since the last drain.
    fn wire(rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>) -> Vec<ClientMsg> {
        let mut sent = Vec::new();
        while let Ok(m) = rx.try_recv() {
            sent.push(m);
        }
        sent
    }

    /// Pressing the AI tab repoints the attached process at the AI CLI (feature 027 FR-002).
    ///
    /// # The defect this exists for, which is live on `main`
    ///
    /// `Message::TerminalAiCliSelected` had **no arm in `main.rs`** — it fell through to the
    /// catch-all, which runs the pure reducer and nothing else. So the AI tab moved the mark and
    /// the mode while the daemon went on streaming and driving whichever shell instance was
    /// attached: the user pressed the AI tab, the strip said AI, and the keys went to bash.
    ///
    /// It survived feature 026 because the mode toggle was still there doing the attach, and
    /// because the AI process is normally attached already — the divergence only opens after a
    /// trip through Regular mode, which is exactly the trip a tab strip invites. Feature 027
    /// deletes the toggle, so this stops being a redundancy and becomes the only route.
    #[test]
    fn selecting_the_ai_tab_attaches_the_ai_process() {
        let (mut app, mut rx, id) = app_showing_a_shell();
        let _ = wire(&mut rx); // the setup's traffic, if any

        let _ = on_terminal_ai_cli_selected(&mut app, id);

        let sent = wire(&mut rx);
        assert!(
            sent.iter().any(|m| matches!(
                m,
                ClientMsg::SessionAttachProcess {
                    session,
                    process: SessionProcess::Primary
                } if *session == id
            )),
            "pressing the AI tab must tell the daemon to attach the session's primary process, \
             or the pane shows the AI CLI while the keyboard still drives the shell. Sent: {sent:?}"
        );
    }

    /// "+" opens an instance from the AI pane too, and lands the user on it (feature 027 FR-004).
    ///
    /// Feature 011's FR-019 made both the control and its `Ctrl+Shift+T` chord a **no-op outside
    /// Regular mode**, which was coherent while a mode toggle existed: the "+" was hidden there,
    /// so the no-op was unreachable. With the toggle gone the "+" is always shown, and a session
    /// sitting on its AI tab with no instances yet would otherwise have no way to make one.
    #[test]
    fn opening_a_terminal_from_the_ai_pane_switches_to_it() {
        let (mut app, mut rx) = connected_app();
        let project = PathBuf::from("/repo");
        let session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
        let id = session.id;
        assert_eq!(
            session.mode,
            TerminalMode::AiCli,
            "precondition: a new session starts on its AI tab with no instances"
        );
        app.core
            .workspace
            .sessions
            .insert(project.clone(), vec![session]);
        app.core.workspace.active = Some(project);
        app.core.active_session = Some(id);
        let _ = wire(&mut rx);

        let _ = on_shell_instance_open_requested(&mut app);

        let opened = app
            .core
            .workspace
            .find_session(id)
            .expect("the session is still there")
            .1;
        assert_eq!(
            opened.mode,
            TerminalMode::Regular,
            "opening a terminal from the AI pane has to show it, or the control reports success \
             and changes nothing the user can see"
        );
        assert_eq!(opened.shells.len(), 1, "exactly one instance was opened");
        let shell = opened.shells[0].id;

        let sent = wire(&mut rx);
        assert!(
            sent.iter()
                .any(|m| matches!(m, ClientMsg::SessionOpenShell { session, instance }
                    if *session == id && *instance == shell)),
            "the daemon is told to spawn the new instance. Sent: {sent:?}"
        );
        assert!(
            sent.iter().any(|m| matches!(
                m,
                ClientMsg::SessionAttachProcess { session, process: SessionProcess::Shell(s) }
                    if *session == id && *s == shell
            )),
            "and the attachment follows it, so the keyboard drives what is displayed. Sent: {sent:?}"
        );
    }
}
