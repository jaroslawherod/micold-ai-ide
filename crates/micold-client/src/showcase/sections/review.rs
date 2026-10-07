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

use iced::widget::{container, row};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{spacing, Roles};

use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::state::{Message, Showcase};
use std::sync::LazyLock;

use micold_core::review::diff::{parse_unified, FileDiff};
use micold_core::settings::DiffLayout;

use crate::ui::material::{DiffView, Tag, Text, TypeRole, VirtualRows};

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
        b"@@ -10,6 +10,7 @@ fn render(view: &View) {\n \
          let rows = view.rows();\n \
          let width = view.width();\n\
         -    draw(rows, width);\n\
         +    let height = view.height();\n\
         +    draw(rows, width, height);\n \
          view.finish();\n \
          }\n \n",
    )
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

/// `DiffView` — a unified diff, a binary file's message, and the large-file gate.
pub fn diff_view<'a>(_showcase: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let pose = |diff: &'static FileDiff| {
        container(DiffView::new(diff, DiffLayout::Unified, roles).on_show_large(Message::NoOp))
            .height(Length::Fixed(DIFF_HEIGHT))
    };
    arrange(
        vec![
            posed("unified", pose(&UNIFIED), roles),
            posed("binary message", pose(&BINARY), roles),
            posed("large-file gate", pose(&LARGE), roles),
        ],
        Layout::FullWidth,
    )
}
