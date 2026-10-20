//! The showcase's grouped `AccordionSection` pose follows the group's state (feature 1091,
//! FR-008, T016).
//!
//! `showcase_state.rs` holds the reducer; this holds the wiring from that state to what is
//! painted, which the reducer tests cannot see: a pose that ignored the group would still pass
//! them. The section's body text is painted only while it is open, so counting painted bodies
//! counts open sections.

#[path = "support/mod.rs"]
mod support;

use micold_client::showcase::samples;
use micold_client::showcase::sections::surfaces;
use micold_client::showcase::state::{Message, Showcase};
use micold_core::theme::ColorScheme;
use micold_core::tokens;
use support::layout as lay;

/// Bodies painted by the section pose that stand outside the group: the static "open" pose, the
/// live pose (open at rest) and the independent pair's second section.
const OUTSIDE_THE_GROUP: usize = 3;

/// How many section bodies the `AccordionSection` entry paints for `showcase`.
fn bodies(showcase: &Showcase) -> usize {
    let roles = tokens::roles(ColorScheme::Light);
    let mut renderer = lay::renderer();
    lay::painted_text(
        surfaces::accordion_section(showcase, roles, 0),
        &mut renderer,
    )
    .into_iter()
    .filter(|drawn| drawn.content == samples::BODY)
    .count()
}

/// How many of the group's three sections are open, as the showcase state sees it.
fn group_open_count(showcase: &Showcase) -> usize {
    (0..3).filter(|key| showcase.group_open(*key)).count()
}

#[test]
fn the_grouped_pose_paints_one_open_body_by_default() {
    let s = Showcase::new(1);
    assert_eq!(group_open_count(&s), 1);
    assert_eq!(bodies(&s), OUTSIDE_THE_GROUP + 1);
}

#[test]
fn the_grouped_pose_follows_a_press() {
    let mut s = Showcase::new(1);
    s.update(Message::GroupSectionToggled(1));
    assert_eq!(group_open_count(&s), 1);
    assert_eq!(
        bodies(&s),
        OUTSIDE_THE_GROUP + 1,
        "opening one closes the other"
    );
    s.update(Message::GroupSectionToggled(1));
    assert_eq!(group_open_count(&s), 0);
    assert_eq!(
        bodies(&s),
        OUTSIDE_THE_GROUP,
        "pressing the open one closes it"
    );
}
