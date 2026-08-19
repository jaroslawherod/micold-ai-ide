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

/// A consequence a feature returns rather than applies (FR-021).
///
/// **One variant so far, and it is here in Phase 5 rather than at T065 because FR-015a needs it.**
/// Clipboard access cannot be a service capability: all of its call sites return an `iced::Task`
/// rather than a value, so a synchronous port cannot wrap them without blocking. FR-015a's
/// alternative for exactly that case is an explicit effect request in this vocabulary, *interpreted
/// by the shell*.
///
/// That is also why introducing it here is not the Tier 3 work SC-004b's checkpoint forbids.
/// FR-021 sits under "cross-feature coordination" because its subject there is a feature reducer
/// returning a consequence for the **root** to interpret (FR-022) and route to another feature.
/// None of that arrives with this variant: there is no root interpreter, no reducer split, and no
/// feature learns anything about another. T065 adds `SessionsClosed`, `OverlayDismissed` and
/// `NotificationRaised` along with the root's draining interpreter — it extends this enum rather
/// than introducing it, which is the one edit its text needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Put `text` on the system clipboard.
    ///
    /// The shell translates this to `iced::clipboard::write` and decides nothing on the way
    /// (contract C3) — whether anything should be written at all was settled by the feature that
    /// emitted the request.
    ClipboardWrite(String),
}

pub mod connection;
pub mod help;
pub mod notifications;
pub mod project;
pub mod sandbox;
pub mod session;
pub mod settings;
pub mod sidebar;
pub mod worktree;
pub mod worktree_form;
