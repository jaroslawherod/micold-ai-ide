//! The composer's shared parts hold their content (feature 941, T048, FR-008, FR-010).
//!
//! The showcase's poses of `TextArea` with a height limit, `BusyIndicator`, `QueueRow` and
//! `AttachmentChip` are laid out in both schemes. A text area grows with its lines up to its limit
//! and then holds that height, its lines scrolling inside it (FR-008); a busy indicator draws its
//! words whether busy or idle, never its colour alone (FR-010); a queue row cuts its text to one
//! line beside its position and its pictures' count; a chip draws its file name inside itself.
//!
//! Compiled into the `layout_snapshot` binary beside `code_block_fits`.

use crate::support::layout as lay;
use iced::widget::text_editor::Content;
use iced::Size;
use micold_client::showcase::sections::composer::{
    attachment_chip_sample, busy_indicator_sample, queue_row_sample, text_area_sample,
    ATTACHMENT_NAME, BUSY_LABEL, IDLE_LABEL, QUEUED_TEXT, TEXT_AREA_LIMIT,
};
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// A narrow pane, wider than the posed parts.
const ROOM: Size = Size::new(640.0, 800.0);

/// One label line, the most one-line text may take.
const LINE: f32 = 24.0;

fn schemes() -> [tokens::Roles; 2] {
    [ColorScheme::Light, ColorScheme::Dark].map(tokens::roles)
}

fn root_of(records: &[lay::LayoutRecord]) -> &lay::LayoutRecord {
    records
        .iter()
        .find(|r| r.path.is_empty())
        .unwrap_or_else(|| panic!("no root record in {records:?}"))
}

#[test]
fn a_text_area_grows_with_its_lines_up_to_its_limit_and_then_holds_it() {
    for roles in schemes() {
        let heights: Vec<f32> = [1usize, 3, 6, 40, 400]
            .iter()
            .map(|&lines| {
                let text = (1..=lines)
                    .map(|n| format!("line {n}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let content = Content::with_text(&text);
                let mut renderer = lay::renderer();
                let records = lay::resolve(text_area_sample(&content, roles), &mut renderer);
                root_of(&records).height
            })
            .collect();
        assert!(
            heights.windows(2).all(|w| w[0] <= w[1] + TOLERANCE),
            "the area never shrinks as lines are added: {heights:?}"
        );
        assert!(
            heights[1] > heights[0] + TOLERANCE,
            "three lines are taller than one: {heights:?}"
        );
        assert!(
            heights[1] < TEXT_AREA_LIMIT,
            "three lines are under the limit: {heights:?}"
        );
        for h in &heights[3..] {
            assert!(
                (h - TEXT_AREA_LIMIT).abs() <= TOLERANCE,
                "past the limit the area holds it ({TEXT_AREA_LIMIT}): {heights:?}"
            );
        }
    }
}

#[test]
fn a_busy_indicator_says_so_in_words_busy_or_idle() {
    for roles in schemes() {
        for (busy, label) in [(true, BUSY_LABEL), (false, IDLE_LABEL)] {
            let mut renderer = lay::renderer();
            let (painted, _) = lay::painted_text_settled_at(
                busy_indicator_sample(busy, roles),
                &mut renderer,
                ROOM,
            );
            assert!(
                painted.iter().any(|p| p.content == label),
                "busy {busy}: `{label}` is drawn: {:?}",
                painted.iter().map(|p| &p.content).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn a_queue_row_cuts_its_text_to_one_line_beside_its_position_and_count() {
    for roles in schemes() {
        let mut renderer = lay::renderer();
        let (painted, records) =
            lay::painted_text_settled_at(queue_row_sample(roles), &mut renderer, ROOM);
        let root = root_of(&records);
        assert!(
            root.x + root.width <= ROOM.width + TOLERANCE,
            "the row fits its room: {root:?}"
        );
        let contents: Vec<_> = painted.iter().map(|p| p.content.as_str()).collect();
        for part in ["2", "3 pictures", "Edit", "Remove", "Move to composer"] {
            assert!(contents.contains(&part), "`{part}` is drawn: {contents:?}");
        }
        let first_line = QUEUED_TEXT.lines().next().expect("a first line");
        let text = painted
            .iter()
            .find(|p| p.content.starts_with(&first_line[..10]))
            .unwrap_or_else(|| panic!("the queued text is drawn: {contents:?}"));
        assert!(
            !text.content.contains('\n') && text.natural_height <= LINE,
            "the queued text is one line: {:?}",
            text.content
        );
        assert!(
            text.content.ends_with('…'),
            "a text longer than the row is cut with an ellipsis: {:?}",
            text.content
        );
        assert!(
            text.origin.x + text.natural_width.min(text.allowed_width)
                <= root.x + root.width + TOLERANCE,
            "the text stays inside the row: {root:?}"
        );
    }
}

#[test]
fn an_attachment_chip_draws_its_name_inside_itself() {
    for roles in schemes() {
        let mut renderer = lay::renderer();
        let (painted, records) =
            lay::painted_text_settled_at(attachment_chip_sample(roles), &mut renderer, ROOM);
        let root = root_of(&records);
        let name = painted
            .iter()
            .find(|p| p.content == ATTACHMENT_NAME)
            .unwrap_or_else(|| {
                panic!(
                    "`{ATTACHMENT_NAME}` is drawn: {:?}",
                    painted.iter().map(|p| &p.content).collect::<Vec<_>>()
                )
            });
        assert!(
            name.origin.x >= root.x - TOLERANCE
                && name.origin.x + name.natural_width <= root.x + root.width + TOLERANCE,
            "the name is inside the chip {root:?}: {:?}",
            name.origin
        );
    }
}
