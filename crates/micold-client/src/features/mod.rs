//! Feature modules (feature 021, Tier 1).
//!
//! One module per feature, each holding that feature's types **together with** the helper
//! functions that operate on them. A feature is never split across parallel state / update / view
//! files — a type and the functions over it stay in one place (FR-001a).
//!
//! These modules are render-free: they must not name the rendering framework in code, which
//! `tests/features_are_render_free.rs` holds. That property is what lets the application's state
//! live in this crate rather than the core, so it is checked rather than assumed.
//!
//! Views live in `crate::ui`, beside — not inside — the feature they draw. Reducer modules arrive
//! in Tier 3; until then a feature module holds data and operations only, with no message
//! vocabulary of its own.
//!
//! Modules are declared here as each is extracted from `crate::app`, one migration step at a time.

pub mod sidebar;
pub mod worktree_form;
