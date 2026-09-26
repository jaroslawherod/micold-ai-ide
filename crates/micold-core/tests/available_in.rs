//! `available_in` over a `PATH` value the caller supplies (feature 029, BUG-001, FR-003b).
//!
//! `available_here` answers from this process's own `PATH`. That is the wrong question on a host
//! where the session service was started from the desktop: its `PATH` lacks the version-manager
//! directories that a session gets from the environment-include script, so a `pi` installed with
//! `npm install -g` under mise or nvm runs in a session but was never offered. The daemon now
//! resolves the `PATH` a session would be spawned with and asks `available_in` about *that*.

use micold_core::provider::available_in;
use micold_core::session::AiCli;

use std::ffi::OsString;
use std::path::Path;

/// Put a file named `command` in `dir`. Presence is the whole check (the executable bit is
/// deliberately not read — see `resolves_on_path`), so this is what "installed" means here.
fn install(dir: &Path, command: &str) {
    std::fs::write(dir.join(command), "#!/bin/sh\nexit 0\n").unwrap();
}

fn path_of(dirs: &[&Path]) -> OsString {
    std::env::join_paths(dirs).unwrap()
}

/// The process environment and the process's current directory are both process-wide, while Rust
/// runs the tests in this binary on threads. Every test here takes this lock, including the ones
/// that only *read* the environment: `resolves_on_path` reads `PATHEXT` on every call, so a test
/// that writes `PATH` without the lock races a sibling's read.
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn a_cli_present_only_in_the_given_path_is_available() {
    let _guard = env_lock();
    let session_bin = tempfile::tempdir().unwrap();
    install(session_bin.path(), AiCli::Pi.provider().command());

    assert_eq!(
        available_in(&path_of(&[session_bin.path()])),
        vec![AiCli::Pi],
        "a CLI on the PATH a session would be spawned with must be offered, whether or not the \
         session service's own PATH has it (FR-003b)"
    );
}

/// The other direction, and the one the old code path failed: a CLI that only the session
/// service's own `PATH` holds is not on the `PATH` a session is spawned with, so it is not offered
/// for that session.
///
#[test]
fn a_cli_present_only_on_the_process_path_is_not_available_in_another_path() {
    let _guard = env_lock();
    let service_bin = tempfile::tempdir().unwrap();
    let session_bin = tempfile::tempdir().unwrap();
    install(service_bin.path(), AiCli::Pi.provider().command());

    let previous = std::env::var_os("PATH");
    std::env::set_var("PATH", path_of(&[service_bin.path()]));
    let answer = available_in(&path_of(&[session_bin.path()]));
    match previous {
        Some(value) => std::env::set_var("PATH", value),
        None => std::env::remove_var("PATH"),
    }

    assert!(
        answer.is_empty(),
        "the answer is about the given PATH alone: a CLI that only the session service's own \
         PATH holds would not be found by a session spawned with the given one, got {answer:?}"
    );
}

/// An empty `PATH` value finds nothing, rather than looking in the current directory.
///
/// `split_paths("")` yields one *empty* component, and joining a command onto it gives the bare
/// relative name. Before feature 029 the value was always this process's own `PATH` and an absent
/// one answered `false` outright; now it comes from the environment-include result, where an empty
/// `PATH` is an ordinary answer from a script that clears it. Without the empty-component filter a
/// `pi` file in the session service's working directory would read as installed.
#[test]
fn an_empty_path_does_not_resolve_against_the_current_directory() {
    let _guard = env_lock();
    let cwd = tempfile::tempdir().unwrap();
    install(cwd.path(), AiCli::Pi.provider().command());

    let previous = std::env::current_dir().unwrap();
    std::env::set_current_dir(cwd.path()).unwrap();
    let answer = available_in(std::ffi::OsStr::new(""));
    std::env::set_current_dir(previous).unwrap();

    assert!(
        answer.is_empty(),
        "an empty PATH names no directory to look in, so nothing is installed on it; a file in \
         the current directory is not on PATH, got {answer:?}"
    );
}
