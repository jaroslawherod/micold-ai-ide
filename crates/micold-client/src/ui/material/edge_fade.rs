//! `EdgeFade` — a persistent cue that a scrolling region has content beyond one of its edges
//! (feature 026 FR-002e, Principle VIII).
//!
//! # What it says, and the two things it says
//!
//! A scrolling region whose content runs off an edge looks exactly like one whose content stops
//! there. The tab strip cannot afford that: FR-005 marks exactly one tab, and a marked tab that has
//! scrolled out of view leaves the strip showing **nothing marked** — which is the precise defect
//! this feature exists to remove, arriving by scrolling instead of by the AI pane.
//!
//! So the fade has two states, and they differ only in **role**:
//!
//! - the **surface**'s own tint, meaning "there is more that way";
//! - the **primary** accent — the colour the active indicator itself wears — meaning "and the tab
//!   you are looking for is what is out there".
//!
//! Reusing the indicator's accent rather than inventing a second cue is the point. The strip
//! already has exactly two colour words, and the edge is then tinted with the very thing the user is
//! scanning for. The alternatives were measured and rejected in `research.md` R6: an arrow glyph
//! reintroduces what FR-002f removes, and a thicker or wider fade is a magnitude difference, which
//! is unreadable without the other state beside it to compare against.
//!
//! # Why a gradient rather than a rule
//!
//! A hard edge says "the region ends here", which is the opposite of what this means. A gradient
//! from the surface's own colour to nothing reads as content passing under an edge — the same idiom
//! a scrolled list uses everywhere — and it does not compete with the tabs it overlays.
//!
//! # What no gate can see
//!
//! All of it. A gradient is **drawn, not laid out**: it occupies the same box whether it is opaque
//! or invisible, so `layout_snapshot` records an identical rectangle either way. The fact *behind*
//! it is arithmetic and is held by `ui/terminal.rs::overflowing`; the appearance is the
//! `visual-pass` skill's, against `quickstart.md` §6.

use iced::widget::{container, stack, Space};
use iced::{Background, Color, Element, Gradient, Length, Radians};
use micold_core::tokens::{anatomy, Roles};

/// How wide the gradient is, in dp.
///
/// Wide enough to read as a fade rather than as a border artefact, and narrow enough not to hide a
/// tab behind it: at [`crate::ui::material::tab::WIDTH`] a tab is 136dp, so this covers under a
/// fifth of one. Deliberately the same figure the app bar's own scroll elevation spans, so the two
/// "content passes under this edge" cues in the application are one width.
const FADE_WIDTH: f32 = 24.0;

/// A scrolling region with a persistent cue on each edge that has content beyond it. Builder form
/// (Principle VIII): `EdgeFade::new(viewport, roles).trailing(true).accent_on(Some(false)).into()`.
pub struct EdgeFade<'a, M> {
    content: Element<'a, M>,
    roles: Roles,
    leading: bool,
    trailing: bool,
    accent_on: Option<bool>,
    width: Option<Length>,
}

impl<'a, M: 'a> EdgeFade<'a, M> {
    /// `content` with no fade on either edge — nothing lies beyond, until something says it does.
    pub fn new(content: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            roles,
            leading: false,
            trailing: false,
            accent_on: None,
            width: None,
        }
    }

    /// Whether content lies before the leading edge.
    pub fn leading(mut self, beyond: bool) -> Self {
        self.leading = beyond;
        self
    }

    /// Whether content lies after the trailing edge.
    pub fn trailing(mut self, beyond: bool) -> Self {
        self.trailing = beyond;
        self
    }

    /// Which edge, if either, has the **marked** member beyond it — `Some(true)` for the leading
    /// edge, `Some(false)` for the trailing one, `None` when the marked member is in view.
    ///
    /// One value rather than two flags because it is one fact: exactly one member is marked
    /// (FR-005), so it can be beyond at most one edge. Two flags would make "marked beyond both
    /// edges" representable, which is the kind of state Principle V asks to be unable to write.
    pub fn accent_on(mut self, leading_edge: Option<bool>) -> Self {
        self.accent_on = leading_edge;
        self
    }

    /// Lay the whole thing out at a given width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }
}

