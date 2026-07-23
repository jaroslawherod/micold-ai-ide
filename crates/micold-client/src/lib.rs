//! Micold AI IDE — iced client library.
//!
//! The rendering layer and client-only support modules: application state + `update`
//! reducer (`app`), key mapping (`keymap`), icon/token design data (`icons`, `tokens`),
//! animation timing (`motion`), and the iced widget tree (`ui`). The shared domain model
//! and persistence live in [`micold_core`]; the PTY/VT session host lives in the daemon.

pub mod app;
pub mod daemon;
pub mod icons;
pub mod input;
pub mod keymap;
pub mod motion;
pub mod selection;
pub mod tokens;
pub mod ui;
