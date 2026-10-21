//! `ProgressBar` — Material's determinate linear progress indicator (feature 813, FR-016): a
//! `primary` fill over the `secondary_container` track covering `done / total` of it.
//!
//! The track and the fill are `StageProgress`'s own (`progress::paint_track`, `paint_fill`), so
//! the two indicators read as one family. Unlike it this one moves nothing and asks for no frame.
//!
//! Two optional steps serve the context meter (feature 944, research R12): `.marker(Ratio)` draws a
//! [`MARKER_WIDTH`] tick across the track at a second ratio (the compaction threshold), and
//! `.warning(true)` fills in the `error` role. The warning colour is never the only signal: the
//! caller shows an icon and text beside the bar (FR-003).

use crate::ui::material::progress::{paint_fill, paint_track, track_color};
use crate::ui::material::style;
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{layout, mouse, renderer, Layout};
use iced::{Element, Length, Rectangle, Size};
use micold_core::task_progress::Ratio;
use micold_core::tokens::{anatomy, Rgb, Roles};

/// How wide the marker's tick is, in dp. Component-owned like `TOOLTIP_MAX_WIDTH`: feature 018's
/// anatomy contract names no marker figure.
pub const MARKER_WIDTH: f32 = 2.0;

/// A determinate bar showing a [`Ratio`]. Builder form (Principle VIII):
/// `ProgressBar::new(ratio, roles).width(Length::Fixed(120.0))`.
pub struct ProgressBar {
    ratio: Ratio,
    roles: Roles,
    width: Length,
    marker: Option<Ratio>,
    warning: bool,
}

impl ProgressBar {
    /// A bar filled `ratio.done` of `ratio.total`, themed by `roles`, as wide as its room.
    pub fn new(ratio: Ratio, roles: Roles) -> Self {
        Self {
            ratio,
            roles,
            width: Length::Fill,
            marker: None,
            warning: false,
        }
    }

    /// Mark `at` on the track with a [`MARKER_WIDTH`] tick, inside the track at every ratio.
    pub fn marker(mut self, at: Ratio) -> Self {
        self.marker = Some(at);
        self
    }

    /// Fill in the `error` role instead of `primary` while `on`.
    pub fn warning(mut self, on: bool) -> Self {
        self.warning = on;
        self
    }

    /// The bar's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

/// The fill's colour: `error` for a warning bar, `primary` otherwise.
pub(crate) fn fill_color(roles: &Roles, warning: bool) -> Rgb {
    if warning {
        roles.error
    } else {
        roles.primary
    }
}

/// Where the marker's tick starts on a track `track` wide: centred on `at`'s share of the track,
/// and moved in so all of it stays on the track.
pub fn marker_x(at: Ratio, track: f32) -> f32 {
    let centre = fill_width(at, track);
    (centre - MARKER_WIDTH / 2.0).clamp(0.0, (track - MARKER_WIDTH).max(0.0))
}

/// How wide the fill is on a track `track` wide: the ratio's share, never more than the track.
pub fn fill_width(ratio: Ratio, track: f32) -> f32 {
    if ratio.total == 0 {
        return 0.0;
    }
    track * (ratio.done.min(ratio.total) as f32 / ratio.total as f32)
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for ProgressBar
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Fixed(anatomy::progress::THICKNESS))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            self.width,
            Length::Fixed(anatomy::progress::THICKNESS),
            Size::ZERO,
        ))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if bounds.width <= 0.0 {
            return;
        }
        paint_track(renderer, bounds, &self.roles);
        let fill = fill_color(&self.roles, self.warning);
        let width = fill_width(self.ratio, bounds.width);
        if width > 0.0 {
            paint_fill(renderer, Rectangle { width, ..bounds }, fill);
        }
        if let Some(at) = self.marker {
            // The tick's part over the fill takes the track's colour, its part over the empty track
            // the fill's, so a tick the fill's edge splits still shows in both halves.
            let x = marker_x(at, bounds.width);
            let end = (x + MARKER_WIDTH).min(bounds.width);
            let split = width.clamp(x, end);
            for (from, to, color) in [(x, split, track_color(&self.roles)), (split, end, fill)] {
                if to > from {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x + from,
                                width: to - from,
                                ..bounds
                            },
                            ..Default::default()
                        },
                        iced::Background::Color(style::color(color)),
                    );
                }
            }
        }
    }
}

