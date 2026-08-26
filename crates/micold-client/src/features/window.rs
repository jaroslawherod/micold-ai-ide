//! The application window: its size, and which field holds the keyboard (feature 021, T063).
//!
//! # Why this is a feature rather than root state
//!
//! T062 left three arms in the root reducer — `FieldFocusChanged`, `CursorMoved` and
//! `WindowResized` — writing three fields no feature owned. FR-002 asks the root for composition
//! and routing only, so "no feature owns it" is not an answer the root can keep giving; it is a
//! feature that has not been named yet. The precedent is T031, which created `features/help.rs`
//! because the overflow menu had no home either: FR-001 asks where a feature lives, not how big
//! it is.
//!
//! The three belonged together because they answered one question — *what is the window doing
//! right now* — and because every one of them is transient. None is persisted, none survives a
//! restart, and each is reported by the windowing system rather than chosen by the user.
//!
//! **Two of the three are left, and the third was deleted rather than rehoused.** `main`'s 018
//! BUG-008 fix landed while this feature was in flight: a context menu now anchors at the point
//! its own press landed on, carried on the message, rather than at a pointer position tracked in
//! `State::cursor` and read later. That is a better answer than the one this module was
//! defending, and it costs the argument above nothing — a field the root still decides about is
//! still a feature nobody has named. There are two.
//!
//! # Everything here is read across features, and that is fine
//!
//! `window_size` exists so a context menu can be clamped to it; `focused_field` decides every text
//! field's focus chrome. FR-003a permits cross-feature *reads* explicitly — isolation is enforced
//! on writes. So `ui/mod.rs` reading `state.window.window_size` to clamp a menu is not a
//! violation and never was.
//!
//! What *is* watched is who writes them. `tests/feature_write_isolation.rs` attributes both paths
//! to `window`, which turned the writes reaching them from root helpers into cross-feature writes
//! with a named owner instead of an unanswerable question about `root`. That is the point of
//! naming the feature: T067 could then propose an outcome for them, which it could not do while
//! the owner was "nobody". **Both halves were answered, and only one of them needed an outcome.**
//! T067a-5 moved `clear_for_dialog` here and wrote none — a feature writing its own field is not
//! a cross-feature write at all. `focus_terminal` could not follow it: it also writes
//! `terminal_released`, which is the session's, so T067a-7 moved the function into the session and
//! T067a-9 converted its `focused_field` write into `Outcome::FieldFocusCleared`, applied by
//! [`field_focus_cleared`] below.
//!
//! # The vocabulary this feature declares
//!
//! Two transitions in [`Msg`] — `FieldFocusChanged` and `Resized` — routed by [`update`], which is
//! pure (data-model.md §1.1 shape A). Both are reports from the windowing system rather than choices,
//! so neither needs an effect back: the binary matches nothing here a second time.
//!
//! The third arm this module was named for, `CursorMoved`, is gone rather than nested; the reason is
//! the paragraph above about 018 BUG-008.

/// What this feature remembers (feature 028, contract S1).
///
/// The fields keep the names they had as flat members of `app::State`, and the reducers below
/// spell the root's type `crate::app::State` now that `State` here means this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Which text field holds the keyboard, if any (BUG-003). Transient — never persisted.
    ///
    /// Held here rather than on each draft because it is one fact about the application, not four:
    /// see [`FieldId`]. Every filled field's focus chrome is drawn from this and nothing else.
    pub focused_field: Option<FieldId>,
    /// Last known window size in pixels (feature 015), used to clamp a context menu so it cannot
    /// open off-screen. `(0, 0)` means "not reported yet", which disables clamping. Transient.
    pub window_size: (u16, u16),
}

