//! Render-free application core: state, messages, and the `update` reducer.
//!
//! This module has no dependency on iced. All state transitions are pure and
//! unit-testable via `cargo test` (Constitution Principle I). The GUI binary adapts
//! this core to iced's runtime in `src/main.rs`.
//!
//! Side-effectful concerns (reading the filesystem for a directory listing, detecting a
//! folder's git status, and persisting the catalog) are performed by the binary at the
//! I/O boundary; the reducer stays pure. A few messages (`ProjectSelectorOpened`,
//! `FolderChosen`) therefore carry no reducer effect here — they are documented no-ops
//! handled entirely in `src/main.rs`.

use crate::project::{FolderEntry, RenameError};
use crate::selector::Selector;
use crate::theme::{resolve, ColorScheme, SystemScheme, ThemePreference};
use std::path::PathBuf;

/// The labels of the top toolbar's entries, in display order.
///
/// The shell deliberately exposes exactly one entry — "Help" (FR-002, FR-003).
pub const TOOLBAR_ENTRIES: [&str; 1] = ["Help"];

/// The labels of the actions revealed under the "Help" menu, in display order.
///
/// "Help" exposes exactly one action — "About" (FR-003, FR-004).
pub const HELP_ACTIONS: [&str; 1] = ["About"];

/// The top toolbar's entry labels. See [`TOOLBAR_ENTRIES`].
pub fn toolbar_entries() -> &'static [&'static str] {
    &TOOLBAR_ENTRIES
}

/// The actions under the "Help" menu. See [`HELP_ACTIONS`].
pub fn help_actions() -> &'static [&'static str] {
    &HELP_ACTIONS
}

/// Which modal overlay, if any, is currently shown over the main window.
///
/// Modeling the overlay as an enum (rather than a `bool` per dialog) makes
/// "the About dialog is open twice" unrepresentable — satisfying FR-015 at the
/// type level (Constitution Principle V).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overlay {
    /// No modal is open; the main window is fully interactive.
    #[default]
    None,
    /// The About dialog is shown as a modal overlay.
    About,
    /// The in-app project selector (folder browser) is shown as a modal overlay.
    ProjectSelector,
    /// The rename-project dialog is shown as a modal overlay.
    RenameProject,
}

/// In-progress rename state, present only while the rename dialog is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDraft {
    /// The project being renamed, identified by its (canonical) path.
    pub path: PathBuf,
    /// The current editable text in the dialog.
    pub text: String,
    /// The last validation error, if the user tried to confirm an invalid name (FR-020).
    pub error: Option<RenameError>,
}

/// Every user interaction that can change application state.
///
/// No longer `Copy`: some variants carry owned data (`PathBuf`, listing results).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// The "Help" toolbar entry was selected (reveals/collapses its "About" action).
    HelpMenuToggled,
    /// The "About" action was activated.
    AboutOpened,
    /// The About dialog was dismissed (Close button or Esc).
    AboutClosed,
    /// Open the project selector (folder browser). The binary computes the starting
    /// directory and launches the first directory scan (FR-001).
    ProjectSelectorOpened,
    /// Navigate the selector into a subfolder (FR-002).
    SelectorNavigatedInto(PathBuf),
    /// Navigate the selector to the parent directory (FR-002).
    SelectorNavigatedUp,
    /// A directory scan completed; populate the current listing (FR-006).
    SelectorListingReady(Vec<FolderEntry>),
    /// A directory scan failed (e.g. permission denied); show an error, do not crash.
    SelectorListingFailed(String),
    /// Open the currently-browsed folder as a project (FR-003, FR-005). The binary
    /// records git status/availability and persists the catalog.
    FolderChosen(PathBuf),
    /// Dismiss the project selector without choosing (Cancel button or Esc).
    ProjectSelectorClosed,
    /// Reopen a known project from the list without browsing (FR-011). The binary
    /// refreshes its availability, activates it if available (FR-023), and persists.
    KnownProjectReopened(PathBuf),
    /// Begin renaming the given project; opens the rename dialog (FR-017).
    RenameStarted(PathBuf),
    /// The rename dialog's text changed.
    RenameTextChanged(String),
    /// Confirm the rename. Applies it if valid (FR-020); the binary then persists.
    RenameConfirmed,
    /// Dismiss the rename dialog without applying (Cancel or Esc).
    RenameCancelled,
    /// The user selected a theme preference (Follow system / Light / Dark) (FR-007, FR-008).
    /// The binary persists the updated preference afterward.
    ThemePreferenceChanged(ThemePreference),
    /// The OS light/dark preference poll observed a (changed) scheme (FR-006). Transient;
    /// never persisted.
    SystemThemeChanged(SystemScheme),
}