impl<'a, M: 'a, Theme: 'a, Renderer> From<ProgressBar> for Element<'a, M, Theme, Renderer>
where
    Renderer: renderer::Renderer + 'a,
{
    fn from(bar: ProgressBar) -> Self {
        Element::new(bar)
    }
}

#[cfg(test)]
mod tests {
    use super::super::style;
    use super::super::test_support::{roles, Mounted};
    use super::*;
    use crate::ui::material::StageProgress;

    const WIDTH: f32 = 200.0;

    /// The middle row of the bar drawn over the surface: one RGB triple per pixel.
    fn middle_row(element: impl Into<Element<'static, String>>) -> Vec<[u8; 3]> {
        let (pixels, width) = Mounted::new(element).drawn(style::color(roles().surface));
        let height = pixels.len() as u32 / 4 / width;
        let row = (height / 2 * width * 4) as usize;
        pixels[row..row + width as usize * 4]
            .chunks(4)
            .map(|p| [p[0], p[1], p[2]])
            .collect()
    }

    fn rgb(color: micold_core::tokens::Rgb) -> [f32; 3] {
        let c = style::color(color);
        [c.r * 255.0, c.g * 255.0, c.b * 255.0]
    }

    /// How many pixels of the row are nearer the fill's colour than the track's.
    fn filled(row: &[[u8; 3]]) -> usize {
        let (fill, track) = (rgb(roles().primary), rgb(roles().secondary_container));
        let apart = |p: &[u8; 3], to: [f32; 3]| -> f32 {
            p.iter()
                .zip(to)
                .map(|(a, b)| (f32::from(*a) - b).abs())
                .sum()
        };
        row.iter()
            .filter(|p| apart(p, fill) < apart(p, track))
            .count()
    }

    fn bar(done: u32, total: u32) -> ProgressBar {
        ProgressBar::new(Ratio { done, total }, roles()).width(Length::Fixed(WIDTH))
    }

    /// US4.2: the fill is as wide as the ratio says.
    #[test]
    fn the_fill_is_proportional_to_the_ratio() {
        for (done, total, expect) in [(0, 10, 0.0), (6, 10, 120.0), (10, 10, 200.0)] {
            let row = middle_row(bar(done, total));
            assert_eq!(row.len(), WIDTH as usize);
            let got = filled(&row) as f32;
            assert!(
                (got - expect).abs() <= 1.0,
                "{done}/{total}: {got} px filled, expected {expect}"
            );
        }
        assert!((fill_width(Ratio { done: 6, total: 10 }, 100.0) - 60.0).abs() < 0.01);
        assert_eq!(
            fill_width(
                Ratio {
                    done: 12,
                    total: 10
                },
                100.0
            ),
            100.0
        );
    }

    /// The track is `StageProgress`'s: same colour, same thickness, same corner rounding.
    #[test]
    fn the_track_is_the_stage_progress_track() {
        let stage = Mounted::new(StageProgress::new("Creating", roles()))
            .drawn(style::color(roles().surface));
        let mine = Mounted::new(bar(0, 10)).drawn(style::color(roles().surface));
        let (stage_pixels, width) = stage;
        let (pixels, mine_width) = mine;
        // The stage's bar is the top of its stack; the pixels of the bar's rows agree. The stage
        // bar fills its room, mine is `WIDTH` wide: compare the first `WIDTH` columns.
        let thickness = anatomy::progress::THICKNESS as usize;
        assert_eq!(mine_width as usize, WIDTH as usize);
        for y in 0..thickness {
            let theirs = &stage_pixels[y * width as usize * 4..][..4 * 4];
            let ours = &pixels[y * mine_width as usize * 4..][..4 * 4];
            assert_eq!(theirs, ours, "row {y}, left end");
        }
        let middle = thickness / 2 * mine_width as usize * 4 + 100 * 4;
        let stage_middle = thickness / 2 * width as usize * 4 + 100 * 4;
        assert_eq!(
            pixels[middle..middle + 4],
            stage_pixels[stage_middle..stage_middle + 4]
        );
        let (h, _) = (pixels.len() as u32 / 4 / mine_width, ());
        assert_eq!(h as f32, anatomy::progress::THICKNESS);
    }

