//! Micold AI IDE — iced client library.
//!
//! The rendering layer and client-only support modules: application state + `update`
//! reducer (`app`), per-feature data and operations (`features`), key mapping (`keymap`),
//! icon/token design data (`icons`, `tokens`), what a floating surface is (`overlay`), and the
//! iced widget tree (`ui`). Animation timing is not here: every transition is owned by the
//! component that plays it (feature 017).
//! The shared domain model and persistence live in [`micold_core`]; the PTY/VT session host
//! lives in the daemon.

pub mod app;
/// Folding the daemon's catalog snapshot into client state. In the library, not the binary,
/// so `tests/` can drive the daemon → wire → client join (see the module docs).
pub mod catalog_sync;
pub mod daemon;
pub mod features;
pub mod grid;
pub mod icons;
pub mod input;
pub mod keymap;
pub mod overlay;
pub mod selection;
pub mod showcase;
pub mod ui;