/// Which text field holds the keyboard, when one does (BUG-003).
///
/// A filled field's whole focus affordance — the label floating clear of the value, the active
/// indicator thickening to the accent, the focus state layer (§7.7, FR-031, FR-035) — is decided
/// when the field is *built*, from a flag its caller supplies. Nothing supplied it. The component
/// honoured the flag, every anatomy gate proved it honoured the flag, and in the running
/// application every field was drawn permanently at rest.
///
/// One enum for the whole application rather than a focus flag on each of the four drafts: at most
/// one field can hold the keyboard, and this is the shape that says so. `Option<FieldId>` also makes "two fields focused at once" unrepresentable
/// (Principle V), where four booleans would have needed a rule keeping them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldId {
    /// The rename-project dialog's name field.
    RenameProjectName,
    /// The rename-worktree dialog's name field.
    RenameWorktreeName,
    /// The add-worktree form's optional ticket field.
    AddWorktreeTicket,
    /// The add-worktree form's branch-name field.
    AddWorktreeName,
    /// Settings: the terminal scrollback limit.
    SettingsScrollback,
    /// The confirm-worktree-delete dialog's "also delete the branch" checkbox.
    ConfirmDeleteAlsoBranch,
    /// Settings: the environment-include on/off checkbox. Not a text field — the checkbox now
    /// takes the keyboard too, and this is the same fact about the same dialog (BUG-003).
    SettingsEnvIncludeEnabled,
    /// Settings: the environment-include script path.
    SettingsEnvIncludePath,
    /// Settings: the environment-include timeout.
    SettingsEnvIncludeTimeout,
}

/// What the window reports about itself (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// The root's `WindowResized` is `Msg::Resized` here — the type says which thing resized, so the
/// variant does not have to (contract M1). `FieldFocusChanged` carried no prefix to drop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// A text field took or lost the keyboard (BUG-003). Emitted by the field's own container,
    /// which asks the input rather than guessing from the pointer — see
    /// `material::FormField::on_focus_change`. Sole mutation: [`State::focused_field`].
    FieldFocusChanged(FieldId, bool),
    /// The window was resized (or reported its initial size). Feeds context-menu clamping.
    Resized {
        /// The window's new width, in logical pixels.
        width: u16,
        /// The window's new height, in logical pixels.
        height: u16,
    },
}

/// This feature's whole reducer surface: one entry point, shape A (contract M2).
///
/// Both arms are pure writes to fields this module owns, so nothing comes back.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::FieldFocusChanged(field, focused) => field_focus_changed(state, field, focused),
        Msg::Resized { width, height } => resized(state, width, height),
    }
    Vec::new()
}

/// A text field gained or lost the keyboard (BUG-003).
///
/// **A blur is only believed from the field that currently holds focus.** Gaining and losing are
/// reported by two different widgets and arrive in whichever order the frame produced them, so an
/// unguarded `None` on the way out of one field would erase the focus the next one had already
/// claimed — and clicking straight from one field to another would leave both at rest.
pub fn field_focus_changed(state: &mut crate::app::State, field: FieldId, focused: bool) {
    if focused {
        state.window.focused_field = Some(field);
    } else if state.window.focused_field == Some(field) {
        state.window.focused_field = None;
    }
}

/// A terminal took the keyboard (FR-018; T067a-9).
///
/// Reached from `Outcome::FieldFocusCleared`. Unconditional by design — see the outcome's own note.
pub fn field_focus_cleared(state: &mut crate::app::State) {
    state.window.focused_field = None;
}

/// The window was resized (feature 015).
///
/// Used to clamp a context menu so it cannot open off-screen. `(0, 0)` means "not reported yet",
/// which disables clamping rather than pinning every menu to the origin.
pub fn resized(state: &mut crate::app::State, width: u16, height: u16) {
    state.window.window_size = (width, height);
}

impl crate::app::State {
    /// Callers must invoke it **before** setting up the dialog they are opening — otherwise it
    /// closes the one they just prepared. The eight call sites that did it the other way round
    /// were reordered at T037.
    /// **Moved here from `app.rs` by T067a-5.** The slot it clears is `focused_field`, which
    /// `features/window.rs` owns since T063 — so this is the window's operation, and leaving it in
    /// the root made eight feature reducers each look like they wrote window state when one
    /// function does. Same shape T067a-7 found under `focus_terminal`; the guard reports *callers*
    /// when the writer is root code it cannot attribute.
    pub fn clear_for_dialog(&mut self) {
        crate::overlay::registry::close_dialogs(self);
        crate::overlay::registry::close_popovers(self);
        // A dialog opens with nothing focused. The fields that reported focus belong to a widget
        // tree that is being torn down and will never report losing it, so a remembered focus would
        // outlive them — and reopening the same dialog would draw its field focused over an input
        // that has not been clicked (BUG-003).
        self.window.focused_field = None;
    }
}
