//! `DiffView` — one file's diff, unified or side by side (feature 482, contracts/changes-view.md
//! D1–D5).
//!
//! A text diff is laid out as fixed-height rows — hunk headers and lines — through [`VirtualRows`],
//! so only the rows on screen are built however long the diff is (D5). Each line row has two
//! fixed-width number cells (old, new) in front of its text, so the text starts at the same x on
//! every row, and an added or removed line sits on its tint role (D2). A diff that has no lines to
//! show says why instead, with no gutter (D3); one over the size limits shows its counts and a
//! **Show diff** action (D4).
//!
//! What is shown is the pure `body`; the tint of a line is the pure [`tint`].
//!
//! Side by side (D1), each row holds the old version's line on the left half and the new version's
//! on the right, a removed run paired with the added run after it and the shorter side left empty,
//! each half with its own number, marker and tint. Either layout paints a line's syntax colours
//! (`.spans`) over its tint, the bytes they do not cover in `on_surface` (R10).
//!
//! Builder form: `DiffView::new(&diff, layout, roles).spans(&scheme_spans).offset(o).viewport(v)
//! .on_scroll(|o, v| Msg::Scrolled(o, v)).on_show_large(Msg::ShowLarge).into()`.

use iced::alignment::Horizontal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, rich_text, row, span, text, Space};
use iced::{Alignment, Element, Font, Length};
use micold_core::review::diff::{
    Cell, DiffLine, FileDiff, Hunk, LineKind, SchemeSpans, SideIndex, SideRow, SideSpans,
    UnifiedIndex, UnifiedRow,
};
/// The layout a diff is shown in, re-exported so callers need not reach into the settings module.
pub use micold_core::settings::DiffLayout;
use micold_core::tokens::{spacing, Rgb, Roles};

use super::{Button, Text, TypeRole, VirtualRows};

/// One row's height, hunk header or line: fixed, which is what lets `VirtualRows` build only the
/// visible rows.
pub const ROW_HEIGHT: f32 = 20.0;

/// The width of one line-number cell: six digits of the monospace text.
pub const NUMBER_WIDTH: f32 = 64.0;

/// The width of the `+`/`−` marker cell.
pub const MARKER_WIDTH: f32 = 16.0;

/// The size of the diff's monospace text.
pub const CODE_SIZE: f32 = 13.0;

/// D3: a binary file.
pub const BINARY: &str = "Binary file — not shown";

/// D3: a version is not valid UTF-8.
pub const NOT_UTF8: &str = "Not UTF-8 text — not shown";

/// D3: only the file mode changed.
pub const MODE_ONLY: &str = "Only the file mode changed";

/// A text diff with no hunks: the file was renamed or its content is otherwise the same.
pub const NO_CONTENT: &str = "The content did not change";

/// D4: the action that loads a large diff.
pub const SHOW_DIFF: &str = "Show diff";

/// D4: what a large diff says in place of its rows.
pub fn large_message(added: u32, removed: u32) -> String {
    format!("Large diff: +{added} \u{2212}{removed} lines, not shown until you ask")
}

/// What the diff area shows for a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Body<'a> {
    /// The unified rows, found by index: building a view costs the hunk count, not the line
    /// count (D5).
    Rows(UnifiedIndex<'a>),
    /// The side-by-side rows, found by index the same way.
    Side(SideIndex<'a>),
    /// No lines to show, and the sentence that says why (D3).
    Message(&'static str),
    /// Over the size limits: the counts and the Show diff action (D4).
    Large {
        /// Lines added.
        added: u32,
        /// Lines removed.
        removed: u32,
    },
}

/// What the diff area shows for `diff`.
fn body(diff: &FileDiff, layout: DiffLayout) -> Body<'_> {
    match diff {
        FileDiff::Text(hunks) if hunks.is_empty() => Body::Message(NO_CONTENT),
        FileDiff::Text(_) => match layout {
            DiffLayout::Unified => Body::Rows(UnifiedIndex::new(diff)),
            DiffLayout::SideBySide => Body::Side(SideIndex::new(diff)),
        },
        FileDiff::Binary => Body::Message(BINARY),
        FileDiff::NotUtf8 => Body::Message(NOT_UTF8),
        FileDiff::ModeOnly => Body::Message(MODE_ONLY),
        FileDiff::TooLarge { added, removed } => Body::Large {
            added: *added,
            removed: *removed,
        },
    }
}

/// The background of a `kind` line: its tint role, or none for context (D2).
pub fn tint(kind: LineKind, roles: Roles) -> Option<Rgb> {
    match kind {
        LineKind::Added => Some(roles.diff_added),
        LineKind::Removed => Some(roles.diff_removed),
        LineKind::Context => None,
    }
}

/// One file's diff.
pub struct DiffView<'a, M> {
    diff: &'a FileDiff,
    layout: DiffLayout,
    spans: &'a SchemeSpans,
    roles: Roles,
    offset: u32,
    viewport: u32,
    on_scroll: Option<Box<dyn Fn(u32, u32) -> M + 'a>>,
    on_show_large: Option<M>,
}

