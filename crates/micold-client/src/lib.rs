//! Micold AI IDE — iced client library.
//!
//! The rendering layer and client-only support modules: application state + `update`
//! reducer (`app`), key mapping (`keymap`), icon/token design data (`icons`, `tokens`),
//! and the iced widget tree (`ui`). Animation timing is not here: every transition is owned
//! by the component that plays it (feature 017). The shared domain model
//! and persistence live in [`micold_core`]; the PTY/VT session host lives in the daemon.

pub mod app;
pub mod daemon;
pub mod grid;
pub mod icons;
pub mod input;
pub mod keymap;
pub mod selection;
pub mod showcase;
pub mod ui;
