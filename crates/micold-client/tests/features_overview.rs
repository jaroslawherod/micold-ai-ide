//! The Overview feature in isolation (feature 813, T010; SC-004): only its own `State`, `Msg` and
//! `update`.

use micold_client::features::overview::{update, Msg, State};

fn toggled(settings_open: bool) -> Msg {
    Msg::Toggled { settings_open }
}

#[test]
fn toggled_opens_then_closes_and_closed_closes() {
    let mut state = State::default();
    assert!(!state.open);
    update(&mut state, toggled(false));
    assert!(state.open);
    update(&mut state, toggled(false));
    assert!(!state.open);
    update(&mut state, toggled(false));
    update(&mut state, Msg::Closed);
    assert!(!state.open);
}

#[test]
fn toggled_while_settings_is_open_changes_nothing() {
    let mut state = State::default();
    update(&mut state, toggled(true));
    assert!(!state.open);
    state.open = true;
    update(&mut state, toggled(true));
    assert!(state.open);
}

// Feature 970 (FR-001, FR-010, FR-011, SC-005; data-model: `full_screen()` is `open && full_screen`,
// the only reader).

use micold_core::overview::Activation;
use micold_core::session::SessionLocation;

fn full_screen_toggled(settings_open: bool) -> Msg {
    Msg::FullScreenToggled { settings_open }
}

/// An open Overview in full-screen mode.
fn in_full_screen() -> State {
    let mut state = State::default();
    update(&mut state, toggled(false));
    update(&mut state, full_screen_toggled(false));
    assert!(state.full_screen());
    state
}

#[test]
fn full_screen_toggles_only_while_open() {
    let mut state = State::default();
    update(&mut state, full_screen_toggled(false));
    assert!(!state.full_screen(), "closed: the toggle does nothing");

    update(&mut state, toggled(false));
    update(&mut state, full_screen_toggled(false));
    assert!(state.full_screen(), "open: the toggle enters full screen");
    update(&mut state, full_screen_toggled(false));
    assert!(!state.full_screen(), "again: it leaves");

    update(&mut state, full_screen_toggled(true));
    assert!(!state.full_screen(), "Settings open: the toggle does nothing");

    assert!(state.request(Activation::RevealOnly(SessionLocation::Default)));
    update(&mut state, full_screen_toggled(false));
    assert!(
        !state.full_screen(),
        "an activation pending: the first one stands"
    );
}

#[test]
fn opening_the_overview_clears_full_screen() {
    let mut state = in_full_screen();
    update(&mut state, Msg::Closed);
    update(&mut state, toggled(false));
    assert!(state.open);
    assert!(
        !state.full_screen(),
        "the Overview always opens in the normal layout"
    );
}

#[test]
fn every_exit_leaves_full_screen() {
    let exits: [fn(&mut State); 5] = [
        |s| {
            update(s, Msg::Closed);
        },
        |s| {
            update(s, toggled(false));
        },
        |s| {
            update(
                s,
                Msg::CardActivated(Activation::OfferStart(SessionLocation::Default)),
            );
        },
        |s| {
            update(
                s,
                Msg::CardActivated(Activation::RevealOnly(SessionLocation::Default)),
            );
        },
        // `app.rs` sets the flag directly for `MainAreaTaken` and the Changes/Compare views.
        |s| s.open = false,
    ];
    for (index, exit) in exits.into_iter().enumerate() {
        let mut state = in_full_screen();
        exit(&mut state);
        assert!(!state.full_screen(), "exit {index} left full screen on");
    }
}

#[test]
fn a_project_switch_keeps_full_screen() {
    let mut state = in_full_screen();
    update(&mut state, Msg::ProjectSwitched);
    assert!(state.full_screen());
}
