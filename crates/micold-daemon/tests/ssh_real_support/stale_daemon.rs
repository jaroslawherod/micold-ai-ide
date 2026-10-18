//! A real `micold-daemon` of another version (feature 489, T068, research R10): the daemon is built
//! again with `MICOLD_FIXTURE_VERSION_SUFFIX=<suffix>`, which `micold-core/build.rs` honours only
//! under `ssh-real-runtime`, so its package version is this build's plus the suffix. Placed as the
//! remote daemon, it refuses this build's client, and this build's daemon would refuse it.
//!
//! The build goes to its own target directory beside the test's, one per suffix: building into the
//! test's own would replace the very `micold-daemon` the other targets run.

use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::protocol::version::PACKAGE_VERSION;
use micold_core::ssh::argv::build_id;

use super::remote::{Dialer, RemoteDaemon};
use super::sshd::{Sshd, ALIAS};

/// The `-old` suffix makes the daemon older than this build (a pre-release of the same version).
pub const OLDER: &str = "-old";
/// A longer dotted version is newer than its prefix, so this makes the daemon newer.
pub const NEWER: &str = ".999";

/// Build the daemon with `suffix` and return the executable, or why it could not be built.
pub fn build(suffix: &str) -> Result<PathBuf, String> {
    let current = Path::new(env!("CARGO_BIN_EXE_micold-daemon"));
    let profile_dir = current.parent().ok_or("the test binary has no directory")?;
    let target_root = profile_dir.parent().ok_or("the profile has no target")?;
    let profile = profile_dir.file_name().ok_or("no profile name")?;
    let slug: String = suffix
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let target = target_root.join(format!("stale-daemon{slug}"));
    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["build", "-p", "micold-daemon", "--bin", "micold-daemon"])
        .args(["--features", "ssh-real-runtime", "--target-dir"])
        .arg(&target)
        .env("MICOLD_FIXTURE_VERSION_SUFFIX", suffix);
    if profile == "release" {
        cmd.arg("--release");
    }
    let out = cmd
        .output()
        .map_err(|e| format!("cargo could not run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "building the stale daemon failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let exe = target.join(profile).join("micold-daemon");
    exe.is_file()
        .then_some(exe)
        .ok_or_else(|| "the stale daemon was not produced".to_string())
}

/// This build's package version with `suffix`: what the stale daemon reports as its own.
pub fn version_with(suffix: &str) -> String {
    format!("{PACKAGE_VERSION}{suffix}")
}

/// A dialer that copies the stale daemon (under its own build directory) and, as a client of this
/// build, names itself the way the real client does.
pub fn dialer(host: &Sshd, stale: PathBuf, suffix: &str) -> Dialer {
    let mut dialer = Dialer::with_daemon(host.ssh_wrapper.clone(), ALIAS, 30, stale);
    dialer.opts.build_id = build_id(
        &version_with(suffix),
        micold_core::protocol::version::BUILD_FINGERPRINT,
    );
    dialer.opts.client_build = format!("micold-ai-ide/{PACKAGE_VERSION}");
    dialer
}

/// Kill on drop the daemon now listening on the pinned endpoint, if any.
pub fn cleanup_guard() -> RemoteDaemon {
    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    RemoteDaemon(micold_core::spawn::running_daemon_pid(&endpoint))
}
