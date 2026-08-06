//! `Ripple` — Material's press indication, drawn (feature 018, T032 — FR-024a, FR-024b, FR-024c).
//!
//! Wraps any element and draws an expanding circle from wherever it was pressed. The behaviour —
//! origin, expansion, strength, lifetime — is [`cdk::ripple::Ripple`]; this is only the drawing and
//! the colour, which is the split the two layers exist to make.
//!
//! A call site does not opt in per press. Wrapping is the opt-in, so every instance of a wrapped
//! component ripples and none of them carries a flag for it (FR-024c).
//!
//! # One deliberate departure from the contract
//!
//! **Drawn above the content, not beneath it.** Contract §5.1 places the ripple above the container
//! and below the content. A wrapper can only draw before its child (which the child's own
//! background then covers) or after it — there is no seam between a widget's background and its
//! label from outside that widget. Reaching one would mean restructuring every wrapped component to
//! expose its interior, which is a far larger change than the difference it buys: the layer is
//! drawn at the pressed opacity (0.10), so text under it stays fully legible and is tinted only
//! while the circle passes.
//!
//! # Clipping to the shape, and why the obvious way is wrong
//!
//! This originally clipped the ripple to the element's **bounding rectangle**, on the reasoning
//! that the renderer offers no rounded-rectangle clip and the overhang would be a few corner pixels
//! nobody would notice at 10% opacity. That was wrong, and a screen recording of the worktree
//! sidebar settled it: every rippling surface here is `shape::FULL`, so on a 40dp row the corner
//! radius is 20dp and the "few pixels" are two 20×20 square caps sitting outside a pill. The ripple
//! read as a rectangle sliding across a rounded row — the single most visible thing about it.
//!
//! The fix cannot be a rounded quad. The ripple *is* a circle, and rounding its own corners does
//! not make a circle stop at a pill's edge. Nor can it be a gradient: iced 0.14 has `Linear` only,
//! so there is no radial stop to cut the circle with.
//!
//! What works is that layer clipping is a **scissor** — a hard, per-pixel yes/no, with no
//! antialiasing and no blending. So the same circle can be drawn many times under a set of
//! *disjoint* scissor rectangles that tile the rounded shape, and the union is exactly the circle
//! clipped to that shape: every pixel is covered at most once, so there is no double-blended seam,
//! and the circle's own edge stays antialiased inside each band. [`shape_bands`] computes the
//! tiling. It is pure geometry and tested as such, because "does this rectangle lie inside a
//! rounded rectangle" is checkable arithmetic and not something to confirm by looking at it.

use iced::advanced::widget::{operation::Outcome, tree, Id, Operation, Tree, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Vector};
use micold_core::tokens::{motion::duration, state};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::ui::cdk::ripple::Ripple as RippleState;

/// How long the circle takes to cover the element, and how long it then fades (contract §5.1).
///
/// Named here rather than in the behaviour layer: a motion token is part of the design system, and
/// `cdk` names nothing from it.
const EXPAND: Duration = Duration::from_millis(duration::MEDIUM_2);
const FADE: Duration = Duration::from_millis(duration::SHORT_4);

/// How finely the rounded corners are approximated, as bands per corner.
///
/// Each band is one scissor rectangle, so this is the cost knob: the tiling is at most
/// `2 * BANDS_PER_CORNER + 1` rectangles, and only while a ripple is actually animating on one
/// element. At 8, a 20dp corner is stepped every 2.5dp with a worst-case deviation under half a
/// pixel — below what a 10%-opacity overlay can show.
const BANDS_PER_CORNER: usize = 8;

