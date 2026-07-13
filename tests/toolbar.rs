//! US1 integration tests: the top toolbar exposes exactly one entry, "Help" (FR-002, FR-003).

use micold_ai_ide::app::{help_actions, toolbar_entries};

#[test]
fn toolbar_exposes_only_help() {
    assert_eq!(
        toolbar_entries(),
        ["Help"],
        "toolbar must contain exactly the Help entry"
    );
}

#[test]
fn help_menu_exposes_only_about() {
    assert_eq!(
        help_actions(),
        ["About"],
        "Help must expose exactly the About action"
    );
}
