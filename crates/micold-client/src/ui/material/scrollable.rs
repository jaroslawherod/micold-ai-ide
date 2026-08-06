//! `Scrollable` — the library's wrapper around the rendering stack's scrollable (Principle VIII).
//!
//! Two call sites, and they already disagree: the sidebar's list gets a themed 4px scrollbar, and
//! the folder browser's gets whatever the rendering stack's default theme produces. Nobody chose
//! that — one of them was written first.
//!
//! The wrapper gives both the themed scrollbar, which is the appearance the design system actually
//! specifies. That makes the folder browser's scrollbar the one visible difference this component
//! introduces; it is called out in the feature's `behavior-delta.md` alongside the dismissal
//! changes rather than smuggled in as "parity".
//!
//! It is also where dismiss-on-scroll is reported from (FR-009): a surface floating above content
//! needs to know the ground moved, and every scrollable is a place the ground can move.

use crate::ui::material::style;
use iced::widget::scrollable;
use iced::{Element, Length};
use micold_core::tokens::Roles;

/// The scrollbar's width and its scroller's, in pixels, plus the margin holding it off the edge.
/// Matches the sidebar's hand-rolled values exactly (FR-005).
const BAR_WIDTH: f32 = 4.0;
const BAR_MARGIN: f32 = 1.0;

/// Scrollable content with the design system's scrollbar. Builder form (Principle VIII):
/// `Scrollable::new(list, roles).height(Length::Fill).on_scroll(Message::Scrolled).into()`.
pub struct Scrollable<'a, M> {
    content: Element<'a, M>,
    roles: Roles,
    height: Option<Length>,
    width: Option<Length>,
    on_scroll: Option<M>,
    on_scroll_offset: Option<Box<dyn Fn(u32) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> Scrollable<'a, M> {
    /// `content` in a vertically scrolling viewport, themed by `roles`.
    pub fn new(content: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            roles,
            height: None,
            width: None,
            on_scroll: None,
            on_scroll_offset: None,
        }
    }

    /// Lay the viewport out at a given height — `Length::Fill` to take the space its parent has.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Lay the viewport out at a given width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// The message emitted whenever the content scrolls.
    ///
    /// The offset is deliberately not passed on *here*: dismiss-on-scroll cares only *that* the
    /// ground moved. A consumer that needs the number asks for it with [`Self::on_scroll_offset`],
    /// which is a second subscription rather than a change to this one — the two answer different
    /// questions and folding them together would make every dismissal carry a measurement.
    pub fn on_scroll(mut self, message: M) -> Self {
        self.on_scroll = Some(message);
        self
    }

    /// Report the vertical offset as the list scrolls, in whole pixels from the top.
    ///
    /// Added for the app bar's elevate-on-scroll (FR-025a): the sidebar is the only scroll region
    /// beneath the bar, so its offset is the whole of the signal. Rounded and clamped by
    /// [`crate::app::scroll_offset_px`] before it reaches the caller, so an overscroll bounce or an
    /// unsettled viewport reads as "at the top" rather than as movement.
    pub fn on_scroll_offset(mut self, f: impl Fn(u32) -> M + 'a) -> Self {
        self.on_scroll_offset = Some(Box::new(f));
        self
    }
}

impl<'a, M: Clone + 'a> From<Scrollable<'a, M>> for Element<'a, M> {
    fn from(s: Scrollable<'a, M>) -> Self {
        let mut widget = scrollable(s.content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(BAR_WIDTH)
                    .scroller_width(BAR_WIDTH)
                    .margin(BAR_MARGIN),
            ))
            .style(style::scrollbar(s.roles));
        if let Some(height) = s.height {
            widget = widget.height(height);
        }
        if let Some(width) = s.width {
            widget = widget.width(width);
        }
        // One subscription, because the rendering stack gives a scrollable one. The offset form
        // wins when both are set: it carries strictly more information, and its reducer arm runs
        // the dismissal too, so nothing is lost by preferring it.
        if let Some(f) = s.on_scroll_offset {
            widget = widget.on_scroll(move |viewport| {
                f(crate::app::scroll_offset_px(viewport.absolute_offset().y))
            });
        } else if let Some(message) = s.on_scroll {
            widget = widget.on_scroll(move |_| message.clone());
        }
        widget.into()
    }
}
