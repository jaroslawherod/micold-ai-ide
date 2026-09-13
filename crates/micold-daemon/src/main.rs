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