/// Root application state for the single main window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The currently displayed modal overlay.
    pub overlay: Overlay,
    /// Whether the Help menu is currently expanded (transient UI affordance).
    pub help_menu_open: bool,
    /// The known-projects catalog and the active working space (persisted). Per-story
    /// selector/rename working state is added alongside those stories.
    pub workspace: crate::workspace::Workspace,
    /// The folder-browser state, present only while the project-selector overlay is shown.
    pub selector: Option<Selector>,
    /// The in-progress rename, present only while the rename overlay is shown.
    pub rename_draft: Option<RenameDraft>,
    /// How the app chooses its theme (persisted); defaults to following the OS (FR-005).
    pub theme_pref: ThemePreference,
    /// The last light/dark scheme reported by the OS poll (transient, not persisted).
    pub system_scheme: SystemScheme,
}

impl State {
    /// The color scheme to render, resolved from the user's preference and the OS scheme
    /// (FR-005, FR-007, FR-018). See [`crate::theme::resolve`].
    pub fn color_scheme(&self) -> ColorScheme {
        resolve(self.theme_pref, self.system_scheme)
    }

    /// Apply a [`Message`], transitioning the state. Pure and side-effect free.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::HelpMenuToggled => {
                self.help_menu_open = !self.help_menu_open;
            }
            Message::AboutOpened => {
                // Idempotent: opening while already open keeps a single instance (FR-015).
                self.overlay = Overlay::About;
                self.help_menu_open = false;
            }
            Message::AboutClosed => {
                // No-op when nothing is open (edge case); otherwise return to the
                // main window unchanged (FR-012).
                self.overlay = Overlay::None;
            }
            Message::SelectorNavigatedInto(path) => {
                if let Some(selector) = &mut self.selector {
                    selector.enter(path);
                }
            }
            Message::SelectorNavigatedUp => {
                if let Some(selector) = &mut self.selector {
                    selector.up();
                }
            }
            Message::SelectorListingReady(entries) => {
                if let Some(selector) = &mut self.selector {
                    selector.listing_ready(entries);
                }
            }
            Message::SelectorListingFailed(message) => {
                if let Some(selector) = &mut self.selector {
                    selector.listing_failed(message);
                }
            }
            Message::ProjectSelectorClosed => {
                self.overlay = Overlay::None;
                self.selector = None;
            }
            Message::RenameStarted(path) => {
                let current = self
                    .workspace
                    .projects
                    .iter()
                    .find(|p| p.path == path)
                    .map(|p| p.display_name.clone());
                if let Some(name) = current {
                    self.rename_draft = Some(RenameDraft {
                        path,
                        text: name,
                        error: None,
                    });
                    self.overlay = Overlay::RenameProject;
                }
            }
            Message::RenameTextChanged(text) => {
                if let Some(draft) = &mut self.rename_draft {
                    draft.text = text;
                    draft.error = None;
                }
            }
            Message::RenameConfirmed => {
                if let Some((path, text)) = self
                    .rename_draft
                    .as_ref()
                    .map(|draft| (draft.path.clone(), draft.text.clone()))
                {
                    match self.workspace.rename(&path, &text) {
                        // Renaming never touches disk — only the stored name (FR-018).
                        Ok(()) => {
                            self.overlay = Overlay::None;
                            self.rename_draft = None;
                        }
                        Err(error) => {
                            if let Some(draft) = &mut self.rename_draft {
                                draft.error = Some(error);
                            }
                        }
                    }
                }
            }
            Message::RenameCancelled => {
                self.overlay = Overlay::None;
                self.rename_draft = None;
            }
            Message::ThemePreferenceChanged(pref) => {
                // Pure state change; the binary persists it at the I/O boundary (FR-009).
                self.theme_pref = pref;
            }
            Message::SystemThemeChanged(scheme) => {
                self.system_scheme = scheme;
            }
            // Performed by the binary at the I/O boundary (needs the home directory + a
            // scan task, a FolderScanner, or persistence); no pure reducer effect.
            Message::ProjectSelectorOpened
            | Message::FolderChosen(_)
            | Message::KnownProjectReopened(_) => {}
        }
    }
}

/// Map an Escape key press to a [`Message`] given the current state.
///
/// Returns `Some(AboutClosed)` only while the About overlay is open (FR-011); returns
/// `None` otherwise, so pressing Esc with no dialog open has no effect (edge case). The
/// iced keyboard subscription in the binary delegates to this pure function.
pub fn on_escape(state: &State) -> Option<Message> {
    match state.overlay {
        Overlay::About => Some(Message::AboutClosed),
        Overlay::ProjectSelector => Some(Message::ProjectSelectorClosed),
        Overlay::RenameProject => Some(Message::RenameCancelled),
        Overlay::None => None,
    }
}
