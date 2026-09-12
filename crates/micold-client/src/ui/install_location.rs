//! The install-me screen (feature 028, FR-019).
//!
//! Shown instead of the session UI — not over it — when the application is running from a mounted
//! disk image or from a translocated copy. Both are states in which the application half-works:
//! everything the user does is written into something that disappears when the image is ejected or
//! the app is closed, and nothing about the window would otherwise say so.
//!
//! There is no dismissal here, and that is the design rather than an omission. A warning the user
//! can click past is a warning the user learns to click past, and the cost of clicking past this
//! one arrives weeks later as vanished state. The way out is the remedy, performed in Finder.
//!
//! Glue only: which of the two situations this is, what it means, and what to do about it belong to
//! `micold_core::install_location`, and are tested there without a renderer.

use crate::app::{Message, State};
use crate::icons::{icon_role, Icon, IconSurface};
use crate::ui::material::{self, IconLabel, SurfaceKind, Text, TypeRole};
use iced::widget::{column, container};
use iced::{Alignment, Element, Length};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The whole content area, for a copy that is not installed.
pub fn view(state: &State, scheme: ColorScheme) -> Element<'_, Message> {
    let r = tokens::roles(scheme);
    let location = state.install_location;

    let card = material::Surface::new(
        column![
            // The glyph belongs to the headline rather than sitting above it: `IconLabel` is the
            // library's "one piece of text with a picture in front of it", and taking it means the
            // glyph is sized from the headline's role instead of from a figure chosen here.
            IconLabel::new(
                Icon::Unavailable,
                "Install Micold AI IDE first",
                TypeRole::Headline,
                r,
            )
            .tint(icon_role(IconSurface::Unavailable, r)),
            // Which of the two states this is, in one sentence, and then the gesture. Both come
            // from the core so that this file has no opinion to drift from.
            Text::new(location.explanation(), TypeRole::Body, r).muted(),
            Text::new(location.remedy(), TypeRole::Body, r),
        ]
        .spacing(spacing::MD)
        .align_x(Alignment::Center),
        SurfaceKind::Plain,
        r,
    )
    .padding(spacing::LG);

    container(card)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
