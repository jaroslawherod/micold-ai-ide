//! The top bar's meter and Changes button give way in order as the window narrows (feature 1793,
//! T028 — FR-011, US5 scenarios 1 and 2): the ring's text hides first while the ring stays, then
//! the Changes label while its icon stays; nothing overlaps or leaves the bar at any width.
//!
//! **Compiled into the `layout_snapshot` binary** for the `support::layout` passes; it builds its
//! own state and reads no cache.

use crate::support::covered_states::transcript_pane_insights;
use crate::support::layout::{self as lay, StateUnderTest};
use iced::Size;
use micold_client::icons::Icon;
use micold_core::tokens::anatomy::app_bar;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The bar's lane, and the children it lays out.
const LANE: &[usize] = &[0, 0, 0, 0, 0];

/// What the bar draws at `width`: the ring's text, the Changes label, the Git icon.
struct Drawn {
    ring_text: bool,
    label: bool,
    icon: bool,
}

/// Lay the bar out at `width` and read what it draws, asserting no child overlaps or leaves it.
fn bar_at(width: f32) -> Drawn {
    bar_of(transcript_pane_insights(), width)
}

/// [`bar_at`] for a given state.
fn bar_of(state: micold_client::app::State, width: f32) -> Drawn {
    let window = Size::new(width, 700.0);
    let under = StateUnderTest::new(state);
    let (painted, records) =
        lay::painted_text_settled_at(lay::view_of(&under), &mut lay::renderer(), window);
    let children: Vec<_> = records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == LANE.len() + 1
                && r.path.starts_with(LANE)
        })
        .collect();
    for (i, a) in children.iter().enumerate() {
        assert!(
            a.width > 0.0
                && a.x >= -TOLERANCE
                && a.x + a.width <= width + TOLERANCE
                && a.y + a.height <= app_bar::HEIGHT + TOLERANCE,
            "{width}: child {i} at {a:?} leaves the bar"
        );
        for b in &children[i + 1..] {
            assert!(
                a.x + a.width <= b.x + TOLERANCE || b.x + b.width <= a.x + TOLERANCE,
                "{width}: {a:?} overlaps {b:?}"
            );
        }
    }
    let in_bar: Vec<&str> = painted
        .iter()
        .filter(|p| p.origin.y < app_bar::HEIGHT)
        .map(|p| p.content.as_str())
        .collect();
    let git = Icon::Git.glyph().to_string();
    Drawn {
        ring_text: in_bar.iter().any(|c| c.contains('%') && c.contains('$')),
        label: in_bar.iter().any(|c| c.starts_with("Changes")),
        icon: in_bar.iter().any(|c| c.contains(git.as_str())),
    }
}

#[test]
fn at_a_wide_window_the_ring_text_and_the_changes_label_are_drawn() {
    let wide = bar_at(1280.0);
    assert!(wide.ring_text, "the ring's text at 1280");
    assert!(
        wide.label && wide.icon,
        "the Changes label and icon at 1280"
    );
}

#[test]
fn the_ring_text_hides_first_then_the_changes_label_and_the_icon_stays() {
    let mut seen_text_hidden = false;
    let mut seen_label_hidden = false;
    let mut width = 1280.0;
    while width >= 640.0 {
        let d = bar_at(width);
        assert!(d.icon, "{width}: the Changes icon stays");
        if !d.ring_text {
            seen_text_hidden = true;
        }
        assert!(
            !(seen_text_hidden && d.ring_text),
            "{width}: the ring's text came back after it hid"
        );
        if !d.label {
            seen_label_hidden = true;
            assert!(
                !d.ring_text,
                "{width}: the label hid before the ring's text"
            );
        }
        assert!(
            !(seen_label_hidden && d.label),
            "{width}: the Changes label came back after it hid"
        );
        width -= 20.0;
    }
    assert!(seen_text_hidden, "the ring's text hides by 640");
    assert!(seen_label_hidden, "the Changes label hides by 640");
}

/// The widest counts a branch ever shows (five-digit files, six-digit lines) keep the label whole
/// beside the ring at the layout window and overlap nothing at the smallest one (the gate the
/// header's own Changes button had before the header moved here, M4 review).
#[test]
fn the_widest_counts_fit_at_the_layout_window_and_the_smallest() {
    let widest = micold_core::context_header::ChangeSummary {
        files: 12_345,
        added: 987_654,
        removed: 876_543,
    };
    for width in [1280.0, 800.0] {
        let mut state = transcript_pane_insights();
        let location = state
            .current_session_location()
            .expect("a session is shown");
        let project = state.workspace.active.clone().expect("a project is active");
        state.changes.header.entry = Some((project, location));
        state.changes.header.load = micold_client::features::changes::Load::Ready(widest);
        let d = bar_of(state, width);
        assert!(d.icon, "{width}: the Changes icon stays");
        if width >= 1280.0 {
            assert!(
                d.label,
                "{width}: the label with the widest counts is drawn"
            );
        }
    }
}