/// Disjoint rectangles tiling `bounds` rounded by `radius`, every one of them inside the shape.
///
/// The tiling is **conservative**: each band is inset by the widest inset anywhere in its own
/// vertical span, so a band never protrudes past the curve. It can fall a fraction of a pixel short
/// of it instead, which is the right way round — an overhang is the bug this exists to fix, and a
/// sliver of un-tinted pixel at a corner is invisible.
///
/// Bands with the same inset are merged, so a square element collapses to one rectangle and the
/// straight middle of a pill costs one rectangle rather than a dozen.
fn shape_bands(bounds: Rectangle, radius: f32) -> Vec<Rectangle> {
    // A radius over half the shorter side is not expressible — `shape::FULL` is deliberately a huge
    // number meaning "as round as this can be", so it arrives here needing exactly this clamp.
    let limit = (bounds.width.min(bounds.height) / 2.0).max(0.0);
    let r = radius.clamp(0.0, limit);
    if r <= 0.5 || !bounds.width.is_finite() || !bounds.height.is_finite() {
        return vec![bounds];
    }

    // How far the shape's edge sits inside `bounds` at height `y`. Zero along the straight middle.
    let inset_at = |y: f32| {
        let into_top = (bounds.y + r) - y;
        let into_bottom = y - (bounds.y + bounds.height - r);
        let d = into_top.max(into_bottom);
        if d <= 0.0 {
            0.0
        } else {
            // The corner is a quarter circle: at `d` above its centre the edge has moved in by
            // `r - sqrt(r² - d²)`. Clamped because `d` can reach `r` exactly at the very top row.
            r - (r * r - d * d).max(0.0).sqrt()
        }
    };

    let bands = BANDS_PER_CORNER * 2 + BANDS_PER_CORNER.max(1);
    let step = bounds.height / bands as f32;
    let mut out: Vec<Rectangle> = Vec::with_capacity(bands);
    for i in 0..bands {
        let top = bounds.y + i as f32 * step;
        let bottom = if i + 1 == bands {
            bounds.y + bounds.height
        } else {
            top + step
        };
        // The widest inset over the band's whole span, so no part of it leaves the shape. The
        // extremes are at the ends: the inset is monotone within each corner and flat between them.
        let inset = inset_at(top).max(inset_at(bottom));
        let width = bounds.width - inset * 2.0;
        if width <= 0.0 {
            continue;
        }
        // Merge with the previous band when the inset matches, so the straight middle is one rect.
        match out.last_mut() {
            Some(prev) if (prev.width - width).abs() < f32::EPSILON => {
                prev.height = bottom - prev.y;
            }
            _ => out.push(Rectangle {
                x: bounds.x + inset,
                y: top,
                width,
                height: bottom - top,
            }),
        }
    }
    out
}

/// `content` with Material's press indication.
///
/// ```ignore
/// Ripple::new(button, roles.on_surface, shape::FULL).into()
/// ```
pub struct Ripple<'a, M, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, M, Theme, Renderer>,
    /// The element's own content colour — the ripple is that colour at the pressed opacity, which
    /// is what makes it read as a state layer rather than as a decoration.
    tint: Color,
    /// The corner radius of the surface being wrapped, so the ripple can be clipped to its shape.
    radius: f32,
}

