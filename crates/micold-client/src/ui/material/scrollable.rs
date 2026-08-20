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
use iced::advanced::widget::Id;
use iced::widget::{scrollable, Sensor};
use iced::{Element, Length, Size};
use micold_core::tokens::Roles;

/// A subscription to the offset, the viewport's extent and the content's, all along the
/// scrollable's own axis and all in whole pixels — see [`Scrollable::on_scroll_metrics`].
///
/// Its own name because the three arguments have no meaning in an inline `Box<dyn Fn(u32, u32, u32)>`
/// and every reader would have to go and find out which is which.
type ScrollMetrics<'a, M> = Box<dyn Fn(u32, u32, u32) -> M + 'a>;

/// The scrollbar's width and its scroller's, in pixels, plus the margin holding it off the edge.
/// Matches the sidebar's hand-rolled values exactly (FR-005).
const BAR_WIDTH: f32 = 4.0;
const BAR_MARGIN: f32 = 1.0;

/// The axis a viewport scrolls along (feature 026 FR-002a).
///
/// The wrapper built a vertical viewport and only a vertical one, because both call sites wanted
/// one. The tab strip wants the other axis, and it wants it **from this component**: the themed 4px
/// scrollbar and the dismiss-on-scroll report both live here, so a hand-rolled horizontal scroller
/// at the call site would reintroduce exactly the divergence the wrapper was created to end — and
/// would silently drop the scroll-dismissal the tab menu depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Top to bottom. The default, and what the sidebar's list and the folder browser have always
    /// had.
    Vertical,
    /// Leading to trailing. The tab strip's axis.
    Horizontal,
}

/// Scrollable content with the design system's scrollbar. Builder form (Principle VIII):
/// `Scrollable::new(list, roles).height(Length::Fill).on_scroll(Message::Scrolled).into()`.
pub struct Scrollable<'a, M> {
    content: Element<'a, M>,
    roles: Roles,
    direction: ScrollDirection,
    height: Option<Length>,
    width: Option<Length>,
    on_scroll: Option<M>,
    on_scroll_offset: Option<Box<dyn Fn(u32) -> M + 'a>>,
    on_scroll_metrics: Option<ScrollMetrics<'a, M>>,
    id: Option<Id>,
    on_viewport_resize: Option<Box<dyn Fn(Size) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> Scrollable<'a, M> {
    /// `content` in a vertically scrolling viewport, themed by `roles`.
    pub fn new(content: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            roles,
            direction: ScrollDirection::Vertical,
            height: None,
            width: None,
            on_scroll: None,
            on_scroll_offset: None,
            on_scroll_metrics: None,
            id: None,
            on_viewport_resize: None,
        }
    }

    /// Scroll along `direction` instead of the default [`ScrollDirection::Vertical`].
    ///
    /// A step rather than a second constructor, and defaulted rather than required, because the two
    /// existing call sites must not have to say what they already meant. Every other property of the
    /// viewport — the themed scrollbar, the offset report, the dismissal — is unchanged by it.
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.direction = direction;
        self
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

    /// Report the offset, the viewport's extent and the content's, all along this scrollable's own
    /// axis and all in whole pixels (feature 026 FR-002e).
    ///
    /// Three numbers in one call because they answer **one** question — "does anything lie beyond
    /// this edge" — and the rendering stack delivers them together in a single `Viewport`. Split
    /// across subscriptions there would be frames where one is stale, and a fade computed from a
    /// stale pair points at nothing or fails to point at something.
    ///
    /// Along the scrollable's own axis, not always vertically: this component now has two
    /// ([`ScrollDirection`]), and a horizontal viewport reporting its `y` would report zero forever.
    /// [`Self::on_scroll_offset`] is the older, one-number form, kept because the sidebar's
    /// elevate-on-scroll asks a strictly smaller question.
    pub fn on_scroll_metrics(mut self, f: impl Fn(u32, u32, u32) -> M + 'a) -> Self {
        self.on_scroll_metrics = Some(Box::new(f));
        self
    }

    /// Make this viewport addressable, so it can be scrolled by
    /// [`iced::widget::operation::scroll_to`] (feature 024, FR-008).
    ///
    /// Unset by default: a scrollable without an id behaves exactly as it did before.
    ///
    /// The id belongs on the scrollable **itself**, never on a wrapper around it. Scroll operations
    /// reach widgets by traversal, and a wrapper that does not forward `operate` swallows them for
    /// its whole subtree — the trap `ripple.rs` documents, and the reason the sensor below is
    /// wrapped *outside* rather than the id moved inside.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Report the **viewport's** laid-out size — the scrolling window, not the content in it
    /// (feature 024).
    ///
    /// Fires on first layout as well as on every later size change, which is why it exists rather
    /// than reusing [`Self::on_scroll`]'s viewport: `on_scroll` fires only when something scrolls,
    /// and the case that matters most is the first frame after a project switch, where nothing has.
    ///
    /// Independent of the two scroll subscriptions — setting this disturbs neither, and a consumer
    /// that never sets it pays nothing, since no sensor is inserted into the tree.
    pub fn on_viewport_resize(mut self, f: impl Fn(Size) -> M + 'a) -> Self {
        self.on_viewport_resize = Some(Box::new(f));
        self
    }
}

