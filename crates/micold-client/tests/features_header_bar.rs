//! The top bar's model and the header menus' state, from outside the crate (feature 1793, T003 and
//! T007; FR-003, FR-004, FR-005, FR-011; Principle II): what shows for the selected worktree or
//! session, the order the bar draws it in, the meter's text, tooltip and accessible label, the
//! Idle/Working mapping, and at most one open menu that a selection change closes.

use crate::support::covered_states::{title_bar_crowded, transcript_pane_insights};
use micold_client::app::State;
use micold_client::features::header_bar::{
    HeaderBarModel, HeaderMenu, Msg, State as Menus, Status, CHANGES_LABEL_RANK, NO_USAGE_TEXT,
    NO_USAGE_TOOLTIP, RING_TEXT_RANK,
};
use micold_core::session::{AiCli, Session, SessionLifecycle, SessionLocation};

/// The first session's lifecycle set to `lifecycle`.
fn with_lifecycle(mut state: State, lifecycle: SessionLifecycle) -> State {
    for session in state.workspace.sessions.values_mut().flatten() {
        session.lifecycle = lifecycle.clone();
    }
    state
}

#[test]
fn nothing_shows_on_the_overview_or_with_nothing_selected() {
    let mut state = transcript_pane_insights();
    assert!(HeaderBarModel::of(&state).is_some(), "a session is shown");
    state.overview.open = true;
    assert_eq!(HeaderBarModel::of(&state), None, "the Overview shows none");
    assert_eq!(HeaderBarModel::of(&State::default()), None, "no selection");
}

#[test]
fn the_meter_carries_the_percent_cost_and_a_tooltip_with_tokens_cost_and_turns() {
    let model = HeaderBarModel::of(&transcript_pane_insights()).expect("shown");
    let meter = &model.meter;
    assert!(meter.ratio.is_some_and(|r| (0.0..=1.0).contains(&r)));
    assert!(
        meter.text.contains('%') && meter.text.contains('$') && meter.text.contains(" \u{b7} "),
        "percent, a dot, then cost: {:?}",
        meter.text
    );
    for wanted in ["Tokens: ", "Cost: $", "Turns: "] {
        assert!(
            meter.tooltip.contains(wanted),
            "{wanted:?} in {:?}",
            meter.tooltip
        );
    }
    assert_eq!(
        meter.accessible, meter.tooltip,
        "the accessible label keeps the cost when the text hides"
    );
    assert!(meter.warning, "the recording sits above its threshold");
}

#[test]
fn a_session_with_no_usage_shows_the_empty_state_and_invents_no_cost() {
    let state = title_bar_crowded();
    let model = HeaderBarModel::of(&state).expect("a session is shown");
    assert_eq!(model.meter.ratio, None);
    assert_eq!(model.meter.text, NO_USAGE_TEXT);
    assert_eq!(model.meter.tooltip, NO_USAGE_TOOLTIP);
    assert!(!model.meter.text.contains('$') && !model.meter.tooltip.contains('$'));
}

#[test]
fn the_changes_label_and_location_follow_the_shown_location() {
    let state = transcript_pane_insights();
    let model = HeaderBarModel::of(&state).expect("shown");
    assert_eq!(Some(model.location), state.shown_location());
    assert!(model.changes.starts_with("Changes"), "{:?}", model.changes);
}

#[test]
fn only_a_running_process_is_working() {
    for (lifecycle, expected) in [
        (SessionLifecycle::Running, Status::Working),
        (SessionLifecycle::Idle, Status::Idle),
        (SessionLifecycle::Starting, Status::Idle),
        (SessionLifecycle::Restarting { attempts: 1 }, Status::Idle),
        (
            SessionLifecycle::Failed {
                reason: "boom".into(),
                attempts: 3,
            },
            Status::Idle,
        ),
    ] {
        let state = with_lifecycle(transcript_pane_insights(), lifecycle.clone());
        assert_eq!(
            HeaderBarModel::of(&state).expect("shown").status,
            expected,
            "{lifecycle:?}"
        );
    }
    assert_eq!(Status::Idle.label(), "Idle");
    assert_eq!(Status::Working.label(), "Working");
}

#[test]
fn another_sessions_figures_never_show_for_the_shown_one() {
    let mut state = transcript_pane_insights();
    let location = state.shown_location().expect("shown");
    let other = Session::start_new(location, AiCli::ClaudeCode);
    let other_id = other.id;
    state
        .workspace
        .sessions
        .values_mut()
        .next()
        .expect("a project")
        .push(other);
    state.session.active = Some(other_id);
    let model = HeaderBarModel::of(&state).expect("shown");
    assert_eq!(model.meter.text, NO_USAGE_TEXT, "no leak from the first");
    assert_eq!(model.status, Status::Idle, "the new session is not running");
}

#[test]
fn the_ring_text_hides_before_the_changes_label() {
    assert!(RING_TEXT_RANK < CHANGES_LABEL_RANK);
    assert_eq!(RING_TEXT_RANK, 0);
    assert_eq!(CHANGES_LABEL_RANK, 1);
}

#[test]
fn at_most_one_menu_is_open_and_a_second_press_closes_it() {
    let mut menus = Menus::default();
    assert_eq!(menus.menu, HeaderMenu::None);
    menus.update(Msg::Toggled(HeaderMenu::Plus));
    assert_eq!(menus.menu, HeaderMenu::Plus);
    menus.update(Msg::Toggled(HeaderMenu::Worktree));
    assert_eq!(
        menus.menu,
        HeaderMenu::Worktree,
        "opening one closes the other"
    );
    menus.update(Msg::Toggled(HeaderMenu::Worktree));
    assert_eq!(menus.menu, HeaderMenu::None);
    menus.update(Msg::Toggled(HeaderMenu::Plus));
    menus.update(Msg::Dismissed);
    assert_eq!(menus.menu, HeaderMenu::None, "Escape or an outside press");
    menus.update(Msg::Toggled(HeaderMenu::Plus));
    menus.update(Msg::Chosen);
    assert_eq!(menus.menu, HeaderMenu::None, "a chosen entry closes it");
}

#[test]
fn a_selection_change_closes_the_open_menu() {
    let here = Some((SessionLocation::Worktree("a".into()), None));
    let there = Some((SessionLocation::Worktree("b".into()), None));
    let mut menus = Menus::default();
    menus.follow(here.clone());
    menus.update(Msg::Toggled(HeaderMenu::Worktree));
    menus.follow(here);
    assert_eq!(
        menus.menu,
        HeaderMenu::Worktree,
        "the same selection keeps it"
    );
    menus.follow(there);
    assert_eq!(menus.menu, HeaderMenu::None, "another selection closes it");
}
