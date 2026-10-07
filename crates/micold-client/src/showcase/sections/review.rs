//! The review surfaces (feature 482, T024).
//!
//! `VirtualRows` is what the Changes view lists a worktree's files in, and the point of it is a list
//! too long to build whole. So it is posed at the length that matters — 2,000 rows — with the
//! scroll it reports held in [`Showcase`], so scrolling to the end shows rows being built rather than
//! a blank band past the first screen.
//!
//! `DiffView` (M2, T038) is posed in the three shapes a selected file takes: a unified text diff with
//! added, removed and context rows on their tints, a binary file's message, and the large-file gate.
//! The page's scheme switch shows each in both schemes.
//!
//! M3 (T047) adds the side-by-side layout and the syntax colours: the same Rust diff side by side,
//! then coloured in each layout, the colours highlighted by the very function the Changes view runs
//! (`ui::syntax::highlight`) for the scheme the page is in.

use iced::widget::{container, row};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{spacing, Roles};

use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::state::{Message, Showcase};
use std::sync::LazyLock;

use micold_core::review::diff::{parse_unified, FileDiff, LoadedDiff, SideLines, Spans};
use micold_core::theme::ColorScheme;
use micold_core::tokens;

use crate::ui::syntax;

use crate::ui::material::{DiffLayout, DiffView, Tag, Text, TypeRole, VirtualRows};

/// Rows in the long-list pose: the length research R11 measures the list at.
pub const LONG_LIST: usize = 2_000;

/// One row's height, the same fixed height the Changes view's file rows use.
const ROW_HEIGHT: f32 = 32.0;

/// The pane the list scrolls in: a few screens' worth of rows would hide everything below it.
const PANE_HEIGHT: f32 = 240.0;

/// One fabricated changed-file row: a path, a kind tag and `+a −r`, as the Changes view lays it out.
fn file_row<'a>(index: usize, roles: Roles) -> Element<'a, Message> {
    let (kind, accent) = match index % 3 {
        0 => ("Modified", roles.secondary),
        1 => ("Added", roles.primary),
        _ => ("Deleted", roles.error),
    };
    container(
        row![
            container(Text::new(
                format!("src/module_{index:04}.rs"),
                TypeRole::Body,
                roles
            ))
            .width(Length::Fill),
            Tag::new(kind, accent),
            Text::new(
                format!("+{} \u{2212}{}", index % 40, index % 7),
                TypeRole::Caption,
                roles
            )
            .muted(),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center),
    )
    .height(Length::Fixed(ROW_HEIGHT))
    .padding([0.0, spacing::SM])
    .into()
}

/// `VirtualRows` — 2,000 rows, of which only those in view (plus an overscan) are built.
pub fn virtual_rows<'a>(showcase: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let (offset, viewport) = showcase.rows_scroll();
    let list: Element<'a, Message> =
        VirtualRows::new(LONG_LIST, ROW_HEIGHT, move |i| file_row(i, roles), roles)
            .offset(offset)
            .viewport(viewport)
            .on_scroll(|offset, viewport| Message::RowsScrolled { offset, viewport })
            .into();
    arrange(
        vec![posed(
            "2,000 rows",
            container(list).height(Length::Fixed(PANE_HEIGHT)),
            roles,
        )],
        Layout::FullWidth,
    )
}

/// A short Rust diff: a hunk header with its section, context, a removed and two added lines.
static UNIFIED: LazyLock<FileDiff> = LazyLock::new(|| {
    parse_unified(
        b"@@ -10,6 +10,7 @@ fn render(view: &View) {\n\
         \x20    let rows = view.rows();\n\
         \x20    let width = view.width();\n\
         -    draw(rows, width);\n\
         +    let height = view.height();\n\
         +    draw(rows, width, height);\n\
         \x20    view.finish();\n\
         \x20}\n\
         \x20\n",
    )
});

/// The file before and after [`UNIFIED`], so the coloured poses highlight real versions: the nine
/// lines in front of the hunk put the highlighter inside `fn render` by line 10, as in a real file.
const BEFORE: &str = "use crate::view::View;\n\n/// Draws `view` and marks it finished.\n#[inline]\n\
                      fn render(view: &View) {\n    // The rows and the width come first.\n\
                      \x20   let _title = \"rows\";\n    let _count = 42_u32;\n    debug_assert!(true);\n\
                      \x20   let rows = view.rows();\n    let width = view.width();\n\
                      \x20   draw(rows, width);\n    view.finish();\n}\n\n";

/// [`BEFORE`] with the hunk applied.
static AFTER: LazyLock<String> = LazyLock::new(|| {
    BEFORE.replace(
        "    draw(rows, width);\n",
        "    let height = view.height();\n    draw(rows, width, height);\n",
    )
});

/// [`UNIFIED`] with both versions and their syntax colours in both schemes.
static COLOURED: LazyLock<LoadedDiff> = LazyLock::new(|| {
    let mut loaded = LoadedDiff {
        diff: UNIFIED.clone(),
        old: SideLines::from_bytes(BEFORE.as_bytes()),
        new: SideLines::from_bytes(AFTER.as_bytes()),
        spans: Spans::default(),
    };
    loaded.spans = syntax::highlight(&loaded, "src/render.rs");
    loaded
});

/// A binary file.
static BINARY: FileDiff = FileDiff::Binary;

/// A file over the size limits.
static LARGE: FileDiff = FileDiff::TooLarge {
    added: 6_000,
    removed: 12,
};

/// The diff area's height in a pose.
const DIFF_HEIGHT: f32 = 200.0;

/// `DiffView` — a unified and a side-by-side diff, both again syntax-coloured, a binary file's
/// message, and the large-file gate.
pub fn diff_view<'a>(_showcase: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let pose = |diff: &'static FileDiff| {
        container(DiffView::new(diff, DiffLayout::Unified, roles).on_show_large(Message::NoOp))
            .height(Length::Fixed(DIFF_HEIGHT))
    };
    let spans = if roles == tokens::roles(ColorScheme::Dark) {
        &COLOURED.spans.dark
    } else {
        &COLOURED.spans.light
    };
    let coloured = |layout| {
        container(DiffView::new(&COLOURED.diff, layout, roles).spans(spans))
            .height(Length::Fixed(DIFF_HEIGHT))
    };
    arrange(
        vec![
            posed("unified", pose(&UNIFIED), roles),
            posed(
                "side by side",
                container(DiffView::new(&UNIFIED, DiffLayout::SideBySide, roles))
                    .height(Length::Fixed(DIFF_HEIGHT)),
                roles,
            ),
            posed(
                "unified, syntax-coloured",
                coloured(DiffLayout::Unified),
                roles,
            ),
            posed(
                "side by side, syntax-coloured",
                coloured(DiffLayout::SideBySide),
                roles,
            ),
            posed("binary message", pose(&BINARY), roles),
            posed("large-file gate", pose(&LARGE), roles),
        ],
        Layout::FullWidth,
    )
}
