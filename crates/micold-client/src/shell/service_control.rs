//! Starting, stopping and outliving the session service *as an OS process* (feature 021, T055 —
//! FR-019a).
//!
//! **Not the same external system as `shell/daemon_sync.rs`, and the distinction is the reason
//! this file exists.** That module is the conversation with a daemon that is running and speaking
//! our protocol. This one is what you do when it is not: terminate a mismatched build by pid,
//! and ask the platform's service manager to keep sessions alive across logout. Neither can be
//! expressed as a `ClientMsg` — the first is used precisely when the protocol is unusable
//! (FR-022), and the second is `loginctl`/`systemctl`, which the daemon does not speak at all.
//!
//! It is the same argument T054 made for splitting the OS-theme probe from the clock that drives
//! it: two systems that meet at one message are still two systems.
//!
//! # Both of these run off the update thread
//!
//! Each spawns a process and waits for it. On the update thread that is a frozen window for as
//! long as `systemctl` takes to answer, so both are `Task::perform` over `spawn_blocking`, and
//! both report their outcome as an ordinary message rather than a return value.

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

/// Make sessions survive logout (US7, FR-038; Linux only). Runs off-thread — it spawns
/// `loginctl`/`systemctl` — and reports the outcome as a toast. Never enabled by install.
pub(crate) fn on_logout_survival_requested() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                let endpoint = micold_core::endpoint::resolve().map_err(|e| {
                    micold_core::logout_survival::SurvivalOutcome::Failed(e.to_string())
                })?;
                Ok(micold_core::logout_survival::enable(&endpoint))
            })
            .await
            .unwrap_or_else(|e| {
                Err(micold_core::logout_survival::SurvivalOutcome::Failed(
                    e.to_string(),
                ))
            })
        },
        |r: Result<
            micold_core::logout_survival::SurvivalOutcome,
            micold_core::logout_survival::SurvivalOutcome,
        >| {
            let outcome = r.unwrap_or_else(|e| e);
            Message::LogoutSurvivalOutcome(outcome.user_message())
        },
    )
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

    /// The survival outcome reaches the user whichever way it went.
    #[test]
    fn the_survival_outcome_is_reported() {
        let mut app = base_app();

        let _ = on_logout_survival_outcome(&mut app, "it worked".to_string());

        assert!(app.core.notify.is_active());
    }
}
