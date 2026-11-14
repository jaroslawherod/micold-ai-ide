//! A user group's header fits the sidebar's one-line row (feature 1786, T015, research R7).
//!
//! The showcase's `group_header_row` — the header exactly as the sidebar draws it — is laid out at a
//! sidebar's narrow width: the row is the height `row_heights` gives a header, its name and count
//! are drawn inside it, and a name longer than the row is cut with an ellipsis (the full name is the
//! row's tooltip, see `GroupHeader`).
//!
//! Compiled into the `layout_snapshot` binary beside `disclosure_card_header_fits`.

use crate::support::layout as lay;
use iced::Size;
use micold_client::features::sidebar::{row_heights, GroupHeaderNode, SidebarEntry};
use micold_client::showcase::sections::surfaces::{group_header_row, LONG_GROUP_NAME};
use micold_core::sidebar_layout::UserGroupId;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// A narrow sidebar.
const ROOM: Size = Size::new(240.0, 200.0);

fn header_height() -> f32 {
    let heights = row_heights(&[SidebarEntry::UserGroup(GroupHeaderNode {
        id: UserGroupId(1),
        name: LONG_GROUP_NAME.to_string(),
        collapsed: false,
        count: 3,
    })]);
    assert_eq!(heights.len(), 1, "a header is one row: {heights:?}");
    heights[0]
}

#[test]
fn a_group_header_fits_its_one_line_row_and_cuts_a_long_name() {
    let height = header_height();
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let roles = tokens::roles(scheme);
        for (name, collapsed, count, words) in [
            ("Review", false, 3, "3"),
            (LONG_GROUP_NAME, true, 12, "12"),
            ("Later", false, 0, "empty"),
        ] {
            let what = format!("{scheme:?}, {name:?}");
            let mut renderer = lay::renderer();
            let (painted, records) = lay::painted_text_settled_at(
                group_header_row(name, collapsed, count, roles),
                &mut renderer,
                ROOM,
            );
            let row = records
                .iter()
                .find(|r| (r.height - height).abs() <= TOLERANCE && r.width > ROOM.width / 2.0)
                .unwrap_or_else(|| panic!("{what}: no {height} dp header row in {records:?}"));
            let (left, right) = (row.x, row.x + row.width);
            let (top, bottom) = (row.y, row.y + row.height);
            assert!(
                right <= ROOM.width + TOLERANCE,
                "{what}: the row fits: {row:?}"
            );
            let contents: Vec<_> = painted.iter().map(|p| p.content.as_str()).collect();
            let label = painted
                .iter()
                .find(|p| p.content.starts_with(&name[..5]))
                .unwrap_or_else(|| panic!("{what}: the name is drawn: {contents:?}"));
            assert!(
                painted.iter().any(|p| p.content == words),
                "{what}: `{words}` is drawn: {contents:?}"
            );
            if name == LONG_GROUP_NAME {
                assert!(
                    label.content.ends_with('…'),
                    "{what}: a long name is cut with an ellipsis: {:?}",
                    label.content
                );
            }
            for p in &painted {
                let width = p.natural_width.min(p.allowed_width);
                assert!(
                    p.origin.x >= left - TOLERANCE
                        && p.origin.x + width <= right + TOLERANCE
                        && p.origin.y >= top - TOLERANCE
                        && p.origin.y + p.natural_height.min(height) <= bottom + TOLERANCE,
                    "{what}: `{}` at {:?} ({width} wide) runs out of the header row {row:?}",
                    p.content,
                    p.origin,
                );
            }
        }
    }
}
