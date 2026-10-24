//! `BusyIndicator` — a circular indeterminate indicator beside a text label (feature 941, T053,
//! FR-010).
//!
//! A structured session's pane says whether Claude is answering. Colour alone cannot say it (a
//! person who does not see the hue, or a screenshot in grey, loses it), so the indicator always
//! carries its words: "Claude is answering…" while busy, "Ready" or whatever the caller names
//! while idle. Busy, a ring of dots turns beside the words — Material's circular indeterminate
//! indicator, which says "working" and claims no fraction of the work done, drawn in quads like
//! every other part of this library. Idle, the ring is not drawn and nothing asks for a frame, but
//! its room is kept, so the words do not move when the session changes state.
//!
//! The turn is driven through [`Progress`], the one sanctioned frame-request path, the way
//! `StageProgress`'s bar travels.
//!
//! Builder form: `BusyIndicator::new("Claude is answering…", roles).busy(true).into()`.

use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Layout, Shell};
use iced::widget::{row, Space};
use iced::{Alignment, Border, Element, Event, Length, Rectangle, Size};
use micold_core::tokens::{motion::duration, spacing, Roles};
use std::time::Duration;

use crate::ui::cdk::motion::Progress;
use crate::ui::material::style;
use crate::ui::material::{Text, TypeRole};

/// The ring's box: a glyph's 24dp, so it sits on the label's line as an icon would.
pub const SIZE: f32 = 24.0;

/// The dots around the ring.
const DOTS: usize = 8;

/// One dot's diameter.
const DOT: f32 = 4.0;

/// How long one turn of the ring takes: twice §7.9's `long_2`, a calm pace at this size.
const TURN: Duration = Duration::from_millis(2 * duration::LONG_2);

/// The faintest a trailing dot is drawn, so the ring reads as a ring at every moment.
const FAINTEST: f32 = 0.2;

/// A busy or idle state in words, with a turning ring while busy.
pub struct BusyIndicator {
    label: String,
    busy: bool,
    roles: Roles,
}

impl BusyIndicator {
    /// An indicator saying `label`, busy until [`Self::busy`] says otherwise, themed by `roles`.
    pub fn new(label: impl Into<String>, roles: Roles) -> Self {
        Self {
            label: label.into(),
            busy: true,
            roles,
        }
    }

    /// Whether the ring turns; idle, only the words are drawn (FR-010).
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
}

impl<'a, M: 'a> From<BusyIndicator> for Element<'a, M> {
    fn from(b: BusyIndicator) -> Self {
        let r = b.roles;
        let ring: Element<'a, M> = if b.busy {
            Ring { roles: r }.into()
        } else {
            Space::new().width(SIZE).height(SIZE).into()
        };
        let label = Text::new(b.label, TypeRole::Label, r)
            .tint(if b.busy {
                r.on_surface
            } else {
                r.on_surface_variant
            })
            .wrapping(iced::widget::text::Wrapping::None);
        row![ring, label]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .into()
    }
}

/// The opacity of the dot `index` places behind the head at `phase` (0 → 1 one turn): the head is
/// whole, each dot behind it fainter, down to [`FAINTEST`].
pub(super) fn dot_alpha(index: usize, phase: f32) -> f32 {
    let head = (phase.rem_euclid(1.0) * DOTS as f32).floor() as usize % DOTS;
    let behind = (head + DOTS - index) % DOTS;
    1.0 - (1.0 - FAINTEST) * behind as f32 / (DOTS - 1) as f32
}

/// The turning ring alone, for a component that sets its own words beside it (feature 1124: a
/// status bar item draws the ring before its shortened text).
pub(super) fn ring<'a, M: 'a>(roles: Roles) -> Element<'a, M> {
    Ring { roles }.into()
}

/// The turning ring.
struct Ring {
    roles: Roles,
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for Ring
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Progress>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Progress::new(0.0))
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(SIZE), Length::Fixed(SIZE))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fixed(SIZE), Length::Fixed(SIZE), Size::ZERO))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        _viewport: &Rectangle,
    ) {
        let phase = tree.state.downcast_mut::<Progress>();
        phase.on_frame(event, 1.0, TURN, shell);
        // Loop, as `StageProgress`'s bar does: an arrived track asks for nothing, so the restart
        // asks for the frame that begins the next turn.
        if phase.value() >= 1.0 {
            phase.restart_at(0.0);
            phase.aim(1.0, shell);
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let phase = tree.state.downcast_ref::<Progress>().value();
        let centre = bounds.center();
        let radius = (SIZE - DOT) / 2.0 - 2.0;
        let colour = style::color(self.roles.primary);
        for index in 0..DOTS {
            let angle = std::f32::consts::TAU * index as f32 / DOTS as f32;
            let (x, y) = (
                centre.x + radius * angle.sin() - DOT / 2.0,
                centre.y - radius * angle.cos() - DOT / 2.0,
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x,
                        y,
                        width: DOT,
                        height: DOT,
                    },
                    border: Border {
                        radius: (DOT / 2.0).into(),
                        ..Border::default()
                    },
                    ..Default::default()
                },
                iced::Background::Color(colour.scale_alpha(dot_alpha(index, phase))),
            );
        }
    }
}

impl<'a, M: 'a, Theme: 'a, Renderer> From<Ring> for Element<'a, M, Theme, Renderer>
where
    Renderer: renderer::Renderer + 'a,
{
    fn from(ring: Ring) -> Self {
        Element::new(ring)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, roles, size_of, Mounted};
    use super::*;

    fn words(b: BusyIndicator) -> Vec<String> {
        Mounted::new(b)
            .painted_text()
            .into_iter()
            .map(|p| p.content)
            .collect()
    }

    /// FR-010: the words are drawn busy and idle, in both schemes.
    #[test]
    fn the_label_is_drawn_busy_and_idle() {
        for r in [roles(), dark_roles()] {
            for busy in [true, false] {
                let drawn = words(BusyIndicator::new("Claude is answering…", r).busy(busy));
                assert_eq!(
                    drawn,
                    vec!["Claude is answering…".to_string()],
                    "busy {busy}"
                );
            }
        }
    }

    /// Idle keeps the ring's room, so the words do not move between states.
    #[test]
    fn idle_keeps_the_rings_room() {
        let room = Size::new(400.0, 100.0);
        let busy = size_of::<()>(BusyIndicator::new("Ready", roles()).into(), room);
        let idle = size_of::<()>(
            BusyIndicator::new("Ready", roles()).busy(false).into(),
            room,
        );
        assert_eq!(busy, idle);
        assert!(busy.width > SIZE && busy.height >= SIZE);
    }

    /// The head dot is whole and the one behind it the faintest; the head moves with the phase.
    #[test]
    fn the_head_turns_with_the_phase() {
        assert_eq!(dot_alpha(0, 0.0), 1.0);
        assert!(
            (dot_alpha(1, 0.0) - FAINTEST).abs() < 1e-6,
            "just ahead is the tail"
        );
        assert_eq!(dot_alpha(4, 0.5), 1.0);
        assert!(dot_alpha(3, 0.5) > dot_alpha(2, 0.5));
    }
}
