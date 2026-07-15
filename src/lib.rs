//! Micold AI IDE — render-free application core.
//!
//! This library crate holds the pure, GUI-independent logic (application state, the
//! `update` reducer, and application metadata). It has no dependency on iced, so the
//! entire core is unit-testable with `cargo test --no-default-features` without
//! building the GUI stack (Constitution Principle I).
//!
//! The iced rendering layer lives in the `micold-ai-ide` binary (`src/main.rs`) behind
//! the `gui` feature.

pub mod app;
pub mod fs_scan;
pub mod metadata;
pub mod project;
pub mod selector;
pub mod settings;
pub mod store;
pub mod theme;
pub mod tokens;
pub mod workspace;
