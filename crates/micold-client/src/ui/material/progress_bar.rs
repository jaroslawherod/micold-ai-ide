//! `ProgressBar` — Material's determinate linear progress indicator (feature 813, FR-016): a
//! `primary` fill over the `secondary_container` track covering `done / total` of it.
//!
//! The track and the fill are `StageProgress`'s own (`progress::paint_track`, `paint_fill`), so
//! the two indicators read as one family. Unlike it this one moves nothing and asks for no frame.

use crate::ui::material::progress::{paint_fill, paint_track};
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{layout, mouse, renderer, Layout};
use iced::{Element, Length, Rectangle, Size};
use micold_core::task_progress::Ratio;
use micold_core::tokens::{anatomy, Roles};

/// A determinate bar showing a [`Ratio`]. Builder form (Principle VIII):
/// `ProgressBar::new(ratio, roles).width(Length::Fixed(120.0))`.
pub struct ProgressBar {
    ratio: Ratio,
    roles: Roles,
    width: Length,
}

impl ProgressBar {
    /// A bar filled `ratio.done` of `ratio.total`, themed by `roles`, as wide as its room.
    pub fn new(ratio: Ratio, roles: Roles) -> Self {
        Self {
            ratio,
            roles,
            width: Length::Fill,
        }
    }

    /// The bar's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
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
        let width = fill_width(self.ratio, bounds.width);
        if width > 0.0 {
            paint_fill(renderer, Rectangle { width, ..bounds }, &self.roles);
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
}
