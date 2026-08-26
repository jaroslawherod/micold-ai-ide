//! Folding the daemon's catalog snapshot back into the client's state.
//!
//! The daemon owns sessions, projects and worktrees; this module is where what it publishes becomes
//! what the client renders. [`reconcile_catalog`] is that fold, and `wire_to_lifecycle` /
//! `wire_to_worktree_status` are the two wire-enum translations it needs.
//!
//! # Why this is in the library rather than in `shell/daemon_sync.rs`
//!
//! It used to live in the binary, beside the arms that call it. That placement cost the project
//! three bugs of one shape, because a function in the binary crate cannot be reached from `tests/`:
//!
//! - `010` BUG-011 — `Session::start`/`mark_running` were correct and had no production caller.
//! - `012` BUG-003 — the daemon knew which shell instances were alive and never put it on the wire.
//!   The **first** fix was still incomplete and shipped, because the daemon-side test closed a shell
//!   explicitly (which removes the process) instead of letting one die on its own.
//! - `012` BUG-004 — the bar's restart control decided correctly and the button never asked.
//!
//! Each time both halves had tests and the join had none. The daemon's tests called the transitions
//! themselves; the client's fed hand-built snapshots in. Neither drove *daemon → wire → client*,
//! which is the only place those bugs live.
//!
//! `micold-daemon` already dev-depends on `micold-client` (see its manifest — not a cycle, the
//! client never depends on the daemon), so with the fold out here a test can build a real
//! `DaemonState`, take the snapshot it would actually publish, and assert on the client state that
//! results. `crates/micold-daemon/tests/catalog_join.rs` is that test.
//!
//! Nothing here changed in the move: the bodies are the ones the binary shipped, so a behavioural
//! difference between this and what was reviewed would be a defect and not a decision.

use std::collections::HashSet;
use std::path::Path;

use micold_core::protocol::messages::{CatalogSnapshot, WireLifecycle};
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, ShellLifecycle,
    TerminalMode,
};