impl<'a, M: Clone + 'a> DiffView<'a, M> {
    /// `diff` in `layout`, themed by `roles`, in plain text until [`Self::spans`].
    pub fn new(diff: &'a FileDiff, layout: DiffLayout, roles: Roles) -> Self {
        Self {
            diff,
            layout,
            spans: &NO_SPANS,
            roles,
            offset: 0,
            viewport: 0,
            on_scroll: None,
            on_show_large: None,
        }
    }

    /// The scroll offset last reported, in pixels from the top.
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    /// The viewport height last reported, in pixels; 0 assumes a full window.
    pub fn viewport(mut self, viewport: u32) -> Self {
        self.viewport = viewport;
        self
    }

    /// Report the offset and the viewport height as the rows scroll.
    pub fn on_scroll(mut self, f: impl Fn(u32, u32) -> M + 'a) -> Self {
        self.on_scroll = Some(Box::new(f));
        self
    }

    /// The message the large gate's Show diff sends (D4); without it the action is disabled.
    pub fn on_show_large(mut self, message: M) -> Self {
        self.on_show_large = Some(message);
        self
    }

    /// Paint the lines in `spans`' syntax colours (R10); without it they are plain text.
    pub fn spans(mut self, spans: &'a SchemeSpans) -> Self {
        self.spans = spans;
        self
    }
}

/// A side-by-side row: the old version's cell on the left half, the new version's on the right.
fn side_row<'a, M: 'a>(
    left: Option<Cell<'a>>,
    right: Option<Cell<'a>>,
    spans: &'a SchemeSpans,
    r: Roles,
) -> Element<'a, M> {
    row![half(left, &spans.old, r), half(right, &spans.new, r)]
        .height(Length::Fixed(ROW_HEIGHT))
        .width(Length::Fill)
        .into()
}

/// One half of a side-by-side row: number, marker and text on the line's tint, the same cells as
/// a unified row's so the columns align on every row; an empty half where the other side has no
/// partner, the row's height all the same.
fn half<'a, M: 'a>(cell: Option<Cell<'a>>, spans: &'a SideSpans, r: Roles) -> Element<'a, M> {
    let Some(cell) = cell else {
        return container(Space::new())
            .width(Length::FillPortion(1))
            .height(Length::Fixed(ROW_HEIGHT))
            .into();
    };
    let cells = row![
        number(Some(cell.number), r),
        marker(cell.kind, r),
        container(line_text(cell.text, spans.line(cell.number), r)).width(Length::Fill),
    ]
    .height(Length::Fixed(ROW_HEIGHT))
    .align_y(Alignment::Center);
    tinted(cells.into(), cell.kind, r, Length::FillPortion(1))
}

/// `text` cut where `spans` start and end, each piece with its colour or none for the text colour.
fn segments<'t>(
    text: &'t str,
    spans: &[(std::ops::Range<usize>, Rgb)],
) -> Vec<(&'t str, Option<Rgb>)> {
    let floor = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let mut pieces = Vec::new();
    let mut at = 0;
    for (range, colour) in spans {
        let (start, end) = (floor(range.start).max(at), floor(range.end));
        if start >= end {
            continue;
        }
        if at < start {
            pieces.push((&text[at..start], None));
        }
        pieces.push((&text[start..end], Some(*colour)));
        at = end;
    }
    if at < text.len() || pieces.is_empty() {
        pieces.push((&text[at..], None));
    }
    pieces
}

/// The spans of a unified line: a removed line's from the old version, any other's from the new.
fn line_spans<'s>(
    line: &DiffLine,
    spans: Option<&'s SchemeSpans>,
) -> &'s [(std::ops::Range<usize>, Rgb)] {
    match (spans, line.kind) {
        (None, _) => &[],
        (Some(s), LineKind::Removed) => line.old.map_or(&[], |n| s.old.line(n)),
        (Some(s), _) => line.new.map_or(&[], |n| s.new.line(n)),
    }
}

