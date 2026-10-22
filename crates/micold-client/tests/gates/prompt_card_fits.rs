//! A permission prompt's action row stays inside its card (feature 942, T057, FR-006–FR-008).
//!
//! The showcase's three request views — a tool request, a plan approval and a question — are laid
//! out in both schemes at the narrowest pane width a split leaves (`MIN_PANE_COLS` cells of 8 px).
//! Every action is drawn, and drawn inside the card: a button pushed past the card's edge is one
//! the user cannot see, let alone press. The row wraps rather than overflowing.
//!
//! Compiled into the `layout_snapshot` binary beside `composer_parts_fit`.

use crate::support::layout as lay;
use iced::Size;
use micold_client::showcase::sections::prompts::{prompt_card_sample, PromptPose};
use micold_client::ui::transcript::permission::{
    ALLOW, ALWAYS_ALLOW, APPROVE, APPROVE_ACCEPT_EDITS, DECLINE, DENY, KEEP_PLANNING, SUBMIT,
};
use micold_core::pane_layout::MIN_PANE_COLS;
use micold_core::theme::ColorScheme;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The narrowest pane: `MIN_PANE_COLS` terminal cells of 8 px, as `split_view` measures a pane.
const NARROWEST: Size = Size::new(MIN_PANE_COLS as f32 * 8.0, 4_000.0);

fn root_of(records: &[lay::LayoutRecord]) -> &lay::LayoutRecord {
    records
        .iter()
        .find(|r| r.path.is_empty())
        .unwrap_or_else(|| panic!("no root record in {records:?}"))
}

#[test]
fn every_action_of_each_request_view_is_drawn_inside_its_card_at_the_narrowest_pane() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        for (pose, actions) in [
            (PromptPose::Tool, &[ALLOW, DENY, ALWAYS_ALLOW][..]),
            (
                PromptPose::Plan,
                &[APPROVE_ACCEPT_EDITS, APPROVE, KEEP_PLANNING][..],
            ),
            (PromptPose::Question, &[SUBMIT, DECLINE][..]),
        ] {
            let mut renderer = lay::renderer();
            let (painted, records) = lay::painted_text_settled_at(
                prompt_card_sample(pose, scheme),
                &mut renderer,
                NARROWEST,
            );
            let card = root_of(&records);
            assert!(
                card.width <= NARROWEST.width + TOLERANCE,
                "{scheme:?} {pose:?}: the card fits the pane: {card:?}"
            );
            for action in actions {
                let label = painted
                    .iter()
                    .find(|p| p.content == *action)
                    .unwrap_or_else(|| {
                        panic!(
                            "{scheme:?} {pose:?}: `{action}` is drawn: {:?}",
                            painted.iter().map(|p| &p.content).collect::<Vec<_>>()
                        )
                    });
                let right = label.origin.x + label.natural_width.min(label.allowed_width);
                assert!(
                    label.origin.x >= card.x - TOLERANCE
                        && right <= card.x + card.width + TOLERANCE
                        && label.natural_width <= label.allowed_width + TOLERANCE,
                    "{scheme:?} {pose:?}: `{action}` stays inside the card {card:?}: {label:?}"
                );
                assert!(
                    label.origin.y >= card.y - TOLERANCE
                        && label.origin.y + label.natural_height
                            <= card.y + card.height + TOLERANCE,
                    "{scheme:?} {pose:?}: `{action}` is inside the card's height {card:?}: \
                     {label:?}"
                );
            }
        }
    }
}
