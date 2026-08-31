//! Un-enabling the systemd user units a previous release shipped (feature 028, T025 — packaging
//! contract §2.6–2.7, research R7).
//!
//! # Why the application does this and not the package
//!
//! Upgrading removes the two unit *files*: dpkg deletes what the new version no longer ships, and
//! no maintainer script is needed for that. What dpkg cannot remove is the per-user **enablement** —
//! the symlink `systemctl --user enable` wrote under `~/.config/systemd/user/`. A `postinst` runs as
//! root with no login session and cannot reach a per-user manager (the same asymmetry research R5.1
//! wrote down when the opt-in was added). So the leftover is the application's to clean up, at its
//! next start, without asking.
//!
//! # The ordering hazard
//!
//! This must run **before** the client connects or auto-spawns. A stale enablement plus a removed
//! unit file is a live socket unit whose `ExecStart` points at nothing: connecting first would have
//! systemd try to activate a service it can no longer start, and the client would be diagnosing a
//! failure the upgrade caused and this function fixes. That is why [`disable_legacy_units`] is
//! called from `main` rather than from anything in the connection path.
//!
//! # Every failure is swallowed, on purpose
//!
//! There being no user manager at all is the common case on a machine that never enabled the
//! opt-in, and on macOS and Windows it is the only case. "No `systemctl` on PATH" is not a fault to
//! report; neither is a manager that refuses. Nothing the user asked for has failed, because the
//! user did not ask for this — so the outcome is returned for tests and dropped by the caller.
//!
//! # Render-free
//!
//! The decision is a function of a directory and a runner. Both are parameters, which is what lets
//! the four properties §2.6–2.7 states be tested against a temporary directory and a closure rather
//! than against the machine the suite happens to run on.

use std::path::{Path, PathBuf};

/// The units a previous release shipped and the removed opt-in could have enabled.
///
/// Both, not just the socket: the socket is what the opt-in enabled, but a user who ran
/// `systemctl --user enable micold-daemon.service` by hand has the same leftover, and this is the
/// one chance to clear it.
const LEGACY_UNITS: &[&str] = &["micold-daemon.socket", "micold-daemon.service"];

/// What the migration did. Returned rather than logged, so the properties in §2.6–2.7 are testable;
/// [`disable_legacy_units`] drops it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Migration {
    /// Nothing was enabled — either this machine never enabled the opt-in, or a previous start
    /// already cleared it. Nothing ran. This is what §2.7's "MUST NOT be repeated" looks like.
    NothingEnabled,
    /// Something was enabled and the un-enable succeeded.
    Disabled,
    /// Something was enabled and the un-enable failed. Carries the detail for a test to read; the
    /// user is never told, because the user never asked (§2.7).
    Failed(String),
}

/// Every enablement under `systemd_user_dir` that names one of [`LEGACY_UNITS`].
///
/// A recursive scan by file name rather than a lookup of `sockets.target.wants/`: which
/// `<target>.wants/` directory an enablement lands in is decided by the unit's own `[Install]`
/// section, and hard-coding the target here would make this a check on one unit's current wiring.
/// The directory is a handful of symlinks, so walking it costs nothing.
///
/// A missing directory yields nothing. No user manager is not a fault (§2.7).
pub fn enablements(systemd_user_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(systemd_user_dir, &mut found);
    found.sort();
    found
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return; // missing, or not ours to read — either way there is nothing enabled here
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let names_a_legacy_unit = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| LEGACY_UNITS.contains(&n));
        if names_a_legacy_unit {
            out.push(path);
        } else if path.is_dir() {
            collect(&path, out);
        }
    }
}

/// The decision, over its two inputs (§2.6–2.7).
///
/// `disable` is invoked **only** when something is enabled, at most once, with the arguments to
/// pass `systemctl`. Its error is returned, never raised.
pub fn migrate(
    systemd_user_dir: &Path,
    mut disable: impl FnMut(&[&str]) -> Result<(), String>,
) -> Migration {
    if enablements(systemd_user_dir).is_empty() {
        return Migration::NothingEnabled;
    }
    // Both units in one invocation. `--now` also stops a socket that is currently listening, which
    // is the half that matters at start-up: an enablement cleared while the socket still holds the
    // endpoint would leave the client unable to bind its own.
    let mut argv = vec!["--user", "disable", "--now"];
    argv.extend_from_slice(LEGACY_UNITS);
    match disable(&argv) {
        Ok(()) => Migration::Disabled,
        Err(detail) => Migration::Failed(detail),
    }
}

/// Where systemd keeps this user's own enablements.
fn systemd_user_dir() -> Option<PathBuf> {
    let config = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(config.join("systemd/user"))
}

