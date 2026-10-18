//! The quick switcher in isolation (feature 968, SC-004 of feature 021): its own state and
//! messages only.

use micold_client::app::State;
use micold_client::features::quick_switch::{update, Msg};

#[test]
fn the_chord_opens_the_palette_and_dismissal_closes_it() {
    let mut state = State::default();
    assert!(state.quick_switch.open.is_none());
    update(
        &mut state,
        Msg::Toggled {
            terminal_had_focus: true,
        },
    );
    let open = state
        .quick_switch
        .open
        .as_ref()
        .expect("the chord opens the palette");
    assert!(open.terminal_had_focus);
    assert!(open.query.is_empty());
    update(&mut state, Msg::QueryChanged("log".into()));
    assert_eq!(
        state.quick_switch.open.as_ref().map(|o| o.query.as_str()),
        Some("log")
    );
    update(&mut state, Msg::Dismissed);
    assert!(state.quick_switch.open.is_none());
}

#[test]
fn the_chord_again_closes_it() {
    let mut state = State::default();
    for _ in 0..2 {
        update(
            &mut state,
            Msg::Toggled {
                terminal_had_focus: false,
            },
        );
    }
    assert!(state.quick_switch.open.is_none());
}
