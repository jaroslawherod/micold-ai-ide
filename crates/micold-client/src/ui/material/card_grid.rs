//! `CardGrid`: the Overview's responsive grid of [`OverviewCard`]s (feature 813, FR-021).
//!
//! The number of columns follows the width the grid is given (iced `responsive`): as many as fit at
//! the minimum card width, never fewer than one. The cards share the width equally and the grid
//! scrolls vertically.

use crate::ui::material::{OverviewCard, Scrollable};
use iced::widget::{column, responsive, row, Space};
use iced::{Element, Length};
use micold_core::tokens::{spacing, Roles};

/// The narrowest a card is drawn.
pub const MIN_WIDTH: f32 = 280.0;

/// The columns that fit in `width`: `max(1, floor((width + gap) / (min + gap)))`.
pub fn columns(width: f32, min: f32, gap: f32) -> usize {
    (((width + gap) / (min + gap)).floor() as usize).max(1)
}

/// The grid. See the module documentation.
pub struct CardGrid<'a, M> {
    cards: Vec<OverviewCard<'a, M>>,
    roles: Roles,
    min_width: f32,
    spacing: f32,
}

impl<'a, M: Clone + 'a> CardGrid<'a, M> {
    /// A grid of `cards`, themed by `roles`.
    pub fn new(cards: Vec<OverviewCard<'a, M>>, roles: Roles) -> Self {
        Self {
            cards,
            roles,
            min_width: MIN_WIDTH,
            spacing: spacing::MD,
        }
    }

    /// The narrowest a card may be drawn.
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// The gap between cards, across and down.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

/// The cards in rows of `columns`, the last row padded so its cards keep the others' width.
fn rows<'a, M: Clone + 'a>(
    cards: &[OverviewCard<'a, M>],
    columns: usize,
    gap: f32,
) -> Element<'a, M> {
    let mut grid = column![].spacing(gap).width(Length::Fill);
    for chunk in cards.chunks(columns.max(1)) {
        let mut line = row![].spacing(gap).width(Length::Fill);
        for card in chunk {
            line = line.push(Element::from(card.clone()));
        }
        for _ in chunk.len()..columns.max(1) {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }
    grid.into()
}

impl<'a, M: Clone + 'a> From<CardGrid<'a, M>> for Element<'a, M> {
    fn from(g: CardGrid<'a, M>) -> Self {
        let CardGrid {
            cards,
            roles,
            min_width,
            spacing,
        } = g;
        responsive(move |size| {
            let n = columns(size.width, min_width, spacing);
            Scrollable::new(rows(&cards, n, spacing), roles)
                .height(Length::Fill)
                .into()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::test_support::{laid_out, leaves, roles};
    use iced::{Rectangle, Size};
    use micold_core::overview::{CardModel, CardReason, CardState, LocationType};
    use micold_core::session::SessionLocation;

    fn cards(n: usize) -> Vec<OverviewCard<'static, ()>> {
        (0..n)
            .map(|i| {
                OverviewCard::new(
                    &CardModel {
                        location: SessionLocation::Worktree(format!("w{i}")),
                        display_name: format!("feat-{i}"),
                        branch: None,
                        kind: LocationType::Feature,
                        sessions: (1, 0),
                        state: CardState::Idle,
                        reason: CardReason::Idle,
                        summary: "s".into(),
                        prs: micold_core::overview::PrDisplay::Hidden,
                        progress: Default::default(),
                        activation: micold_core::overview::Activation::RevealOnly(
                            micold_core::session::SessionLocation::Default,
                        ),
                    },
                    roles(),
                )
            })
            .collect()
    }

    fn grid(n: usize) -> Element<'static, ()> {
        CardGrid::new(cards(n), roles())
            .min_width(280.0)
            .spacing(16.0)
            .into()
    }

    /// The card-sized boxes (full card height) of a laid-out grid, left to right, top to bottom.
    fn card_boxes(node: &iced::advanced::layout::Node) -> Vec<Rectangle> {
        fn walk(n: &iced::advanced::layout::Node, at: iced::Vector, out: &mut Vec<Rectangle>) {
            let b = n.bounds() + at;
            let before = out.len();
            for c in n.children() {
                walk(c, iced::Vector::new(b.x, b.y), out);
            }
            // The innermost box of the card's height is the card: its row is as tall, but holds it.
            if out.len() == before && (b.height - super::super::overview_card::HEIGHT).abs() < 0.01
            {
                out.push(b);
            }
        }
        let mut out = Vec::new();
        walk(node, iced::Vector::ZERO, &mut out);
        out
    }

    /// U4: columns = max(1, floor((w + gap) / (min + gap))).
    #[test]
    fn columns_follow_the_width_and_never_drop_below_one() {
        assert_eq!(columns(279.0, 280.0, 16.0), 1);
        assert_eq!(columns(280.0, 280.0, 16.0), 1);
        assert_eq!(columns(575.0, 280.0, 16.0), 1);
        assert_eq!(columns(576.0, 280.0, 16.0), 2);
        assert_eq!(columns(1000.0, 280.0, 16.0), 3);
        assert_eq!(columns(0.0, 280.0, 16.0), 1);
    }

    /// U4: at 279 px the grid is one column, at 900 px three.
    #[test]
    fn the_laid_out_grid_has_the_columns_its_width_gives() {
        for (width, expected) in [(279.0, 1), (900.0, 3)] {
            let node = laid_out(grid(6), Size::new(width, 800.0));
            let boxes = card_boxes(&node);
            assert_eq!(boxes.len(), 6, "{width}");
            let first_row = boxes.iter().filter(|b| b.y == boxes[0].y).count();
            assert_eq!(first_row, expected, "{width}");
            assert!(
                boxes.iter().all(|b| b.x + b.width <= width + 0.01),
                "{width}"
            );
        }
    }

    /// U4: no card overlaps another, and no piece of one grid overlaps any other.
    #[test]
    fn no_card_overlaps_another() {
        for width in [279.0, 600.0, 1000.0] {
            let node = laid_out(grid(7), Size::new(width, 800.0));
            let boxes = card_boxes(&node);
            for (i, a) in boxes.iter().enumerate() {
                for b in &boxes[i + 1..] {
                    let apart = a.x + a.width <= b.x + 0.01
                        || b.x + b.width <= a.x + 0.01
                        || a.y + a.height <= b.y + 0.01
                        || b.y + b.height <= a.y + 0.01;
                    assert!(apart, "{width}: {a:?} overlaps {b:?}");
                }
            }
            assert!(!leaves(&node).is_empty());
        }
    }

    /// U4: 30 cards in a 400 px viewport scroll: the content is taller than the grid.
    #[test]
    fn thirty_cards_scroll_vertically() {
        let node = laid_out(grid(30), Size::new(600.0, 400.0));
        let bottom = card_boxes(&node)
            .iter()
            .map(|b| b.y + b.height)
            .fold(0.0, f32::max);
        assert_eq!(card_boxes(&node).len(), 30);
        assert!(bottom > 400.0, "content ends at {bottom}");
        assert!(node.size().height <= 400.0, "{:?}", node.size());
    }
}
