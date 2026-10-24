//! Driving the review's source control through the Changes reducer (feature 1272, M2), shared by
//! `features_changes` and `features_pr_changes`.

#![allow(dead_code)]

use micold_client::features::changes::{
    self, source_menu_input, source_menu_stops, Effect, Msg, OpenView, SourceMenu, State,
};
use micold_core::review::source_menu::{label, Stop};
use micold_core::typeahead::Direction;

pub fn view(state: &State) -> &OpenView {
    state.tabs.shown_view().expect("open")
}

pub fn menu(state: &State) -> SourceMenu {
    view(state).source_menu
}

pub fn stops(state: &State) -> Vec<(Stop, bool)> {
    source_menu_stops(view(state), state.pr_enabled)
}

/// The stop the highlight is on.
pub fn highlighted(state: &State) -> Option<Stop> {
    let stops = stops(state);
    menu(state)
        .highlight
        .and_then(|i| stops.get(i).map(|(stop, _)| *stop))
}

/// What the closed control reads.
pub fn menu_label(state: &State) -> String {
    label(&source_menu_input(view(state), state.pr_enabled, 1)).full()
}

/// Open the list when closed and walk the highlight onto `stop` with the arrow keys.
pub fn highlight(state: &mut State, stop: Stop) {
    if !menu(state).open {
        changes::update(state, Msg::SourceMenuToggled);
    }
    let target = stops(state)
        .iter()
        .position(|(s, _)| *s == stop)
        .expect("a stop of the list");
    while menu(state).highlight != Some(target) {
        let direction = match menu(state).highlight {
            Some(at) if at > target => Direction::Prev,
            _ => Direction::Next,
        };
        changes::update(state, Msg::SourceMenuMoved(direction));
    }
}

/// Pick `stop` through the control.
pub fn pick(state: &mut State, stop: Stop) -> Effect {
    highlight(state, stop);
    changes::update(state, Msg::SourceMenuPicked)
}
