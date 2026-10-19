//! The `micold-daemon` binary entry point (feature 010).
//!
//! Started by the client (feature 028 US1); can also be run directly (`mise run daemon`) when you
//! want it in the foreground. All logic lives in the library so it is testable headlessly.
//!
//! # The process ends by returning, never by exiting
//!
//! [`micold_daemon::run`] returns `Ok(())` when the idle window expires (data-model G5 step 6), and
//! this function then returns normally — so the `#[tokio::main]` runtime shuts down, every `Drop`
//! runs, and the process leaves nothing behind (research R4, lifecycle contract §3.12–3.13). A
//! `std::process::exit` on that path would skip all of it: the bound listener's socket file would
//! stay on disk and each live session's process tree would be orphaned to the machine's init,
//! turning a clean idle stop into exactly the residue the feature exists to avoid.
//!
//! The `exit` on the fatal-error path comes after `start` has already returned and its runtime has
//! already been dropped. It sets a non-zero status for a daemon that could not start; it does not
//! cut a shutdown short. The `status-line` relay exits before any runtime exists (feature 490).

// A release build on Windows opens no console window of its own, for the app or for the daemon it
// starts (feature 030, FR-005). Debug builds keep the console so `cargo run` output stays visible.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    // `status-line <relay-file>`: Claude Code's status-line command for a session the app started
    // (feature 490, contract S2). Before the runtime, the log and the singleton: the relay is a
    // short-lived child of Claude Code, not a daemon, and must stay silent.
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("status-line")) {
        let file = args.next().unwrap_or_default();
        std::process::exit(micold_daemon::status_relay::main(std::path::Path::new(
            &file,
        )));
    }
    // `remote bridge | prune`: the helper the client runs over `ssh` (feature 489). Same reasons as
    // `status-line`: no runtime, log or singleton, and stdout carries only relayed bytes.
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("remote")) {
        let rest: Vec<std::ffi::OsString> = args.collect();
        std::process::exit(micold_daemon::remote::main(&rest));
    }
    // `git-credential <get|store|erase>`: git's credential helper inside a sandbox (feature 1002).
    // Same reasons as `status-line`: no runtime, log or singleton, and stdout is git's answer.
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("git-credential")) {
        let rest: Vec<std::ffi::OsString> = args.collect();
        std::process::exit(micold_daemon::git_credential::main(&rest));
    }
    // No `announce_running` here: the installer's `AppMutex` prompt asks the user to close the app,
    // and a windowless daemon cannot be closed, so setup would never get past it. Setup stops the
    // daemon itself instead, in `[Code] PrepareToInstall` (feature 030, research R12).
    if let Err(e) = start() {
        let line = format!("micold-daemon: fatal: {e}");
        // A detached daemon (and on Windows, a GUI-subsystem one) has no stderr anyone reads, so
        // the reason it could not start also goes to its log file (feature 030, E6.4).
        append_to_log(&line);
        eprintln!("{line}");
        std::process::exit(1);
    }
}

/// The runtime is built here rather than by `#[tokio::main]` so its end can be bounded: dropping
/// a runtime waits for every blocking task, and an orderly stop abandons a history save that a
/// stuck disk never finishes (feature 041, stop-request contract §4). Everything the stop needs has
/// finished by the time `run` returns, so what is left is given 5 s and then left behind.
fn start() -> std::io::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(micold_daemon::run());
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    result
}

/// Best-effort: a daemon failing to start has nowhere better to report a failure to log.
fn append_to_log(line: &str) {
    use std::io::Write;
    let Some(path) = micold_daemon::logging::default_log_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{line}");
    }
}