use crate::app::State;

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
                // Shell-instance liveness (`012` FR-008, BUG-003). The client allocates the ids and
                // owns the set; the daemon owns which of them have a process. Adopted here with
                // every other value it publishes, rather than from a `ShellInstanceRunning` message
                // — that variant exists and nothing has ever emitted it.
                //
                // Absence is read as death only for an instance already seen `Running`: a spawn is
                // in flight for a tick or two after `SessionOpenShell`, and treating that as an exit
                // would flap the bar and offer `restart` for a shell on its way up.
                for instance in existing.shells.iter_mut() {
                    if summary.live_shells.contains(&instance.id) {
                        instance.lifecycle.mark_running();
                    } else if matches!(instance.lifecycle, ShellLifecycle::Running) {
                        instance.lifecycle.mark_exited();
                    }
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
                    // From the summary, not defaulted (feature 026, T065 — FR-012, FR-016).
                    //
                    // This is the **only** path a daemon-reported session takes into the client
                    // model: every session discovered by the FR-014 pass, and every session at all
                    // after a client restart. A default here would leave the row label, the
                    // terminal bar and the split affordance all reading `claude` for a Copilot
                    // session, with every other test still green.
                    summary.provider,
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
                // Drained the way the root does: the worktree feature reports what survived and
                // the sidebar prunes its own expansion set (T066, and T067a-4 for the form's
                // error line). Dropping the outcomes here would leave the sidebar expanded on
                // worktrees this snapshot has just removed.
                let outcomes = core.set_worktrees(
                    project
                        .worktrees
                        .iter()
                        .map(|w| micold_core::worktree::Worktree {
                            dir_name: w.dir_name.clone(),
                            // The daemon's path, not one rebuilt from the app's own worktree root:
                            // an included worktree is not under that root, and reconstructing the
                            // location would put every one of them somewhere they are not
                            // (016 BUG-002, FR-029).
                            path: w.path.clone(),
                            branch: w.branch.clone(),
                            status: wire_to_worktree_status(w.status),
                            included: w.included,
                        })
                        .collect(),
                );
                crate::app::drain(outcomes, |o| crate::app::interpret(core, o));
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
            // T067a-6: committing the outgoing row is an outcome now, so it has to be applied —
            // a dropped `Vec<Outcome>` here is the row snapping shut, which is the thing FR-001c
            // is about.
            let outcomes = core.set_current_session(None);
            crate::app::drain(outcomes, |o| crate::app::interpret(core, o));
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

/// One line describing what attaching to the daemon actually produced (`010` BUG-013).
///
/// Written to `micold-client.log` at every connect outcome. It exists because BUG-013 could not be
/// diagnosed from outside the process: the client's whole log for a failing run was a single
/// project-switch line emitted *before* any catalog could arrive, and byte-identical in the run
/// that worked and the run that failed. Three layers had to be eliminated by rebuilding old
/// commits and probing on-disk stores, and the question that would have settled it in one glance —
/// *did this client attach, and what did it receive?* — had no answer anywhere.
///
/// So the counts are the point, not the fact of connecting. "Attached" with `sessions=0` against a
/// daemon that is hosting one is a different bug from never attaching, and the two want opposite
/// fixes. Formatting lives here, in the library, so it can be asserted on: a diagnostic nothing
/// tests is a diagnostic that quietly stops being written, which is the failure mode this whole
/// report is about.
pub fn attach_log_line(catalog: &CatalogSnapshot, active: Option<&Path>) -> String {
    let sessions: usize = catalog.projects.iter().map(|p| p.sessions.len()).sum();
    let active_sessions = active
        .and_then(|a| catalog.projects.iter().find(|p| p.path == a))
        .map(|p| p.sessions.len());
    format!(
        "attach: connected projects={} sessions={} active={} active_sessions={}",
        catalog.projects.len(),
        sessions,
        active
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".into()),
        active_sessions
            .map(|n| n.to_string())
            .unwrap_or_else(|| "<project not in catalog>".into()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::protocol::messages::{ProjectSnapshot, SessionSummary};
    use micold_core::session::SessionId;
    use std::path::PathBuf;

    fn snapshot(projects: Vec<(&str, usize)>) -> CatalogSnapshot {
        CatalogSnapshot {
            schema_version: 1,
            last_active: None,
            projects: projects
                .into_iter()
                .map(|(path, n)| ProjectSnapshot {
                    path: PathBuf::from(path),
                    display_name: "p".into(),
                    is_git_repo: true,
                    available: true,
                    worktrees: Vec::new(),
                    sessions: (0..n)
                        .map(|_| SessionSummary {
                            id: SessionId::new(),
                            worktree_dir: None,
                            title: SessionLabel::Pending,
                            lifecycle: WireLifecycle::Idle,
                            activity: micold_core::protocol::messages::ActivitySignal::Unknown,
                            input_serial: 0,
                            live_shells: Vec::new(),
                            provider: micold_core::session::AiCli::ClaudeCode,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// `010` BUG-013. The counts are the whole value: "attached with zero sessions" and "never
    /// attached" are different bugs wanting opposite fixes, and the log has to tell them apart.
    #[test]
    fn the_attach_line_reports_what_arrived_not_merely_that_something_did() {
        let line = attach_log_line(&snapshot(vec![("/a", 2), ("/b", 1)]), Some(Path::new("/a")));
        assert!(line.contains("projects=2"), "{line}");
        assert!(line.contains("sessions=3"), "{line}");
        assert!(line.contains("active=/a"), "{line}");
        assert!(
            line.contains("active_sessions=2"),
            "the active project's own count is the one that answers \"my session vanished\": {line}"
        );
    }

    /// The case BUG-013 presents: a catalog arrives and the project the window is showing has no
    /// sessions in it. That must be visibly different from not attaching at all.
    #[test]
    fn an_attach_that_brought_nothing_is_still_recorded_as_an_attach() {
        let line = attach_log_line(&snapshot(vec![("/a", 0)]), Some(Path::new("/a")));
        assert!(line.starts_with("attach: connected"), "{line}");
        assert!(line.contains("active_sessions=0"), "{line}");
    }

    /// A project the daemon does not list at all is a third state again — and saying `0` for it
    /// would claim the daemon answered about a project it never mentioned.
    #[test]
    fn a_project_absent_from_the_catalog_is_not_reported_as_zero_sessions() {
        let line = attach_log_line(&snapshot(vec![("/a", 1)]), Some(Path::new("/elsewhere")));
        assert!(
            line.contains("active_sessions=<project not in catalog>"),
            "{line}"
        );
    }
}
