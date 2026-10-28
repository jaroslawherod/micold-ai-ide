//! `VirtualRows` — a long list that builds only the rows on screen (Principle VIII, feature 482,
//! research R11, contracts/changes-view.md L4).
//!
//! iced 0.14 has no virtual list, and a `column` of 2,000 rows lays out every one of them on each
//! frame. This one builds the rows inside the viewport plus an overscan, with a spacer above and
//! below so the scrollbar still measures `len * row_height`. Rows have one fixed height, which is
//! what keeps the arithmetic exact; the arithmetic itself is the pure [`visible_range_with`].
//!
//! The scroll offset and viewport height are the caller's state: the view is rebuilt from them, so
//! the caller stores what [`VirtualRows::on_scroll`] reports and passes it back.
//!
//! Builder form: `VirtualRows::new(len, 32.0, |i| row(i), roles).offset(o).viewport(v)
//! .on_scroll(|o, v| Msg::Scrolled(o, v)).into()`.

use std::ops::Range;
use std::rc::Rc;

use iced::widget::{column, Space};
use iced::{Element, Length};
use micold_core::tokens::Roles;

use crate::wide_content::{Pane, PaneWidth, Ruler};
use crate::{ScrollDirection, Scrollable};

/// Rows built beyond each edge of the viewport, so a fast scroll does not show a blank band.
pub const OVERSCAN: usize = 8;

/// The viewport assumed before the first scroll report arrives: tall enough to fill a window.
pub const ASSUMED_VIEWPORT: u32 = 1_200;

/// The spacer heights above and below the built `rows` of a list of `len` rows, so the content is
/// `len * row_height` tall in total.
pub fn spacers(rows: &Range<usize>, row_height: f32, len: usize) -> (f32, f32) {
    let above = rows.start as f32 * row_height;
    let below = len.saturating_sub(rows.end) as f32 * row_height;
    (above, below)
}

/// The rows to build for a viewport `viewport` pixels tall scrolled `offset` pixels down a list of
/// `len` rows of `row_height` pixels, widened by `overscan` rows on each side and clamped to the
/// list, where `extras` hang slots under rows: each `(index, height)` adds `height` pixels under
/// row `index`, sorted by index (feature 482, T054).
pub fn visible_range_with(
    offset: u32,
    viewport: u32,
    row_height: f32,
    len: usize,
    overscan: usize,
    extras: &[(usize, f32)],
) -> Range<usize> {
    if len == 0 || row_height <= 0.0 {
        return 0..0;
    }
    let first = row_at(offset as f32, row_height, len, extras);
    // The row holding the last visible pixel, one past it as the end.
    let last = rows_before((offset + viewport) as f32, row_height, len, extras);
    let start = first.min(len).saturating_sub(overscan);
    let end = last.saturating_add(overscan).min(len);
    start..end.max(start)
}

/// How many rows start above the pixel `y`: one past the row holding the pixel just above it.
fn rows_before(y: f32, row_height: f32, len: usize, extras: &[(usize, f32)]) -> usize {
    let mut extra = 0.0;
    for &(index, height) in extras {
        let top = index as f32 * row_height + extra;
        if y <= top {
            break;
        }
        if y <= top + row_height + height {
            return (index + 1).min(len);
        }
        extra += height;
    }
    (((y - extra) / row_height).ceil().max(0.0) as usize).min(len)
}

/// The row whose own height or slot holds the pixel `y`.
fn row_at(y: f32, row_height: f32, len: usize, extras: &[(usize, f32)]) -> usize {
    let mut extra = 0.0;
    for &(index, height) in extras {
        let top = index as f32 * row_height + extra;
        if y < top {
            break;
        }
        if y < top + row_height + height {
            return index.min(len);
        }
        extra += height;
    }
    (((y - extra) / row_height).floor().max(0.0) as usize).min(len)
}

/// [`spacers`] for a list whose rows carry the slots `extras`.
pub fn spacers_with(
    rows: &Range<usize>,
    row_height: f32,
    len: usize,
    extras: &[(usize, f32)],
) -> (f32, f32) {
    let (mut above, mut below) = spacers(rows, row_height, len);
    for &(index, height) in extras {
        if index < rows.start {
            above += height;
        } else if index >= rows.end && index < len {
            below += height;
        }
    }
    (above, below)
}

/// Builds one row by its index.
type BuildRow<'a, M> = Box<dyn Fn(usize) -> Element<'a, M> + 'a>;

/// Builds the scroll report from the offset, the viewport height and the content height.
type OnScroll<'a, M> = Box<dyn Fn(u32, u32, u32) -> M + 'a>;

