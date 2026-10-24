//! A secondary (right) click on arbitrary content, reported with the point it happened at
//! (feature 012, BUG-005, FR-010b).
//!
//! # Why this exists as a primitive
//!
//! One widget in this application already answers a right click — `material::terminal_pane` — and
//! it cannot be reused, because the handling is fused into a bespoke widget that draws a terminal
//! grid. There was no way to give a *button* a context menu, and that is the whole of BUG-005's fix:
//! a terminal instance's restart affordance moves off its tab, where a fixed tab width had squeezed
//! it to nothing, and onto a menu the tab opens.
//!
//! It lives in the `cdk` rather than in `material` because it holds no appearance at all — it draws
//! nothing of its own, it names no colour and no size, and it delegates every rendering method to
//! its child. `tests/cdk_no_appearance.rs` enforces that boundary, and this file is on the side of
//! it that behaviour belongs to.
//!
//! # What it does, and what it deliberately does not
//!
//! It intercepts exactly one thing: `ButtonPressed(Right)` while the cursor is over its own bounds.
//! Everything else — a primary press, a hover, a scroll, the keyboard — passes to the child
//! untouched and unexamined. That matters more than it sounds: the tab this wraps *is* a button, and
//! its own `on_press` is what selects the instance. A wrapper that captured presses generally would
//! break selecting a tab in order to give it a menu.
//!
//! It does not decide what the menu contains, where it is drawn, or when it closes. It publishes a
//! message built from the press point and stops there; the surface, its anchoring and its dismissal
//! belong to [`super::overlay`], which already owns all three.
//!
//! The point is in **window** coordinates, taken from the cursor rather than from the widget's own
//! bounds, because that is what a menu anchor needs — `overlay::Anchor::Point` positions in the same
//! space. Deriving it from the layout instead would anchor every menu at its tab's corner, which is
//! not where the user clicked.
//!
//! **Not the cursor this widget is handed, though, when it sits in a scrolled viewport.** A
//! scrollable gives its content the cursor in *content* coordinates — the window point plus the
//! offset — so inside one the handed cursor is the scrolled distance away from the click. The
//! pointer's own events are never translated, so the area remembers the window point the last
//! `CursorMoved` carried and reports that (002 BUG-005: a right-click on a scrolled switcher row
//! opened its menu at the window's bottom edge). The handed cursor still decides *whether* the
//! press is over this area — in content coordinates, which is what its bounds are in.

use crate::cdk::passthrough::forward_to_child;
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{mouse, Clipboard, Layout, Shell};
use iced::{Element, Event, Rectangle};

/// What a press is turned into: a message built from the press point, in window pixels.
type OnPress<'a, M> = Box<dyn Fn((u16, u16)) -> M + 'a>;

/// What a press is turned into when the control's own box matters: a message built from its
/// bottom-left corner, in window pixels.
type OnBounds<'a, M> = Box<dyn Fn((u16, u16)) -> M + 'a>;

/// Wraps `content` and reports a secondary click on it.
///
/// Builder form (Principle VIII): `ContextArea::new(content).on_secondary_press(|(x, y)| msg)`.
pub struct ContextArea<'a, M> {
    content: Element<'a, M>,
    on_secondary_press: Option<OnPress<'a, M>>,
    on_primary_press: Option<OnPress<'a, M>>,
    on_primary_bounds: Option<OnBounds<'a, M>>,
}

