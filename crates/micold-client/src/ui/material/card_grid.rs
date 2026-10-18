//! `CardGrid`: the Overview's responsive grid of [`OverviewCard`]s (feature 813, FR-021).
//!
//! The number of columns follows the width the grid is given (iced `responsive`): as many as fit at
//! the minimum card width, never fewer than one and at most [`MAX_COLUMNS`] (five, feature 1024).
//! The cards share the width equally, so a wider grid makes wider cards, and the grid scrolls
//! vertically.

use crate::ui::material::{OverviewCard, Scrollable};
use iced::widget::{column, responsive, row, Space};
use iced::{Element, Length};
use micold_core::tokens::{spacing, Roles};

/// The narrowest a card is drawn.
pub const MIN_WIDTH: f32 = 280.0;

/// The most cards a row holds (feature 1024): wider panes make wider cards, not more of them.
pub const MAX_COLUMNS: usize = 5;

/// The columns that fit in `width`, capped at `max` (`0` is treated as `1`):
/// `clamp(floor((width + gap) / (min + gap)), 1, max)`.
pub fn columns(width: f32, min: f32, gap: f32, max: usize) -> usize {
    (((width + gap) / (min + gap)).floor() as usize).clamp(1, max.max(1))
}

/// The grid. See the module documentation.
pub struct CardGrid<'a, M> {
    cards: Vec<OverviewCard<'a, M>>,
    roles: Roles,
    min_width: f32,
    spacing: f32,
    max_columns: usize,
}