/// A virtualised list of fixed-height rows.
pub struct VirtualRows<'a, M> {
    len: usize,
    row_height: f32,
    build_row: BuildRow<'a, M>,
    roles: Roles,
    offset: u32,
    viewport: u32,
    on_scroll: Option<OnScroll<'a, M>>,
    extras: Vec<(usize, f32)>,
    id: Option<iced::advanced::widget::Id>,
    least_width: Option<f32>,
    pane: PaneWidth,
    clear_of_bar: bool,
}

impl<'a, M: Clone + 'a> VirtualRows<'a, M> {
    /// `len` rows, each `row_height` pixels tall, built by `build_row(index)`, themed by `roles`.
    pub fn new(
        len: usize,
        row_height: f32,
        build_row: impl Fn(usize) -> Element<'a, M> + 'a,
        roles: Roles,
    ) -> Self {
        Self {
            len,
            row_height,
            build_row: Box::new(build_row),
            roles,
            offset: 0,
            viewport: 0,
            on_scroll: None,
            extras: Vec::new(),
            id: None,
            least_width: None,
            pane: PaneWidth::default(),
            clear_of_bar: false,
        }
    }

    /// The slots under rows: each `(index, height)`, sorted by index, adds `height` pixels under
    /// row `index`, which then builds taller by that much (feature 482).
    pub fn extras(mut self, extras: Vec<(usize, f32)>) -> Self {
        self.extras = extras;
        self
    }

    /// The scroll offset last reported, in pixels from the top.
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    /// The viewport height last reported, in pixels; 0 (not yet reported) assumes
    /// [`ASSUMED_VIEWPORT`].
    pub fn viewport(mut self, viewport: u32) -> Self {
        self.viewport = viewport;
        self
    }

    /// Name the list's scrollable, so a scroll operation can address it.
    pub fn id(mut self, id: iced::advanced::widget::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Rows at least `width` pixels wide: the list also scrolls sideways when the pane is narrower
    /// than that, and its rows stay as wide as the pane (or the widest content) so their tint
    /// does not stop short (#1466). Rows must fill their width.
    pub fn least_width(mut self, width: f32) -> Self {
        self.least_width = Some(width);
        self
    }

    /// Record the pane's width in `pane` as the list is laid out, so a row's content can be held to
    /// it with [`within_pane`](crate::within_pane) while the list scrolls sideways (#1519). Only
    /// a list with a [`Self::least_width`] records it.
    pub fn pane(mut self, pane: &PaneWidth) -> Self {
        self.pane = Rc::clone(pane);
        self
    }

    /// Keep the rows clear of the floating scrollbar, which is drawn over the content's right edge,
    /// so a row's outline or a mark near its edge is not under it (#1540). Rows still fill the width
    /// that is left.
    pub fn clear_of_bar(mut self) -> Self {
        self.clear_of_bar = true;
        self
    }

    /// Report the offset and the viewport height as the list scrolls.
    pub fn on_scroll(self, f: impl Fn(u32, u32) -> M + 'a) -> Self {
        self.on_scroll_metrics(move |offset, viewport, _content| f(offset, viewport))
    }

    /// Report the offset, the viewport height and the content height the list measured as it
    /// scrolls: a caller that judges "at the end" from it is not misled by a report laid out
    /// before its rows changed.
    pub fn on_scroll_metrics(mut self, f: impl Fn(u32, u32, u32) -> M + 'a) -> Self {
        self.on_scroll = Some(Box::new(f));
        self
    }

    /// The rows this list builds as it stands.
    fn rows(&self) -> Range<usize> {
        let viewport = if self.viewport == 0 {
            ASSUMED_VIEWPORT
        } else {
            self.viewport
        };
        visible_range_with(
            self.offset,
            viewport,
            self.row_height,
            self.len,
            OVERSCAN,
            &self.extras,
        )
    }
}

impl<'a, M: Clone + 'a> From<VirtualRows<'a, M>> for Element<'a, M> {
    fn from(list: VirtualRows<'a, M>) -> Self {
        let rows = list.rows();
        let (above, below) = spacers_with(&rows, list.row_height, list.len, &list.extras);
        let pane = list.pane;
        let top: Element<'a, M> = match list.least_width {
            Some(least) => Ruler::new(&pane, least, above).into(),
            None => Space::new().height(Length::Fixed(above)).into(),
        };
        let mut content = column![top].width(Length::Fill);
        if list.clear_of_bar {
            content = content.padding(iced::Padding::ZERO.right(crate::scrollable::BAR_FOOTPRINT));
        }
        for index in rows {
            content = content.push((list.build_row)(index));
        }
        content = content.push(Space::new().height(Length::Fixed(below)));
        let mut scroll = Scrollable::new(content, list.roles)
            .height(Length::Fill)
            .width(Length::Fill);
        if let Some(id) = list.id {
            scroll = scroll.id(id);
        }
        if let Some(f) = list.on_scroll {
            scroll = scroll.on_scroll_metrics(f);
        }
        if list.least_width.is_some() {
            scroll = scroll.direction(ScrollDirection::Both);
            return Pane::new(pane, scroll.into()).into();
        }
        scroll.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::Cell;
    use std::rc::Rc;

