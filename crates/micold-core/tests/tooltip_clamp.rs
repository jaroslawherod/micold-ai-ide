//! A text cut to at most `n` lines (038 contracts/rest-tooltip.md §5, data-model §6).
//!
//! `clamp_to_lines` knows nothing about fonts: the caller hands it a measure. Here the measure is
//! a fake — forty characters to a line — so every case is exact and needs no renderer.

use std::borrow::Cow;

use micold_core::tooltip::clamp_to_lines;

const MAX_LINES: usize = 3;
const PER_LINE: usize = 40;

/// The fake measure: `ceil(chars / 40)` lines.
fn lines_of(text: &str) -> usize {
    text.chars().count().div_ceil(PER_LINE)
}

/// `len` characters of words of differing lengths, separated by single spaces.
fn words(len: usize) -> String {
    const WORDS: [&str; 5] = ["the", "issue", "picker", "cuts", "descriptions"];
    let mut text = String::new();
    let mut n = 0;
    while text.chars().count() < len {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(WORDS[n % WORDS.len()]);
        n += 1;
    }
    text.chars().take(len).collect()
}

#[test]
fn a_text_that_fits_is_returned_borrowed_and_unchanged() {
    for len in [100, MAX_LINES * PER_LINE] {
        let text = words(len);

        let out = clamp_to_lines(&text, MAX_LINES, lines_of);

        assert!(
            matches!(out, Cow::Borrowed(_)),
            "{len} characters fit {MAX_LINES} lines: borrowed, not rebuilt",
        );
        assert_eq!(out, text.as_str());
        assert!(!out.contains('…'));
    }
}

#[test]
fn overflowing_words_are_cut_after_a_whole_word_and_end_in_one_ellipsis() {
    let text = words(500);

    let out = clamp_to_lines(&text, MAX_LINES, lines_of);

    assert!(lines_of(&out) <= MAX_LINES, "{} lines", lines_of(&out));
    let kept = out.strip_suffix('…').expect("a cut text ends in `…`");
    assert!(!kept.ends_with('…'), "one ellipsis, not two: {out:?}");
    assert!(
        !kept.ends_with(' '),
        "no space before the ellipsis: {out:?}"
    );
    assert!(text.starts_with(kept), "the start of the text is kept");
    assert_eq!(
        text[kept.len()..].chars().next(),
        Some(' '),
        "the cut falls after a whole word: {out:?}",
    );
    assert!(
        kept.chars().count() >= MAX_LINES * PER_LINE - 1 - 24,
        "it backs up to a word boundary no further than 24 characters: kept {}",
        kept.chars().count(),
    );
}

#[test]
fn a_text_already_ending_in_an_ellipsis_ends_in_one() {
    // The longest prefix that fits with its `…` is 119 characters; the 119th here is itself `…`.
    let mut text = "x".repeat(118);
    text.push('…');
    text.push_str(&"y".repeat(200));

    let out = clamp_to_lines(&text, MAX_LINES, lines_of);

    assert_eq!(out, format!("{}…", "x".repeat(118)));
}

#[test]
fn a_text_without_spaces_is_cut_at_a_character_and_ends_in_an_ellipsis() {
    let text = "é".repeat(500);

    let out = clamp_to_lines(&text, MAX_LINES, lines_of);

    assert!(lines_of(&out) <= MAX_LINES);
    assert_eq!(
        out,
        format!("{}…", "é".repeat(MAX_LINES * PER_LINE - 1)),
        "as much as fits, cut at a character boundary",
    );
}

#[test]
fn no_length_yields_more_than_the_limit() {
    // No property library in this workspace: lengths sampled across the boundary and far past it,
    // up to GitHub's body limit, for spaced and unspaced text.
    let lengths = (0..=400).chain([599, 600, 601, 4_096, 65_536]);
    for len in lengths {
        for text in [words(len), "x".repeat(len)] {
            let out = clamp_to_lines(&text, MAX_LINES, lines_of);
            assert!(
                lines_of(&out) <= MAX_LINES,
                "{len} characters gave {} lines: {out:?}",
                lines_of(&out),
            );
            assert_eq!(
                out.ends_with('…'),
                lines_of(&text) > MAX_LINES,
                "an ellipsis exactly when the text was cut ({len} characters)",
            );
        }
    }
}
