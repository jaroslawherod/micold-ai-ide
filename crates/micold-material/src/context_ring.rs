//! `ContextRing` — a compact progress ring with a short text beside it (Constitution Principle VIII;
//! feature 1793, R3): the context meter of the top bar, `8% · $1.28`.
//!
//! The ring is a track in `secondary_container` with an arc in `primary` for the share used, or in
//! `error` once [`ContextRing::warning`] is set. With no ratio only the track is drawn (the empty
//! state). The text sits beside the ring and is the first thing to give way when the row is
//! narrow ([`GiveWayRow::hide`]); the tooltip is on the whole control, so what the text said stays
//! reachable once it is gone.
//!
//! Exposed as a chainable builder terminating in `.into()`:
//! `ContextRing::new(roles).ratio(Some(0.08)).text("8% · $1.28").tooltip("…").into()`.

use crate::give_way::GiveWayRow;
use crate::style::color;
use crate::{Text, Tooltip, TooltipPosition, TypeRole};
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{mouse, Element, Length, Point, Radians, Rectangle, Renderer, Theme};
use micold_core::tokens::{Rgb, Roles};

/// The ring's side, in dp.
pub const SIDE: f32 = 20.0;

/// The ring's stroke width, in dp.
pub const STROKE: f32 = 3.0;

/// The space between the ring and its text, in dp.
const GAP: f32 = 6.0;

/// The angle the arc sweeps for a ratio of the whole: `0` at nothing, a full turn at `1` and above.
pub fn sweep(ratio: f32) -> f32 {
    ratio.clamp(0.0, 1.0) * std::f32::consts::TAU
}

/// A progress ring with text. See the module documentation.
pub struct ContextRing {
    roles: Roles,
    ratio: Option<f32>,
    text: Option<String>,
    warning: bool,
    tooltip: Option<String>,
}

impl ContextRing {
    /// An empty ring (the track alone), themed by `roles`.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            ratio: None,
            text: None,
            warning: false,
            tooltip: None,
        }
    }

    /// The share of the whole used, `0.0..=1.0`; `None` draws the empty state.
    pub fn ratio(mut self, ratio: Option<f32>) -> Self {
        self.ratio = ratio;
        self
    }

    /// The text beside the ring.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Draw the arc in the `error` role: the share is near its limit.
    pub fn warning(mut self, warning: bool) -> Self {
        self.warning = warning;
        self
    }

    /// Say on hover what the ring stands for.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// The role the arc is drawn in.
    fn arc_colour(&self) -> Rgb {
        if self.warning {
            self.roles.error
        } else {
            self.roles.primary
        }
    }
}

/// The canvas program that draws the ring.
struct Ring {
    ratio: Option<f32>,
    track: Rgb,
    arc: Rgb,
}

impl<M> canvas::Program<M> for Ring {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (bounds.width.min(bounds.height) - STROKE) / 2.0;
        frame.stroke(
            &Path::circle(centre, radius),
            Stroke::default()
                .with_width(STROKE)
                .with_color(color(self.track)),
        );
        if let Some(ratio) = self.ratio.filter(|r| *r > 0.0) {
            let start = -std::f32::consts::FRAC_PI_2;
            let path = Path::new(|b| {
                b.arc(canvas::path::Arc {
                    center: centre,
                    radius,
                    start_angle: Radians(start),
                    end_angle: Radians(start + sweep(ratio).min(std::f32::consts::TAU - 0.001)),
                });
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(STROKE)
                    .with_color(color(self.arc)),
            );
        }
        vec![frame.into_geometry()]
    }
}

impl<'a, M: 'a> From<ContextRing> for Element<'a, M> {
    fn from(ring: ContextRing) -> Self {
        let r = ring.roles;
        let drawn: Element<'a, M> = canvas::Canvas::new(Ring {
            ratio: ring.ratio,
            track: r.secondary_container,
            arc: ring.arc_colour(),
        })
        .width(Length::Fixed(SIDE))
        .height(Length::Fixed(SIDE))
        .into();
        let mut row = GiveWayRow::new(GAP).keep(drawn);
        if let Some(text) = ring.text {
            row = row.hide(0, Text::new(text, TypeRole::Label, r));
        }
        let whole: Element<'a, M> = row.into();
        match ring.tooltip {
            Some(tip) => Tooltip::new(whole, tip, r)
                .position(TooltipPosition::Bottom)
                .into(),
            None => whole,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, roles, size_of};
    use super::*;
    use iced::Size;

    fn size(ring: ContextRing, width: f32) -> Size {
        size_of::<()>(ring.into(), Size::new(width, 100.0))
    }

    /// T001: the arc sweeps in proportion to the ratio, nothing at 0, a full turn at 1 and past it.
    #[test]
    fn the_arc_sweeps_in_proportion_to_the_ratio() {
        assert_eq!(sweep(0.0), 0.0);
        assert!((sweep(0.5) - std::f32::consts::PI).abs() < 1e-5);
        assert_eq!(sweep(1.0), std::f32::consts::TAU);
        assert_eq!(sweep(7.0), std::f32::consts::TAU, "never past a full turn");
        assert_eq!(sweep(-1.0), 0.0);
    }

    /// T001: the arc is `primary`, `error` when warning; the builder keeps its text and tooltip;
    /// with no ratio the ring is the empty state.
    #[test]
    fn the_builder_sets_ratio_text_warning_and_tooltip() {
        for r in [roles(), dark_roles()] {
            let ring = ContextRing::new(r);
            assert_eq!(ring.ratio, None, "empty until given a ratio");
            assert_eq!(ring.arc_colour(), r.primary);
            let ring = ring
                .ratio(Some(0.08))
                .text("8% · $1.28")
                .tooltip("84,671 of 1,000,000")
                .warning(true);
            assert_eq!(ring.ratio, Some(0.08));
            assert_eq!(ring.text.as_deref(), Some("8% · $1.28"));
            assert_eq!(ring.tooltip.as_deref(), Some("84,671 of 1,000,000"));
            assert_eq!(ring.arc_colour(), r.error, "a warning draws in `error`");
        }
    }

    /// T001: with room the ring has its text beside it; with none only the ring is left; a tooltip
    /// changes no size.
    #[test]
    fn the_text_gives_way_and_the_ring_stays() {
        let r = roles();
        let full = size(
            ContextRing::new(r).ratio(Some(0.5)).text("8% · $1.28"),
            400.0,
        );
        let none = size(ContextRing::new(r).ratio(Some(0.5)), 400.0);
        let squeezed = size(
            ContextRing::new(r).ratio(Some(0.5)).text("8% · $1.28"),
            SIDE,
        );
        assert_eq!(none.width, SIDE);
        assert!(
            full.width > SIDE + GAP,
            "the text is beside the ring: {full:?}"
        );
        assert_eq!(
            squeezed.width, SIDE,
            "room for the ring only: the ring alone"
        );
        let tipped = size(
            ContextRing::new(r)
                .ratio(Some(0.5))
                .text("8% · $1.28")
                .tooltip("tip"),
            400.0,
        );
        assert_eq!(tipped, full);
    }
}
