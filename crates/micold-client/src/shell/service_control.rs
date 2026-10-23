//! Starting, stopping and outliving the session service *as an OS process* (feature 021, T055 —
//! FR-019a).
//!
//! **Not the same external system as `shell/daemon_sync.rs`, and the distinction is the reason
//! this file exists.** That module is the conversation with a daemon that is running and speaking
//! our protocol. This one is what you do when it is not: terminate a mismatched build by pid, and
//! report what the configured placement does about surviving a logout. Neither can be expressed as
//! a `ClientMsg` — the first is used precisely when the protocol is unusable (FR-022), and the
//! second is a fact about where the service runs rather than anything the service could answer.
//!
//! It is the same argument T054 made for splitting the OS-theme probe from the clock that drives
//! it: two systems that meet at one message are still two systems.
//!
//! # One of these runs off the update thread, and it is no longer the survival one
//!
//! Stopping a mismatched build spawns a process and waits for it, so it is a `Task::perform` over
//! `spawn_blocking` that reports its outcome as an ordinary message. The survival opt-in used to
//! need the same treatment because the host-process arm spawned `loginctl`/`systemctl`; feature 028
//! removed that arm (FR-005, packaging contract §4.11), so every placement now answers from memory
//! and the task is only there to keep one message shape.

use iced::Task;

use micold_client::app::Message;
use micold_core::daemons::{DaemonId, DaemonRuntime};
use micold_core::sandbox::placement::Placement;

use crate::App;
use micold_client::features::connection::Msg as ConnectionMsg;

/// What "Restart service" acts on: the mismatched daemon of the worktree in view, and only that one
/// (spec 491 FR-007; spec 806 FR-007, BUG-1165).
#[derive(Debug, Clone)]
enum RestartTarget {
    /// The container the sandbox boot owns: stopped and removed, and the boot brings it up again.
    Boot(crate::shell::sandbox::BootPlan),
    /// Another container daemon: its container is stopped and kept, then the daemon started again.
    Container(DaemonId),
    /// The host daemon, stopped by its recorded pid over its endpoint.
    Host,
    /// A daemon on a host or cluster the app does not own: nothing is stopped from here.
    Elsewhere(DaemonId),
}

fn restart_target(app: &App) -> RestartTarget {
    let boot = app.sandbox_boot.clone();
    let Some(id) = app.active_daemon() else {
        return boot.map_or(RestartTarget::Host, RestartTarget::Boot);
    };
    if let Some(plan) = boot.filter(|_| id == app.sandbox_daemon()) {
        return RestartTarget::Boot(plan);
    }
    match app.core.settings.daemons.get(id).map(|e| &e.runtime) {
        Some(DaemonRuntime::Container(_)) => RestartTarget::Container(id),
        Some(DaemonRuntime::Host) | None => RestartTarget::Host,
        Some(_) => RestartTarget::Elsewhere(id),
    }
}

