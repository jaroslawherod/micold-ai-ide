//! The worktree Overview (feature 813): a main-pane grid with one card per sidebar location,
//! each showing its type, branch, session counts and card state (research R1, R4).

use crate::features::sidebar::{RunRow, SidebarEntry, WorktreeNode};
use micold_core::location_state::location_state;
use micold_core::overview::{
    card_state, location_type, order, session_counts, CardModel, CardSignals, CurrentPr,
    LocationType, PrSignal,
};
use micold_core::session::{Session, SessionLocation};

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
pub fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Toggled {
            settings_open: true,
        } => {}
        Msg::Toggled {
            settings_open: false,
        } => state.open = !state.open,
        Msg::Closed => state.open = false,
    }
}

/// The cards for the active project: one per sidebar entry (a group contributes its runs'
/// worktrees), worst state first and in the sidebar's order within a state (FR-001, FR-007).
/// Derived from what the window holds on every call; nothing is cached (research R14).
pub fn cards(app: &crate::app::State) -> Vec<CardModel> {
    let mut cards: Vec<CardModel> = app
        .sidebar_entries()
        .iter()
        .flat_map(|entry| match entry {
            SidebarEntry::Default(_) => vec![card(app, &Place::Default)],
            SidebarEntry::Worktree(node) => vec![card(app, &Place::Worktree(node))],
            SidebarEntry::Group(group) => group
                .runs
                .iter()
                .filter_map(|run| match &run.row {
                    RunRow::Worktree(node) => Some(card(app, &Place::Worktree(node))),
                    RunRow::NoWorktree => None,
                })
                .collect(),
        })
        .collect();
    order(&mut cards);
    cards
}

/// What a card stands for.
enum Place<'a> {
    Default,
    Worktree(&'a WorktreeNode),
}

fn card(app: &crate::app::State, place: &Place) -> CardModel {
    let (location, display_name, branch, kind) = match place {
        Place::Default => (
            SessionLocation::Default,
            "Default".to_string(),
            None,
            LocationType::ProjectRoot,
        ),
        Place::Worktree(node) => {
            let location = SessionLocation::Worktree(node.worktree.dir_name.clone());
            let kind = location_type(&location, node.worktree.included, node.worktree.status);
            (
                location,
                node.display_name.clone(),
                node.worktree.branch.clone(),
                kind,
            )
        }
    };
    let sessions: Vec<&Session> = app
        .active_sessions()
        .iter()
        .filter(|s| s.location == location && !s.archived)
        .collect();
    let agent_request = app
        .agent_confirm
        .pending
        .iter()
        .any(|prompt| sessions.iter().any(|s| s.id == prompt.session));
    let signals = CardSignals {
        location: location_state(sessions.iter().copied()),
        agent_request,
        pr: pr_signal(&app.pr_status, branch.as_deref()),
    };
    let (state, reason) = card_state(&signals);
    let summary = match (&location, &branch) {
        (SessionLocation::Default, _) => "Project root".to_string(),
        (_, Some(branch)) => branch.clone(),
        (_, None) => display_name.clone(),
    };
    CardModel {
        location,
        display_name,
        branch,
        kind,
        sessions: session_counts(sessions.iter().copied()),
        state,
        reason,
        summary,
    }
}

/// The pull request reading for a branch: unknown while the reading is off or holds nothing (never
/// read, or released), and for a location with no branch, such as Default (research R6, R14).
fn pr_signal(pr: &crate::features::pr_status::State, branch: Option<&str>) -> PrSignal {
    match branch {
        Some(branch) if pr.enabled && pr.read_at.is_some() => PrSignal::Known {
            current: pr.statuses.get(branch).map(CurrentPr::from_status),
            removable: pr.removable.contains(branch),
        },
        _ => PrSignal::Unknown,
    }
}
