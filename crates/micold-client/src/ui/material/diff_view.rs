//! `DiffView` — one file's diff, in the unified layout (feature 482, contracts/changes-view.md
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
//! Builder form: `DiffView::new(&diff, layout, roles).offset(o).viewport(v)
//! .on_scroll(|o, v| Msg::Scrolled(o, v)).on_show_large(Msg::ShowLarge).into()`. The side-by-side
//! layout arrives with M3; until then both layouts render unified.

#[cfg(test)]
use std::ops::Range;

use iced::alignment::Horizontal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Font, Length};
use micold_core::review::diff::{unified_rows, DiffLine, FileDiff, Hunk, LineKind, UnifiedRow};
/// The layout a diff is shown in, re-exported so callers need not reach into the settings module.
pub use micold_core::settings::DiffLayout;
use micold_core::tokens::{spacing, Rgb, Roles};

#[cfg(test)]
use super::virtual_rows::{visible_range, ASSUMED_VIEWPORT, OVERSCAN};
use super::{Button, Text, TypeRole, VirtualRows};

/// One row's height, hunk header or line: fixed, which is what lets `VirtualRows` build only the
/// visible rows.
pub const ROW_HEIGHT: f32 = 20.0;

/// The width of one line-number cell: six digits of the monospace text.
pub const NUMBER_WIDTH: f32 = 52.0;

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
    /// The unified rows, in order.
    Rows(Vec<UnifiedRow<'a>>),
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
fn body(diff: &FileDiff) -> Body<'_> {
    match diff {
        FileDiff::Text(hunks) if hunks.is_empty() => Body::Message(NO_CONTENT),
        FileDiff::Text(_) => Body::Rows(unified_rows(diff)),
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
    roles: Roles,
    offset: u32,
    viewport: u32,
    on_scroll: Option<Box<dyn Fn(u32, u32) -> M + 'a>>,
    on_show_large: Option<M>,
}

impl<'a, M: Clone + 'a> DiffView<'a, M> {
    /// `diff` in `layout`, themed by `roles`. Side by side renders unified until M3 (T041–T049).
    pub fn new(diff: &'a FileDiff, layout: DiffLayout, roles: Roles) -> Self {
        let _ = layout;
        Self {
            diff,
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

    /// The rows this view builds as it stands: the visible ones and the overscan (D5).
    #[cfg(test)]
    fn built_rows(&self) -> Range<usize> {
        let len = match body(self.diff) {
            Body::Rows(rows) => rows.len(),
            Body::Message(_) | Body::Large { .. } => return 0..0,
        };
        let viewport = if self.viewport == 0 {
            ASSUMED_VIEWPORT
        } else {
            self.viewport
        };
        visible_range(self.offset, viewport, ROW_HEIGHT, len, OVERSCAN)
    }
}

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
fn line_row<'a, M: 'a>(line: &'a DiffLine, r: Roles) -> Element<'a, M> {
    let marker = match line.kind {
        LineKind::Context => " ",
        LineKind::Added => "+",
        LineKind::Removed => "\u{2212}",
    };
    let cells = row![
        number(line.old, r),
        number(line.new, r),
        container(code(marker, r.on_surface_variant))
            .width(Length::Fixed(MARKER_WIDTH))
            .align_x(Horizontal::Center),
        container(code(line.text.as_str(), r.on_surface)).width(Length::Fill),
    ]
    .height(Length::Fixed(ROW_HEIGHT))
    .align_y(Alignment::Center);
    let fill = tint(line.kind, r);
    container(cells)
        .height(Length::Fixed(ROW_HEIGHT))
        .width(Length::Fill)
        .clip(true)
        .style(move |_| fill.map(background).unwrap_or_default())
        .into()
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
        match body(view.diff) {
            Body::Rows(rows) => {
                let len = rows.len();
                let mut list = VirtualRows::new(
                    len,
                    ROW_HEIGHT,
                    move |index| match rows[index] {
                        UnifiedRow::Header(hunk) => header_row(hunk, r),
                        UnifiedRow::Line(line) => line_row(line, r),
                    },
                    r,
                )
                .offset(view.offset)
                .viewport(view.viewport);
                if let Some(f) = view.on_scroll {
                    list = list.on_scroll(f);
                }
                list.into()
            }
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

    const TOLERANCE: f32 = 0.5;

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
        let mut element: Element<'_, ()> = line_row(line, LIGHT);
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
        let Body::Rows(rows) = body(&diff) else {
            panic!("a text diff has rows");
        };
        assert_eq!(rows.len(), 50_001, "one header and 50,000 lines");
        let view = DiffView::<()>::new(&diff, DiffLayout::Unified, LIGHT)
            .offset(400_000)
            .viewport(600);
        let built = view.built_rows();
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
        assert_eq!(body(&FileDiff::Binary), Body::Message(BINARY));
        assert_eq!(body(&FileDiff::NotUtf8), Body::Message(NOT_UTF8));
        assert_eq!(body(&FileDiff::ModeOnly), Body::Message(MODE_ONLY));
        assert_eq!(body(&FileDiff::Text(vec![])), Body::Message(NO_CONTENT));
        for diff in [FileDiff::Binary, FileDiff::NotUtf8, FileDiff::ModeOnly] {
            let view = DiffView::<()>::new(&diff, DiffLayout::Unified, DARK);
            assert_eq!(view.built_rows(), 0..0, "no rows, so no gutter");
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
            body(&diff),
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
        assert_eq!(view.built_rows(), 0..0);
        let _element: Element<'_, ()> = view.into();
    }
}