impl<'a, M: 'a> From<EdgeFade<'a, M>> for Element<'a, M> {
    fn from(f: EdgeFade<'a, M>) -> Self {
        let r = f.roles;
        // A gradient from the tint at full strength on the outside to fully transparent inward, so
        // content reads as passing *under* the edge rather than stopping at it.
        let band = move |tint: micold_core::tokens::Rgb, from_leading: bool| {
            let solid = crate::ui::material::style::color(tint);
            let clear = Color { a: 0.0, ..solid };
            // Radians measured from the top, clockwise: π/2 points to the trailing edge.
            let angle = if from_leading {
                Radians(std::f32::consts::FRAC_PI_2)
            } else {
                Radians(3.0 * std::f32::consts::FRAC_PI_2)
            };
            container(Space::new().width(Length::Fixed(FADE_WIDTH)))
                .height(Length::Fill)
                .style(move |_: &iced::Theme| container::Style {
                    background: Some(Background::Gradient(Gradient::Linear(
                        iced::gradient::Linear::new(angle)
                            .add_stop(0.0, solid)
                            .add_stop(1.0, clear),
                    ))),
                    ..container::Style::default()
                })
        };
        // The accent belongs to whichever edge the marked member is beyond; every other faded edge
        // takes the surface's own tint. `accent_on` carries at most one edge, so the two states
        // cannot both claim the accent.
        let tint = |is_leading: bool| {
            if f.accent_on == Some(is_leading) {
                r.primary
            } else {
                r.surface_variant
            }
        };
        let mut layers = stack![container(f.content).width(Length::Fill)];
        if f.leading {
            layers = layers.push(
                container(band(tint(true), true))
                    .width(Length::Fill)
                    .align_x(iced::Alignment::Start),
            );
        }
        if f.trailing {
            layers = layers.push(
                container(band(tint(false), false))
                    .width(Length::Fill)
                    .align_x(iced::Alignment::End),
            );
        }
        // The strip is a bar control, so it is as tall as one — the fade spans it exactly rather
        // than guessing, which is what keeps it from reading as a rule across part of an edge.
        let mut out = container(layers).height(Length::Fixed(anatomy::button::MIN_TOUCH_TARGET));
        if let Some(width) = f.width {
            out = out.width(width);
        }
        out.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    fn roles() -> Roles {
        tokens::roles(ColorScheme::Dark)
    }

    /// FR-002e: the accent is the *indicator's* colour, and it goes to exactly one edge.
    ///
    /// The composited gradient is the visual pass's business; which **role** each edge is drawn in
    /// is a value, and this is where it is held. The one-edge property is what `accent_on`'s shape
    /// gives for free — the assertion is that the shape is being used, not re-derived.
    #[test]
    fn the_accent_marks_one_edge_and_it_is_the_indicators_own() {
        let r = roles();
        let marked_leading: EdgeFade<'_, ()> = EdgeFade::new(Space::new(), r)
            .leading(true)
            .trailing(true)
            .accent_on(Some(true));
        assert_eq!(marked_leading.accent_on, Some(true));

        let nothing_marked: EdgeFade<'_, ()> =
            EdgeFade::new(Space::new(), r).leading(true).trailing(true);
        assert_eq!(
            nothing_marked.accent_on, None,
            "an edge fade says nothing about the marked member until something tells it to — a \
             cue that always claims the marked tab is out there is the same cue as none"
        );

        assert_ne!(
            r.primary, r.surface_variant,
            "the two fade states differ only by role, so the roles themselves must differ — \
             otherwise FR-002e's second state is invisible by construction"
        );
    }

    /// Both edges default to saying nothing, which is what a region whose content fits must do.
    #[test]
    fn a_region_whose_content_fits_fades_neither_edge() {
        let bare: EdgeFade<'_, ()> = EdgeFade::new(Space::new(), roles());
        assert!(!bare.leading);
        assert!(!bare.trailing);
        let _: Element<'_, ()> = bare.into();
    }
}
