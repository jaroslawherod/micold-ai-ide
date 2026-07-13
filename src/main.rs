//! Micold AI IDE — GUI binary entry point.
//!
//! This binary adapts the render-free core (`micold_ai_ide::app`) to the iced runtime.
//! All state transitions live in the core and are unit-tested there; this layer renders
//! state, translates input into core [`Message`]s, and performs the feature's I/O at the
//! boundary: resolving the starting directory, scanning directories off the render path
//! (research R6), detecting git status, and (User Story 2) persisting the catalog.

mod ui;

use iced::Task;
use micold_ai_ide::app::{Message, Overlay, State};
use micold_ai_ide::fs_scan::{FolderScanner, StdFolderScanner};
use micold_ai_ide::selector::{Selector, SelectorStatus};
use micold_ai_ide::store::{JsonFileStore, ProjectStore};
use std::path::PathBuf;

pub fn main() -> iced::Result {
    iced::application("Micold AI IDE", update, view)
        .subscription(subscription)
        .run_with(boot)
}

/// Build the initial state: load the known-projects catalog from local storage and
/// recompute each project's availability against the filesystem (FR-008, FR-022). A
/// missing or corrupt store yields an empty catalog (research R8).
fn boot() -> (State, Task<Message>) {
    let mut state = State::default();
    if let Some(store) = JsonFileStore::default_location() {
        state.workspace = store.load().workspace;
        state
            .workspace
            .refresh_availability(&StdFolderScanner::new());
    }
    (state, Task::none())
}

/// Persist the current catalog to local storage. Local-first: a save failure is
/// non-fatal and never crashes the app (Constitution Principle IV).
fn persist(state: &State) {
    if let Some(store) = JsonFileStore::default_location() {
        let _ = store.save(&state.workspace);
    }
}

/// Apply a message. Pure UI transitions go through the core reducer; messages that need
/// filesystem access or spawn work are handled here and may return a [`Task`].
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
        // Confirm a rename via the pure reducer, then persist the updated name (FR-019).
        Message::RenameConfirmed => {
            state.update(Message::RenameConfirmed);
            persist(state);
            Task::none()
        }
        // Everything else is a pure reducer transition.
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

/// Event subscriptions (keyboard handling for the open overlay).
fn subscription(state: &State) -> iced::Subscription<Message> {
    ui::subscription(state)
}

/// The directory the selector opens at: the user's home directory, falling back to a
/// filesystem root if it cannot be determined. Cross-platform via `directories`.
fn start_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

/// Scan a directory off the render path and deliver the result as a [`Message`]
/// (research R6).
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
