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
//! with [`reconcile_catalog`], their only caller — both translate a daemon wire enum and nothing
//! else in the client has a reason to (FR-001a).
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

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, WireLifecycle};
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, TerminalMode,
};

use crate::{App, State};

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
            PendingOp::ProjectAdd => "add the project".into(),
            PendingOp::ProjectRemove => "remove the project".into(),
            PendingOp::ProjectRename => "rename the project".into(),
            PendingOp::SettingsSet => "update the settings".into(),
        }
    }
}

/// Map a wire lifecycle back to the domain one (inverse of the daemon's `wire_lifecycle`).
/// `InterruptedResumable` — a session the daemon found durably-running after a restart, never
/// auto-relaunched — is carried through as its own state so the sidebar/status can present it
/// distinctly and its select action resumes it (FR-006a).
pub fn wire_to_lifecycle(w: &WireLifecycle) -> SessionLifecycle {
    match w {
        WireLifecycle::Idle => SessionLifecycle::Idle,
        WireLifecycle::InterruptedResumable => SessionLifecycle::InterruptedResumable,
        WireLifecycle::Starting => SessionLifecycle::Starting,
        WireLifecycle::Running => SessionLifecycle::Running,
        WireLifecycle::Restarting { attempts } => SessionLifecycle::Restarting {
            attempts: *attempts,
        },
        WireLifecycle::Failed { .. } => SessionLifecycle::Failed,
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
        Some(session) => crate::view_and_start(app, session),
        None => daemon.send(ClientMsg::SetViewedSession {
            project: new.to_path_buf(),
            session: None,
        }),
    }
}

