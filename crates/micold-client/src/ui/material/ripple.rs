//! `Ripple` — Material's press indication, drawn (feature 018, T032 — FR-024a, FR-024b, FR-024c).
//!
//! Wraps any element and draws an expanding circle from wherever it was pressed. The behaviour —
//! origin, expansion, strength, lifetime — is [`cdk::ripple::Ripple`]; this is only the drawing and
//! the colour, which is the split the two layers exist to make.
//!
//! A call site does not opt in per press. Wrapping is the opt-in, so every instance of a wrapped
//! component ripples and none of them carries a flag for it (FR-024c).
//!
//! # Two deliberate departures from the contract, both about what the renderer can express
//!
//! **Drawn above the content, not beneath it.** Contract §5.1 places the ripple above the container
//! and below the content. A wrapper can only draw before its child (which the child's own
//! background then covers) or after it — there is no seam between a widget's background and its
//! label from outside that widget. Reaching one would mean restructuring every wrapped component to
//! expose its interior, which is a far larger change than the difference it buys: the layer is
//! drawn at the pressed opacity (0.10), so text under it stays fully legible and is tinted only
//! while the circle passes.
//!
//! **Clipped to the element's rectangle, not its shape.** `start_layer` clips to a rectangle; the
//! renderer has no rounded-rectangle clip. On a pill-shaped button the circle can therefore reach a
//! few pixels into the corner outside the fill. At 10% opacity over the surface behind it this is
//! very hard to see, and the alternative — drawing the ripple as a rounded quad the size of the
//! element — would stop it being a circle at all.
//!
//! Both are recorded here rather than in a note somewhere, because the next person to look will be
//! asking exactly why it is not what §5.1 says.

use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Vector};
use micold_core::tokens::{motion::duration, state};
use std::time::Duration;

use crate::ui::cdk::ripple::Ripple as RippleState;

/// How long the circle takes to cover the element, and how long it then fades (contract §5.1).
///
/// Named here rather than in the behaviour layer: a motion token is part of the design system, and
/// `cdk` names nothing from it.
const EXPAND: Duration = Duration::from_millis(duration::MEDIUM_2);
const FADE: Duration = Duration::from_millis(duration::SHORT_4);

/// `content` with Material's press indication.
///
/// ```ignore
/// Ripple::new(button, roles.on_surface).into()
/// ```
pub struct Ripple<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, M, Theme, Renderer>,
    /// The element's own content colour — the ripple is that colour at the pressed opacity, which
    /// is what makes it read as a state layer rather than as a decoration.
    tint: Color,
}

impl<'a, M> Ripple<'a, M> {
    /// Wrap `content`, rippling in `tint` — the content colour of the surface being pressed.
    pub fn new(content: impl Into<Element<'a, M>>, tint: micold_core::tokens::Rgb) -> Self {
        Self {
            content: content.into(),
            tint: super::style::color(tint),
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Ripple<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<RippleState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(RippleState::new())
    }

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
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        // A press starts a ripple from where it landed. `position_in` is element-relative — the
        // frame the geometry and the drawing below both work in. `position()` would be absolute
        // window coordinates and would place the origin outside the element entirely (FR-024g).
        if let Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) = event {
            if cursor.position_over(bounds).is_some() {
                let state = tree.state.downcast_mut::<RippleState>();
                state.press(cursor.position_in(bounds), bounds.size());
                // No `request_redraw` here, and not only because 017's gate forbids one outside the
                // motion primitive. Handling this event already causes a redraw, and the `on_frame`
                // below sees that redraw and asks `Progress` for the next — so the animation chains
                // itself from the press for free. A direct request would be a second thing capable
                // of holding the render loop awake, which is what FR-039e keeps to one.
            }
        }

        {
            let state = tree.state.downcast_mut::<RippleState>();
            if !state.is_idle() {
                state.on_frame(event, EXPAND, FADE, shell);
            }
        }

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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
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

        let state = tree.state.downcast_ref::<RippleState>();
        let Some(origin) = state.origin() else {
            return;
        };
        let radius = state.radius();
        if radius <= 0.0 {
            return;
        }
        // The state-layer opacity, faded out over the ripple's tail. The behaviour layer supplies a
        // fraction and this supplies the opacity, so neither knows the other's business.
        let alpha = state::PRESSED * state.strength();
        if alpha <= 0.0 {
            return;
        }

        let bounds = layout.bounds();
        // A circle is a quad of side 2r with a radius of r. Cheaper than the canvas facility and it
        // needs no separate render pass.
        let circle = Rectangle {
            x: bounds.x + origin.x - radius,
            y: bounds.y + origin.y - radius,
            width: radius * 2.0,
            height: radius * 2.0,
        };
        renderer.with_layer(bounds, |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: circle,
                    border: Border {
                        radius: radius.into(),
                        ..Border::default()
                    },
                    ..Default::default()
                },
                iced::Background::Color(Color {
                    a: alpha,
                    ..self.tint
                }),
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: 'a> From<Ripple<'a, M>> for Element<'a, M> {
    fn from(r: Ripple<'a, M>) -> Self {
        Element::new(r)
    }
}