impl<'a, M: 'a> ContextArea<'a, M> {
    /// An area over `content` that reports nothing until [`Self::on_secondary_press`] is given.
    pub fn new(content: impl Into<Element<'a, M>>) -> Self {
        Self {
            content: content.into(),
            on_secondary_press: None,
            on_primary_press: None,
            on_primary_bounds: None,
        }
    }

    /// Publish `f(corner)` when a primary (left) button is pressed over the content, where `corner`
    /// is the content's **bottom-left** corner in window pixels rather than the press point.
    ///
    /// For a trigger whose panel hangs directly below it, aligned with its leading edge, wherever
    /// inside the trigger the click landed (BUG-1023). Like [`Self::on_primary_press`] it does not
    /// capture, and it arrives before the child's own release-time message. The corner is the
    /// layout's, so the area must not sit in a scrolled viewport.
    pub fn on_primary_bounds(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_primary_bounds = Some(Box::new(f));
        self
    }

    /// Publish `f(point)` when a secondary (right) button is pressed over the content.
    ///
    /// `point` is in window coordinates — what an overlay anchor takes.
    pub fn on_secondary_press(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_secondary_press = Some(Box::new(f));
        self
    }

    /// Publish `f(point)` when a **primary** (left) button is pressed over the content — *in
    /// addition to* whatever the content does with that press, which is left untouched.
    ///
    /// This is the reporting half of a control that opens a surface: the child button says *what*
    /// was pressed and this says *where*, so the panel can hang from the press point rather than
    /// from a figure written into the view (018 BUG-008, FR-029d). It does not capture, because
    /// the press belongs to the child; a wrapper that swallowed it would leave the button inert
    /// and, being an ordinary enabled button, still drawn as though it were not.
    ///
    /// The two messages arrive from one click, **this one first**: a press and its release are
    /// separate events, this is published on the press, and the child button — an ordinary iced
    /// button — publishes on the release. So a reducer cannot treat the point as a correction
    /// applied after the surface opens; it has to hold it and read it back when the open arrives.
    /// Feature 026's start menu had it the other way round and opened at the window origin (T089).
    pub fn on_primary_press(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_primary_press = Some(Box::new(f));
        self
    }
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for ContextArea<'a, M> {
    // `operate` is delegated whole: focus traversal must reach the tab through this wrapper, or
    // wrapping a control would quietly remove it from the keyboard's order (feature 023).
    forward_to_child!(content, M, iced::Theme, iced::Renderer;
        children, diff, size, layout, draw, mouse_interaction, operate, overlay);

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Pointer>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Pointer::default())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        // The child answers first, and always. A primary press on the tab has to reach the tab's own
        // `on_press`, and a press landing on the close control nested inside it has to reach that —
        // neither is this wrapper's business, and inserting itself ahead of them is how a wrapper
        // silently steals a click.
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let pointer = tree.state.downcast_mut::<Pointer>();
        if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            pointer.window = Some(*position);
        }
        let over = cursor
            .position_over(layout.bounds())
            .map(|handed| pointer.window.unwrap_or(handed));
        if let Some(build) = &self.on_primary_press {
            if let Some(point) = reported_press(event, mouse::Button::Left, over) {
                // Not captured: the child's own `on_press` is what this press is for, and it has
                // already run. This only says where it landed.
                shell.publish(build(point));
            }
        }
        if let Some(build) = &self.on_primary_bounds {
            if let Some(corner) = reported_corner(event, over.is_some(), layout.bounds()) {
                shell.publish(build(corner));
            }
        }
        let Some(build) = &self.on_secondary_press else {
            return;
        };
        // A nested area answers first and captures: this one is "behind" it and stays silent, or an
        // Overview pane would dismiss the menu a card's own area has just opened (feature 972).
        if shell.is_event_captured() {
            return;
        }
        let Some(point) = reported_press(event, mouse::Button::Right, over) else {
            return;
        };
        shell.publish(build(point));
        // Captured so the press does not also travel to whatever is behind — an ancestor area, or
        // the terminal pane's own context menu, which would otherwise open a second menu under the
        // first for the same click.
        shell.capture_event();
    }
}

/// What the area remembers between events: where the pointer last was, in window pixels.
///
/// `None` until the first `CursorMoved`, and then the handed cursor is used as before — outside a
/// scrolled viewport the two agree anyway.
#[derive(Debug, Default)]
struct Pointer {
    window: Option<iced::Point>,
}

