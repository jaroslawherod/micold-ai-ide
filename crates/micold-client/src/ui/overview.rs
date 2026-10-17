//! The Overview's main-pane view (feature 813, contract U1) and the cards it draws.
//!
//! The cards are derived from several features' state at once, which is composition, not a
//! feature's own business; so it lives here beside the view rather than in `features::overview`.

use crate::app::{Message, State};
use crate::features::sidebar::{RunRow, SidebarEntry, WorktreeNode};
use crate::ui::material::{CardGrid, OverviewCard};
use iced::widget::container;
use iced::{Element, Length};
use micold_core::location_state::location_state;
use micold_core::overview::{
    card_state, location_type, order, session_counts, CardModel, CardSignals, CurrentPr,
    LocationType, PrSignal,
};
use micold_core::session::{Session, SessionLocation};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The Overview: a grid with one card per location of the active project.
pub fn view<'a>(state: &State, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let cards = cards(state)
        .iter()
        .map(|model| OverviewCard::new(model, r))
        .collect();
    // The grid scrolls itself; an outer scrollable would give it unbounded height.
    container(CardGrid::new(cards, r))
        .padding(spacing::LG)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The cards for the active project: one per sidebar entry (a group contributes its runs'
/// worktrees), worst state first and in the sidebar's order within a state (FR-001, FR-007).
/// Derived from what the window holds on every call; nothing is cached (research R14).
pub fn cards(app: &State) -> Vec<CardModel> {
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

fn card(app: &State, place: &Place) -> CardModel {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::laid_out;
    use iced::Size;

    /// U1: the Overview outranks Changes and Compare in the crossfade key, and Settings outranks it.
    #[test]
    fn the_overview_key_sits_between_settings_and_changes() {
        let mut state = State::default();
        state.overview.open = true;
        assert_eq!(crate::ui::main_content_key(&state), u64::MAX - 3);
        state.settings.settings_draft = Some(Default::default());
        assert_eq!(
            crate::ui::main_content_key(&state),
            u64::MAX,
            "Settings first"
        );
        state.settings.settings_draft = None;
        state.overview.open = false;
        assert_ne!(crate::ui::main_content_key(&state), u64::MAX - 3);
    }

    /// F1: thirty cards in a short window lay out inside it; the grid scrolls, not the page.
    #[test]
    fn thirty_cards_lay_out_bounded_in_a_short_window() {
        let r = crate::ui::material::test_support::roles();
        let models: Vec<CardModel> = (0..30)
            .map(|i| CardModel {
                location: SessionLocation::Worktree(format!("w{i}")),
                display_name: format!("feat-{i}"),
                branch: None,
                kind: LocationType::Feature,
                sessions: (1, 0),
                state: micold_core::overview::CardState::Idle,
                reason: micold_core::overview::CardReason::Idle,
                summary: "s".into(),
            })
            .collect();
        let element: Element<'_, ()> = container(CardGrid::new(
            models.iter().map(|m| OverviewCard::new(m, r)).collect(),
            r,
        ))
        .padding(spacing::LG)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
        let room = Size::new(900.0, 300.0);
        // `bounds()`, not the size accessor: the material boundary scan reads any `.size(` call
        // as a raw text size.
        let laid = laid_out(element, room).bounds();
        assert!(
            laid.height <= room.height + 0.01,
            "grew to {laid:?} in {room:?}"
        );
    }
}
