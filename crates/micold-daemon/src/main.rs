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
//! The one `exit` below is on the fatal-error path, after `start` has already returned and its
//! runtime has already been dropped. It sets a non-zero status for a daemon that could not start;
//! it does not cut a shutdown short.

// A release build on Windows opens no console window of its own, for the app or for the daemon it
// starts (feature 030, FR-005). Debug builds keep the console so `cargo run` output stays visible.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
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

#[tokio::main]
async fn start() -> std::io::Result<()> {
    micold_daemon::run().await
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
