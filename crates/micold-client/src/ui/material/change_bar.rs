//! `ChangeBar` — a file's change size at a glance (feature 1273, FR-006, research R7): two adjacent
//! segments, lines added in `success` green and lines removed in `error` red, at the lengths
//! `micold_core::review::tree::bar` computed against the whole list.
//!
//! The box is always `.max_width(px)` wide, so the `+a −r` counts beside it line up from row to
//! row; the segments start at its leading edge. A zero part draws nothing, and the outer ends of
//! the bar are rounded, the join between the parts square, so the two read as one bar.

use crate::ui::material::style;
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{layout, mouse, renderer, Layout};
use iced::border::Radius;
use iced::{Border, Element, Length, Rectangle, Size};
use micold_core::review::tree::BarParts;
use micold_core::tokens::{Rgb, Roles};

/// The bar's box width when no `.max_width` is given: the file row's bar (feature 1273).
pub const CHANGE_BAR_WIDTH: f32 = 48.0;
/// How tall the bar is, in dp. Component-owned like `progress_bar::MARKER_WIDTH`: thicker than a
/// progress track so a short part still reads as a colour.
const CHANGE_BAR_THICKNESS: f32 = 6.0;

/// Which part of the bar a segment draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BarPart {
    /// Lines added, in `success`.
    Added,
    /// Lines removed, in `error`.
    Removed,
}

/// One drawn segment: where it starts from the bar's leading edge and how long it is, in px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Segment {
    /// The part it draws.
    pub part: BarPart,
    /// Its start, from the bar's leading edge.
    pub x: f32,
    /// Its length.
    pub width: f32,
}

/// A file's added / removed bar. Builder form (Principle VIII):
/// `ChangeBar::new(parts, roles).max_width(48.0)`.
pub struct ChangeBar {
    parts: BarParts,
    roles: Roles,
    max_width: f32,
}

impl ChangeBar {
    /// A bar drawing `parts`, themed by `roles`, in a [`CHANGE_BAR_WIDTH`] box.
    pub fn new(parts: BarParts, roles: Roles) -> Self {
        Self {
            parts,
            roles,
            max_width: CHANGE_BAR_WIDTH,
        }
    }

    /// The box's width, which no part ever draws past: the `max_width` the parts were computed
    /// for.
    pub fn max_width(mut self, px: f32) -> Self {
        self.max_width = px.max(0.0);
        self
    }
}

/// The segments `parts` draws in a bar `max_width` wide: the added part from the leading edge,
/// the removed part directly after it, each cut so the two never pass `max_width`, and none for a
/// part with no length.
pub(crate) fn segments(parts: BarParts, max_width: f32) -> Vec<Segment> {
    let max_width = max_width.max(0.0);
    let added = parts.added.max(0.0).min(max_width);
    let removed = parts.removed.max(0.0).min(max_width - added);
    [
        (BarPart::Added, 0.0, added),
        (BarPart::Removed, added, removed),
    ]
    .into_iter()
    .filter(|&(_, _, width)| width > 0.0)
    .map(|(part, x, width)| Segment { part, x, width })
    .collect()
}

/// A part's colour: `success` for added lines, `error` for removed ones.
fn part_color(part: BarPart, roles: &Roles) -> Rgb {
    match part {
        BarPart::Added => roles.success,
        BarPart::Removed => roles.error,
    }
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for ChangeBar
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
        Size::new(
            Length::Fixed(self.max_width),
            Length::Fixed(CHANGE_BAR_THICKNESS),
        )
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            Length::Fixed(self.max_width),
            Length::Fixed(CHANGE_BAR_THICKNESS),
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
        let drawn = segments(self.parts, bounds.width.min(self.max_width));
        let end = drawn.last().map_or(0.0, |s| s.x + s.width);
        let round = bounds.height / 2.0;
        for segment in &drawn {
            // Only the bar's outer ends are rounded: the join between the parts stays square.
            let lead = if segment.x <= 0.0 { round } else { 0.0 };
            let trail = if segment.x + segment.width >= end {
                round
            } else {
                0.0
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + segment.x,
                        width: segment.width,
                        ..bounds
                    },
                    border: Border {
                        radius: Radius {
                            top_left: lead,
                            bottom_left: lead,
                            top_right: trail,
                            bottom_right: trail,
                        },
                        ..Border::default()
                    },
                    ..Default::default()
                },
                iced::Background::Color(style::color(part_color(segment.part, &self.roles))),
            );
        }
    }
}

