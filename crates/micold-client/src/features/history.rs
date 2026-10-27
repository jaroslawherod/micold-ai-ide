//! Back/Forward navigation (feature 1465): the window's one history of places, what a press
//! resolves to, and whether a place still exists.
//!
//! Render-free. The list rules are core's ([`micold_core::history::History`]); this module feeds
//! them the shown place and the existence check (research R1–R5). The root drives it
//! (`State::press_history`, `State::observe_place`, `State::finish_history_move`) and applies a
//! [`Step`] (`State::apply_history`); the shell entry is `shell/history.rs` (shape B).

use micold_core::history::{Dir, History, Place, View};
use micold_core::session::SessionLocation;

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
pub fn resolve(app: &mut crate::app::State, msg: Msg) -> Step {
    let Msg::Pressed { dir, source: _ } = msg;
    if crate::features::navigation::blocked(app) {
        return Step::None;
    }
    let shown = app.shown_place();
    let mut list = std::mem::take(&mut app.history.list);
    let found = list.step(dir, shown.as_ref(), |p| exists(app, p));
    app.history.list = list;
    match found {
        Some(place) => {
            app.history.moving = true;
            Step::Show(place)
        }
        None => Step::None,
    }
}

/// Whether `place` can still be shown (research R5), for the active project. A place of another
/// project counts as gone (until cross-project moves, US4).
pub fn exists(app: &crate::app::State, place: &Place) -> bool {
    if app.workspace.active.as_ref() != Some(&place.project) {
        return false;
    }
    let location_exists = |location: &SessionLocation| match location {
        SessionLocation::Default => true,
        SessionLocation::Worktree(dir) => app.worktree.worktrees.iter().any(|w| &w.dir_name == dir),
    };
    match &place.view {
        View::Session { location, id } => {
            location_exists(location)
                && app
                    .workspace
                    .sessions
                    .get(&place.project)
                    .is_some_and(|sessions| {
                        sessions
                            .iter()
                            .any(|s| s.id == *id && !s.archived && &s.location == location)
                    })
        }
        View::Review(location) => location_exists(location),
        View::Overview => true,
        View::Compare(group) => app.runs.groups.iter().any(|g| g.id == *group),
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