/// Run the migration against this machine. Blocking (it may spawn `systemctl`), silent, and a no-op
/// on every start after the first that finds something.
///
/// Call this before connecting or auto-spawning — see the module note on ordering.
pub fn disable_legacy_units() {
    let Some(dir) = systemd_user_dir() else {
        return;
    };
    let _ = migrate(&dir, |args| {
        let output = std::process::Command::new("systemctl")
            .args(args)
            .output()
            .map_err(|e| format!("could not run systemctl: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `<dir>/<target>.wants/<unit>` enablement the way `systemctl --user enable` does.
    ///
    /// A file rather than a symlink: what makes it an enablement is its name and its place, and a
    /// symlink to a unit file that this feature has just deleted would be dangling anyway — which
    /// is precisely the state an upgraded machine is in when this runs.
    fn enable(dir: &Path, target: &str, unit: &str) {
        let wants = dir.join(format!("{target}.wants"));
        std::fs::create_dir_all(&wants).expect("create the wants directory");
        std::fs::write(wants.join(unit), "").expect("write the enablement");
    }

    /// §2.6: an enablement left by the removed opt-in is un-enabled at the next start.
    #[test]
    fn an_enabled_unit_is_disabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        enable(dir.path(), "sockets.target", "micold-daemon.socket");

        let mut invocations: Vec<Vec<String>> = Vec::new();
        let outcome = migrate(dir.path(), |args| {
            invocations.push(args.iter().map(|a| a.to_string()).collect());
            Ok(())
        });

        assert_eq!(outcome, Migration::Disabled);
        assert_eq!(
            invocations.len(),
            1,
            "exactly one invocation: {invocations:?}"
        );
        assert_eq!(
            invocations[0],
            vec![
                "--user",
                "disable",
                "--now",
                "micold-daemon.socket",
                "micold-daemon.service"
            ],
            "`--user` because the enablement is per-user, and `--now` because an enablement \
             cleared while the socket still listens leaves the endpoint held"
        );
    }

    /// §2.7: with nothing enabled, nothing runs. Not "runs and succeeds" — runs at all. A
    /// `systemctl` spawned on every start of every machine that never opted in is a cost paid
    /// forever for a migration that finished years ago.
    #[test]
    fn nothing_enabled_runs_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");

        let outcome = migrate(dir.path(), |args| {
            panic!("ran `systemctl {}` with nothing enabled", args.join(" "))
        });

        assert_eq!(outcome, Migration::NothingEnabled);
    }

    /// The overwhelmingly common case, and on macOS and Windows the only one: there is no such
    /// directory. §2.7 says that is not a fault.
    #[test]
    fn a_machine_with_no_user_manager_is_not_a_fault() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("no/systemd/here");

        let outcome = migrate(&missing, |_| panic!("ran systemctl with no user manager"));

        assert_eq!(outcome, Migration::NothingEnabled);
    }

    /// §2.7: every failure is ignored. The function returns it rather than raising it, and
    /// `disable_legacy_units` drops what it returns — a manager that refuses is not something the
    /// user asked for and so not something to interrupt them about.
    #[test]
    fn a_failure_is_swallowed_rather_than_raised() {
        let dir = tempfile::tempdir().expect("tempdir");
        enable(dir.path(), "sockets.target", "micold-daemon.socket");

        let outcome = migrate(dir.path(), |_| {
            Err("Interactive authentication required".into())
        });

        assert_eq!(
            outcome,
            Migration::Failed("Interactive authentication required".into()),
            "the detail is preserved for a test to read, and dropped by the caller"
        );
    }

    /// §2.7: not repeated once there is nothing enabled. The runner clears the enablement, the way
    /// `systemctl --user disable` does, and the second start must find nothing to do.
    #[test]
    fn it_does_not_repeat_once_nothing_is_enabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        enable(dir.path(), "sockets.target", "micold-daemon.socket");

        let mut runs = 0usize;
        let mut run_once = |_: &[&str]| {
            runs += 1;
            std::fs::remove_file(dir.path().join("sockets.target.wants/micold-daemon.socket"))
                .expect("systemctl removes the enablement");
            Ok(())
        };

        assert_eq!(migrate(dir.path(), &mut run_once), Migration::Disabled);
        assert_eq!(
            migrate(dir.path(), &mut run_once),
            Migration::NothingEnabled
        );
        assert_eq!(runs, 1, "the second start ran the migration again");
    }

    /// A hand-enabled `.service` is the same leftover, in a different `.wants` directory. Finding
    /// it by name is what keeps this from being a check on one unit's `[Install]` section.
    #[test]
    fn an_enablement_under_any_target_is_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        enable(dir.path(), "default.target", "micold-daemon.service");

        assert_eq!(enablements(dir.path()).len(), 1);
    }

    /// …and somebody else's units are not ours to disable.
    #[test]
    fn an_unrelated_unit_is_left_alone() {
        let dir = tempfile::tempdir().expect("tempdir");
        enable(dir.path(), "default.target", "some-other-daemon.service");

        assert!(enablements(dir.path()).is_empty());
        assert_eq!(
            migrate(dir.path(), |_| panic!("disabled a unit that is not ours")),
            Migration::NothingEnabled
        );
    }
}
