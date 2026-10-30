//! Rendering is a pure function of the Settings: a second render gives the same bytes, so delivery
//! writes only on a change (research D10, #1525).

#[path = "support/daemon_config_fixtures.rs"]
mod fixtures;

use fixtures::{settings, KINDS};
use micold_client::daemon_config_render::render;

#[test]
fn rendering_twice_gives_identical_bytes() {
    for kind in KINDS {
        let s = settings(kind);
        assert_eq!(render(&s, kind), render(&s.clone(), kind), "{kind:?}");
    }
}

#[test]
fn a_change_to_the_settings_changes_the_bytes() {
    let mut s = settings(micold_core::daemon_config::Kind::Host);
    let before = render(&s, micold_core::daemon_config::Kind::Host);
    s.log = Some("trace".into());
    assert_ne!(before, render(&s, micold_core::daemon_config::Kind::Host));
}