    const ROW: f32 = 32.0;

    #[test]
    fn at_the_top_the_viewport_and_the_lower_overscan_are_built() {
        // 320 px shows rows 0..10; nothing above row 0 to overscan.
        assert_eq!(visible_range_with(0, 320, ROW, 2_000, 8, &[]), 0..18);
    }

    #[test]
    fn in_the_middle_both_overscans_are_built() {
        // Scrolled 100 rows down: rows 100..110 visible.
        assert_eq!(visible_range_with(3_200, 320, ROW, 2_000, 8, &[]), 92..118);
        // A row cut by the top edge is still built.
        assert_eq!(visible_range_with(3_210, 320, ROW, 2_000, 0, &[]), 100..111);
    }

    #[test]
    fn at_the_end_the_range_is_clamped_to_the_list() {
        assert_eq!(
            visible_range_with(63_680, 320, ROW, 2_000, 8, &[]),
            1_982..2_000
        );
        // An offset past the end (a list that shrank under the viewport) builds the tail only.
        assert_eq!(
            visible_range_with(1_000_000, 320, ROW, 2_000, 8, &[]),
            1_992..2_000
        );
    }

    #[test]
    fn an_empty_list_builds_nothing() {
        assert_eq!(visible_range_with(0, 320, ROW, 0, 8, &[]), 0..0);
        assert_eq!(spacers(&(0..0), ROW, 0), (0.0, 0.0));
    }

    #[test]
    fn the_spacers_keep_the_total_height() {
        let len = 2_000;
        let rows = visible_range_with(3_200, 320, ROW, len, 8, &[]);
        let (above, below) = spacers(&rows, ROW, len);
        assert_eq!(above, rows.start as f32 * ROW);
        assert_eq!(above + rows.len() as f32 * ROW + below, len as f32 * ROW);
    }

    #[test]
    fn slot_rows_keep_their_measured_heights() {
        // Row 10 carries a 100 px slot: it spans 320..452, so row 11 starts at 452.
        let extras = [(10, 100.0)];
        assert_eq!(visible_range_with(460, 320, ROW, 2_000, 0, &extras), 11..22);
        // An offset inside the slot still builds row 10, which holds it.
        assert_eq!(visible_range_with(400, 32, ROW, 2_000, 0, &extras), 10..11);
        let rows = 11..22;
        let (above, below) = spacers_with(&rows, ROW, 2_000, &extras);
        assert_eq!(
            above,
            11.0 * ROW + 100.0,
            "the slot above is kept in the spacer"
        );
        assert_eq!(
            above + rows.len() as f32 * ROW + below,
            2_000.0 * ROW + 100.0,
            "the content measures every row and every slot"
        );
        // A slot inside the built rows is not in either spacer.
        assert_eq!(
            spacers_with(&(5..15), ROW, 2_000, &extras),
            (5.0 * ROW, 1_985.0 * ROW)
        );
    }

    #[test]
    fn with_two_thousand_rows_only_the_visible_ones_are_built() {
        let built = Rc::new(Cell::new(0usize));
        let counter = Rc::clone(&built);
        let list = VirtualRows::<()>::new(
            2_000,
            ROW,
            move |_| {
                counter.set(counter.get() + 1);
                Space::new().height(ROW).into()
            },
            micold_core::tokens::LIGHT,
        )
        .offset(3_200)
        .viewport(320);
        let expected = list.rows().len();
        let _element: Element<'_, ()> = list.into();
        assert_eq!(expected, 26);
        assert_eq!(
            built.get(),
            expected,
            "only the visible rows and the overscan"
        );
    }

    /// #1540: with `clear_of_bar` a row that fills the width ends before the scrollbar's footprint.
    #[test]
    fn rows_clear_of_the_bar_end_before_it() {
        let right_edge = |clear: bool| {
            let mut list = VirtualRows::<String>::new(
                3,
                ROW,
                |_| {
                    iced::widget::container(iced::widget::text("edge"))
                        .width(Length::Fill)
                        .align_right(Length::Fill)
                        .into()
                },
                micold_core::tokens::LIGHT,
            )
            .viewport(200);
            if clear {
                list = list.clear_of_bar();
            }
            let mut mounted =
                crate::test_support::Mounted::new_in(list, iced::Size::new(300.0, 200.0));
            let text = mounted.painted_text();
            let line = text.first().expect("a row is painted");
            line.bounds.x + line.bounds.width
        };
        let (plain, clear) = (right_edge(false), right_edge(true));
        assert!(
            clear <= 300.0 - crate::scrollable::BAR_FOOTPRINT + 0.01,
            "clear: {clear}"
        );
        assert!(plain > clear, "plain {plain} vs clear {clear}");
    }
}
