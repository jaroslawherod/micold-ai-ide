//! Truncation, measured with the real shaper (feature 021, T011 — FR-011d).
//!
//! `crates/micold-core/tests/typeahead_fit.rs` proves the search logic against a monospace
//! stand-in, where the arithmetic is checkable by eye. This proves the same rule survives contact
//! with the measurements it will actually be given: a proportional font where `i` and `W` differ by
//! a factor of four, and where the ellipsis is not one character wide.
//!
//! It lives in this crate rather than beside its sibling because `micold-core` is deliberately
//! iced-free and the shaper is iced's. `fit_around` takes its measurement as a parameter, which is
//! what lets one rule be proved in two places. `material/ellipsized.rs` splits its own tests the
//! same way, for the same reason.

use iced::advanced::text::{self, Paragraph as _};
use iced::{alignment, Pixels, Size};
use micold_core::typeahead::fit_around;
use std::ops::Range;

/// A long, realistic branch name and where `retry` sits inside it.
const LONG: &str = "feat/JIRA-412-checkout-flow_retry-v2";
const RETRY: Range<usize> = 28..33;

/// Roughly the width a result row leaves a branch name inside the add-worktree dialog.
const ROW_WIDTH: f32 = 180.0;

/// The width `content` really shapes to at the body type size.
fn shaped(content: &str) -> f32 {
    use iced::advanced::graphics::text::Paragraph;

    Paragraph::with_text(text::Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(14.0),
        line_height: text::LineHeight::default(),
        font: iced::Font::DEFAULT,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    })
    .min_bounds()
    .width
}

/// The text the returned spans point at.
fn emphasised(text: &str, spans: &[Range<usize>]) -> String {
    spans.iter().map(|s| &text[s.clone()]).collect()
}

/// The case the requirement exists for, at a width a row really has: the match sits past the point
/// where a trailing ellipsis would fall, so the window has to move.
#[test]
fn real_shaping_keeps_the_match_in_view() {
    assert!(
        shaped(LONG) > ROW_WIDTH,
        "the test is vacuous unless this name really does overflow a row"
    );

    let (out, spans) = fit_around(LONG, &[RETRY], ROW_WIDTH, shaped);

    assert!(
        shaped(&out) <= ROW_WIDTH,
        "{out:?} shapes to {} wide, over the {ROW_WIDTH} available",
        shaped(&out)
    );
    assert_eq!(
        emphasised(&out, &spans),
        "retry",
        "the emphasised text must survive the cut — {out:?}"
    );
}

/// Real glyph widths vary, so "as much as fits" cannot be an exact equality as it is for the
/// stand-in. Instead: the result must be close enough to the limit that it is not throwing away
/// room that was there.
#[test]
fn real_shaping_uses_the_room_it_has() {
    let (out, _) = fit_around(LONG, &[RETRY], ROW_WIDTH, shaped);
    let used = shaped(&out);

    assert!(
        used > ROW_WIDTH * 0.8,
        "{out:?} uses only {used} of {ROW_WIDTH} — the window stopped far short"
    );
}

/// A name that fits must be left exactly as it is: no ellipsis, no rebasing, nothing.
#[test]
fn real_shaping_leaves_a_short_name_untouched() {
    let (out, spans) = fit_around("main", &[0..4], ROW_WIDTH, shaped);

    assert_eq!(out, "main");
    assert_eq!(spans, vec![0..4]);
}
