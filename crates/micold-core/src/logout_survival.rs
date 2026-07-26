//! Linux logout survival via systemd user linger (US7, FR-038).
//!
//! Surviving a full logout — not just closing the window — is **Linux-only** and MUST NEVER be
//! enabled silently (FR-038). This is the in-session enable path the GUI offers on request. It needs
//! the user's own login session: a running `systemd --user` and polkit's self-linger policy — exactly
//! what a root `postinst` lacks (research R5.1), which is why it lives here and the client triggers it
//! rather than the installer.
//!
//! The order is load-bearing (research R3.5). Enabling linger starts the user manager immediately but
//! does **not** migrate already-running processes: a daemon the client self-spawned into the login
//! session's scope stays there and still dies at logout. So the sequence is: enable linger → stop the
//! session-scoped daemon → enable+start the socket unit, which re-activates a fresh daemon inside the
//! now-lingering user manager. Failure is *detected*, never assumed — hardened deployments can refuse
//! self-linger via polkit (research R3.5).

use crate::endpoint::Endpoint;

/// The result of an [`enable`] attempt, with a user-facing explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurvivalOutcome {
    /// Linger is on and the daemon now runs inside the lingering user manager; sessions will outlive
    /// logout.
    Enabled,
    /// This platform does not support surviving logout (macOS/Windows) — FR-038 scopes it to Linux.
    Unsupported,
    /// A required step failed (linger refused by policy, no `systemd --user`, …). Carries the detail.
    Failed(String),
}

impl SurvivalOutcome {
    /// A single user-facing sentence describing the outcome.
    pub fn user_message(&self) -> String {
        match self {
            SurvivalOutcome::Enabled => "Sessions will now survive logout on this machine — the \
                 session service runs under your lingering user manager."
                .to_string(),
            SurvivalOutcome::Unsupported => "Surviving logout isn't supported on this platform — \
                 it's a Linux-only feature. Sessions still survive closing the window."
                .to_string(),
            SurvivalOutcome::Failed(detail) => {
                format!("Couldn't enable logout survival: {detail}")
            }
        }
    }
}

/// Enable logout survival for the current user (Linux). Idempotent — safe to run when already
/// enabled. **Blocking** (spawns `loginctl`/`systemctl`), so the caller runs it off any async
/// runtime / the UI thread. On non-Linux it is a pure [`SurvivalOutcome::Unsupported`].
#[cfg(target_os = "linux")]
pub fn enable(endpoint: &Endpoint) -> SurvivalOutcome {
    // 1. Enable linger for *ourselves* (no privilege needed under the default self-linger policy;
    //    detect failure rather than assume, per research R3.5).
    if let Err(detail) = run("loginctl", &["enable-linger"]) {
        return SurvivalOutcome::Failed(format!("enabling linger failed ({detail})"));
    }

    // 2. Stop any daemon the client self-spawned into the login-session scope: enabling linger does
    //    not migrate it, so it would still die at logout. Stopping it frees the socket for the unit
    //    and lets the survivor be a fresh, manager-hosted daemon. Best-effort — no daemon is fine.
    let _ = crate::spawn::stop_running_daemon(endpoint);

    // 3. Enable + start the socket unit inside the user manager. Socket activation then spawns the
    //    service (the daemon) on the next client connection, now under the lingering manager.
    if let Err(detail) = run(
        "systemctl",
        &["--user", "enable", "--now", "micold-daemon.socket"],
    ) {
        return SurvivalOutcome::Failed(format!(
            "enabling the systemd user socket failed ({detail}) — is a user systemd manager running?"
        ));
    }

    SurvivalOutcome::Enabled
}

/// Non-Linux: logout survival is unsupported (FR-038); this is a no-op that says so.
#[cfg(not(target_os = "linux"))]
pub fn enable(_endpoint: &Endpoint) -> SurvivalOutcome {
    SurvivalOutcome::Unsupported
}

/// Run `program args...`, mapping a non-zero exit or a spawn error to `Err(detail)` with the
/// program's own stderr preserved where there is one.
#[cfg(target_os = "linux")]
fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        Err(if detail.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            detail.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_outcome_has_a_clear_user_message() {
        assert!(SurvivalOutcome::Enabled
            .user_message()
            .contains("survive logout"));
        // The unsupported message must name the limitation plainly (FR-038, acceptance scenario 3).
        let unsupported = SurvivalOutcome::Unsupported.user_message();
        assert!(unsupported.contains("Linux-only"));
        assert!(
            unsupported.to_lowercase().contains("not supported")
                || unsupported.to_lowercase().contains("isn't supported")
        );
        // A failure surfaces its detail verbatim so the user can act on it.
        assert!(SurvivalOutcome::Failed("polkit denied".into())
            .user_message()
            .contains("polkit denied"));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn non_linux_is_always_unsupported() {
        let endpoint = crate::endpoint::Endpoint {
            socket_path: std::path::PathBuf::from("/tmp/x.sock"),
            lock_path: std::path::PathBuf::from("/tmp/x.lock"),
        };
        assert_eq!(enable(&endpoint), SurvivalOutcome::Unsupported);
    }
}
