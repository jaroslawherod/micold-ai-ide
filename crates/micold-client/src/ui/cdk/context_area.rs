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

use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// What a press is turned into: a message built from the press point, in window pixels.
type OnPress<'a, M> = Box<dyn Fn((u16, u16)) -> M + 'a>;

/// Wraps `content` and reports a secondary click on it.
///
/// Builder form (Principle VIII): `ContextArea::new(content).on_secondary_press(|(x, y)| msg)`.
pub struct ContextArea<'a, M> {
    content: Element<'a, M>,
    on_secondary_press: Option<OnPress<'a, M>>,
    on_primary_press: Option<OnPress<'a, M>>,
}

impl<'a, M: 'a> ContextArea<'a, M> {
    /// An area over `content` that reports nothing until [`Self::on_secondary_press`] is given.
    pub fn new(content: impl Into<Element<'a, M>>) -> Self {
        Self {
            content: content.into(),
            on_secondary_press: None,
            on_primary_press: None,
        }
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
    /// The two messages arrive from one press, child first, and iced applies every queued message
    /// before it draws again — so the surface is never rendered at the point the opening message
    /// left it.
    pub fn on_primary_press(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_primary_press = Some(Box::new(f));
        self
    }
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for ContextArea<'a, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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

        let over = cursor.position_over(layout.bounds());
        if let Some(build) = &self.on_primary_press {
            if let Some(point) = reported_press(event, mouse::Button::Left, over) {
                // Not captured: the child's own `on_press` is what this press is for, and it has
                // already run. This only says where it landed.
                shell.publish(build(point));
            }
        }
        let Some(build) = &self.on_secondary_press else {
            return;
        };
        let Some(point) = reported_press(event, mouse::Button::Right, over) else {
            return;
        };
        shell.publish(build(point));
        // Captured so the press does not also travel to whatever is behind — an ancestor area, or
        // the terminal pane's own context menu, which would otherwise open a second menu under the
        // first for the same click.
        shell.capture_event();
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        // Delegated whole. Focus traversal must reach the tab through this wrapper, or wrapping a
        // control would quietly remove it from the keyboard's order (feature 023).
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }

    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }
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

impl<'a, M: 'a> From<ContextArea<'a, M>> for Element<'a, M> {
    fn from(area: ContextArea<'a, M>) -> Self {
        Element::new(area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

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
