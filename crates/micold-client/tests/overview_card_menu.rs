//! Feature 972 (M1): the Overview card menu's state, its entries, and when it closes.
//!
//! The reducer half (T004) drives `features::overview::update` and `app::State::update`; the
//! composition half (T005) checks that the card menu is *Open* followed by exactly the location's
//! row-menu entries (FR-002, SC-002), and that `settle_card_menu` closes a menu whose Overview or
//! target went away (FR-012).

use micold_client::app::{Message, State};
use micold_client::features::overview::{
    update, CardMenu, MenuOrigin, Msg, OverviewCardMenu, State as OverviewState,
};
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::sidebar::Msg as SidebarMsg;
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_client::features::Outcome;
use micold_client::icons::Icon;
use micold_client::ui::location_menu_items;
use micold_client::ui::overview::{card_menu_items, cards};
use micold_core::project::{Availability, Project};
use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
use micold_core::session::{AiCli, Session, SessionLifecycle, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

const ANCHOR: (u16, u16) = (120, 340);
const OTHER_ANCHOR: (u16, u16) = (400, 80);

fn project_path() -> PathBuf {
    PathBuf::from("/repo")
}

fn worktree(dir: &str, status: WorktreeStatus, included: bool) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("feat/{dir}")),
        status,
        included,
    }
}

fn project_with(worktrees: Vec<Worktree>) -> State {
    let mut state = State::default();
    let path = project_path();
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state.worktree.worktrees = worktrees;
    state
}

fn add_session(state: &mut State, location: SessionLocation) -> Session {
    let mut session = Session::start_new(location, AiCli::ClaudeCode);
    session.lifecycle = SessionLifecycle::Idle;
    state
        .workspace
        .sessions
        .entry(project_path())
        .or_default()
        .push(session.clone());
    session
}

fn wt(dir: &str) -> SessionLocation {
    SessionLocation::Worktree(dir.to_string())
}

fn opened(location: SessionLocation, anchor: (u16, u16), origin: MenuOrigin) -> Msg {
    Msg::CardMenuOpened {
        location,
        anchor,
        origin,
    }
}

/// A project with one plain worktree, the Overview open and `feat-a`'s card menu open.
fn menu_open_on_feat_a(origin: MenuOrigin) -> State {
    let mut state = project_with(vec![worktree("feat-a", WorktreeStatus::Valid, false)]);
    state.overview.open = true;
    state.update(Message::Overview(opened(wt("feat-a"), ANCHOR, origin)));
    assert!(state.overview.card_menu.is_some(), "fixture: menu open");
    state
}

/// The id of the open dialog, if any.
fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

/// Nothing happens, but it is an outermost update: the epilogue runs.
fn tick(state: &mut State) {
    state.update(Message::Overview(Msg::TitlesRequested(Vec::new())));
}

// ---- T004: the reducer ----

#[test]
fn a_pointer_open_holds_target_anchor_and_origin_with_nothing_highlighted() {
    let mut overview = OverviewState {
        open: true,
        ..OverviewState::default()
    };
    let outcomes = update(
        &mut overview,
        opened(wt("feat-a"), ANCHOR, MenuOrigin::Pointer),
    );
    assert_eq!(
        overview.card_menu,
        Some(CardMenu {
            location: wt("feat-a"),
            anchor: ANCHOR,
            origin: MenuOrigin::Pointer,
            highlighted: None,
        }),
        "a right-click opens the menu at the press point, nothing highlighted until Up/Down"
    );
    assert_eq!(
        outcomes,
        vec![Outcome::SurfaceOpened(OverviewCardMenu::ID)],
        "opening reports the surface so the registry can displace the row menus (FR-014)"
    );
}

