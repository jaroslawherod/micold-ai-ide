//! Integration test: the overflow menu's "Help" action exposes exactly "About" (FR-004).

mod support;

use micold_client::features::help::help_actions;

#[test]
fn help_menu_exposes_only_about() {
    assert_eq!(
        help_actions(),
        ["About"],
        "Help must expose exactly the About action"
    );
}

/// Feature 1124, US5 s2: the plan usage moved to the status bar; the app bar (the window's top
/// strip) draws none of it even with a current reading.
#[test]
fn the_app_bar_draws_no_usage_indicator() {
    let under = support::covered_states::status_bar_usage();
    let painted = support::layout::painted_text(
        support::layout::view_of(&under),
        &mut support::layout::renderer(),
    );
    assert!(
        painted.iter().any(|p| p.content.contains("37%")),
        "the reading is drawn somewhere"
    );
    let app_bar: Vec<_> = painted.iter().filter(|p| p.origin.y < 56.0).collect();
    assert!(
        !app_bar.iter().any(|p| p.content.contains("37%")),
        "{:?}",
        app_bar.iter().map(|p| &p.content).collect::<Vec<_>>()
    );
}
