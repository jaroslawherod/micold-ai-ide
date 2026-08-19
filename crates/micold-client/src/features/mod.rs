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

use crate::overlay::SurfaceId;
use micold_core::session::SessionId;

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
    /// These sessions are gone; the session feature should drop them (FR-021, contract O2).
    ///
    /// Emitted by the worktree delete path, which owns *worktrees* and must not reach into session
    /// state to close what lived in one — the anti-pattern the contract names by name. T066 is the
    /// conversion; this variant is the vocabulary it needs to exist first.
    SessionsClosed(Vec<SessionId>),
    /// This surface should close, whoever owns it.
    ///
    /// Also the worktree delete path: confirming a delete dismisses its own dialog, and the
    /// registry — not the deleting feature — is what knows how a surface closes.
    OverlayDismissed(SurfaceId),
    /// Raise a notification, from any feature.
    ///
    /// The one outcome the contract lists as "emitted by: any feature", and the reason is that a
    /// notification is nobody's feature: every path that can fail wants one, and `notify` belongs
    /// to none of them. Carries the queue's own `Notification` rather than the banner's
    /// `NoticeLevel`, so the translation stays where `State::push_notification` already put it.
    NotificationRaised(micold_core::notify::Notification),
    /// Discovery replaced the worktree list; these `dir_name`s are what survived (T066).
    ///
    /// **The first outcome anything actually emits**, and the one that gave `app::drain` its first
    /// caller. The worktree feature owns the list and prunes its own state when it changes; the
    /// sidebar's expansion set is not its to prune, so it reports what survived and the root routes
    /// that to `sidebar::worktrees_replaced`.
    WorktreesReplaced(std::collections::BTreeSet<String>),
    /// The shell created this worktree; the list it joins is not the form's to write (T067a-4).
    ///
    /// `worktree_form` is a separate feature because its lifecycle is independent (FR-003), but
    /// what it creates lands in `worktree`'s list. The form reports the creation and closes; the
    /// worktree feature performs the insert, keeping its own two identities distinct — a create
    /// names the directory, a daemon include answers with a path.
    WorktreeCreated(micold_core::worktree::Worktree),
}

pub mod connection;
pub mod help;
pub mod notifications;
pub mod project;
pub mod session;
pub mod settings;
pub mod sidebar;
pub mod window;
pub mod worktree;
pub mod worktree_form;