#[test]
fn a_keyboard_open_highlights_the_first_entry() {
    let mut overview = OverviewState::default();
    update(
        &mut overview,
        opened(wt("feat-a"), ANCHOR, MenuOrigin::Keyboard),
    );
    assert_eq!(
        overview.card_menu.map(|menu| menu.highlighted),
        Some(Some(0)),
        "a keyboard-opened menu starts on its first entry (FR-009)"
    );
}

#[test]
fn opening_on_another_card_replaces_the_open_menu() {
    let mut overview = OverviewState::default();
    update(
        &mut overview,
        opened(wt("feat-a"), ANCHOR, MenuOrigin::Pointer),
    );
    update(
        &mut overview,
        opened(SessionLocation::Default, OTHER_ANCHOR, MenuOrigin::Pointer),
    );
    let menu = overview.card_menu.expect("B's menu is open");
    assert_eq!(
        (menu.location, menu.anchor),
        (SessionLocation::Default, OTHER_ANCHOR),
        "a right-click on card B closes A's menu and opens B's at the new point (US2.4)"
    );
}

#[test]
fn dismissing_closes_and_arms_focus_return_only_for_a_keyboard_menu() {
    for (origin, expected) in [
        (MenuOrigin::Pointer, None),
        (MenuOrigin::Keyboard, Some(wt("feat-a"))),
    ] {
        let mut overview = OverviewState::default();
        update(&mut overview, opened(wt("feat-a"), ANCHOR, origin));
        let outcomes = update(&mut overview, Msg::CardMenuDismissed);
        assert_eq!(overview.card_menu, None, "{origin:?}: dismissal closes it");
        assert!(outcomes.is_empty(), "{origin:?}: dismissal runs nothing");
        assert_eq!(
            overview.focus_return, expected,
            "{origin:?}: only a keyboard-opened menu hands focus back to its card (FR-011)"
        );
    }
}

#[test]
fn choosing_an_entry_closes_and_arms_focus_return_only_for_a_keyboard_menu() {
    for (origin, expected) in [
        (MenuOrigin::Pointer, None),
        (MenuOrigin::Keyboard, Some(wt("feat-a"))),
    ] {
        let mut overview = OverviewState::default();
        update(&mut overview, opened(wt("feat-a"), ANCHOR, origin));
        update(
            &mut overview,
            Msg::CardMenuEntryChosen(Box::new(Message::Worktree(WorktreeMsg::RenameStarted(
                "feat-a".into(),
            )))),
        );
        assert_eq!(overview.card_menu, None, "{origin:?}: choosing closes it");
        assert_eq!(overview.focus_return, expected, "{origin:?}");
    }
}

#[test]
fn a_grid_scroll_closes_the_menu() {
    let mut state = menu_open_on_feat_a(MenuOrigin::Pointer);
    state.update(Message::Overview(Msg::GridScrolled));
    assert_eq!(
        state.overview.card_menu, None,
        "the grid scrolling beneath the menu closes it (FR-012)"
    );
    assert!(state.overview.open, "and leaves the Overview open");
}

#[test]
fn an_in_place_entry_keeps_the_overview_open_and_opens_its_dialog() {
    for (inner, dialog) in [
        (
            Message::Worktree(WorktreeMsg::RenameStarted("feat-a".into())),
            "rename_worktree",
        ),
        (
            Message::Worktree(WorktreeMsg::DeleteRequested("feat-a".into())),
            "confirm_worktree_delete",
        ),
    ] {
        let mut state = menu_open_on_feat_a(MenuOrigin::Pointer);
        state.update(Message::Overview(Msg::CardMenuEntryChosen(Box::new(
            inner.clone(),
        ))));
        // `main.rs` sends the inner message next, through the row menu's own path (R4).
        state.update(inner);
        assert_eq!(state.overview.card_menu, None);
        assert!(
            state.overview.open,
            "{dialog}: an in-place entry leaves the Overview open (FR-005)"
        );
        assert_eq!(
            open_dialog(&state),
            Some(dialog),
            "its dialog opens over it"
        );
    }
}

