//! Micold AI IDE — GUI binary entry point.
//!
//! This binary adapts the render-free core (`micold_ai_ide::app`) to the iced runtime.
//! All state transitions live in the core and are unit-tested there; this layer renders
//! state, selects the Material theme for the resolved color scheme, translates input into
//! core [`Message`]s, and performs the feature's I/O at the boundary: resolving the starting
//! directory, scanning directories off the render path, detecting git status, detecting the
//! OS light/dark preference, and persisting the catalog and settings.

mod ui;

use iced::time::every;
use iced::{Subscription, Task};
use micold_ai_ide::app::{Message, Overlay, State};
use micold_ai_ide::fs_scan::{FolderScanner, StdFolderScanner};
use micold_ai_ide::selector::{Selector, SelectorStatus};
use micold_ai_ide::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_ai_ide::store::{JsonFileStore, ProjectStore};
use micold_ai_ide::theme::SystemScheme;
use std::path::PathBuf;
use std::time::Duration;

/// How often the OS light/dark preference is polled. Sub-second so a system theme change is
/// reflected within SC-003's 1-second bound; iced 0.13 emits no theme-changed event
/// (research R4).
const OS_THEME_POLL: Duration = Duration::from_millis(500);

pub fn main() -> iced::Result {
    iced::application("Micold AI IDE", update, view)
        .theme(theme)
        .default_font(iced::Font::DEFAULT)
        .subscription(subscription)
        .run_with(boot)
}

/// Build the initial state: load the known-projects catalog and the theme preference from
/// local storage, recompute project availability, and seed the OS color scheme so the first
/// frame renders in the correct theme (FR-005, SC-002).
fn boot() -> (State, Task<Message>) {
    let mut state = State::default();
    if let Some(store) = JsonFileStore::default_location() {
        state.workspace = store.load().workspace;
        state
            .workspace
            .refresh_availability(&StdFolderScanner::new());
    }
    if let Some(store) = JsonFileSettingsStore::default_location() {
        state.theme_pref = store.load().settings.theme;
    }
    state.system_scheme = detect_system_scheme();
    (state, Task::none())
}

/// Persist the current catalog to local storage. Local-first: a save failure is non-fatal
/// and never crashes the app (Constitution Principle IV).
fn persist(state: &State) {
    if let Some(store) = JsonFileStore::default_location() {
        let _ = store.save(&state.workspace);
    }
}

/// Persist the current theme preference (FR-009). Non-fatal on failure (Principle IV).
fn persist_settings(state: &State) {
    if let Some(store) = JsonFileSettingsStore::default_location() {
        let _ = store.save(&Settings {
            theme: state.theme_pref,
        });
    }
}

/// Apply a message. Pure UI transitions go through the core reducer; messages that need
/// filesystem access, persistence, or spawn work are handled here and may return a [`Task`].
fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        // Open the selector at the starting directory and kick off the first scan.
        Message::ProjectSelectorOpened => {
            let dir = start_dir();
            state.selector = Some(Selector::open_at(dir.clone()));
            state.overlay = Overlay::ProjectSelector;
            scan_task(dir)
        }
        // Navigation: apply the pure transition, then scan the newly-current directory.
        Message::SelectorNavigatedInto(_) | Message::SelectorNavigatedUp => {
            state.update(message);
            match &state.selector {
                Some(selector) if selector.status == SelectorStatus::Loading => {
                    scan_task(selector.current_dir.clone())
                }
                _ => Task::none(),
            }
        }
        // Open the chosen folder as a project (records git status/availability), then
        // persist the updated catalog.
        Message::FolderChosen(path) => {
            let scanner = StdFolderScanner::new();
            state.workspace.open_or_activate(path, &scanner);
            state.selector = None;
            state.overlay = Overlay::None;
            persist(state);
            Task::none()
        }
        // Reopen a known project without browsing: refresh availability, activate if
        // available (FR-023), and persist the new active pointer.
        Message::KnownProjectReopened(path) => {
            state
                .workspace
                .refresh_availability(&StdFolderScanner::new());
            if state.workspace.activate(&path) {
                persist(state);
            }
            Task::none()
        }
        // Confirm a rename via the pure reducer, then persist the updated name.
        Message::RenameConfirmed => {
            state.update(Message::RenameConfirmed);
            persist(state);
            Task::none()
        }
        // A theme preference change is a pure transition, then we persist it (FR-009).
        Message::ThemePreferenceChanged(pref) => {
            state.update(Message::ThemePreferenceChanged(pref));
            persist_settings(state);
            Task::none()
        }
        // Everything else (including SystemThemeChanged) is a pure reducer transition.
        other => {
            state.update(other);
            Task::none()
        }
    }
}

/// Render the current state.
fn view(state: &State) -> iced::Element<'_, Message> {
    ui::view(state)
}

/// Select the iced theme for the resolved color scheme (FR-004, FR-005).
fn theme(state: &State) -> iced::Theme {
    ui::style::theme(state.color_scheme())
}

/// Event subscriptions: keyboard handling for the open overlay, plus the OS theme poll.
fn subscription(state: &State) -> Subscription<Message> {
    Subscription::batch([ui::subscription(state), os_theme_poll()])
}

/// Poll the OS light/dark preference (FR-006). iced 0.13 has no theme-changed event, so we
/// poll and map to [`Message::SystemThemeChanged`]; the pure reducer applies it and the next
/// frame's `theme()` reflects it. A fixed Light/Dark preference makes the resolver ignore
/// the OS scheme, so overrides are unaffected.
fn os_theme_poll() -> Subscription<Message> {
    every(OS_THEME_POLL).map(|_instant| Message::SystemThemeChanged(detect_system_scheme()))
}

/// Detect the OS light/dark preference, mapping `dark_light::Mode` to [`SystemScheme`].
/// `Mode::Default` (undetectable — e.g. a Linux session without an appearance portal) maps
/// to `Unspecified`, which the resolver falls back to light (FR-018).
fn detect_system_scheme() -> SystemScheme {
    match dark_light::detect() {
        dark_light::Mode::Dark => SystemScheme::Dark,
        dark_light::Mode::Light => SystemScheme::Light,
        dark_light::Mode::Default => SystemScheme::Unspecified,
    }
}

/// The directory the selector opens at: the user's home directory, falling back to a
/// filesystem root if it cannot be determined. Cross-platform via `directories`.
fn start_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

/// Scan a directory off the render path and deliver the result as a [`Message`].
fn scan_task(dir: PathBuf) -> Task<Message> {
    Task::perform(async move { scan(dir) }, |message| message)
}

/// Perform the (blocking) directory scan, mapping the result to a listing message.
fn scan(dir: PathBuf) -> Message {
    match StdFolderScanner::new().list_subdirs(&dir) {
        Ok(entries) => Message::SelectorListingReady(entries),
        Err(error) => Message::SelectorListingFailed(error.to_string()),
    }
}