/// "Restart service" (FR-022/022a): stop the mismatched daemon of the worktree in view.
///
/// A mismatched client can't send it a control message, so termination is the version-agnostic
/// stop — which is exactly why this cannot live in `daemon_sync`. Once it exits, a matching daemon
/// takes its place; previously-live sessions then reload as interrupted-resumable (FR-006a). Live
/// processes are lost — we say so — but the durable sessions survive. Only that daemon is acted on:
/// never the host daemon for a container's mismatch, nor another container (spec 491 FR-007).
pub(crate) fn on_restart_service_requested(app: &mut App) -> Task<Message> {
    // Only the daemon the window is working with is restarted, and only its mismatch is cleared.
    if let Some(id) = app.active_daemon() {
        app.version_mismatch.remove(&id);
        app.build_mismatch.remove(&id);
    }
    match restart_target(app) {
        // When the service lives in the boot's container, stopping it over its endpoint would leave
        // the container up with a dead process inside — the orphan US6 scenario 4 is about. Stop
        // the container itself; the banner that results carries the action that brings a fresh one
        // up (FR-036, T110).
        RestartTarget::Boot(plan) => {
            app.core.notify_info(
                "Stopping the sandbox — running processes are stopped, but your sessions are \
                 preserved and can be resumed.",
            );
            crate::shell::sandbox::stop(&plan)
        }
        // The old daemon keeps listening in its container, so a re-dial meets it again: stop the
        // container, then start the daemon, whose bring-up starts the container with this app's
        // daemon.
        RestartTarget::Container(id) => {
            let name = app
                .core
                .settings
                .daemons
                .get(id)
                .map(|e| e.name.as_str().to_string())
                .unwrap_or_default();
            app.core.notify_info(format!(
                "Restarting the daemon {name} — running processes are stopped, but your sessions \
                 are preserved and can be resumed."
            ));
            crate::shell::daemons::stop_container_of(app, id, move |_| {
                Message::Daemons(micold_client::features::daemons::Msg::Started(id))
            })
            .unwrap_or_else(Task::none)
        }
        RestartTarget::Elsewhere(id) => {
            let name = app
                .core
                .settings
                .daemons
                .get(id)
                .map(|e| e.name.as_str().to_string())
                .unwrap_or_default();
            app.core.notify_info(format!(
                "The daemon {name} does not run on this machine; its row in Settings → Daemons \
                 says how to bring it to this app's version."
            ));
            Task::none()
        }
        RestartTarget::Host => {
            app.core.notify_info(
                "Restarting the session service — running processes are stopped, but your \
                 sessions are preserved and can be resumed.",
            );
            Task::perform(
                async {
                    tokio::task::spawn_blocking(|| {
                        let endpoint = micold_core::endpoint::resolve()?;
                        micold_core::spawn::stop_running_daemon(&endpoint)
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e: std::io::Error| e.to_string())
                },
                |r: Result<bool, String>| match r {
                    Ok(_) => Message::NoOp,
                    Err(e) => Message::Connection(ConnectionMsg::ConnectFailed(format!(
                        "could not stop the mismatched service: {e}"
                    ))),
                },
            )
        }
    }
}

/// Apply the logout-survival opt-in for the configured placement (feature 027, FR-014a/b/d).
///
/// One entry point for both directions and all three placements, because there is one control: the
/// Settings checkbox. It used to be a menu command that only ever *enabled*, and only ever through
/// the Linux service manager — so a sandboxed user's menu item silently configured the wrong
/// mechanism, and nobody could turn it off at all.
///
/// Answers immediately on every placement since feature 028 took the host-process mechanism away
/// (FR-005): the sandbox's policy is a property of a container that already exists, and the other
/// two placements have nothing to arrange. It still returns a `Task`, because the outcome still
/// reaches the user as an ordinary message and the caller should not have to care which.
///
/// Never enabled by install (FR-038) — this runs only from a save that changed the value.
pub(crate) fn on_survival_opt_in_changed(placement: Placement, enabling: bool) -> Task<Message> {
    let outcome = if enabling {
        micold_core::logout_survival::enable_for(&placement)
    } else {
        micold_core::logout_survival::disable_for(&placement)
    };
    Task::done(Message::Connection(ConnectionMsg::LogoutSurvivalOutcome(
        outcome.user_message(),
    )))
}

/// Both halves of the survival attempt come back here.
///
/// `notify_info`, not `notify_error`, whichever way it went: `SurvivalOutcome::user_message`
/// already phrases a failure as a failure, and raising it as an error would style a "this
/// platform does not support it" as something broken.
pub(crate) fn on_logout_survival_outcome(app: &mut App, message: String) -> Task<Message> {
    app.core.notify_info(message);
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::base_app;

    /// A sandboxed service is stopped by stopping its container, not by talking to it (T110).
    ///
    /// The endpoint path stops the *process*. Inside a container that leaves the container running
    /// with nothing in it — an orphan the next start then finds, has to recognise, and has to
    /// replace. US6 scenario 4 asks for exactly the opposite: "the sandbox is stopped and not left
    /// orphaned."
    #[test]
    fn stopping_a_sandboxed_service_stops_the_container_rather_than_the_process() {
        let mut app = base_app();
        app.sandbox_boot = Some(crate::shell::sandbox::BootPlan {
            profile: micold_core::sandbox::SandboxProfile::default(),
            state_dir: std::path::PathBuf::from("/tmp/micold-test"),
            projects: Vec::new(),
            container_name: micold_core::sandbox::CONTAINER_NAME.to_string(),
            port: micold_core::endpoint::DEFAULT_SANDBOX_PORT,
        });

        let _ = on_restart_service_requested(&mut app);

        let said = app
            .core
            .notifications
            .queue
            .visible()
            .is_some_and(|n| n.message.contains("sandbox"));
        assert!(
            said,
            "the user was told the *process* was being restarted, but what is actually being \
             stopped is the container it lives in"
        );
    }

    /// Asking for a restart clears the banner that offered it.
    ///
    /// Both mismatch kinds, because the banner is rendered from either and a restart that cleared
    /// only one would leave it on screen offering an action already taken — and the connection
    /// subscription re-sets whichever is still true on its next retry anyway, so clearing both is
    /// not lossy.
    #[test]
    fn asking_for_a_restart_takes_down_the_banner_that_offered_it() {
        let mut app = base_app();
        app.version_mismatch.insert(
            micold_core::daemons::DaemonId(1),
            (1, 2, "some-build".to_string()),
        );
        app.build_mismatch
            .insert(micold_core::daemons::DaemonId(1), ("a".into(), "b".into()));

        let _ = on_restart_service_requested(&mut app);

        assert!(app.version_mismatch.is_empty());
        assert!(app.build_mismatch.is_empty());
    }

    /// …and says so, because the restart is not free: it stops running processes, and a user who
    /// was not told would read the terminals going quiet as a crash.
    #[test]
    fn the_restart_warns_that_running_processes_stop() {
        let mut app = base_app();

        let _ = on_restart_service_requested(&mut app);

        assert!(
            app.core.notifications.queue.is_active(),
            "restarting the service must tell the user what it costs"
        );
    }

    /// Adds container daemons named `names` and returns their ids.
    fn with_containers(app: &mut App, names: &[&str]) -> Vec<DaemonId> {
        names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let runtime = DaemonRuntime::Container(micold_core::daemons::ContainerSettings {
                    profile: micold_core::sandbox::SandboxProfile::default(),
                    container_name: format!("{name}-ctr"),
                    port: 7001 + i as u16,
                    migration: None,
                });
                let id = app.core.settings.daemons.add(name, runtime, false).unwrap();
                app.core.settings.links.insert(
                    id,
                    micold_core::daemons::DaemonState::VersionMismatch {
                        client: "0.5.3".into(),
                        daemon: "0.5.2".into(),
                    },
                );
                id
            })
            .collect()
    }

    /// The worktree in view is bound to `id`.
    fn view_worktree_on(app: &mut App, id: DaemonId) {
        let project = std::path::PathBuf::from("/p/a");
        app.core.workspace.active = Some(project.clone());
        app.core
            .workspace
            .bindings
            .insert(project, [(String::new(), id)].into_iter().collect());
    }

    fn told(app: &App) -> String {
        app.core
            .notifications
            .queue
            .visible()
            .map(|n| n.message.clone())
            .unwrap_or_default()
    }

    /// BUG-1165: with no boot sandbox, the restart for a mismatched container daemon stops that
    /// container, never the host daemon's endpoint (spec 491 FR-007).
    #[test]
    fn a_mismatched_container_daemon_is_restarted_not_the_host_daemon() {
        let mut app = base_app();
        let [boxed] = with_containers(&mut app, &["Box"])[..] else {
            unreachable!()
        };
        view_worktree_on(&mut app, boxed);

        assert!(matches!(restart_target(&app), RestartTarget::Container(id) if id == boxed));
        let _ = on_restart_service_requested(&mut app);

        let said = told(&app);
        assert!(said.contains("daemon Box"), "{said}");
        assert!(!said.contains("session service"), "{said}");
    }

    /// BUG-1165: with a boot sandbox and a second container daemon in view, the second is restarted
    /// and the boot's container is left alone.
    #[test]
    fn a_second_container_in_view_is_restarted_not_the_boot_one() {
        let mut app = base_app();
        let [first, second] = with_containers(&mut app, &["Box", "Other"])[..] else {
            unreachable!()
        };
        app.sandbox_boot = Some(crate::shell::sandbox::BootPlan {
            profile: micold_core::sandbox::SandboxProfile::default(),
            state_dir: std::path::PathBuf::from("/tmp/micold-test"),
            projects: Vec::new(),
            container_name: "Box-ctr".to_string(),
            port: 7001,
        });
        assert_eq!(
            app.sandbox_daemon(),
            first,
            "the boot owns the first container"
        );
        view_worktree_on(&mut app, second);

        assert!(matches!(restart_target(&app), RestartTarget::Container(id) if id == second));
        let _ = on_restart_service_requested(&mut app);

        let said = told(&app);
        assert!(said.contains("daemon Other"), "{said}");
        assert!(!said.contains("sandbox"), "{said}");

        view_worktree_on(&mut app, first);
        assert!(matches!(restart_target(&app), RestartTarget::Boot(_)));
    }

    /// A daemon on a host the app does not own is not stopped from here: its row says how.
    #[test]
    fn a_remote_daemon_in_view_is_pointed_to_its_row() {
        let mut app = base_app();
        let target = micold_core::ssh::SshTarget::new("devbox", None, None, None).unwrap();
        let remote = app
            .core
            .settings
            .daemons
            .add(
                "Dev",
                DaemonRuntime::Ssh(micold_core::ssh::SshSettings { target }),
                false,
            )
            .unwrap();
        view_worktree_on(&mut app, remote);

        assert!(matches!(restart_target(&app), RestartTarget::Elsewhere(id) if id == remote));
        let _ = on_restart_service_requested(&mut app);
        assert!(told(&app).contains("Settings → Daemons"), "{}", told(&app));
    }

    /// The survival outcome reaches the user whichever way it went.
    #[test]
    fn the_survival_outcome_is_reported() {
        let mut app = base_app();

        let _ = on_logout_survival_outcome(&mut app, "it worked".to_string());

        assert!(app.core.notifications.queue.is_active());
    }
}