#[test]
fn a_navigating_entry_closes_the_overview() {
    let mut state = menu_open_on_feat_a(MenuOrigin::Pointer);
    let review = Message::Sidebar(SidebarMsg::ReviewChangesRequested(wt("feat-a")));
    state.update(Message::Overview(Msg::CardMenuEntryChosen(Box::new(
        review.clone(),
    ))));
    state.update(review);
    assert!(!state.overview.open, "Review changes takes the main area");

    let mut state = menu_open_on_feat_a(MenuOrigin::Pointer);
    let session = add_session(&mut state, wt("feat-a"));
    let select = Message::Session(SessionMsg::Selected(session.id));
    state.update(Message::Overview(Msg::CardMenuEntryChosen(Box::new(
        select.clone(),
    ))));
    state.update(select);
    assert!(
        !state.overview.open,
        "selecting a session takes the main area"
    );
}

#[test]
fn open_as_the_inner_message_closes_the_overview_as_a_primary_click_does() {
    let mut state = menu_open_on_feat_a(MenuOrigin::Pointer);
    let activation = cards(&state)
        .into_iter()
        .find(|card| card.location == wt("feat-a"))
        .expect("feat-a's card")
        .activation;
    let open = Message::Overview(Msg::CardActivated(activation));
    state.update(Message::Overview(Msg::CardMenuEntryChosen(Box::new(
        open.clone(),
    ))));
    state.update(open);
    assert!(
        !state.overview.open,
        "Open is the card's activation (FR-003)"
    );
    assert_eq!(state.overview.card_menu, None);
}

// ---- T005: composition ----

/// Every location type the row menus distinguish, each in a state of its own.
fn every_location_type() -> (State, Vec<SessionLocation>) {
    let mut state = project_with(vec![
        worktree("plain", WorktreeStatus::Valid, false),
        worktree("included", WorktreeStatus::Valid, true),
        worktree("with-pr", WorktreeStatus::Valid, false),
        worktree("with-session", WorktreeStatus::Valid, false),
        worktree("orphan", WorktreeStatus::Missing, false),
        worktree("invalid", WorktreeStatus::Invalid, false),
    ]);
    state.pr_status.enabled = true;
    state.pr_status.read_at = Some(100);
    state.pr_status.statuses.insert(
        "feat/with-pr".to_string(),
        PullRequestStatus {
            number: 7,
            title: "t".to_string(),
            url: "https://github.com/o/r/pull/7".to_string(),
            state: PrState::Open {
                checks: CheckStatus::Passing,
            },
            review: ReviewState::None,
            head: "abc".to_string(),
            conflict: false,
        },
    );
    add_session(&mut state, wt("with-session"));
    add_session(&mut state, SessionLocation::Default);
    let locations = vec![
        SessionLocation::Default,
        wt("plain"),
        wt("included"),
        wt("with-pr"),
        wt("with-session"),
        wt("orphan"),
        wt("invalid"),
    ];
    (state, locations)
}

macro_rules! labels_and_icons {
    ($items:expr) => {
        $items
            .iter()
            .map(|item| (item.label.clone(), item.icon))
            .collect::<Vec<(String, Option<Icon>)>>()
    };
}

#[test]
fn the_card_menu_is_open_then_exactly_the_row_menu() {
    let (state, locations) = every_location_type();
    let mut seen = std::collections::BTreeSet::new();
    for location in locations {
        let card = card_menu_items(&state, &location);
        let row = location_menu_items(&state, &location);
        assert_eq!(
            card.first().map(|item| (item.label.as_str(), item.icon)),
            Some(("Open", Some(Icon::Open))),
            "{location:?}: Open comes first"
        );
        assert_eq!(
            labels_and_icons!(card[1..]),
            labels_and_icons!(row),
            "{location:?}: after Open, the row menu's entries in its order (FR-002, SC-002)"
        );
        seen.extend(row.iter().map(|item| item.label.clone()));
    }
    // The fixture really reaches the conditional entries, so the equality above is not vacuous.
    for label in ["Stop showing", "Open pull request", "Rename", "Delete"] {
        assert!(
            seen.contains(label),
            "no location offered `{label}`: {seen:?}"
        );
    }
}