    /// The columns of `row` nearer `a` than `b`.
    fn nearer(
        row: &[[u8; 3]],
        a: micold_core::tokens::Rgb,
        b: micold_core::tokens::Rgb,
    ) -> Vec<usize> {
        let (a, b) = (rgb(a), rgb(b));
        let apart = |p: &[u8; 3], to: [f32; 3]| -> f32 {
            p.iter()
                .zip(to)
                .map(|(x, y)| (f32::from(*x) - y).abs())
                .sum()
        };
        row.iter()
            .enumerate()
            .filter(|(_, p)| apart(p, a) < apart(p, b))
            .map(|(x, _)| x)
            .collect()
    }

    /// 944 FR-002, R12: the marker is a 2 px tick inside the track at its ratio — at 0, in the
    /// middle and at 1. On the empty part of the track it is drawn in the fill's colour; on the
    /// filled part in the track's, so it shows wherever the fill has reached.
    #[test]
    fn a_marker_is_a_two_px_tick_inside_the_track_at_its_ratio() {
        let (fill, track) = (roles().primary, roles().secondary_container);
        for (done, expect) in [(0, 0usize), (8, 159), (10, 198)] {
            let at = Ratio { done, total: 10 };
            let ticks = vec![expect, expect + 1];
            let on_empty = middle_row(bar(0, 10).marker(at));
            assert_eq!(
                nearer(&on_empty, fill, track),
                ticks,
                "{done}/10 on an empty bar"
            );
            let on_full = middle_row(bar(10, 10).marker(at));
            assert_eq!(
                nearer(&on_full, track, fill),
                ticks,
                "{done}/10 on a full bar"
            );
            assert!(
                (marker_x(at, WIDTH) - expect as f32).abs() < 0.01,
                "{done}/10: marker_x"
            );
        }
        assert_eq!(MARKER_WIDTH, 2.0);
    }

    /// 944 FR-003, R12: a warning bar fills in the `error` role, as wide as without the warning.
    #[test]
    fn a_warning_bar_fills_in_the_error_role() {
        let r = roles();
        assert_eq!(fill_color(&r, true), r.error);
        assert_eq!(fill_color(&r, false), r.primary);
        let row = middle_row(bar(6, 10).warning(true));
        let red = nearer(&row, r.error, r.secondary_container);
        assert!(
            (red.len() as f32 - 120.0).abs() <= 1.0,
            "{} px in the error role, expected 120",
            red.len()
        );
        assert!(
            nearer(&row[..red.len()], r.primary, r.error).is_empty(),
            "no filled pixel is in the primary role"
        );
    }

    /// Without either step the bar draws as before: the first 120 columns in `primary`, the rest
    /// in the track's colour, and `.warning(false)` changes no pixel.
    #[test]
    fn without_a_marker_or_warning_the_bar_draws_as_before() {
        let (fill, track) = (roles().primary, roles().secondary_container);
        let row = middle_row(bar(6, 10));
        assert_eq!(nearer(&row, fill, track), (0..120).collect::<Vec<_>>());
        let plain = Mounted::new(bar(6, 10)).drawn(style::color(roles().surface));
        let off = Mounted::new(bar(6, 10).warning(false)).drawn(style::color(roles().surface));
        assert_eq!(plain, off);
    }

    /// A marker the fill's edge splits shows in both halves: the half over the fill in the track's
    /// colour, the half over the track in the fill's, so it is never drawn colour on same colour.
    #[test]
    fn a_marker_on_the_fills_edge_shows_in_both_halves() {
        let (fill, track) = (roles().primary, roles().secondary_container);
        let row = middle_row(bar(8, 10).marker(Ratio { done: 8, total: 10 }));
        let filled = nearer(&row, fill, track);
        assert!(filled.contains(&158), "the fill reaches the tick");
        assert!(
            !filled.contains(&159),
            "the tick's half over the fill is in the track's colour"
        );
        assert!(
            filled.contains(&160),
            "the tick's half over the track is in the fill's colour"
        );
        assert!(!filled.contains(&161), "the track follows the tick");
    }
}
