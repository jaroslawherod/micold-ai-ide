//! A vertical wheel over a code block scrolls the transcript, not the block (feature 940, T025,
//! research R12).
//!
//! The block's lines sit in a horizontal-only viewport. iced moves such a viewport by the wheel's
//! horizontal part alone, so a vertical turn of the wheel over the block must pass through it to
//! the vertical list that holds it. Here the showcase's code block sits between two tall spacers
//! in a vertical `Scrollable` that reports its offset: a wheel turned down over the block must move
//! that list.
//!
//! Compiled into the `layout_snapshot` binary beside `code_block_fits`.

use crate::support::layout::{self as lay, Input};
use iced::widget::{column, scrollable, Space};
use iced::{Length, Point};
use micold_client::showcase::sections::surfaces::{code_block_sample, CodePose};
use micold_client::showcase::state::Message;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// Taller than the window, so the list has somewhere to scroll.
const SPACER: f32 = 600.0;

#[test]
fn a_vertical_wheel_over_a_code_block_scrolls_the_list_that_holds_it() {
    let roles = tokens::roles(ColorScheme::Light);
    let list = scrollable(
        column![
            Space::new().height(Length::Fixed(SPACER / 2.0)),
            code_block_sample(CodePose::Idle, Message::NoOp, roles),
            Space::new().height(Length::Fixed(SPACER * 2.0)),
        ]
        .width(Length::Fill),
    )
    .on_scroll(|_| Message::Reversed(0))
    .width(Length::Fill)
    .height(Length::Fill);
    // The block's first line: below the first spacer, inside the block's lines.
    let over_the_block = Point::new(40.0, SPACER / 2.0 + 40.0);
    let published = lay::messages_after(
        list.into(),
        &lay::renderer(),
        &[Input::Wheel(over_the_block)],
    );
    assert!(
        published.iter().any(|m| matches!(m, Message::Reversed(0))),
        "the wheel over the block did not scroll the list: {published:?}"
    );
}
