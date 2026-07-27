//! Micold AI IDE — render-free shared core.
//!
//! This crate holds the pure, GUI-independent domain model shared by the client and the
//! daemon: application/session/worktree state, local-first persistence, project discovery,
//! the AI-CLI provider abstraction, and (feature 010) the client<->daemon wire protocol.
//!
//! It has **no dependency on iced or on any PTY/VT crate** — that boundary is enforced by
//! this crate's `Cargo.toml` (FR-040), so the entire core is unit-testable with
//! `cargo test -p micold-core` without building the GUI stack (Constitution Principle I).
//!
//! The iced rendering layer lives in `micold-client`; the PTY/VT session host lives in
//! `micold-daemon`.

pub mod connect;
pub mod endpoint;
pub mod env_include;
pub mod fs_scan;
pub mod git;
pub mod input;
pub mod logout_survival;
pub mod metadata;
pub mod naming;
pub mod overlay;
pub mod project;
pub mod protocol;
pub mod provider;
pub mod selector;
pub mod session;
pub mod settings;
pub mod spawn;
pub mod store;
pub mod terminal;
pub mod theme;
pub mod tokens;
pub mod workspace;
pub mod worktree;
