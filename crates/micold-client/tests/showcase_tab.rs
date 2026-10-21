//! Tab reaches the showcase although the gallery captures it (feature 1122, M1 visual pass r2 F1).
//!
//! The gallery poses a focused `TerminalPane`, which writes Tab to its process and captures the
//! event. The showcase used to listen only to uncaptured keys, so Tab never moved the focus and no
//! focus ring could be seen. This drives a Tab through the real gallery and hands the event, with
//! the status the gallery gave it, to the showcase's listener.

#[path = "support/mod.rs"]
mod support;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::Tree;
use iced::event::Status;
use iced::{Rectangle, Size};
use micold_client::showcase::catalogue::{COMPONENTS, MOTION};
use micold_client::showcase::gallery;
use micold_client::showcase::state::{event_message, Showcase};
use support::layout as lay;

const WINDOW: Size = Size::new(1280.0, 900.0);

fn tab(shift: bool) -> iced::Event {
    let key = iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab);
    let mut modifiers = iced::keyboard::Modifiers::empty();
    modifiers.set(iced::keyboard::Modifiers::SHIFT, shift);
    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        location: iced::keyboard::Location::Standard,
        modifiers,
        text: Some("\t".into()),
        repeat: false,
    })
}

/// The status the real gallery, at rest, gives `event`.
fn gallery_status(event: &iced::Event) -> Status {
    let showcase = Showcase::new(COMPONENTS.len() + MOTION.len());
    let mut element = gallery::view(&showcase);
    let renderer = lay::renderer();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, WINDOW),
    );
    let mut messages = Vec::new();
    let mut shell = iced::advanced::Shell::new(&mut messages);
    element.as_widget_mut().update(
        &mut tree,
        event,
        Layout::new(&node),
        iced::mouse::Cursor::Unavailable,
        &renderer,
        &mut iced::advanced::clipboard::Null,
        &mut shell,
        &Rectangle::with_size(WINDOW),
    );
    shell.event_status()
}

/// Tab and Shift+Tab move the focus after the gallery has seen them, captured or not.
#[test]
fn tab_moves_the_focus_after_the_gallery_has_seen_it() {
    for shift in [false, true] {
        let event = tab(shift);
        let status = gallery_status(&event);
        let moved = event_message(event, status, iced::window::Id::unique())
            .and_then(|message| message.focus_move());
        assert_eq!(
            moved,
            Some(!shift),
            "Tab (shift: {shift}, gallery status: {status:?}) did not move the focus"
        );
    }
}
