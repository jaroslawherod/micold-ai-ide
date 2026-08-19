//! The application window: its size, the pointer in it, and which field holds the keyboard
//! (feature 021, T063).
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
//! The three belong together because they answer one question — *what is the window doing right
//! now* — and because every one of them is transient. None is persisted, none survives a restart,
//! and each is reported by the windowing system rather than chosen by the user.
//!
//! # Everything here is read across features, and that is fine
//!
//! `cursor` and `window_size` exist so a context menu can be anchored and clamped; `focused_field`
//! decides every text field's focus chrome. FR-003a permits cross-feature *reads* explicitly —
//! isolation is enforced on writes. So `features/project.rs` reading `state.cursor` to anchor its
//! menu is not a violation and never was.
//!
//! What *is* watched is who writes them. `tests/feature_write_isolation.rs` now attributes these
//! three paths to `window`, which turns the writes reaching them from root helpers
//! (`State::clear_for_dialog`, `State::focus_terminal`) into cross-feature writes with a named
//! owner instead of an unanswerable question about `root`. That is the point of naming the
//! feature: T067 can now propose an outcome for them, which it could not do while the owner was
//! "nobody".

use crate::app::State;

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

/// A text field gained or lost the keyboard (BUG-003).
///
/// **A blur is only believed from the field that currently holds focus.** Gaining and losing are
/// reported by two different widgets and arrive in whichever order the frame produced them, so an
/// unguarded `None` on the way out of one field would erase the focus the next one had already
/// claimed — and clicking straight from one field to another would leave both at rest.
pub fn field_focus_changed(state: &mut State, field: FieldId, focused: bool) {
    if focused {
        state.focused_field = Some(field);
    } else if state.focused_field == Some(field) {
        state.focused_field = None;
    }
}

/// The pointer moved (feature 015).
///
/// Tracked only while the project switcher is open — see the shell's cursor subscription — purely
/// so a right-click can anchor its context menu where the pointer is.
pub fn cursor_moved(state: &mut State, x: u16, y: u16) {
    state.cursor = (x, y);
}

/// The window was resized (feature 015).
///
/// Used to clamp a context menu so it cannot open off-screen. `(0, 0)` means "not reported yet",
/// which disables clamping rather than pinning every menu to the origin.
pub fn resized(state: &mut State, width: u16, height: u16) {
    state.window_size = (width, height);
}

impl State {
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
        self.focused_field = None;
    }
}
