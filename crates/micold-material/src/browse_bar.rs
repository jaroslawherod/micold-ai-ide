//! `BrowseBar` — the strip above a browsed repository file (feature 1150, FR-013, FR-015).
//!
//! Back, the file's path on one clamped line, and — only when the change modifies the file — "Open
//! diff". The strip is one height with or without that button and the path never widens it, so the
//! file below does not move when the button comes and goes.
//!
//! Builder form: `BrowseBar::new("src/lib.rs", roles).on_back(Msg::Back).on_open_diff(Msg::Diff)`.
//! Without `on_back` the Back button is drawn disabled (nothing to return to).

use iced::widget::{container, row};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{spacing, Roles};

use super::line_clamp::LineClamped;
use super::{Button, TypeRole};

/// The strip's height, whatever it shows.
pub const HEIGHT: f32 = 48.0;

/// The strip above a browsed file.
pub struct BrowseBar<'a, M> {
    path: &'a str,
    roles: Roles,
    on_back: Option<M>,
    on_open_diff: Option<M>,
}

impl<'a, M: Clone + 'a> BrowseBar<'a, M> {
    /// The strip of the file at `path`, themed by `roles`.
    pub fn new(path: &'a str, roles: Roles) -> Self {
        Self {
            path,
            roles,
            on_back: None,
            on_open_diff: None,
        }
    }

    /// The message Back sends; without it Back is drawn disabled.
    pub fn on_back(mut self, message: M) -> Self {
        self.on_back = Some(message);
        self
    }

    /// Offer "Open diff", sending `message`.
    pub fn on_open_diff(mut self, message: M) -> Self {
        self.on_open_diff = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<BrowseBar<'a, M>> for Element<'a, M> {
    fn from(bar: BrowseBar<'a, M>) -> Self {
        let r = bar.roles;
        let mut back = Button::outlined("Back", r);
        if let Some(message) = bar.on_back {
            back = back.on_press(message);
        }
        let mut strip = row![
            back,
            LineClamped::<M>::new(bar.path, TypeRole::Body, 1).width(Length::Fill)
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
        if let Some(message) = bar.on_open_diff {
            strip = strip.push(Button::outlined("Open diff", r).on_press(message));
        }
        container(strip)
            .width(Length::Fill)
            .height(Length::Fixed(HEIGHT))
            .align_y(Alignment::Center)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;
    use micold_core::tokens::LIGHT;

    fn size(bar: BrowseBar<'_, ()>, width: f32) -> Size {
        super::super::test_support::size_of(bar.into(), Size::new(width, 600.0))
    }

    #[test]
    fn the_strip_is_one_height_with_and_without_open_diff() {
        let long = "crates/very/deeply/nested/module/".repeat(12) + "file.rs";
        let heights: Vec<f32> = [
            BrowseBar::new("a.rs", LIGHT),
            BrowseBar::new("a.rs", LIGHT).on_back(()).on_open_diff(()),
            BrowseBar::new(&long, LIGHT).on_back(()).on_open_diff(()),
            BrowseBar::new(&long, LIGHT),
        ]
        .into_iter()
        .map(|bar| size(bar, 320.0).height)
        .collect();
        assert!(heights.iter().all(|h| *h == HEIGHT), "{heights:?}");
    }

    #[test]
    fn a_long_path_stays_within_the_width() {
        let long = "x/".repeat(300) + "file.rs";
        let width = size(BrowseBar::new(&long, LIGHT).on_open_diff(()), 280.0).width;
        assert!(width <= 280.0, "{width}");
    }

    #[test]
    fn open_diff_is_offered_only_on_request() {
        assert!(BrowseBar::<()>::new("a", LIGHT).on_open_diff.is_none());
        assert!(BrowseBar::new("a", LIGHT)
            .on_open_diff(())
            .on_open_diff
            .is_some());
    }
}
