//! A desktop launch finds the real `gh` on this machine (feature 034, FR-026, quickstart §B10,
//! research R3).
//!
//! `github_locate.rs` proves the lookup's rules against a fake file set. This file proves the
//! other half on real hosts: CI runs it on Linux, macOS and Windows, each with `gh` installed
//! where that platform's installer put it, and it resolves `gh` the way the client does
//! (`shell/capabilities.rs::locate_gh_here`) — except that `PATH` is the one a desktop launcher
//! hands the app instead of the terminal's, and the environment include contributes nothing (off,
//! or a profile that does not set `PATH`). That is the macOS and Windows arms of §B10 (Dock or
//! Finder, Start menu); the Linux arm is also recorded by hand in `evidence/quickstart-b.md`.
//!
//! No network and no GitHub sign-in: the only process run is `gh --version`.

use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::github::{locate_gh, HostOs, LocateInputs};

/// The `PATH` a desktop launcher hands the app: the system's own directories and nothing the
/// user's shell profile or a tool's installer added.
fn launcher_path(os: HostOs) -> String {
    match os {
        // What `launchd` gives a Dock- or Finder-launched app.
        HostOs::MacOs => "/usr/bin:/bin:/usr/sbin:/sbin".to_string(),
        // systemd's default user-manager `PATH`, what a `.desktop` launch gets when the session
        // imported nothing from `~/.profile`.
        HostOs::Linux => "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
        // A fresh install's machine `PATH`, before any installer — `gh`'s included — appended to
        // it. Stricter than what Explorer passes on, so a pass does not lean on the installer
        // having edited `PATH` and Explorer having picked the edit up.
        HostOs::Windows => {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
            format!(
                r"{root}\system32;{root};{root}\System32\Wbem;{root}\System32\WindowsPowerShell\v1.0\"
            )
        }
    }
}

/// `locate_gh` over this machine's real files and environment, with `process_path` as the
/// process's `PATH` and no environment-include contribution.
fn locate_with_path(process_path: &str) -> Option<PathBuf> {
    let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    locate_gh(&LocateInputs {
        os: HostOs::current(),
        env_include_path: None,
        process_path,
        home: home.as_deref(),
        env: &|name| std::env::var(name).ok(),
        exists: &|path: &Path| path.is_file(),
    })
}

fn on_ci() -> bool {
    std::env::var("CI").is_ok_and(|value| value.eq_ignore_ascii_case("true"))
}

#[test]
fn a_desktop_launch_finds_the_gh_a_terminal_finds() {
    let terminal_path = std::env::var("PATH").unwrap_or_default();
    // `locate_gh` also walks the well-known install directories, so `None` here means `gh` is on
    // neither this `PATH` nor any directory the lookup knows: there is nothing to find.
    let Some(from_terminal) = locate_with_path(&terminal_path) else {
        let reason = "`gh` is on neither PATH nor any well-known install directory of this OS";
        assert!(
            !on_ci(),
            "{reason}; CI runners ship `gh`, so on CI this test must run, not skip"
        );
        eprintln!("skipped: {reason}");
        return;
    };

    let desktop_path = launcher_path(HostOs::current());
    let from_desktop = locate_with_path(&desktop_path);
    eprintln!(
        "terminal launch: {}\ndesktop launch (PATH={desktop_path}): {from_desktop:?}",
        from_terminal.display()
    );
    let from_desktop = from_desktop.unwrap_or_else(|| {
        panic!(
            "a desktop launch did not find `gh`, which a terminal launch finds at {}",
            from_terminal.display()
        )
    });

    let output = Command::new(&from_desktop)
        .arg("--version")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .output()
        .unwrap_or_else(|err| panic!("running {} failed: {err}", from_desktop.display()));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.starts_with("gh version"),
        "{} is not a working GitHub CLI: status {}, stdout {stdout:?}",
        from_desktop.display(),
        output.status
    );
}
