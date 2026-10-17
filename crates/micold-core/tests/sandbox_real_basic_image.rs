//! FR-002 against the built image: the basic image carries an SSH server and what a session needs,
//! and neither the session daemon nor any AI CLI (feature 806).
//!
//! The client copies its own daemon build in over SSH and installs the AI CLIs the user selected,
//! so a daemon or CLI baked into the image would be one the client did not choose, of a version it
//! did not build. `node`/`npm` and `flock` are what those installs need; glibc 2.41 is the floor the
//! glibc daemon builds link against (trixie).
//!
//! The image is `MICOLD_BASIC_IMAGE_TAG`, default `micold-basic:dev` (`docker build -f
//! packaging/basic/Containerfile -t micold-basic:dev packaging/basic`). Behind
//! `sandbox-real-runtime` for the reason every real-runtime target here is: the default suite must
//! need nothing installed on any platform (Principle VI).

#![cfg(feature = "sandbox-real-runtime")]

use std::process::Command;

use micold_core::sandbox::basic_image::DEV_BASIC_IMAGE;

/// What must not be in the image: the daemon and every AI CLI the application offers.
/// `gemini` is not offered today; it is listed because the spec names it.
const ABSENT: [&str; 7] = [
    "micold-daemon",
    "claude",
    "copilot",
    "codex",
    "gemini",
    "opencode",
    "pi",
];

/// What must be: the SSH server, the runtime the npm-distributed CLIs install with, the tool the
/// daemon runs for worktrees, and the lock the install step serialises on.
const PRESENT: [&str; 5] = ["sshd", "node", "npm", "git", "flock"];

/// The oldest glibc the image may carry: trixie's, which the daemon's glibc builds need.
const MIN_GLIBC: (u32, u32) = (2, 41);

fn image() -> String {
    std::env::var("MICOLD_BASIC_IMAGE_TAG").unwrap_or_else(|_| DEV_BASIC_IMAGE.to_string())
}

/// Run `script` with `sh -c` inside a throwaway container of the image, past its entrypoint.
fn in_image(script: &str) -> std::process::Output {
    let out = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--entrypoint",
            "/bin/sh",
            &image(),
            "-c",
            script,
        ])
        .output()
        .expect("docker runs");
    assert!(
        out.status.success() || !out.stdout.is_empty(),
        "docker run {} failed: {}",
        image(),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// The commands of `names` that `command -v` finds in the image, one per line.
fn found(names: &[&str]) -> Vec<String> {
    let script = names
        .iter()
        .map(|n| format!("command -v {n} >/dev/null 2>&1 && echo {n}"))
        .collect::<Vec<_>>()
        .join("; ");
    let out = in_image(&format!("{script}; true"));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// `(major, minor)` from the first line of `ldd --version`, which ends in the version.
fn glibc_version(ldd: &str) -> Option<(u32, u32)> {
    let first = ldd.lines().next()?;
    let version = first.split_whitespace().last()?;
    let (major, minor) = version.split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

#[test]
fn sandbox_real_basic_image_has_no_daemon_or_cli() {
    let present = found(&ABSENT);
    assert!(
        present.is_empty(),
        "{} carries what the client installs itself: {present:?}",
        image()
    );

    let present = found(&PRESENT);
    let missing: Vec<_> = PRESENT
        .iter()
        .filter(|n| !present.iter().any(|p| p == *n))
        .collect();
    assert!(missing.is_empty(), "{} lacks {missing:?}", image());

    let ldd = in_image("ldd --version 2>&1");
    let ldd = String::from_utf8_lossy(&ldd.stdout);
    let version = glibc_version(&ldd).unwrap_or_else(|| panic!("no glibc version in: {ldd}"));
    assert!(version >= MIN_GLIBC, "glibc {version:?} < {MIN_GLIBC:?}");
}