impl<'a, M> Ripple<'a, M> {
    /// Wrap `content`, rippling in `tint` — the content colour of the surface being pressed.
    ///
    /// `radius` is the wrapped surface's own corner radius, and is required rather than defaulted
    /// because a wrapper cannot see the shape its child draws: getting it wrong is precisely the
    /// bug this argument exists to prevent, and a default would let a new call site inherit it
    /// silently. Pass the same `shape::*` token the child's style uses.
    pub fn new(
        content: impl Into<Element<'a, M>>,
        tint: micold_core::tokens::Rgb,
        radius: f32,
    ) -> Self {
        Self {
            content: content.into(),
            tint: super::style::color(tint),
            radius,
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

    /// Offers this ripple's own state, then forwards.
    ///
    /// Forwarding is the load-bearing half. The default implementation does nothing, so focus
    /// traversal, `text_input::focus(id)` and `scrollable::scroll_to` would all stop at this
    /// wrapper and silently skip the subtree beneath it — no error, no warning, just a control that
    /// cannot be reached. `animation.rs`'s wrappers each forward for the same reason
    /// (`operate_direct_child!`), and this one returns its child's layout node unchanged, so it
    /// forwards its own layout too.
    ///
    /// Offering its state first is what makes [`pulse`] possible: an operation is how iced reaches
    /// per-widget state, and it is the *only* way to reach a ripple's, because FR-024e puts that
    /// state inside the instance with nothing keeping a list of instances. A traversal visits what
    /// is on screen rather than consulting a registry, so this stays true to the requirement rather
    /// than working around it.
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let tree::State::Some(state) = &mut tree.state {
            operation.custom(None, layout.bounds(), state.as_mut());
        }
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let quad = renderer::Quad {
            bounds: circle,
            border: Border {
                radius: radius.into(),
                ..Border::default()
            },
            ..Default::default()
        };
        let background = iced::Background::Color(Color {
            a: alpha,
            ..self.tint
        });
        // The same circle under each disjoint scissor rectangle. Layer clipping does not blend, so
        // the bands tile into exactly the circle intersected with the element's rounded shape —
        // no seam where they meet, and nothing painted outside the shape.
        //
        // Each band is cut to `viewport` first. A pushed clip *replaces* the enclosing one rather
        // than intersecting with it, so a band that reaches outside the visible region escapes
        // whatever was clipping this element: press a sidebar row that is half-scrolled out of its
        // list and the ripple would paint the row's hidden half over the elements beyond the
        // scrollable's edge. Every iced widget that clips does this same intersection first.
        for band in shape_bands(bounds, self.radius) {
            let Some(visible) = band.intersection(viewport) else {
                continue;
            };
            renderer.with_layer(visible, |renderer| {
                renderer.fill_quad(quad, background);
            });
        }
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

/// Keep one ripple running, and report how many are.
///
/// The reference scene's `full` slot is "the baseline scene plus a ripple mid-animation"
/// (FR-039b, quickstart §B8), and §B8 requires the scene to compose *itself* — nothing is clicked,
/// so nothing differs between runs. A ripple, though, only starts from a press, and the frame probe
/// has no way to press anything.
///
/// So this presses one, through the mechanism iced provides for reaching widget state: a traversal.
/// It visits the ripples that are on screen, which is exactly the set FR-024e refuses to keep a list
/// of — the requirement is that no *registry* exists, not that the tree cannot be walked.
///
/// **One ripple, kept going** — the scene is the baseline plus *a* ripple, singular. Two rules get
/// it there, and the second is easy to leave out:
///
/// - A ripple already running is never pressed again. Re-pressing one every frame holds it at zero
///   expansion for ever, which is a ripple that never gets anywhere rather than one mid-animation.
/// - Nothing is pressed at all unless the previous traversal found nothing animating. Without this
///   the rule degrades to "press whichever one is idle", and since the probe pulses on *every*
///   frame, that starts a second ripple while the first is still running, a third on the frame
///   after, and has every ripple on screen animating within as many frames as there are rows. The
///   scene check cannot see it — it only asks whether *a* ripple is animating — so the figure would
///   be recorded against a screen full of them.
///
/// That is why `found` is read as well as written: it is the previous frame's answer, and the only
/// thing a single traversal can know about ripples it has not reached yet.
///
/// `found` receives how many ripples the traversal saw mid-animation. It is what `Scene::check`
/// reads, and it is *observed* rather than assumed: a run that reported "a ripple is animating"
/// because it had asked for one would record a `full` figure for whatever was actually on screen.
///
/// Reported through a counter rather than as the task's outcome, which is not a detail. An outcome
/// arrives as a message, a message runs an update, and iced composes the view again after every
/// update — so a run that asked for this each frame would compose *twice* per frame, and the probe,
/// which times composition, would count the second one. Those extra compositions are cheaper than a
/// real frame, and the `full` figure came out at half the baseline's: a heavier scene reported as
/// faster, by a measurement the measurement had changed.
pub fn pulse(found: Arc<AtomicUsize>) -> impl Operation<()> {
    struct Pulse {
        /// Whether this traversal is allowed to start one — false while another is still running.
        may_press: bool,
        pressed: bool,
        animating: usize,
        found: Arc<AtomicUsize>,
    }

    impl Operation for Pulse {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn custom(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn std::any::Any) {
            let Some(ripple) = state.downcast_mut::<RippleState>() else {
                return;
            };
            if ripple.is_idle() {
                if self.may_press && !self.pressed {
                    // `None` starts it from the centre — the documented origin for an activation
                    // that carries no pointer position, which is precisely what this is.
                    ripple.press(None, bounds.size());
                    self.pressed = true;
                    self.animating += 1;
                }
            } else {
                self.animating += 1;
            }
        }

        fn finish(&self) -> Outcome<()> {
            self.found.store(self.animating, Ordering::Relaxed);
            Outcome::None
        }
    }

    Pulse {
        // The previous frame's count, which is the only view a single forward traversal has of the
        // ripples it has not visited yet.
        may_press: found.load(Ordering::Relaxed) == 0,
        pressed: false,
        animating: 0,
        found,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference case: a sidebar row. 40dp tall at `shape::FULL`, which is where the original
    /// rectangular clip put two 20×20 square caps outside a pill.
    const ROW: Rectangle = Rectangle {
        x: 10.0,
        y: 20.0,
        width: 400.0,
        height: 40.0,
    };

    /// `shape::FULL` as the tokens spell it — a number far larger than any element, meaning "as
    /// round as this shape can be".
    const FULL: f32 = 9999.0;

    /// Is `(x, y)` inside `bounds` rounded by `radius`? The predicate the tiling must respect.
    fn inside(bounds: Rectangle, radius: f32, x: f32, y: f32) -> bool {
        let r = radius.clamp(0.0, bounds.width.min(bounds.height) / 2.0);
        if x < bounds.x || y < bounds.y {
            return false;
        }
        if x > bounds.x + bounds.width || y > bounds.y + bounds.height {
            return false;
        }
        let cx = if x < bounds.x + r {
            bounds.x + r
        } else if x > bounds.x + bounds.width - r {
            bounds.x + bounds.width - r
        } else {
            return true;
        };
        let cy = if y < bounds.y + r {
            bounds.y + r
        } else if y > bounds.y + bounds.height - r {
            bounds.y + bounds.height - r
        } else {
            return true;
        };
        // Half a pixel of slack: this checks the tiling did not *overhang*, not that it landed on
        // the curve to the last bit of float precision.
        (x - cx).hypot(y - cy) <= r + 0.5
    }

    /// The whole point. Every rectangle the tiling produces lies inside the rounded shape, so
    /// nothing the ripple draws can appear outside the surface being pressed.
    #[test]
    fn no_band_reaches_outside_the_rounded_shape() {
        for radius in [FULL, 20.0, 12.0, 4.0] {
            for band in shape_bands(ROW, radius) {
                for (x, y) in [
                    (band.x, band.y),
                    (band.x + band.width, band.y),
                    (band.x, band.y + band.height),
                    (band.x + band.width, band.y + band.height),
                ] {
                    assert!(
                        inside(ROW, radius, x, y),
                        "at radius {radius} a band corner ({x}, {y}) is outside the shape — this \
                         is the overhang that made the ripple read as a rectangle on a pill"
                    );
                }
            }
        }
    }

    /// Disjoint, or overlapping bands would blend the ripple with itself and draw a seam at every
    /// join — brighter lines across the element, a worse artefact than the one being fixed.
    #[test]
    fn the_bands_do_not_overlap() {
        let bands = shape_bands(ROW, FULL);
        for (i, a) in bands.iter().enumerate() {
            for b in &bands[i + 1..] {
                // A tolerance, because abutting bands are computed from different expressions
                // — one from the band index, the merged one from a running top — and can land an
                // ulp apart. An ulp of overlap is not a seam; a pixel of it would be.
                let overlap = a.y.max(b.y) + 1e-3 < (a.y + a.height).min(b.y + b.height);
                assert!(!overlap, "bands {a:?} and {b:?} overlap vertically");
            }
        }
    }

    /// …and contiguous, or the gaps between them would show as unpainted stripes.
    #[test]
    fn the_bands_tile_the_full_height() {
        let bands = shape_bands(ROW, FULL);
        assert_eq!(bands.first().map(|b| b.y), Some(ROW.y));
        assert_eq!(
            bands.last().map(|b| b.y + b.height),
            Some(ROW.y + ROW.height),
            "the tiling stops short of the bottom edge"
        );
        for pair in bands.windows(2) {
            assert!(
                (pair[0].y + pair[0].height - pair[1].y).abs() < 0.001,
                "a gap between {:?} and {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    /// The tiling covers nearly all of the shape. A conservative tiling that clipped away half the
    /// element would satisfy every rule above and show the ripple as a thin stripe.
    #[test]
    fn the_bands_cover_almost_all_of_the_shape() {
        let covered: f32 = shape_bands(ROW, FULL)
            .iter()
            .map(|b| b.width * b.height)
            .sum();
        // A pill's area: the middle rectangle plus the two semicircular caps.
        let r = ROW.height / 2.0;
        let shape = (ROW.width - 2.0 * r) * ROW.height + std::f32::consts::PI * r * r;
        assert!(
            covered / shape > 0.97,
            "the tiling covers {:.1}% of the pill — the ripple would visibly stop short of its \
             own element",
            100.0 * covered / shape
        );
    }

    /// A square element costs one rectangle. Banding a shape with no corners to follow would be
    /// pure overhead on every frame of every ripple.
    #[test]
    fn a_square_element_is_a_single_band() {
        assert_eq!(shape_bands(ROW, 0.0), vec![ROW]);
        assert_eq!(shape_bands(ROW, 0.4), vec![ROW]);
    }

    /// A shape with a genuine straight middle spends one rectangle on it, not one per band.
    ///
    /// A *pill* has no straight middle — at `shape::FULL` the two corner arcs meet, so every band
    /// differs and none can merge. That is the worst case, and it is bounded below. The merge earns
    /// its keep on everything gentler: a 12dp corner on a 40dp row is mostly straight edge.
    #[test]
    fn a_straight_edge_costs_one_band() {
        let gentle = shape_bands(ROW, 12.0);
        assert!(
            gentle.iter().any(|b| b.height > ROW.height / 4.0),
            "no tall middle band at a 12dp radius, so the straight edge was banded needlessly: \
             {gentle:#?}"
        );
        assert!(
            gentle.len() < shape_bands(ROW, FULL).len(),
            "a gentler corner did not cost fewer bands than a pill"
        );
    }

    /// The cost is bounded, and paid per frame while a ripple animates. A tiling proportional to
    /// the element's height would make a tall surface arbitrarily expensive to press.
    #[test]
    fn the_band_count_is_bounded_by_the_configured_resolution() {
        for height in [24.0, 40.0, 200.0, 800.0] {
            let tall = Rectangle { height, ..ROW };
            let bands = shape_bands(tall, FULL);
            assert!(
                bands.len() <= BANDS_PER_CORNER * 3,
                "{} bands for a {height}dp element — the count follows the element, not the \
                 configured resolution",
                bands.len()
            );
        }
    }

    /// Degenerate geometry occurs for a frame during layout. A zero-sized element must produce
    /// nothing drawable rather than a `NaN` rectangle, which would poison the clip.
    #[test]
    fn degenerate_bounds_do_not_produce_nonsense() {
        for bounds in [
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            Rectangle {
                x: 5.0,
                y: 5.0,
                width: 30.0,
                height: 0.0,
            },
        ] {
            for band in shape_bands(bounds, FULL) {
                assert!(
                    band.x.is_finite()
                        && band.y.is_finite()
                        && band.width.is_finite()
                        && band.height.is_finite(),
                    "{band:?} is not a drawable rectangle"
                );
            }
        }
    }
}
