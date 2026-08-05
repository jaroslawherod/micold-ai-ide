//! Truncation that keeps the match in view (feature 021, contract `match-ranking.md` §4, FR-011d).
//!
//! A branch name too long for its row has to give way somewhere, and where it gives way is not a
//! detail: cut the end off `feat/JIRA-412-checkout-flow_retry-v2` and a developer who searched
//! `retry` gets a row with no visible reason for being there. So the window follows the emphasis
//! rather than always starting at the beginning.
//!
//! Two levels, as `material/ellipsized.rs` does it. Most of the file uses a monospace stand-in, so
//! the arithmetic is checkable by eye; the last few use the real shaper, so the rule is also proved
//! against the measurements it will actually be given. A proportional font differs in the numbers,
//! never in the rule.

use micold_core::typeahead::fit_around;
use std::ops::Range;

/// One unit per character.
fn mono(s: &str) -> f32 {
    s.chars().count() as f32
}

/// The text the returned spans point at — the thing that actually has to survive truncation.
fn emphasised(text: &str, spans: &[Range<usize>]) -> String {
    spans.iter().map(|s| &text[s.clone()]).collect()
}

const ELLIPSIS: char = '…';

/// A long, realistic name and where `retry` sits inside it.
const LONG: &str = "feat/JIRA-412-checkout-flow_retry-v2";
const RETRY: Range<usize> = 28..33;
const JIRA: Range<usize> = 5..9;

/// Q4.3 — a name that fits is not to be touched. No ellipsis, no rebasing, nothing.
#[test]
fn a_name_that_fits_comes_back_untouched() {
    let (out, spans) = fit_around("feat/login", &[5..8], 20.0, mono);

    assert_eq!(out, "feat/login");
    assert_eq!(spans, vec![5..8]);
}

/// Exactly filling the space is fitting, not overflowing.
#[test]
fn a_name_that_exactly_fits_comes_back_untouched() {
    let (out, _) = fit_around("feat/login", &[5..8], 10.0, mono);
    assert_eq!(out, "feat/login");
}

/// Q4.1 — the case the requirement exists for. The match is near the end, so the cut has to come
/// off the **front**, and the row opens with an ellipsis.
#[test]
fn a_match_near_the_end_keeps_its_text_and_cuts_the_front() {
    let (out, spans) = fit_around(LONG, &[RETRY], 20.0, mono);

    assert!(out.starts_with(ELLIPSIS), "expected a leading ellipsis, got {out:?}");
    assert_eq!(emphasised(&out, &spans), "retry", "the match must survive the cut");
    assert!(mono(&out) <= 20.0, "{out:?} is {} wide, over the 20 available", mono(&out));
}

/// Q4.2 — and the mirror image: a match near the start cuts the tail instead.
#[test]
fn a_match_near_the_start_keeps_its_text_and_cuts_the_tail() {
    let (out, spans) = fit_around(LONG, &[JIRA], 20.0, mono);

    assert!(out.ends_with(ELLIPSIS), "expected a trailing ellipsis, got {out:?}");
    assert!(!out.starts_with(ELLIPSIS), "nothing was cut from the front, so nothing marks it");
    assert_eq!(emphasised(&out, &spans), "JIRA");
    assert!(mono(&out) <= 20.0);
}

/// A match stranded in the middle of a very long name needs both ends cut, and says so at both
/// ends.
#[test]
fn a_match_in_the_middle_of_a_long_name_cuts_both_ends() {
    let name = "aaaaaaaaaaaaaaaaaaaaretryaaaaaaaaaaaaaaaaaaaa";
    let (out, spans) = fit_around(name, &[20..25], 15.0, mono);

    assert!(out.starts_with(ELLIPSIS) && out.ends_with(ELLIPSIS), "got {out:?}");
    assert_eq!(emphasised(&out, &spans), "retry");
    assert!(mono(&out) <= 15.0);
}