/// Reconcile the client's core session state from the daemon's authoritative catalog snapshot
/// (FR-011). The daemon owns sessions now, so each project's session list is made to mirror the
/// snapshot: existing sessions have their lifecycle + label updated; sessions the daemon reports
/// but the client lacks are added; sessions the daemon no longer reports (archived/removed) are
/// dropped. A dangling `active_session` pointer is cleared.
pub fn reconcile_catalog(core: &mut State, snapshot: &CatalogSnapshot, sync_worktrees: bool) {
    // Mirror the daemon's project list into the client (T055). Add projects the daemon reports that
    // the client lacks (e.g. opened in another window), and adopt the daemon's display name for known
    // ones. Deliberately NOT a full mirror: projects are not *removed* here — a `CatalogChanged` that
    // predates this client's own in-flight `ProjectAdd` must not drop the project it just opened, and
    // an ephemeral (non-persisting) daemon reporting an empty catalog must not wipe the list. Forget
    // drops the record locally (optimistically) and durably on the daemon.
    for snap in &snapshot.projects {
        if let Some(existing) = core
            .workspace
            .projects
            .iter_mut()
            .find(|p| p.path == snap.path)
        {
            existing.display_name = snap.display_name.clone();
        } else {
            let availability = if snap.available {
                micold_core::project::Availability::Available
            } else {
                micold_core::project::Availability::Unavailable
            };
            let mut project = micold_core::project::Project::new(
                snap.path.clone(),
                snap.is_git_repo,
                availability,
            );
            project.display_name = snap.display_name.clone();
            core.workspace.projects.push(project);
        }
    }
    // Sessions observed transitioning into `Restarting` this reconciliation (feature 008,
    // FR-011/SC-007) — collected here and applied after the loop below, since
    // `note_background_restart` needs `&mut core` while `list` still holds `core.workspace`
    // borrowed. `note_background_restart` itself no-ops for the active project's session, so
    // background-ness isn't checked here.
    let mut newly_restarting: Vec<SessionId> = Vec::new();
    for project in &snapshot.projects {
        let list = core
            .workspace
            .sessions
            .entry(project.path.clone())
            .or_default();
        let snap_ids: HashSet<SessionId> = project.sessions.iter().map(|s| s.id).collect();
        for summary in &project.sessions {
            let lifecycle = wire_to_lifecycle(&summary.lifecycle);
            if let Some(existing) = list.iter_mut().find(|s| s.id == summary.id) {
                if !matches!(existing.lifecycle, SessionLifecycle::Restarting { .. })
                    && matches!(lifecycle, SessionLifecycle::Restarting { .. })
                {
                    newly_restarting.push(existing.id);
                }
                existing.lifecycle = lifecycle;
                existing.activity = summary.activity.clone();
                // Adopt the daemon's title only when it has a real one. The daemon now overlays the
                // live OSC-0 title onto the summary (T047), but a summary can still be `Pending`
                // before the first title arrives; don't let that clobber a title already learned.
                if let SessionLabel::Named(_) = summary.title {
                    existing.label = summary.title.clone();
                }
            } else {
                let location = summary
                    .worktree_dir
                    .clone()
                    .map(SessionLocation::Worktree)
                    .unwrap_or(SessionLocation::Default);
                let mut s = Session::restored(
                    summary.id,
                    location,
                    summary.title.clone(),
                    TerminalMode::AiCli,
                );
                s.lifecycle = lifecycle;
                s.activity = summary.activity.clone();
                list.push(s);
            }
        }
        // Drop sessions the daemon no longer reports (archived/removed on its side).
        list.retain(|s| snap_ids.contains(&s.id));
    }
    for id in newly_restarting {
        core.note_background_restart(id);
    }
    // Mirror the active project's worktrees from the daemon's git discovery into the render state
    // (the sidebar reads `core.worktrees` + `worktree_names`). Only on `CatalogChanged` pushes, not
    // the initial welcome: the welcome's worktree cache is empty until the post-attach refresh, so
    // syncing it would briefly blank the list boot-time local discovery had populated (T055).
    if sync_worktrees {
        if let Some(active) = core.workspace.active.clone() {
            if let Some(project) = snapshot.projects.iter().find(|p| p.path == active) {
                let root = active.join(".claude/worktrees");
                core.set_worktrees(
                    project
                        .worktrees
                        .iter()
                        .map(|w| micold_core::worktree::Worktree {
                            dir_name: w.dir_name.clone(),
                            path: root.join(&w.dir_name),
                            branch: w.branch.clone(),
                            status: wire_to_worktree_status(w.status),
                        })
                        .collect(),
                );
                // Mirror display-name overrides from the catalog (a second window sees a rename).
                let names: std::collections::BTreeMap<String, String> = project
                    .worktrees
                    .iter()
                    .filter(|w| w.display_name != w.dir_name)
                    .map(|w| (w.dir_name.clone(), w.display_name.clone()))
                    .collect();
                if names.is_empty() {
                    core.workspace.worktree_names.remove(&active);
                } else {
                    core.workspace.worktree_names.insert(active, names);
                }
            }
        }
    }
    // Clear a dangling active-session pointer if its session is gone.
    //
    // Feature 024: through `set_current_session`, like every other app-initiated clear, so the row
    // the vanished session was in is committed open rather than snapping shut under the user
    // (FR-001c). Nothing is armed: there is no session to scroll to.
    if let Some(id) = core.active_session {
        if core.workspace.find_session(id).is_none() {
            core.set_current_session(None);
        }
    }
}

/// Project the wire [`WorktreeStatus`] back onto the client's core status enum (T055). The inverse of
/// the daemon's mapping; `Locked`/`Prunable` both collapse to `Invalid` (the client renders both as
/// an unusable/removable worktree).
pub fn wire_to_worktree_status(
    status: micold_core::protocol::messages::WorktreeStatus,
) -> micold_core::worktree::WorktreeStatus {
    use micold_core::protocol::messages::WorktreeStatus as Wire;
    use micold_core::worktree::WorktreeStatus as Core;
    match status {
        Wire::Clean => Core::Valid,
        Wire::Missing => Core::Missing,
        Wire::Locked | Wire::Prunable => Core::Invalid,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
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
        assert!(core.switch_active(Path::new("/a")));
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
}
