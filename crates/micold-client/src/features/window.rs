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
//! Three transitions in [`Msg`] — `FieldFocusChanged`, `Resized` and `InstallLocationReported` —
//! routed by [`update`], which is pure (data-model.md §1.1 shape A). All three are reports rather
//! than choices, so none needs an effect back: the binary matches nothing here a second time. The
//! third (feature 028) is a report about the application rather than about the window, and it is
//! here for the same reason the other two are: it is transient, nobody chose it, and the window is
//! what it decides.
//!
//! The third arm this module was named for, `CursorMoved`, is gone rather than nested; the reason is
//! the paragraph above about 018 BUG-008.
//!
//! # The state this feature remembers (feature 028, contract S1)
//!
//! Three fields in [`State`], reached as `state.window`: `window_size`, the last size the windowing
//! system reported; `focused_field`, which application field holds the keyboard — `None` when
//! none does, which is the state a terminal needs before it can take input; and `install_location`
//! (feature 028), where this copy is running from, which decides whether there is a session UI to
//! show at all.
//!
//! Both keep the names they had flat on the root (T030). `window.window_size` stutters and is kept
//! anyway: `window.size` would read as a geometry accessor on a window handle rather than as the
//! last report received, and the distinction between the two is the whole of why this is stored
//! rather than asked for.
//!
//! `focused_field` is written from more than this feature, which is why
//! [`Outcome::FieldFocusCleared`](crate::features::Outcome) exists — the session clears it by
//! reporting rather than by reaching in (T067a-9).

use micold_core::install_location::InstallLocation;

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
    /// Where this copy of the application is running from, answered once at boot (feature 028,
    /// FR-019).
    ///
    /// Transient like the other two, and reported rather than chosen — which is what puts it in
    /// this feature: `current_exe()` is the windowing system's question turned on the application
    /// itself. Defaults to [`InstallLocation::Installed`], the answer that lets the application
    /// start, so a state built before anyone asked is not a state that refuses to run.
    pub install_location: InstallLocation,
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
    /// Settings: the sandbox image's reference (feature 027).
    SettingsImageReference,
    /// Settings: the archive an imported image is loaded from (feature 027).
    SettingsImagePath,
    /// Settings: one host credential's share opt-in (feature 027, FR-004c).
    ///
    /// Parameterised rather than four variants, because the four are the *same* control repeated
    /// over `CredentialShare::ALL` — spelling them out would mean a fifth share silently losing
    /// its keyboard focus rather than failing to compile.
    SettingsCredential(micold_core::sandbox::CredentialShare),
    /// Settings: the "keep sessions running after I sign out" checkbox (feature 027).
    SettingsSurviveLogout,
    /// Settings: the "report Pi activity" checkbox (feature 029, FR-012e).
    SettingsPiActivityComponent,
    /// Settings: the sandbox's processor limit, in cores (feature 027, FR-012).
    SettingsCpuLimit,
    /// Settings: the sandbox's memory limit, in MiB (feature 027, FR-013).
    SettingsMemoryLimit,
    /// Settings: the sandbox's process-count limit (feature 027, FR-014).
    SettingsPidLimit,
    /// Settings: the sandbox's writable-storage limit, in MiB (feature 027, FR-015).
    SettingsStorageLimit,
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
    /// Where the executable was found, answered once at boot (feature 028, FR-019).
    ///
    /// Sent by the binary rather than by a gesture: `current_exe()` is a syscall, so the question
    /// belongs at the I/O boundary and the verdict is a value from there on. It arrives as a
    /// message like every other report because the root is the only thing that drives a feature --
    /// a second driver is a second place that has to learn about every feature added after it.
    ///
    /// There is no variant that clears it. See [`install_location_reported`].
    InstallLocationReported(InstallLocation),
}

/// This feature's whole reducer surface: one entry point, shape A (contract M2).
///
/// Both arms are pure writes to fields this module owns, so nothing comes back.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::FieldFocusChanged(field, focused) => field_focus_changed(state, field, focused),
        Msg::Resized { width, height } => resized(state, width, height),
        Msg::InstallLocationReported(location) => install_location_reported(state, location),
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

/// Where this executable is running from, as the boot path found it (feature 028, FR-019).
///
/// The syscall (`current_exe()`) stays at the shell boundary and the verdict arrives here as a
/// value, which is what lets this be a reducer: the interesting behaviour is not reading the path
/// but what the window is then allowed to show, and that is decided here and tested without a
/// window.
///
/// Written once, at boot. There is no message that clears it and no gesture that dismisses the
/// screen it produces -- see [`crate::app::State::install_blocked`].
pub fn install_location_reported(state: &mut crate::app::State, location: InstallLocation) {
    state.window.install_location = location;
}

impl crate::app::State {
    /// Whether the application must show the install-me screen instead of the session UI.
    ///
    /// The whole of FR-019's "MUST NOT run in a way that looks installed" is this one question,
    /// asked in one place. A copy running from a mounted image or a translocated path is going to
    /// disappear, taking whatever was done in it; a warning the user can dismiss is a warning the
    /// user learns to dismiss, so there is no message that turns this off.
    ///
    /// On the root rather than on [`State`] for the reason `clear_for_dialog` is: every caller
    /// holds the root, and `state.install_blocked()` is the question they are asking.
    pub fn install_blocked(&self) -> bool {
        !self.window.install_location.is_installed()
    }

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
