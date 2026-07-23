//! Micold session daemon — the user-space host that owns every PTY/VT session so they outlive the
//! UI (feature 010). Headless by design: no iced, fully testable without a graphical environment
//! (Constitution Principle I, FR-039).
//!
//! Phase 2 laid the transport spine — endpoint location, the single-instance startup sequence, and
//! the connection accept loop with the strict handshake. Phase 3 (plan W3) adds the PTY/VT stack:
//! the daemon owns every session's `Term` and child process ([`supervisor`], [`terminal`]) so they
//! outlive the UI. The shadow-diff framer that streams the interpreted grid to clients lands next.

pub mod activity;
pub mod catalog;
pub mod framer;
pub mod lifecycle;
pub mod logging;
pub mod server;
pub mod singleton;
pub mod state;
pub mod supervisor;
pub mod terminal;

pub use server::run;