/// A line's text in its syntax colours, the rest in `on_surface`; monospace, never wrapped.
fn line_text<'a, M: 'a>(
    content: &'a str,
    spans: &[(std::ops::Range<usize>, Rgb)],
    r: Roles,
) -> Element<'a, M> {
    if spans.is_empty() {
        return code(content, r.on_surface);
    }
    let pieces: Vec<text::Span<'a, (), Font>> = segments(content, spans)
        .into_iter()
        .map(|(piece, colour)| {
            span(piece).color(super::style::color(colour.unwrap_or(r.on_surface)))
        })
        .collect();
    rich_text(pieces)
        .font(Font::MONOSPACE)
        .size(CODE_SIZE)
        .wrapping(Wrapping::None)
        .into()
}

/// The `+`/`−` marker cell of a `kind` line.
fn marker<'a, M: 'a>(kind: LineKind, r: Roles) -> Element<'a, M> {
    let glyph = match kind {
        LineKind::Context => " ",
        LineKind::Added => "+",
        LineKind::Removed => "\u{2212}",
    };
    container(code(glyph, r.on_surface_variant))
        .width(Length::Fixed(MARKER_WIDTH))
        .align_x(Horizontal::Center)
        .into()
}

/// `cells` on the tint of a `kind` line, `width` wide, clipped to the row.
fn tinted<'a, M: 'a>(
    cells: Element<'a, M>,
    kind: LineKind,
    r: Roles,
    width: Length,
) -> Element<'a, M> {
    let fill = tint(kind, r);
    container(cells)
        .height(Length::Fixed(ROW_HEIGHT))
        .width(width)
        .clip(true)
        .style(move |_| fill.map(background).unwrap_or_default())
        .into()
}

/// No syntax colours: what a view shows before [`DiffView::spans`].
static NO_SPANS: SchemeSpans = SchemeSpans {
    old: SideSpans(Vec::new()),
    new: SideSpans(Vec::new()),
};

/// A hunk's header row: `@@ -a,b +c,d @@ section`, muted, on the container tone.
fn header_row<'a, M: 'a>(hunk: &'a Hunk, r: Roles) -> Element<'a, M> {
    let mut header = hunk.header();
    if !hunk.section.is_empty() {
        header.push(' ');
        header.push_str(&hunk.section);
    }
    container(code(header, r.on_surface_variant))
        .padding([0.0, spacing::SM])
        .height(Length::Fixed(ROW_HEIGHT))
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .clip(true)
        .style(move |_| background(r.surface_container))
        .into()
}

/// One line row: old number, new number, marker, text — the cells fixed so the text aligns (D2).
fn line_row<'a, M: 'a>(
    line: &'a DiffLine,
    spans: Option<&'a SchemeSpans>,
    r: Roles,
) -> Element<'a, M> {
    let cells = row![
        number(line.old, r),
        number(line.new, r),
        marker(line.kind, r),
        container(line_text(&line.text, line_spans(line, spans), r)).width(Length::Fill),
    ]
    .height(Length::Fixed(ROW_HEIGHT))
    .align_y(Alignment::Center);
    tinted(cells.into(), line.kind, r, Length::Fill)
}

/// A line-number cell: right-aligned in its fixed width, muted; empty for the side without one.
fn number<'a, M: 'a>(n: Option<u32>, r: Roles) -> Element<'a, M> {
    let label = n.map(|n| n.to_string()).unwrap_or_default();
    container(code(label, r.on_surface_variant))
        .width(Length::Fixed(NUMBER_WIDTH))
        .padding([0.0, spacing::XS])
        .align_x(Horizontal::Right)
        .into()
}

/// Diff text: monospace, one line, never wrapped (a long line is clipped by its row).
fn code<'a, M: 'a>(content: impl text::IntoFragment<'a>, color: Rgb) -> Element<'a, M> {
    text(content)
        .font(Font::MONOSPACE)
        .size(CODE_SIZE)
        .wrapping(Wrapping::None)
        .color(super::style::color(color))
        .into()
}

/// A container filled with `color`.
fn background(color: Rgb) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(super::style::color(color))),
        ..container::Style::default()
    }
}

