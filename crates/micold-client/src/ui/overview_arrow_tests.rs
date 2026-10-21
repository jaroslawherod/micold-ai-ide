//! The Overview's arrow listener (feature 1122, R9).

use super::*;

/// Feature 1122, R9: the Overview's arrow keys ask the root to step between cards, not to
/// move the keyboard through every focusable (which would stop on issue links).
#[test]
fn the_overview_arrows_step_between_cards() {
    use crate::features::overview::Msg;
    use iced::keyboard::key::Named;
    let arrow = |named: Named| {
        let iced::Event::Keyboard(event) = crate::ui::material::test_support::named_key(named)
        else {
            unreachable!("a key press is a keyboard event")
        };
        overview_arrow(event)
    };
    for (named, forward) in [
        (Named::ArrowRight, true),
        (Named::ArrowDown, true),
        (Named::ArrowLeft, false),
        (Named::ArrowUp, false),
    ] {
        assert_eq!(
            arrow(named),
            Some(Message::Overview(Msg::CardStepped { forward })),
            "{named:?}"
        );
    }
    assert_eq!(arrow(Named::Enter), None);
}