impl<'a, M: Clone + 'a> CardGrid<'a, M> {
    /// A grid of `cards`, themed by `roles`.
    pub fn new(cards: Vec<OverviewCard<'a, M>>, roles: Roles) -> Self {
        Self {
            cards,
            roles,
            min_width: MIN_WIDTH,
            spacing: spacing::MD,
            max_columns: MAX_COLUMNS,
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

    /// The most cards a row holds; `0` is treated as `1`. Defaults to [`MAX_COLUMNS`].
    pub fn max_columns(mut self, n: usize) -> Self {
        self.max_columns = n;
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
            max_columns,
        } = g;
        responsive(move |size| {
            let n = columns(size.width, min_width, spacing, max_columns);
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
                        activity: None,
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
        assert_eq!(columns(279.0, 280.0, 16.0, MAX_COLUMNS), 1);
        assert_eq!(columns(280.0, 280.0, 16.0, MAX_COLUMNS), 1);
        assert_eq!(columns(575.0, 280.0, 16.0, MAX_COLUMNS), 1);
        assert_eq!(columns(576.0, 280.0, 16.0, MAX_COLUMNS), 2);
        assert_eq!(columns(1000.0, 280.0, 16.0, MAX_COLUMNS), 3);
        assert_eq!(columns(0.0, 280.0, 16.0, MAX_COLUMNS), 1);
    }

    /// U4: at 279 px the grid is one column, at 900 px three.
    #[test]
    fn the_laid_out_grid_has_the_columns_its_width_gives() {
        for (width, expected) in [(279.0, 1), (900.0, 3), (1463.0, 4)] {
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
        for width in [279.0, 600.0, 1000.0, 1464.0, 2400.0, 3840.0] {
            let node = laid_out(grid(7), Size::new(width, 800.0));
            let boxes = card_boxes(&node);
            assert!(
                boxes.iter().all(|b| b.x + b.width <= width + 0.01),
                "{width}: a card passes the right edge"
            );
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

    /// U4: 30 cards in a 400 px viewport scroll: the content is taller than the grid. Feature 1024
    /// (U7): at 2400 px too, and the grid never grows sideways.
    #[test]
    fn thirty_cards_scroll_vertically() {
        for width in [600.0, 2400.0] {
            let node = laid_out(grid(30), Size::new(width, 400.0));
            let boxes = card_boxes(&node);
            let bottom = boxes.iter().map(|b| b.y + b.height).fold(0.0, f32::max);
            assert_eq!(boxes.len(), 30, "{width}");
            assert!(bottom > 400.0, "{width}: content ends at {bottom}");
            assert!(node.size().height <= 400.0, "{width}: {:?}", node.size());
            assert!(node.size().width <= width, "{width}: {:?}", node.size());
            assert!(
                boxes.iter().all(|b| b.x + b.width <= width + 0.01),
                "{width}"
            );
        }
        let node = laid_out(grid(30), Size::new(2400.0, 400.0));
        assert_eq!(
            row_counts(&card_boxes(&node)),
            vec![5; 6],
            "30 cards at 2400 px"
        );
    }

    /// The card width a full row of `n` columns gives in `width`, with the tests' 16 px gap.
    fn row_width(width: f32, n: f32) -> f32 {
        (width - (n - 1.0) * 16.0) / n
    }

    /// The cards of each row, top to bottom.
    fn row_counts(boxes: &[Rectangle]) -> Vec<usize> {
        let mut counts: Vec<(f32, usize)> = Vec::new();
        for b in boxes {
            match counts.iter_mut().find(|(y, _)| (*y - b.y).abs() < 0.01) {
                Some((_, n)) => *n += 1,
                None => counts.push((b.y, 1)),
            }
        }
        counts.into_iter().map(|(_, n)| n).collect()
    }

    /// U1 (1024): the column count stops at `max`; one pixel under five columns' threshold is four.
    #[test]
    fn columns_are_capped_at_max() {
        assert_eq!(columns(1463.0, 280.0, 16.0, MAX_COLUMNS), 4);
        for width in [1464.0, 2400.0, 3840.0, 10_000.0] {
            assert_eq!(columns(width, 280.0, 16.0, MAX_COLUMNS), 5, "{width}");
        }
        assert_eq!(
            columns(2400.0, 280.0, 16.0, 0),
            1,
            "a cap of 0 is treated as 1"
        );
    }

    /// U2 (1024): from the five-card threshold up, full rows hold five equal cards and the last,
    /// partial row starts where the first does.
    #[test]
    fn full_rows_hold_five_cards_from_the_threshold_up() {
        for width in [1464.0, 2400.0, 3840.0] {
            let node = laid_out(grid(12), Size::new(width, 800.0));
            let boxes = card_boxes(&node);
            assert_eq!(boxes.len(), 12, "{width}");
            assert_eq!(row_counts(&boxes), vec![5, 5, 2], "{width}");
            let w = row_width(width, 5.0);
            for b in &boxes {
                assert!((b.width - w).abs() < 0.01, "{width}: {} != {w}", b.width);
            }
            let last_y = boxes.iter().map(|b| b.y).fold(0.0, f32::max);
            let last_x = boxes
                .iter()
                .filter(|b| (b.y - last_y).abs() < 0.01)
                .map(|b| b.x)
                .fold(f32::MAX, f32::min);
            assert!(
                (last_x - boxes[0].x).abs() < 0.01,
                "{width}: last row not left-aligned"
            );
        }
    }

    /// U3 (1024): a grid with fewer cards than columns keeps the five-card width, from the left.
    #[test]
    fn fewer_cards_than_columns_keep_the_five_card_width() {
        let w = row_width(2400.0, 5.0);
        for n in [3, 1] {
            let node = laid_out(grid(n), Size::new(2400.0, 800.0));
            let boxes = card_boxes(&node);
            assert_eq!(boxes.len(), n);
            for b in &boxes {
                assert!((b.width - w).abs() < 0.01, "{n} cards: {} != {w}", b.width);
            }
            assert!(
                boxes[0].x.abs() < 0.01,
                "{n} cards: first starts at {}",
                boxes[0].x
            );
        }
    }

    /// U4 (1024): `max_columns` sets the cap; without it the cap is `MAX_COLUMNS`.
    #[test]
    fn max_columns_sets_the_cap() {
        let capped: Element<'static, ()> = CardGrid::new(cards(6), roles())
            .min_width(280.0)
            .spacing(16.0)
            .max_columns(3)
            .into();
        let node = laid_out(capped, Size::new(2400.0, 800.0));
        assert_eq!(row_counts(&card_boxes(&node))[0], 3);
        let default: Element<'static, ()> = CardGrid::new(cards(12), roles()).into();
        let node = laid_out(default, Size::new(2400.0, 800.0));
        assert_eq!(row_counts(&card_boxes(&node))[0], MAX_COLUMNS);
    }

    /// U5 (1024): at 2400 px every card is at least half again as wide as feature 813's eight
    /// columns made it, and every card keeps the one card height.
    #[test]
    fn cards_at_2400_are_half_again_as_wide_as_before_and_as_tall() {
        let before = row_width(2400.0, 8.0);
        let node = laid_out(grid(12), Size::new(2400.0, 800.0));
        let boxes = card_boxes(&node);
        assert_eq!(boxes.len(), 12);
        for b in &boxes {
            assert!(b.width >= 1.5 * before, "{} < 1.5 × {before}", b.width);
            assert!((b.height - super::super::overview_card::HEIGHT).abs() < 0.01);
        }
    }
}
