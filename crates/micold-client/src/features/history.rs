//! Back/Forward navigation (feature 1465): the window's one history of places, what a press
//! resolves to, and whether a place still exists.
//!
//! Render-free. The list rules are core's ([`micold_core::history::History`]); this module feeds
//! them the shown place and the existence check (research R1–R5). The root drives it
//! (`State::press_history`, `State::observe_place`, `State::finish_history_move`) and applies a
//! [`Step`] (`State::apply_history`); the shell entry is `shell/history.rs` (shape B).

use std::path::Path;

use micold_core::history::{Dir, History, Place, View};
use micold_core::session::SessionLocation;

/// The worktrees of a project that is not active, by `dir_name` (research R5): the shell reads
/// them from disk as a project switch does, tests pass a map. `None`: unreadable, so that
/// project's worktree places count as gone.
pub type Lookup<'a> = &'a dyn Fn(&Path) -> Option<Vec<String>>;

/// The window's history (FR-001), on the root as `history`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The places the user has been and the position in them.
    pub list: History<Place>,
    /// A press is moving to its target: observation is off until the move lands (research R2).
    pub moving: bool,
}

/// Where a press came from: a side button acts over a focused field or terminal, a key does not
/// (FR-008, FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The mouse's Back or Forward side button.
    Button,
    /// Alt+←/→ (Cmd+[ / Cmd+] on macOS).
    Key,
}

/// The history's vocabulary at the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    /// Back or Forward was pressed.
    Pressed { dir: Dir, source: Source },
}

/// What one press resolves to (contracts/ui.md U3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing changes: blocked, or nothing to go to.
    None,
    /// The shown Review tab's own Back (FR-011).
    ReviewBack,
    /// Show this place; the list's position already moved to it and `moving` is set.
    Show(Place),
}

/// What `msg` resolves to in `app` (contracts/ui.md U3). A move sets `moving`.
pub fn resolve(app: &mut crate::app::State, msg: Msg, lookup: Lookup) -> Step {
    let Msg::Pressed { dir, source } = msg;
    if crate::features::navigation::blocked(app) {
        return Step::None;
    }
    // FR-009: a focused terminal or text field keeps the keys (a side button still acts, FR-008).
    if source == Source::Key && (app.terminal_focused() || app.live_focused_field().is_some()) {
        return Step::None;
    }
    let shown = app.shown_place();
    // FR-011: a Review tab with a browse frame goes back inside itself first; the list is untouched.
    if dir == Dir::Back
        && matches!(shown.as_ref().map(|p| &p.view), Some(View::Review(_)))
        && app
            .changes
            .tabs
            .shown_view()
            .is_some_and(crate::features::changes::can_go_back)
    {
        return Step::ReviewBack;
    }
    let mut list = std::mem::take(&mut app.history.list);
    let found = list.step(dir, shown.as_ref(), |p| exists(app, p, lookup));
    app.history.list = list;
    match found {
        Some(place) => {
            app.history.moving = true;
            Step::Show(place)
        }
        None => Step::None,
    }
}

/// Whether `place` can still be shown (research R5), in any project: the project known and
/// available (or the active one); its location listed (`worktree.worktrees` for the active project,
/// `lookup` for another); a session not archived there; a Compare group in that project's last
/// group list (`runs.groups`, or `runs.known` for another project).
pub fn exists(app: &crate::app::State, place: &Place, lookup: Lookup) -> bool {
    let project = &place.project;
    let active = app.workspace.active.as_ref() == Some(project);
    let available = active
        || app.workspace.projects.iter().any(|p| {
            &p.path == project && p.availability == micold_core::project::Availability::Available
        });
    if !available {
        return false;
    }
    let location_exists = |location: &SessionLocation| match location {
        SessionLocation::Default => true,
        SessionLocation::Worktree(dir) if active => {
            app.worktree.worktrees.iter().any(|w| &w.dir_name == dir)
        }
        SessionLocation::Worktree(dir) => lookup(project).is_some_and(|dirs| dirs.contains(dir)),
    };
    match &place.view {
        View::Session { location, id } => {
            location_exists(location)
                && app.workspace.sessions.get(project).is_some_and(|sessions| {
                    sessions
                        .iter()
                        .any(|s| s.id == *id && !s.archived && &s.location == location)
                })
        }
        View::Review(location) => location_exists(location),
        View::Overview => true,
        View::Compare(group) => {
            let groups = if active {
                Some(&app.runs.groups)
            } else {
                app.runs.known.get(project)
            };
            groups.is_some_and(|groups| groups.iter().any(|g| g.id == *group))
        }
    }
}

/// Records the shown place after a message (FR-002, FR-003): nothing while a press is moving.
pub fn observe(state: &mut State, shown: Option<Place>) {
    if state.moving {
        return;
    }
    if let Some(place) = shown {
        state.list.record(place);
    }
}

/// A move's follow-ups have run (research R2): observation is back on, and a landing elsewhere
/// than the target is recorded like any move.
pub fn finish(state: &mut State, shown: Option<Place>) {
    state.moving = false;
    observe(state, shown);
}