#[test]
fn every_entry_is_wrapped_and_open_is_the_cards_activation() {
    let (state, locations) = every_location_type();
    let models = cards(&state);
    for location in locations {
        let card = card_menu_items(&state, &location);
        let row = location_menu_items(&state, &location);
        let activation = models
            .iter()
            .find(|model| model.location == location)
            .expect("a card per location")
            .activation
            .clone();
        assert_eq!(
            card[0].message,
            Some(Message::Overview(Msg::CardMenuEntryChosen(Box::new(
                Message::Overview(Msg::CardActivated(activation))
            )))),
            "{location:?}: Open runs the card's own activation (FR-003)"
        );
        for (wrapped, plain) in card[1..].iter().zip(&row) {
            assert_eq!(
                wrapped.message,
                plain
                    .message
                    .clone()
                    .map(|m| Message::Overview(Msg::CardMenuEntryChosen(Box::new(m)))),
                "{location:?} `{}`: the row menu's own message, wrapped (FR-004)",
                plain.label
            );
        }
    }
}

#[test]
fn the_menu_closes_when_the_overview_closes() {
    let closers: Vec<(&str, Box<dyn Fn(&mut State)>)> = vec![
        (
            "Closed",
            Box::new(|state: &mut State| state.update(Message::Overview(Msg::Closed))),
        ),
        (
            "session selected",
            Box::new(|state: &mut State| {
                let session = add_session(state, wt("feat-a"));
                state.update(Message::Session(SessionMsg::Selected(session.id)));
            }),
        ),
        (
            "Review changes",
            Box::new(|state: &mut State| {
                state.update(Message::Sidebar(SidebarMsg::ReviewChangesRequested(wt(
                    "feat-a",
                ))));
            }),
        ),
        (
            "project switch",
            Box::new(|state: &mut State| {
                state.update(Message::Overview(Msg::ProjectSwitched));
            }),
        ),
    ];
    for (name, close) in closers {
        let mut state = menu_open_on_feat_a(MenuOrigin::Keyboard);
        close(&mut state);
        assert_eq!(
            state.overview.card_menu, None,
            "{name}: the menu closes (FR-012)"
        );
        assert_eq!(
            state.overview.focus_return, None,
            "{name}: nothing to return focus to"
        );
    }
}

#[test]
fn the_menu_closes_when_its_target_stops_being_listed() {
    let mut state = menu_open_on_feat_a(MenuOrigin::Pointer);
    state.worktree.worktrees.clear();
    tick(&mut state);
    assert_eq!(
        state.overview.card_menu, None,
        "a target whose card went away closes its menu (Edge: target disappears)"
    );
    assert!(state.overview.open);
}

#[test]
fn the_menu_stays_when_its_target_changes_state_or_the_cards_reorder() {
    let mut state = menu_open_on_feat_a(MenuOrigin::Pointer);
    state
        .worktree
        .worktrees
        .insert(0, worktree("feat-b", WorktreeStatus::Valid, false));
    let mut session = add_session(&mut state, wt("feat-a"));
    session.lifecycle = SessionLifecycle::Running;
    state
        .workspace
        .sessions
        .get_mut(&project_path())
        .expect("sessions")
        .retain(|s| s.id != session.id);
    state
        .workspace
        .sessions
        .get_mut(&project_path())
        .expect("sessions")
        .push(session);
    tick(&mut state);
    assert_eq!(
        state.overview.card_menu.map(|menu| menu.location),
        Some(wt("feat-a")),
        "the menu follows its location, not a grid index (Edge: target changes, cards reorder)"
    );
}
