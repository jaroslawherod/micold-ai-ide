//! Micold session daemon — the user-space host that owns every PTY/VT session so they outlive the
//! UI (feature 010). Headless by design: no iced, fully testable without a graphical environment
//! (Constitution Principle I, FR-039).
//!
//! Phase 2 (this layer): the transport spine — endpoint location, the single-instance startup
//! sequence, and the connection accept loop with the strict handshake. PTY ownership, the catalog,
//! and grid streaming land in Phases 2b–3.

pub mod catalog;
pub mod lifecycle;
pub mod logging;
pub mod server;
pub mod singleton;
pub mod state;

pub use server::run;
