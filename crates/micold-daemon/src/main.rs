//! micold-daemon — the user-space session host (feature 010).
//!
//! Skeleton at the Phase 1 checkpoint. The transport, single-instance binding, catalog
//! state ownership, and PTY/VT session hosting are implemented in Phases 2–3 (plan W1–W3).
//! This entry point exists so the workspace's third crate compiles and the render-free
//! boundary (`micold-core` has no iced/PTY deps) is exercised end-to-end.

fn main() {
    // Intentionally empty until T020 (daemon startup/bind/accept loop).
    eprintln!("micold-daemon: not yet implemented (feature 010, Phase 2+)");
}
