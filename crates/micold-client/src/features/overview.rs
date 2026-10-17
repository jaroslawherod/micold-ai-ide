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

/// Which colour family the badge is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Some card needs action: the `error` family.
    Error,
    /// Some card needs attention: the `warning` family.
    Warning,
}

/// What the Overview button shows beside its label, whether or not the Overview is open
/// (FR-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badge {
    /// The colour family.
    pub tone: Tone,
    /// How many cards are in the worst state.
    pub count: usize,
    /// The words for the tooltip: *N need action* or *N need attention*.
    pub label: String,
}

/// The badge over the cards, or `None` when no card needs the user.
pub fn badge(cards: &[micold_core::overview::CardModel]) -> Option<Badge> {
    use micold_core::overview::{attention, CardState};
    let attention = attention(cards)?;
    let (tone, words) = match attention.state {
        CardState::NeedsAction => (Tone::Error, "need action"),
        _ => (Tone::Warning, "need attention"),
    };
    Some(Badge {
        tone,
        count: attention.count,
        label: format!("{} {words}", attention.count),
    })
}
