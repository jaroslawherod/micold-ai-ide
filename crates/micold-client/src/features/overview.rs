//! The worktree Overview (feature 813): a main-pane grid with one card per sidebar location,
//! each showing its type, branch, session counts and card state (research R1, R4).
//!
//! Only the open flag lives here. The cards compose the sidebar, sessions, agent requests and
//! pull request readings, so they are built in [`crate::ui::overview::cards`], not by this module
//! (a feature module names no other feature).

/// What the Overview remembers: whether it is on screen. The cards are derived per render.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct State {
    /// The Overview replaces the main area.
    pub open: bool,
}

/// What happens to the Overview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    /// The shortcut or the top-bar button. Does nothing while Settings is open, which cannot yield
    /// to another view without losing or saving its draft (research R2).
    Toggled {
        /// Whether Settings is open.
        settings_open: bool,
    },
    /// Leave the Overview.
    Closed,
}

/// Apply a message to the Overview's state.
/// It never asks the root for anything, so the outcomes are always empty.
pub fn update(state: &mut State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::Toggled {
            settings_open: true,
        } => {}
        Msg::Toggled {
            settings_open: false,
        } => state.open = !state.open,
        Msg::Closed => state.open = false,
    }
    Vec::new()
}
