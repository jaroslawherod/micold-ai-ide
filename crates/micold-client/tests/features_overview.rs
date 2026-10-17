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