/// Truncation gives back as much as it can. A window far short of the space available would be
/// correct and useless.
#[test]
fn truncation_keeps_as_much_as_the_row_allows() {
    let (out, _) = fit_around(LONG, &[RETRY], 20.0, mono);
    assert_eq!(mono(&out), 20.0, "{out:?} left room unused");
}

/// Q4.4 — every returned range must be inside the returned string and land on character
/// boundaries, or the caller slices it and panics.
#[test]
fn returned_spans_are_valid_ranges_of_the_returned_string() {
    for available in [8.0, 12.0, 20.0, 30.0, 100.0] {
        let (out, spans) = fit_around(LONG, &[RETRY], available, mono);
        for span in &spans {
            assert!(span.end <= out.len(), "span {span:?} runs past {out:?}");
            assert!(out.is_char_boundary(span.start) && out.is_char_boundary(span.end));
        }
    }
}

/// Q4.5 — cutting mid-character would panic on a byte index. Cutting mid-word is merely ugly.
#[test]
fn multibyte_names_are_cut_at_character_boundaries() {
    let name = "機能/日本語のブランチ名前";
    let spans = [name.find("ブランチ").unwrap()..name.find("ブランチ").unwrap() + "ブランチ".len()];

    let (out, spans) = fit_around(name, &spans, 6.0, mono);

    assert!(out.chars().count() <= 6);
    assert!(emphasised(&out, &spans).contains('ブ'), "the match must still be visible: {out:?}");
}

/// Several scattered spans — a subsequence match — must all be kept when they fit, and rebased
/// together.
#[test]
fn scattered_spans_are_all_rebased() {
    let name = "feat/reporting";
    let (out, spans) = fit_around(name, &[0..1, 5..8], 40.0, mono);

    assert_eq!(out, name);
    assert_eq!(emphasised(&out, &spans), "frep");
}

/// A row with no room at all must still return something drawable rather than panicking or
/// looping.
#[test]
fn degenerate_widths_still_terminate() {
    let (out, _) = fit_around(LONG, &[RETRY], 0.0, mono);
    assert!(out.chars().count() <= 2, "expected almost nothing, got {out:?}");

    let (out, spans) = fit_around("", &[], 10.0, mono);
    assert_eq!(out, "");
    assert!(spans.is_empty());
}

/// A match too wide for the row keeps the part of itself that fits, emphasised.
///
/// The window has to cut into the match here — there is no width at which all of it shows — and the
/// tempting shortcut is to keep only spans that survive whole. That returns a row whose visible
/// text *is* the search term, drawn as if it were not: a result with no visible reason for being in
/// the list, which is the outcome this whole function exists to prevent.
#[test]
fn a_match_wider_than_the_row_keeps_the_part_that_fits_emphasised() {
    let name = "feat/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-tail";
    let matched = 5..37; // the 32 `a`s, far wider than the 10 units on offer

    let (out, spans) = fit_around(name, &[matched], 10.0, mono);

    assert!(mono(&out) <= 10.0, "{out:?} is {} wide, over the 10 available", mono(&out));
    assert!(!spans.is_empty(), "{out:?} came back with nothing emphasised at all");
    let shown = emphasised(&out, &spans);
    assert!(
        !shown.is_empty() && shown.chars().all(|c| c == 'a'),
        "the emphasis must land on the match's own characters, got {shown:?} in {out:?}"
    );
}

/// A name with no emphasis at all — the empty-query case — still truncates, just from the end.
#[test]
fn a_name_with_no_emphasis_truncates_from_the_end() {
    let (out, spans) = fit_around(LONG, &[], 12.0, mono);

    assert!(out.ends_with(ELLIPSIS));
    assert!(out.starts_with("feat/"), "with nothing to keep in view, keep the beginning");
    assert!(spans.is_empty());
}

// The same rules against the **real** shaper live in
// `crates/micold-client/tests/typeahead_fit_shaping.rs`, not here: `micold-core` is deliberately
// iced-free (Principle V's crate split), and the shaper is iced's. `fit_around` takes its
// measurement as a parameter precisely so the rule can be proved in both places — the arithmetic
// here, the contact with real glyph widths there.