impl<'a, M: Clone + 'a> From<Scrollable<'a, M>> for Element<'a, M> {
    fn from(s: Scrollable<'a, M>) -> Self {
        // One scrollbar description, placed on whichever axis was asked for: the 4px themed bar is
        // the appearance this component exists to give, and it does not become a different bar
        // because the content runs the other way.
        let bar = scrollable::Scrollbar::new()
            .width(BAR_WIDTH)
            .scroller_width(BAR_WIDTH)
            .margin(BAR_MARGIN);
        let mut widget = scrollable(s.content)
            .direction(match s.direction {
                ScrollDirection::Vertical => scrollable::Direction::Vertical(bar),
                ScrollDirection::Horizontal => scrollable::Direction::Horizontal(bar),
            })
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
        if let Some(f) = s.on_scroll_metrics {
            let axis = s.direction;
            widget = widget.on_scroll(move |viewport| {
                let offset = viewport.absolute_offset();
                let window = viewport.bounds();
                let content = viewport.content_bounds();
                let (o, w, c) = match axis {
                    ScrollDirection::Vertical => (offset.y, window.height, content.height),
                    ScrollDirection::Horizontal => (offset.x, window.width, content.width),
                };
                f(
                    crate::app::scroll_offset_px(o),
                    crate::app::scroll_offset_px(w),
                    crate::app::scroll_offset_px(c),
                )
            });
        } else if let Some(f) = s.on_scroll_offset {
            let axis = s.direction;
            widget = widget.on_scroll(move |viewport| {
                let offset = viewport.absolute_offset();
                f(crate::app::scroll_offset_px(match axis {
                    ScrollDirection::Vertical => offset.y,
                    ScrollDirection::Horizontal => offset.x,
                }))
            });
        } else if let Some(message) = s.on_scroll {
            widget = widget.on_scroll(move |_| message.clone());
        }
        if let Some(id) = s.id {
            widget = widget.id(id);
        }
        match s.on_viewport_resize {
            // `on_show` as well as `on_resize`: a size that never changes is still a size nobody
            // has been told, and the first layout is exactly the frame the reveal needs it for.
            //
            // The sensor wraps the scrollable rather than the other way round, so the id set above
            // stays on the scrollable and `scroll_to` still reaches it — iced's own `Sensor`
            // forwards `operate`, and a future replacement that did not would break the scroll
            // silently.
            Some(f) => {
                // One closure, two subscriptions — `Rc` because both need it and neither owns the
                // other. Wrapping the *scrollable* is what makes the reported size the viewport's:
                // the scrollable's own bounds are the window, and its content's are what scrolls
                // inside them.
                let f = std::rc::Rc::new(f);
                let on_show = std::rc::Rc::clone(&f);
                Sensor::new(widget)
                    .on_show(move |size| on_show(size))
                    .on_resize(move |size| f(size))
                    .into()
            }
            None => widget.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::Space;

    fn roles() -> Roles {
        micold_core::tokens::roles(micold_core::theme::ColorScheme::Dark)
    }

    /// FR-002a needs a horizontally scrolling viewport, and this component only ever built a
    /// vertical one. Both halves are asserted, and the **default** is the half that matters most:
    /// two call sites — the sidebar's list and the folder browser's — depend on it and must not
    /// move because a third one wanted the other axis.
    #[test]
    fn a_scrollable_takes_its_axis_and_still_defaults_to_vertical() {
        let vertical: Scrollable<'_, ()> = Scrollable::new(Space::new(), roles());
        assert_eq!(
            vertical.direction,
            ScrollDirection::Vertical,
            "a scrollable built without an axis must still be the vertical one the sidebar and \
             the folder browser have always got"
        );

        let horizontal: Scrollable<'_, ()> =
            Scrollable::new(Space::new(), roles()).direction(ScrollDirection::Horizontal);
        assert_eq!(
            horizontal.direction,
            ScrollDirection::Horizontal,
            "the axis a caller asked for is not the axis the scrollable carries"
        );
    }
}
