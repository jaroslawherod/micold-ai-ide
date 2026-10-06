//! `VirtualRows` — a long list that builds only the rows on screen (Principle VIII, feature 482,
//! research R11, contracts/changes-view.md L4).
//!
//! iced 0.14 has no virtual list, and a `column` of 2,000 rows lays out every one of them on each
//! frame. This one builds the rows inside the viewport plus an overscan, with a spacer above and
//! below so the scrollbar still measures `len * row_height`. Rows have one fixed height, which is
//! what keeps the arithmetic exact; the arithmetic itself is the pure [`visible_range`].
//!
//! The scroll offset and viewport height are the caller's state: the view is rebuilt from them, so
//! the caller stores what [`VirtualRows::on_scroll`] reports and passes it back.
//!
//! Builder form: `VirtualRows::new(len, 32.0, |i| row(i), roles).offset(o).viewport(v)
//! .on_scroll(|o, v| Msg::Scrolled(o, v)).into()`.

use std::ops::Range;

use iced::widget::{column, Space};
use iced::{Element, Length};
use micold_core::tokens::Roles;

use crate::ui::material::Scrollable;

/// Rows built beyond each edge of the viewport, so a fast scroll does not show a blank band.
pub const OVERSCAN: usize = 8;

/// The viewport assumed before the first scroll report arrives: tall enough to fill a window.
pub const ASSUMED_VIEWPORT: u32 = 1_200;

/// The rows to build for a viewport `viewport` pixels tall scrolled `offset` pixels down a list of
/// `len` rows of `row_height` pixels, widened by `overscan` rows on each side and clamped to the
/// list.
pub fn visible_range(
    offset: u32,
    viewport: u32,
    row_height: f32,
    len: usize,
    overscan: usize,
) -> Range<usize> {
    if len == 0 || row_height <= 0.0 {
        return 0..0;
    }
    let first = (offset as f32 / row_height).floor() as usize;
    let last = ((offset + viewport) as f32 / row_height).ceil() as usize;
    let start = first.min(len).saturating_sub(overscan);
    let end = last.saturating_add(overscan).min(len);
    start..end.max(start)
}

/// The spacer heights above and below the built `rows` of a list of `len` rows, so the content is
/// `len * row_height` tall in total.
pub fn spacers(rows: &Range<usize>, row_height: f32, len: usize) -> (f32, f32) {
    let above = rows.start as f32 * row_height;
    let below = len.saturating_sub(rows.end) as f32 * row_height;
    (above, below)
}

/// Builds one row by its index.
type BuildRow<'a, M> = Box<dyn Fn(usize) -> Element<'a, M> + 'a>;

/// A virtualised list of fixed-height rows.
pub struct VirtualRows<'a, M> {
    len: usize,
    row_height: f32,
    build_row: BuildRow<'a, M>,
    roles: Roles,
    offset: u32,
    viewport: u32,
    on_scroll: Option<Box<dyn Fn(u32, u32) -> M + 'a>>,
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
        }
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

    /// Report the offset and the viewport height as the list scrolls.
    pub fn on_scroll(mut self, f: impl Fn(u32, u32) -> M + 'a) -> Self {
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
        visible_range(self.offset, viewport, self.row_height, self.len, OVERSCAN)
    }
}

impl<'a, M: Clone + 'a> From<VirtualRows<'a, M>> for Element<'a, M> {
    fn from(list: VirtualRows<'a, M>) -> Self {
        let rows = list.rows();
        let (above, below) = spacers(&rows, list.row_height, list.len);
        let mut content = column![Space::new().height(Length::Fixed(above))].width(Length::Fill);
        for index in rows {
            content = content.push((list.build_row)(index));
        }
        content = content.push(Space::new().height(Length::Fixed(below)));
        let mut scroll = Scrollable::new(content, list.roles)
            .height(Length::Fill)
            .width(Length::Fill);
        if let Some(f) = list.on_scroll {
            scroll =
                scroll.on_scroll_metrics(move |offset, viewport, _content| f(offset, viewport));
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
        assert_eq!(visible_range(0, 320, ROW, 2_000, 8), 0..18);
    }

    #[test]
    fn in_the_middle_both_overscans_are_built() {
        // Scrolled 100 rows down: rows 100..110 visible.
        assert_eq!(visible_range(3_200, 320, ROW, 2_000, 8), 92..118);
        // A row cut by the top edge is still built.
        assert_eq!(visible_range(3_210, 320, ROW, 2_000, 0), 100..111);
    }

    #[test]
    fn at_the_end_the_range_is_clamped_to_the_list() {
        assert_eq!(visible_range(63_680, 320, ROW, 2_000, 8), 1_982..2_000);
        // An offset past the end (a list that shrank under the viewport) builds the tail only.
        assert_eq!(visible_range(1_000_000, 320, ROW, 2_000, 8), 1_992..2_000);
    }

    #[test]
    fn an_empty_list_builds_nothing() {
        assert_eq!(visible_range(0, 320, ROW, 0, 8), 0..0);
        assert_eq!(spacers(&(0..0), ROW, 0), (0.0, 0.0));
    }

    #[test]
    fn the_spacers_keep_the_total_height() {
        let len = 2_000;
        let rows = visible_range(3_200, 320, ROW, len, 8);
        let (above, below) = spacers(&rows, ROW, len);
        assert_eq!(above, rows.start as f32 * ROW);
        assert_eq!(above + rows.len() as f32 * ROW + below, len as f32 * ROW);
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
}