/// The whole decision this widget makes: which events become a report, and at what point.
///
/// Pulled out as a value function rather than left inline, following `material::resize_handle`'s
/// `step` — a widget's `update` needs a tree, a layout, a renderer and a shell to call at all, and
/// the rule it applies is four lines that deserve tests of their own. `over` is the cursor's window
/// position when it is inside this area's bounds, and `None` when it is not.
///
/// Only a *press*, and only `button`. A release would fire after the menu is already open and
/// after the surface beneath the cursor has changed; the press is the platform convention and the
/// only moment the point still means the thing that was clicked.
fn reported_press(
    event: &Event,
    button: mouse::Button,
    over: Option<iced::Point>,
) -> Option<(u16, u16)> {
    if !matches!(event, Event::Mouse(mouse::Event::ButtonPressed(b)) if *b == button) {
        return None;
    }
    over.map(|p| (p.x as u16, p.y as u16))
}

/// The bottom-left corner of `bounds` when `event` is a primary press and `over` is true.
fn reported_corner(event: &Event, over: bool, bounds: Rectangle) -> Option<(u16, u16)> {
    if !over
        || !matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        )
    {
        return None;
    }
    Some((bounds.x as u16, (bounds.y + bounds.height) as u16))
}

impl<'a, M: 'a> From<ContextArea<'a, M>> for Element<'a, M> {
    fn from(area: ContextArea<'a, M>) -> Self {
        Element::new(area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    #[test]
    fn a_primary_press_over_the_area_reports_its_bottom_left_corner() {
        let bounds = Rectangle::new(Point::new(300.5, 12.0), iced::Size::new(120.0, 40.0));
        assert_eq!(
            reported_corner(&left_press(), true, bounds),
            Some((300, 52))
        );
        assert_eq!(reported_corner(&left_press(), false, bounds), None);
        assert_eq!(reported_corner(&right_press(), true, bounds), None);
    }

    fn right_press() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
    }

    fn left_press() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    #[test]
    fn a_secondary_press_over_the_area_reports_where_it_landed() {
        assert_eq!(
            reported_press(
                &right_press(),
                mouse::Button::Right,
                Some(Point::new(742.0, 761.5))
            ),
            Some((742, 761))
        );
    }

    /// A press outside is somebody else's press. The tab strip is a row of small targets side by
    /// side, so an area that reported presses it did not receive would open the wrong tab's menu.
    #[test]
    fn a_secondary_press_outside_reports_nothing() {
        assert_eq!(
            reported_press(&right_press(), mouse::Button::Right, None),
            None
        );
    }

    /// The property the tab depends on: a primary press is not a *secondary* report, so the button
    /// this wraps still selects its instance. A wrapper that captured presses generally would trade
    /// selecting a tab for giving it a menu.
    #[test]
    fn a_primary_press_is_not_a_secondary_report() {
        assert_eq!(
            reported_press(
                &left_press(),
                mouse::Button::Right,
                Some(Point::new(1.0, 1.0))
            ),
            None
        );
    }

    /// The other half, for the control that opens a surface from a left press (feature 026's
    /// split action): the same rule, asked about the other button, and still only over the area.
    #[test]
    fn a_primary_press_over_the_area_reports_where_it_landed() {
        assert_eq!(
            reported_press(
                &left_press(),
                mouse::Button::Left,
                Some(Point::new(31.0, 208.9))
            ),
            Some((31, 208))
        );
        assert_eq!(
            reported_press(&left_press(), mouse::Button::Left, None),
            None
        );
        assert_eq!(
            reported_press(
                &right_press(),
                mouse::Button::Left,
                Some(Point::new(1.0, 1.0))
            ),
            None
        );
    }

    /// And nothing else is either — a release, a move, a scroll, a keystroke. Named because the
    /// release is the plausible mistake: it is the half of a click that feels like "the click", and
    /// by the time it arrives the menu is open and the point no longer names what was under it.
    #[test]
    fn only_a_press_reports() {
        let over = Some(Point::new(1.0, 1.0));
        assert_eq!(
            reported_press(
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
                mouse::Button::Right,
                over
            ),
            None
        );
        assert_eq!(
            reported_press(
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(1.0, 1.0)
                }),
                mouse::Button::Right,
                over
            ),
            None
        );
    }
}
