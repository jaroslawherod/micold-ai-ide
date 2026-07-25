//! The `micold-daemon` binary entry point (feature 010).
//!
//! Normally auto-spawned by the client (T026a); can also be run directly (`mise run daemon`) or via
//! a systemd user unit (T076). All logic lives in the library so it is testable headlessly.

fn main() {
    if let Err(e) = start() {
        eprintln!("micold-daemon: fatal: {e}");
        std::process::exit(1);
    }
}

#[tokio::main]
async fn start() -> std::io::Result<()> {
    micold_daemon::run().await
}
