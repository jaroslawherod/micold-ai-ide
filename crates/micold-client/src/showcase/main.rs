//! The component showcase — binary entry point (feature 020, T012).
//!
//! Every shared component on one page, live and interactive, in either colour scheme. A developer runs
//! `mise run showcase` and looks; there is nothing to set up.
//!
//! # What this file deliberately does not name
//!
//! No project store, no settings store, no daemon endpoint, no process spawn, no git, and no host
//! theme preference. That absence *is* FR-017 and FR-020: launching the showcase must not start a
//! session daemon, must not touch a git repository, and must not create, read or modify any saved
//! application state. The import list below is the statement of it, and
//! `tests/showcase_isolation.rs` holds it — because an import added in a hurry is exactly how a
//! development tool starts writing to a user's configuration.
//!
//! # Why this file is thin
//!
//! Every state transition lives in [`micold_client::showcase::state`] and is tested directly; every
//! layout decision lives in [`micold_client::showcase::gallery`]. What is left here is the three lines
//! iced needs — which is what Principle I's GUI-wiring exception covers, and what
//! `tests/showcase_glue.rs` checks rather than assumes.

use iced::{Subscription, Task};
use micold_client::showcase::catalogue::COMPONENTS;
use micold_client::showcase::gallery;
use micold_client::showcase::state::{Message, Showcase};

/// The showcase at rest, sized for the catalogue it will render.
///
/// The motion section's entries index past the components' range, so the per-entry state covers both
/// and no two entries share a replay counter.
fn boot() -> (Showcase, Task<Message>) {
    (
        Showcase::new(COMPONENTS.len() + micold_client::showcase::catalogue::MOTION.len()),
        Task::none(),
    )
}

fn update(showcase: &mut Showcase, message: Message) -> Task<Message> {
    showcase.update(message);
    Task::none()
}

/// Escape dismisses whatever floating surface is open, exactly as it does in the application: the
/// keypress arrives through the keyboard subscription, outside the widget tree, so the owner of the
/// surface has to be the one to hear it.
fn subscription(_showcase: &Showcase) -> Subscription<Message> {
    iced::keyboard::listen().filter_map(|event| {
        use iced::keyboard::{key::Named, Event, Key};
        matches!(
            event,
            Event::KeyPressed {
                key: Key::Named(Named::Escape),
                ..
            }
        )
        .then_some(Message::Dismissed)
    })
}

pub fn main() -> iced::Result {
    iced::application(boot, update, gallery::view)
        .title(gallery::TITLE)
        // The application's own theme function — the one part of the styling layer that reaches
        // beyond the library. Using it rather than copying its result is what makes a component
        // resolve the same colours here as it does in the application (FR-010, SC-006).
        .theme(|showcase: &Showcase| micold_client::ui::theme(showcase.scheme))
        .default_font(iced::Font::DEFAULT)
        // Both Roboto faces, for the same reason as the icon font below: a gallery that renders the
        // library in some other typeface misreports the library.
        //
        // This was missing, and it was not a cosmetic gap. Several roles differ from each other
        // *only* in weight — `Caption` against `Label`, `Body` against `Action`, and (feature 024)
        // `SidebarSession` against `SidebarSessionCurrent`, which is how the current session's row
        // is told apart without relying on colour. With no Roboto Medium registered, the matcher
        // fell back to a serif face for every weight-500 role, so the heavier roles rendered
        // *lighter* than the lighter ones — the gallery showed the distinction backwards, on the
        // one screen a reviewer would go to in order to check it.
        .font(micold_client::ui::ROBOTO_REGULAR_BYTES)
        .font(micold_client::ui::ROBOTO_MEDIUM_BYTES)
        // Without the icon font, every component that draws a glyph renders a fallback box — and a
        // gallery of fallback boxes would misreport the library it exists to display.
        .font(micold_client::ui::MATERIAL_SYMBOLS_BYTES)
        .subscription(subscription)
        .run()
}
