//! Keyboard navigation (feature 803): what one navigation press resolves to.
//!
//! Render-free and read-only: [`update`] reads the root state and returns a [`Step`]; the root
//! applies it (`State::apply_navigation`, contracts/ui.md U3). The orders are derived at each
//! press and never stored (data-model.md *Orders*).

use std::path::PathBuf;

use micold_core::session::{Session, SessionId, SessionLocation};

use crate::app::State;
use crate::features::sidebar::{RunRow, SidebarEntry};
use crate::keymap::{NavAction, NavLevel};

/// The navigation feature's vocabulary at the root (feature 028's wrapper rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    /// A navigation chord was pressed, whether or not a terminal is focused (contracts/keys.md K3).
    Pressed(NavAction),
}

/// What a navigation press lands on (contracts/ui.md U2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing changes: a surface is open, the list is empty, or it holds only the current entry.
    None,
    /// Show this session, as a click on its row does.
    Session(SessionId),
    /// A location with no session to show: it becomes a navigation stop.
    Stop(SessionLocation),
    /// Switch to this project, as the project switcher does.
    Project(PathBuf),
}

/// The active project's locations in the order the sidebar draws them (contracts/ui.md U2):
/// Default, then each worktree row as drawn. Worktrees the tag filter hides and those of a
/// collapsed run group are not drawn, so they are not in the order.
pub fn location_order(state: &State) -> Vec<SessionLocation> {
    let mut order = Vec::new();
    for entry in state.sidebar_entries() {
        match entry {
            SidebarEntry::Default(_) => order.push(SessionLocation::Default),
            SidebarEntry::Worktree(node) => order.push(node.location()),
            SidebarEntry::Group(group) if group.expanded => {
                order.extend(group.runs.iter().filter_map(|run| match &run.row {
                    RunRow::Worktree(node) => Some(node.location()),
                    RunRow::NoWorktree => None,
                }));
            }
            SidebarEntry::Group(_) => {}
        }
    }
    order
}

/// What `action` lands on in `state` (contracts/ui.md U1, U2).
///
/// Nothing while the settings panel or any overlay surface is open (U1). At the location level
/// the current location is the navigation stop when it is still drawn, else the current
/// session's; the target's main session is shown, or the target becomes a stop when it has none.
pub fn update(state: &State, action: NavAction) -> Step {
    if state.settings.settings_draft.is_some() || crate::overlay::registry::topmost(state).is_some()
    {
        return Step::None;
    }
    match action.level {
        NavLevel::Location => location_step(state, action.dir),
        NavLevel::Session => session_step(state, action.dir),
        NavLevel::Project => project_step(state, action.dir),
    }
}

fn location_step(state: &State, dir: micold_core::navigation::Dir) -> Step {
    let order = location_order(state);
    let current = state
        .sidebar
        .nav_stop
        .clone()
        .filter(|stop| order.contains(stop))
        .or_else(|| state.current_session_location());
    let Some(target) = micold_core::navigation::step(&order, current.as_ref(), dir) else {
        return Step::None;
    };
    if current.as_ref() == Some(&target) {
        return Step::None;
    }
    match main_session_at(state, &target) {
        Some(id) => Step::Session(id),
        None => Step::Stop(target),
    }
}

/// The available known projects in switcher order, stepping from the active one; from the welcome
/// screen (no active project) `Next` gives the first and `Previous` the last. The only available
/// project is the active one, so it changes nothing.
fn project_step(state: &State, dir: micold_core::navigation::Dir) -> Step {
    let entries = state.switcher_entries();
    let order: Vec<PathBuf> = entries
        .iter()
        .filter(|e| e.available || e.is_active)
        .map(|e| e.path.clone())
        .collect();
    let current = state.workspace.active.as_ref();
    match micold_core::navigation::step(&order, current, dir) {
        Some(target) if Some(&target) != current => Step::Project(target),
        _ => Step::None,
    }
}

/// The current location's sessions, main first then the helpers, as its expander lists them.
/// Nothing at a navigation stop or without a current session; one session stays where it is.
fn session_step(state: &State, dir: micold_core::navigation::Dir) -> Step {
    let Some(current) = state.session.active else {
        return Step::None;
    };
    if state.sidebar.nav_stop.is_some() {
        return Step::None;
    }
    let Some(location) = state.current_session_location() else {
        return Step::None;
    };
    let ids = |sessions: Vec<&Session>| sessions.iter().map(|s| s.id).collect::<Vec<_>>();
    let order: Vec<SessionId> = state
        .sidebar_entries()
        .into_iter()
        .find_map(|entry| match entry {
            SidebarEntry::Default(node) if location == SessionLocation::Default => {
                Some(ids(node.unfolded_sessions()))
            }
            SidebarEntry::Worktree(node) if node.location() == location => {
                Some(ids(node.unfolded_sessions()))
            }
            SidebarEntry::Group(group) => group.runs.iter().find_map(|run| match &run.row {
                RunRow::Worktree(node) if node.location() == location => {
                    Some(ids(node.unfolded_sessions()))
                }
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or_default();
    match micold_core::navigation::step(&order, Some(&current), dir) {
        Some(target) if target != current => Step::Session(target),
        _ => Step::None,
    }
}

/// The session a click on `location`'s row would select (FR-018), if it has one.
fn main_session_at(state: &State, location: &SessionLocation) -> Option<SessionId> {
    state
        .sidebar_entries()
        .into_iter()
        .find_map(|entry| match entry {
            SidebarEntry::Default(node) if *location == SessionLocation::Default => {
                node.main_session()
            }
            SidebarEntry::Worktree(node) if node.location() == *location => node.main_session(),
            SidebarEntry::Group(group) => group.runs.iter().find_map(|run| match &run.row {
                RunRow::Worktree(node) if node.location() == *location => node.main_session(),
                _ => None,
            }),
            _ => None,
        })
}
