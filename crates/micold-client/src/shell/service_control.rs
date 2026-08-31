//! Stopping the session service *as an OS process* (feature 021, T055 — FR-019a).
//!
//! **Not the same external system as `shell/daemon_sync.rs`, and the distinction is the reason
//! this file exists.** That module is the conversation with a daemon that is running and speaking
//! our protocol. This one is what you do when it is not: terminate a mismatched build by pid.
//! That cannot be expressed as a `ClientMsg`, because it is used precisely when the protocol is
//! unusable (FR-022).
//!
//! It is the same argument T054 made for splitting the OS-theme probe from the clock that drives
//! it: two systems that meet at one message are still two systems.
//!
//! # This runs off the update thread
//!
//! It spawns a process and waits for it. On the update thread that is a frozen window for as long
//! as the stop takes, so it is a `Task::perform` over `spawn_blocking` that reports its outcome as
//! an ordinary message rather than a return value.
//!
//! # What feature 028 removed
//!
//! There used to be a second job here: asking `loginctl`/`systemctl` to keep sessions alive across
//! a logout. Feature 028 removed the mechanism itself (FR-005, packaging contract §4.11) — a
//! service running directly on the computer no longer claims to survive logout on any platform —
//! so the request, its outcome toast and the menu item that raised it went with it. The sandboxed
//! placement still offers survival, and gets it from the container runtime rather than from
//! anything this module could run.

use iced::Task;

use micold_client::app::Message;

use crate::App;

/// "Restart service" (FR-022/022a): stop the mismatched daemon by its recorded pid.
///
/// A mismatched client can't send it a control message, so termination is the version-agnostic
/// stop — which is exactly why this cannot live in `daemon_sync`. Once it exits, the
/// auto-reconnect loop finds nothing listening and spawns a matching daemon; previously-live
/// sessions then reload as interrupted-resumable (FR-006a). Live processes are lost — we say so —
/// but the durable sessions survive.
pub(crate) fn on_restart_service_requested(app: &mut App) -> Task<Message> {
    app.version_mismatch = None;
    app.build_mismatch = None;
    // When the service lives in a container, stopping it over its endpoint would leave the
    // container up with a dead process inside — the orphan US6 scenario 4 is about. Stop the
    // container itself; the banner that results carries the action that brings a fresh one up
    // (FR-036, T110).
    if let Some(plan) = app.sandbox_boot.clone() {
        app.core.notify_info(
            "Stopping the sandbox — running processes are stopped, but your sessions are \
             preserved and can be resumed.",
        );
        return crate::shell::sandbox::stop(&plan);
    }
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
            Err(e) => {
                Message::DaemonConnectFailed(format!("could not stop the mismatched service: {e}"))
            }
        },
    )
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
        });

        let _ = on_restart_service_requested(&mut app);

        let said = app
            .core
            .notify
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
        app.version_mismatch = Some((1, 2, "some-build".to_string()));
        app.build_mismatch = Some(("a".into(), "b".into()));

        let _ = on_restart_service_requested(&mut app);

        assert!(app.version_mismatch.is_none());
        assert!(app.build_mismatch.is_none());
    }

    /// …and says so, because the restart is not free: it stops running processes, and a user who
    /// was not told would read the terminals going quiet as a crash.
    #[test]
    fn the_restart_warns_that_running_processes_stop() {
        let mut app = base_app();

        let _ = on_restart_service_requested(&mut app);

        assert!(
            app.core.notify.is_active(),
            "restarting the service must tell the user what it costs"
        );
    }
}