impl<'a, M: 'a, Theme: 'a, Renderer> From<ChangeBar> for Element<'a, M, Theme, Renderer>
where
    Renderer: renderer::Renderer + 'a,
{
    fn from(bar: ChangeBar) -> Self {
        Element::new(bar)
    }
}

#[cfg(test)]
mod tests {
    use super::super::style;
    use super::super::test_support::{roles, Mounted};
    use super::*;

    const WIDTH: f32 = 48.0;

    fn parts(added: f32, removed: f32) -> BarParts {
        BarParts { added, removed }
    }

    /// The middle row of the bar drawn over the surface: one RGB triple per pixel.
    fn middle_row(bar: ChangeBar) -> Vec<[u8; 3]> {
        let (pixels, width) = Mounted::new(bar).drawn(style::color(roles().surface));
        let height = pixels.len() as u32 / 4 / width;
        let row = (height / 2 * width * 4) as usize;
        pixels[row..row + width as usize * 4]
            .chunks(4)
            .map(|p| [p[0], p[1], p[2]])
            .collect()
    }

    /// How many pixels of `row` are nearest `color` of the bar's two colours and the surface.
    fn nearest(row: &[[u8; 3]], color: Rgb) -> usize {
        let rgb = |c: Rgb| {
            let c = style::color(c);
            [c.r * 255.0, c.g * 255.0, c.b * 255.0]
        };
        let apart = |p: &[u8; 3], to: [f32; 3]| -> f32 {
            p.iter()
                .zip(to)
                .map(|(a, b)| (f32::from(*a) - b).abs())
                .sum()
        };
        let r = roles();
        let candidates = [r.success, r.error, r.surface];
        row.iter()
            .filter(|p| {
                let best = candidates
                    .iter()
                    .min_by(|a, b| apart(p, rgb(**a)).total_cmp(&apart(p, rgb(**b))))
                    .copied();
                best == Some(color)
            })
            .count()
    }

    /// Scenario 2.1, FR-006: the two parts sit side by side at the lengths core gave, added
    /// first, and are drawn in green and red.
    #[test]
    fn the_parts_are_adjacent_at_their_lengths() {
        assert_eq!(
            segments(parts(32.0, 8.0), WIDTH),
            vec![
                Segment {
                    part: BarPart::Added,
                    x: 0.0,
                    width: 32.0
                },
                Segment {
                    part: BarPart::Removed,
                    x: 32.0,
                    width: 8.0
                },
            ]
        );
        let row = middle_row(ChangeBar::new(parts(32.0, 8.0), roles()).max_width(WIDTH));
        assert_eq!(row.len(), WIDTH as usize, "the box is max_width wide");
        let (green, red) = (nearest(&row, roles().success), nearest(&row, roles().error));
        assert!((green as f32 - 32.0).abs() <= 1.0, "{green} px added");
        assert!((red as f32 - 8.0).abs() <= 1.0, "{red} px removed");
    }

    /// Scenario 2.2: whatever the parts say, nothing is drawn past `.max_width(px)`.
    #[test]
    fn the_bar_never_passes_its_max_width() {
        for (added, removed) in [(48.0, 48.0), (60.0, 0.0), (0.0, 70.0), (40.0, 20.0)] {
            let drawn = segments(parts(added, removed), WIDTH);
            let end = drawn.last().map_or(0.0, |s| s.x + s.width);
            assert!(end <= WIDTH, "{added}/{removed} ends at {end}");
        }
        let row = middle_row(ChangeBar::new(parts(60.0, 30.0), roles()).max_width(WIDTH));
        assert_eq!(row.len(), WIDTH as usize);
        assert_eq!(nearest(&row, roles().error), 0, "no room left for removed");
    }

    /// FR-006: a zero part draws nothing, and a bar of two zero parts draws nothing at all.
    #[test]
    fn a_zero_part_draws_nothing() {
        let added_only = segments(parts(20.0, 0.0), WIDTH);
        assert_eq!(added_only.len(), 1);
        assert_eq!(added_only[0].part, BarPart::Added);
        let removed_only = segments(parts(0.0, 20.0), WIDTH);
        assert_eq!(
            removed_only,
            vec![Segment {
                part: BarPart::Removed,
                x: 0.0,
                width: 20.0
            }]
        );
        assert!(segments(parts(0.0, 0.0), WIDTH).is_empty());
        let row = middle_row(ChangeBar::new(parts(0.0, 0.0), roles()).max_width(WIDTH));
        assert_eq!(nearest(&row, roles().surface), WIDTH as usize);
    }
}
