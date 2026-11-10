//! The top bar's order and presence (feature 1793, T009 — FR-003, FR-004, US2 scenarios 1 and 2):
//! with a session shown the context meter, the status, the Changes button, then Settings and the
//! app menu, left to right with no overlap at a wide width; none of the first three on the Overview.
//!
//! **Compiled into the `layout_snapshot` binary** for the `support::layout` passes; it builds its
//! own state and reads no cache.

use crate::support::covered_states::transcript_pane_insights;
use crate::support::layout::{self as lay, StateUnderTest};
use iced::Size;
use micold_client::icons::Icon;
use micold_core::tokens::anatomy::app_bar;

const WIDE: Size = Size::new(1280.0, 800.0);

/// The text drawn in the bar with its left edge, in drawing order.
fn bar_text(state: micold_client::app::State) -> Vec<(String, f32)> {
    let under = StateUnderTest::new(state);
    let (painted, _) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), WIDE);
    painted
        .iter()
        .filter(|p| p.origin.y < app_bar::HEIGHT)
        .map(|p| (p.content.clone(), p.origin.x))
        .collect()
}

#[test]
fn the_meter_status_and_changes_come_before_settings_and_the_menu() {
    let drawn = bar_text(transcript_pane_insights());
    let at = |wanted: &dyn Fn(&str) -> bool, what: &str| {
        drawn
            .iter()
            .find(|(c, _)| wanted(c))
            .unwrap_or_else(|| panic!("{what} is not drawn: {drawn:?}"))
            .1
    };
    let ring = at(&|c| c.contains('%') && c.contains('$'), "the meter text");
    let status = at(&|c| c == "Working", "the status");
    let changes = at(&|c| c.starts_with("Changes"), "the Changes label");
    let settings = Icon::Settings.glyph().to_string();
    let menu = Icon::Menu.glyph().to_string();
    let settings = at(&|c| c.contains(settings.as_str()), "Settings");
    // Two `⋮` triggers (feature 1793): the worktree menu's sits before Settings, the app menu's is
    // the bar's last control.
    let worktree_menu = at(&|c| c.contains(menu.as_str()), "the worktree menu");
    let menu = drawn
        .iter()
        .rev()
        .find(|(c, _)| c.contains(menu.as_str()))
        .map(|(_, x)| *x)
        .expect("the app menu is drawn");
    assert!(
        ring < status
            && status < changes
            && changes < worktree_menu
            && worktree_menu < settings
            && settings < menu,
        "order: ring {ring}, status {status}, Changes {changes}, Settings {settings}, menu {menu}"
    );
}

#[test]
fn the_overview_shows_none_of_them() {
    let mut state = transcript_pane_insights();
    state.overview.open = true;
    let drawn = bar_text(state);
    assert!(
        !drawn
            .iter()
            .any(|(c, _)| c == "Working" || c == "Idle" || c.starts_with("Changes")),
        "the Overview's bar draws a session control: {drawn:?}"
    );
}