impl<'a, M: Clone + 'a> From<DiffView<'a, M>> for Element<'a, M> {
    fn from(view: DiffView<'a, M>) -> Self {
        let r = view.roles;
        let spans = view.spans;
        let list = |len, build: Box<dyn Fn(usize) -> Element<'a, M> + 'a>| {
            let mut list = VirtualRows::new(len, ROW_HEIGHT, build, r)
                .offset(view.offset)
                .viewport(view.viewport);
            if let Some(f) = view.on_scroll {
                list = list.on_scroll(f);
            }
            list.into()
        };
        match body(view.diff, view.layout) {
            Body::Rows(rows) => list(
                rows.len(),
                Box::new(move |index| match rows.row(index) {
                    Some(UnifiedRow::Header(hunk)) => header_row(hunk, r),
                    Some(UnifiedRow::Line(line)) => line_row(line, Some(spans), r),
                    None => blank(),
                }),
            ),
            Body::Side(rows) => list(
                rows.len(),
                Box::new(move |index| match rows.row(index) {
                    Some(SideRow::Header(hunk)) => header_row(hunk, r),
                    Some(SideRow::Pair { left, right }) => side_row(left, right, spans, r),
                    None => blank(),
                }),
            ),
            Body::Message(sentence) => {
                centred(Text::new(sentence, TypeRole::Body, r).muted().into())
            }
            Body::Large { added, removed } => {
                let mut show = Button::filled(SHOW_DIFF, r);
                if let Some(message) = view.on_show_large {
                    show = show.on_press(message);
                }
                centred(
                    column![
                        Text::new(large_message(added, removed), TypeRole::Body, r).muted(),
                        show,
                    ]
                    .spacing(spacing::MD)
                    .align_x(Alignment::Center)
                    .into(),
                )
            }
        }
    }
}

/// An empty row, for an index past the end.
fn blank<'a, M: 'a>() -> Element<'a, M> {
    Space::new().height(Length::Fixed(ROW_HEIGHT)).into()
}

