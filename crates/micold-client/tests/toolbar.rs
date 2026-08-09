//! Integration test: the overflow menu's "Help" action exposes exactly "About" (FR-004).

use micold_client::features::help::help_actions;

#[test]
fn help_menu_exposes_only_about() {
    assert_eq!(
        help_actions(),
        ["About"],
        "Help must expose exactly the About action"
    );
}
