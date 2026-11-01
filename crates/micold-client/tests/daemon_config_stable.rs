//! Rendering is a pure function of the Settings: a second render gives the same bytes, so delivery
//! writes only on a change (research D10, #1525).

use crate::fixtures;

use fixtures::settings;
use micold_client::daemon_config_render::render;
use micold_core::daemon_config::Kind;

/// The keys come out in the contract's fixed order.
#[test]
fn keys_are_written_in_a_fixed_order() {
    let text = render(&settings(Kind::Host), Kind::Host);
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle} in\n{text}"))
    };
    let order = [
        "schema = 1",
        "[daemon]",
        "name = ",
        "kind = ",
        "data_dir = ",
        "time_zone = ",
        "idle_stop = ",
        "log = ",
        "[listen]",
        "[storage]",
        "[[projects]]",
    ];
    let places: Vec<usize> = order.iter().map(|n| at(n)).collect();
    assert!(
        places.windows(2).all(|w| w[0] < w[1]),
        "keys out of order: {places:?}\n{text}"
    );
}

#[test]
fn a_change_to_the_settings_changes_the_bytes() {
    let mut s = settings(Kind::Host);
    let before = render(&s, Kind::Host);
    s.log = Some("trace".into());
    assert_ne!(before, render(&s, Kind::Host));
}
