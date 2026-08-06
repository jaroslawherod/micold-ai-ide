//! `StageProgress` — Material's linear progress indicator, paired with a current-stage label
//! (feature 018, T049/T050 — FR-031e, FR-031f; contract §7.9).
//!
//! # Why it is indeterminate
//!
//! The application does not know how much of a worktree creation is complete: whether the submodule
//! stage runs at all is only known *after* the branch and worktree already exist (research R2). The
//! bar used to sit at a fixed 40% fill, which asserts a completion fraction nothing can back up —
//! a user watching it has every reason to read "40% done", and it will read 40% for the whole
//! operation whether that takes a second or a minute.
//!
//! Material's answer for a duration you cannot measure is the **indeterminate** indicator, whose
//! active segment travels across the track. It says "working" and claims nothing else, which is
//! exactly the amount this application knows.
//!
//! # Frames
//!
//! The travel is driven through [`Progress`], the one sanctioned frame-request path (FR-039e), and
//! the widget only exists while an operation is running — the form drops it when the daemon
//! replies. So nothing animates at rest without this needing a stop condition of its own
//! (SC-017), and `tests/idle_requests_no_frames.rs` stays a single-branch read.

use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Layout, Shell};
use iced::{Border, Element, Event, Length, Rectangle, Size};
use micold_core::tokens::{anatomy, motion::duration, shape, Roles};
use std::time::Duration;

use crate::ui::cdk::motion::Progress;
use crate::ui::material::style;
use crate::ui::material::{Text, TypeRole};

/// How long the active segment takes to cross the track once (§7.9: `long_2` on `standard`).
const TRAVEL: Duration = Duration::from_millis(duration::LONG_2);

/// How much of the track the moving segment covers.
///
/// Material's indeterminate bar uses two segments of varying length; one of a fixed proportion is
/// the readable part of that at this size, and the difference at 4dp tall is not perceptible.
const SEGMENT: f32 = 0.3;

/// A stage-progress indicator: a thin indeterminate bar plus the current stage's plain-language
/// label. Builder form (Principle VIII): `StageProgress::new(label, roles).into()`.
pub struct StageProgress {
    label: String,
    roles: Roles,
}

impl StageProgress {
    /// A progress indicator showing `label` as the current stage, themed by `roles`.
    pub fn new(label: impl Into<String>, roles: Roles) -> Self {
        Self {
            label: label.into(),
            roles,
        }
    }
}

impl<'a, M: 'a> From<StageProgress> for Element<'a, M> {
    fn from(p: StageProgress) -> Self {
        let r = p.roles;
        iced::widget::column![
            Element::from(Bar { roles: r }),
            // The stage label is a plain-language sentence about what is happening, so it is prose:
            // `Caption`, at the body weight (§7.9).
            Text::new(p.label, TypeRole::Caption, r).muted(),
        ]
        .spacing(anatomy::progress::LABEL_GAP)
        .into()
    }
}

/// The bar itself: a `secondary_container` track with a `primary` segment travelling across it.
struct Bar {
    roles: Roles,
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for Bar
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
        Size::new(Length::Fill, Length::Fixed(anatomy::progress::THICKNESS))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            Length::Fill,
            Length::Fixed(anatomy::progress::THICKNESS),
            Size::ZERO,
        ))
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
        phase.on_frame(event, 1.0, TRAVEL, shell);
        // Loop rather than arrive. An arrived `Progress` asks for nothing, so the restart has to
        // ask for the frame that begins the next pass — `aim` does exactly that without stepping
        // it, which is why the travel keeps its full duration every time round.
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
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width <= 0.0 {
            return;
        }
        let r = self.roles;
        let rounded = Border {
            radius: shape::FULL.into(),
            ..Border::default()
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: rounded,
                ..Default::default()
            },
            iced::Background::Color(style::color(r.secondary_container)),
        );

        // The segment enters from the left edge and leaves past the right, so the travel reads as
        // continuous rather than as something restarting in place.
        let phase = tree.state.downcast_ref::<Progress>().value();
        let segment = bounds.width * SEGMENT;
        let x = bounds.x + (bounds.width + segment) * phase - segment;
        let indicator = Rectangle {
            x,
            y: bounds.y,
            width: segment,
            height: bounds.height,
        };
        // Clipped to the track, and cut to the viewport first: a pushed clip *replaces* the
        // enclosing one rather than intersecting with it, so a segment overhanging either end would
        // otherwise paint past whatever is clipping this widget.
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        renderer.with_layer(clip, |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: indicator,
                    border: rounded,
                    ..Default::default()
                },
                iced::Background::Color(style::color(r.primary)),
            );
        });
    }
}

impl<'a, M: 'a, Theme: 'a, Renderer> From<Bar> for Element<'a, M, Theme, Renderer>
where
    Renderer: renderer::Renderer + 'a,
{
    fn from(bar: Bar) -> Self {
        Element::new(bar)
    }
}
