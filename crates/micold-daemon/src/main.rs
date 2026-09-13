//! The `micold-daemon` binary entry point (feature 010).
//!
//! Normally auto-spawned by the client (T026a); can also be run directly (`mise run daemon`) or via
//! a systemd user unit (T076). All logic lives in the library so it is testable headlessly.

fn main() {
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
