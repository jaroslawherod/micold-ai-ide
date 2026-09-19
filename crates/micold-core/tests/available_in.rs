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

#[test]
fn a_cli_present_only_in_the_given_path_is_available() {
    let session_bin = tempfile::tempdir().unwrap();
    install(session_bin.path(), AiCli::Pi.provider().command());

    assert_eq!(
        available_in(&path_of(&[session_bin.path()])),
        vec![AiCli::Pi],
        "a CLI on the PATH a session would be spawned with must be offered, whether or not the \
         session service's own PATH has it (FR-003b)"
    );
}