/// `content` in the middle of the diff area.
fn centred<'a, M: 'a>(content: Element<'a, M>) -> Element<'a, M> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::layout;
    use iced::advanced::widget::Tree;
    use iced::Size;
    use micold_core::review::diff::parse_unified;
    use micold_core::tokens::{DARK, LIGHT};
    use std::ops::Range;

    use super::super::virtual_rows::{visible_range, ASSUMED_VIEWPORT, OVERSCAN};

    const TOLERANCE: f32 = 0.5;

    fn body_u(diff: &FileDiff) -> Body<'_> {
        body(diff, DiffLayout::Unified)
    }

    /// The rows `view` builds as it stands: the visible ones and the overscan (D5).
    fn built_rows<M: Clone>(view: &DiffView<'_, M>) -> Range<usize> {
        let len = match body(view.diff, view.layout) {
            Body::Rows(rows) => rows.len(),
            Body::Side(rows) => rows.len(),
            Body::Message(_) | Body::Large { .. } => return 0..0,
        };
        let viewport = if view.viewport == 0 {
            ASSUMED_VIEWPORT
        } else {
            view.viewport
        };
        visible_range(view.offset, viewport, ROW_HEIGHT, len, OVERSCAN)
    }

    fn line(kind: LineKind, old: Option<u32>, new: Option<u32>, text: &str) -> DiffLine {
        DiffLine {
            kind,
            old,
            new,
            text: text.into(),
        }
    }

    /// The x of each cell of a line row laid out 800 px wide.
    fn cell_xs(line: &DiffLine) -> Vec<f32> {
        let mut element: Element<'_, ()> = line_row(line, None, LIGHT);
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(800.0, ROW_HEIGHT)),
        );
        // container → row → cells
        let row = &node.children()[0];
        row.children().iter().map(|c| c.bounds().x).collect()
    }

    #[test]
    fn both_number_columns_and_the_text_align_across_rows() {
        let short = line(LineKind::Context, Some(9), Some(9), "fn a() {}");
        let added = line(LineKind::Added, None, Some(10_000), "    let x = 1;");
        let removed = line(LineKind::Removed, Some(123_456), None, "");
        let a = cell_xs(&short);
        assert_eq!(a.len(), 4, "old number, new number, marker, text");
        for other in [cell_xs(&added), cell_xs(&removed)] {
            for (cell, (x, y)) in a.iter().zip(&other).enumerate() {
                assert!(
                    (x - y).abs() < TOLERANCE,
                    "cell {cell} starts at {x:.1} on one row and {y:.1} on another"
                );
            }
        }
        assert!(
            a[3] >= 2.0 * NUMBER_WIDTH,
            "the text sits after both number cells"
        );
    }

    #[test]
    fn added_removed_and_context_rows_take_their_tint_roles() {
        for r in [LIGHT, DARK] {
            assert_eq!(tint(LineKind::Added, r), Some(r.diff_added));
            assert_eq!(tint(LineKind::Removed, r), Some(r.diff_removed));
            assert_eq!(tint(LineKind::Context, r), None);
        }
    }

    #[test]
    fn only_the_visible_rows_are_built_for_fifty_thousand_lines() {
        let mut raw = String::from("@@ -0,0 +1,50000 @@\n");
        for i in 0..50_000 {
            raw.push_str(&format!("+line {i}\n"));
        }
        let diff = parse_unified(raw.as_bytes());
        let Body::Rows(rows) = body(&diff, DiffLayout::Unified) else {
            panic!("a text diff has rows");
        };
        assert_eq!(rows.len(), 50_001, "one header and 50,000 lines");
        let view = DiffView::<()>::new(&diff, DiffLayout::Unified, LIGHT)
            .offset(400_000)
            .viewport(600);
        let built = built_rows(&view);
        let visible = (600.0 / ROW_HEIGHT) as usize + 1;
        assert!(built.len() <= visible + 2 * OVERSCAN, "built {built:?}");
        assert!(
            built.contains(&20_000),
            "row 20,000 is on screen at 400,000 px: {built:?}"
        );
        let _element: Element<'_, ()> = view.into();
    }

    #[test]
    fn binary_not_utf8_and_mode_only_show_their_message_and_no_gutter() {
        assert_eq!(body_u(&FileDiff::Binary), Body::Message(BINARY));
        assert_eq!(body_u(&FileDiff::NotUtf8), Body::Message(NOT_UTF8));
        assert_eq!(body_u(&FileDiff::ModeOnly), Body::Message(MODE_ONLY));
        assert_eq!(body_u(&FileDiff::Text(vec![])), Body::Message(NO_CONTENT));
        for diff in [FileDiff::Binary, FileDiff::NotUtf8, FileDiff::ModeOnly] {
            let view = DiffView::<()>::new(&diff, DiffLayout::Unified, DARK);
            assert_eq!(built_rows(&view), 0..0, "no rows, so no gutter");
            let _element: Element<'_, ()> = view.into();
        }
    }

    #[test]
    fn a_large_diff_shows_its_counts_and_show_diff() {
        let diff = FileDiff::TooLarge {
            added: 6_000,
            removed: 12,
        };
        assert_eq!(
            body(&diff, DiffLayout::Unified),
            Body::Large {
                added: 6_000,
                removed: 12
            }
        );
        let message = large_message(6_000, 12);
        assert!(
            message.contains("+6000") && message.contains("\u{2212}12"),
            "{message}"
        );
        let view = DiffView::new(&diff, DiffLayout::Unified, LIGHT).on_show_large(());
        assert_eq!(built_rows(&view), 0..0);
        let _element: Element<'_, ()> = view.into();
    }

    fn cell(kind: LineKind, number: u32, text: &str) -> Cell<'_> {
        Cell { number, text, kind }
    }

    /// A laid-out side-by-side row: its height, each half's (x, width, height), each half's cell xs.
    type SideLayout = (f32, Vec<(f32, f32, f32)>, Vec<Vec<f32>>);

    /// A side-by-side row laid out 800 px wide: each half's (x, width, height), and the x of each
    /// cell inside a half that has a line.
    fn side_layout(left: Option<Cell<'_>>, right: Option<Cell<'_>>) -> SideLayout {
        let spans = SchemeSpans::default();
        let mut element: Element<'_, ()> = side_row(left, right, &spans, LIGHT);
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(800.0, ROW_HEIGHT)),
        );
        let halves = node.children();
        let boxes = halves
            .iter()
            .map(|h| (h.bounds().x, h.bounds().width, h.bounds().height))
            .collect();
        // half (container) → row → cells; an empty half has no row of cells.
        let cells = halves
            .iter()
            .map(|h| {
                h.children()
                    .first()
                    .map(|row| row.children().iter().map(|c| c.bounds().x).collect())
                    .unwrap_or_default()
            })
            .collect();
        (node.bounds().height, boxes, cells)
    }

    #[test]
    fn side_by_side_number_columns_align_and_padded_cells_keep_the_row_height() {
        let context = (
            Some(cell(LineKind::Context, 9, "fn a() {}")),
            Some(cell(LineKind::Context, 12, "fn a() {}")),
        );
        let removed_only = (Some(cell(LineKind::Removed, 123_456, "old")), None);
        let added_only = (None, Some(cell(LineKind::Added, 10_000, "    new")));
        let (height, boxes, cells) = side_layout(context.0, context.1);
        assert_eq!(boxes.len(), 2, "a left and a right half");
        assert!(
            (boxes[1].0 - 400.0).abs() < TOLERANCE,
            "halves split the width: {boxes:?}"
        );
        assert_eq!(cells[0].len(), 3, "number, marker, text: {cells:?}");
        assert!(
            cells[0][2] >= NUMBER_WIDTH,
            "the text sits after the number"
        );
        for (l, r) in [removed_only, added_only] {
            let (h, b, c) = side_layout(l, r);
            assert!(
                (h - height).abs() < TOLERANCE && (h - ROW_HEIGHT).abs() < TOLERANCE,
                "{h}"
            );
            for (half, (x, w, hh)) in b.iter().enumerate() {
                assert!(
                    (x - boxes[half].0).abs() < TOLERANCE && (w - boxes[half].1).abs() < TOLERANCE,
                    "half {half} moved: {b:?} against {boxes:?}"
                );
                assert!(
                    (hh - ROW_HEIGHT).abs() < TOLERANCE,
                    "a padded half keeps the row height"
                );
            }
            for (half, xs) in c.iter().enumerate() {
                if xs.is_empty() {
                    continue;
                }
                for (i, (x, y)) in xs.iter().zip(&cells[half]).enumerate() {
                    assert!(
                        (x - y).abs() < TOLERANCE,
                        "half {half} cell {i}: {x} against {y}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_side_by_side_layout_shows_the_side_rows() {
        let raw = "@@ -1,2 +1,3 @@\n ctx\n-old\n+new\n+more\n";
        let diff = parse_unified(raw.as_bytes());
        let Body::Side(rows) = body(&diff, DiffLayout::SideBySide) else {
            panic!("side by side lays out side rows");
        };
        assert_eq!(rows.len(), 4, "header, context, two paired rows");
        assert!(matches!(body(&diff, DiffLayout::Unified), Body::Rows(_)));
        let spans = SchemeSpans::default();
        let _element: Element<'_, ()> = DiffView::new(&diff, DiffLayout::SideBySide, LIGHT)
            .spans(&spans)
            .into();
    }

    #[test]
    fn spans_colour_their_bytes_and_the_rest_keeps_the_text_colour() {
        let red = Rgb { r: 200, g: 0, b: 0 };
        let blue = Rgb { r: 0, g: 0, b: 200 };
        let text = "let é = 1;";
        assert_eq!(
            segments(text, &[(0..3, red), (4..6, blue)]),
            vec![
                ("let", Some(red)),
                (" ", None),
                ("é", Some(blue)),
                (" = 1;", None)
            ]
        );
        // Out of range or off a character boundary: clamped, never a panic.
        assert_eq!(
            segments("ab", &[(1..99, red)]),
            vec![("a", None), ("b", Some(red))]
        );
        assert_eq!(segments("é", &[(0..1, red)]), vec![("é", None)]);
        assert_eq!(segments("plain", &[]), vec![("plain", None)]);
    }

    #[test]
    fn a_coloured_line_keeps_its_tint_and_its_cells() {
        let red = Rgb { r: 200, g: 0, b: 0 };
        let mut spans = SchemeSpans::default();
        spans.new.0 = vec![vec![], vec![(0..3, red)]];
        let added = line(LineKind::Added, None, Some(2), "let x = 1;");
        let plain = cell_xs(&added);
        let mut element: Element<'_, ()> = line_row(&added, Some(&spans), LIGHT);
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(800.0, ROW_HEIGHT)),
        );
        let xs: Vec<f32> = node.children()[0]
            .children()
            .iter()
            .map(|c| c.bounds().x)
            .collect();
        assert_eq!(xs, plain, "colour changes no geometry");
        assert_eq!(
            tint(added.kind, LIGHT),
            Some(LIGHT.diff_added),
            "the spans sit on the tint"
        );
        assert_eq!(line_spans(&added, Some(&spans)), &[(0..3, red)]);
    }
}
