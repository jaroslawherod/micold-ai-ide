//! The two lines an issue row shows, and where a match's emphasis lands on them (feature 038,
//! contracts/issue-fields.md §3–4, data-model §3–4).

// An emphasis is a list of byte ranges, and here it usually holds exactly one: `vec![0..5]` is the
// value under test, not a mistyped `(0..5).collect()`.
#![allow(clippy::single_range_in_vec_init)]

use std::ops::Range;

use micold_core::github::{Issue, RowEmphasis};

/// The separator between the parts of a line: two spaces, a middle dot (2 bytes), two spaces.
const SEPARATOR_BYTES: usize = 6;

fn issue(number: u64, title: &str, reporter: Option<&str>, labels: &[&str]) -> Issue {
    let issue = Issue::new(
        number,
        title.to_string(),
        labels.iter().map(|l| l.to_string()).collect(),
        "t".to_string(),
    );
    match reporter {
        Some(login) => issue.reported_by(login),
        None => issue,
    }
}

/// The contract's issue #7: "Fix it", by `ana`, labels `bug` and `ui`.
fn seven() -> Issue {
    issue(7, "Fix it", Some("ana"), &["bug", "ui"])
}

/// The byte range of `needle` in `text`.
fn range_of(text: &str, needle: &str) -> Range<usize> {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is in {text:?}"));
    start..start + needle.len()
}

/// U7 — the first line is the number and the title (FR-001).
#[test]
fn the_title_line_is_the_number_and_the_title() {
    assert_eq!(seven().title_line(), "#7 Fix it", "`#<number> <title>`");
    assert_eq!(
        issue(8, "Docs", Some("ana"), &[]).title_line(),
        "#8 Docs",
        "labels and reporter are not on the title line"
    );
}

/// U8 — the second line is the reporter, then the labels after the separator (FR-002, FR-003).
#[test]
fn the_details_line_is_the_reporter_then_the_labels() {
    assert_eq!(
        seven().details_line(),
        "ana  ·  bug, ui",
        "the reporter first, then the separator and the comma-joined labels"
    );
}

/// U9 — without labels the reporter stands alone; without an author the reporter is `ghost`.
#[test]
fn the_details_line_without_labels_is_the_reporter_alone() {
    assert_eq!(
        issue(8, "Docs", Some("ana"), &[]).details_line(),
        "ana",
        "no separator and no empty label area when there are no labels"
    );
    assert_eq!(
        issue(9, "Old", None, &["bug"]).details_line(),
        "ghost  ·  bug",
        "an issue without an author is shown as reported by ghost"
    );
}

/// U10 — before reporters are searched (M3), the match text is today's: number, title, labels.
#[test]
fn the_match_text_omits_the_reporter() {
    assert_eq!(
        seven().row_text(),
        "#7 Fix it  ·  bug, ui",
        "the match text is `#N title` and the labels, as before this feature"
    );
    assert_eq!(
        issue(8, "Docs", Some("ana"), &[]).row_text(),
        "#8 Docs",
        "an unlabelled issue's match text is its title line"
    );
}

/// U11 — a match in the title is emphasised at the same place on the title line.
#[test]
fn a_span_in_the_title_maps_to_the_title_line() {
    let seven = seven();
    let fix = range_of(seven.row_text(), "Fix");
    assert_eq!(
        seven.emphasis(std::slice::from_ref(&fix)),
        RowEmphasis {
            title: vec![3..6],
            details: vec![],
        },
        "the title part of the match text is the title line"
    );
    assert_eq!(
        &seven.title_line()[3..6],
        "Fix",
        "the range covers the match"
    );
}

/// U12 — a match in a label is emphasised on the details line, after the reporter and the
/// separator.
#[test]
fn a_span_in_the_labels_maps_to_the_details_line() {
    let seven = seven();
    let ui = range_of(seven.row_text(), "ui");
    let emphasis = seven.emphasis(&[ui]);
    let after_reporter = "ana".len() + SEPARATOR_BYTES;
    let ui_in_labels = "bug, ".len();
    assert_eq!(
        emphasis,
        RowEmphasis {
            title: vec![],
            details: vec![after_reporter + ui_in_labels..after_reporter + ui_in_labels + 2],
        },
        "the labels part starts after the reporter and its 6-byte separator"
    );
    assert_eq!(
        &seven.details_line()[emphasis.details[0].clone()],
        "ui",
        "the range covers the match"
    );
}

/// U13 — a match crossing the separator is emphasised on both lines, and the separator itself
/// never is.
#[test]
fn a_span_crossing_the_separator_is_split() {
    let seven = seven();
    let title_end = "#7 Fix it".len();
    let labels_start = title_end + SEPARATOR_BYTES;
    // "it", the separator, "bu".
    let crossing = title_end - 2..labels_start + 2;
    let emphasis = seven.emphasis(&[crossing]);
    assert_eq!(
        emphasis,
        RowEmphasis {
            title: vec![7..9],
            details: vec![9..11],
        },
        "each side keeps its own part of the span"
    );
    assert_eq!(&seven.title_line()[7..9], "it", "the title's share");
    assert_eq!(&seven.details_line()[9..11], "bu", "the labels' share");

    let inside_separator = title_end + 1..labels_start - 2;
    assert_eq!(
        seven.emphasis(&[inside_separator]),
        RowEmphasis::default(),
        "the separator's bytes carry no emphasis"
    );
}

/// U14 — the mapping is total: a span beyond the match text is dropped, and an issue without
/// labels has nothing to emphasise on its details line.
#[test]
fn a_span_outside_every_part_is_dropped() {
    let seven = seven();
    let end = seven.row_text().len();
    assert_eq!(
        seven.emphasis(&[end + 8..end + 18]),
        RowEmphasis::default(),
        "a span beyond the match text emphasises nothing"
    );
    assert_eq!(
        seven.emphasis(&[end - 2..end + 18]),
        RowEmphasis {
            title: vec![],
            details: vec![14..16],
        },
        "a span running past the end keeps the part inside the labels"
    );

    let unlabelled = issue(8, "Docs", Some("ana"), &[]);
    let whole = 0..unlabelled.row_text().len() + 20;
    assert_eq!(
        unlabelled.emphasis(&[whole]),
        RowEmphasis {
            title: vec![0.."#8 Docs".len()],
            details: vec![],
        },
        "an issue without labels yields no details emphasis"
    );
}

/// Ranges sorted, not overlapping or touching out of order, inside `line` and on its character
/// boundaries.
fn assert_well_formed(line: &str, ranges: &[Range<usize>]) {
    let mut last_end = 0;
    for range in ranges {
        assert!(
            range.start < range.end,
            "a range is never empty: {ranges:?}"
        );
        assert!(
            range.start >= last_end,
            "ranges are sorted and do not overlap: {ranges:?}"
        );
        assert!(
            line.is_char_boundary(range.start) && line.is_char_boundary(range.end),
            "{range:?} lies on character boundaries of {line:?}"
        );
        last_end = range.end;
    }
}

/// U15 — whatever spans come in, each line's ranges are sorted, do not overlap and lie on character
/// boundaries, so a multi-byte title or label can be sliced by them.
#[test]
fn ranges_are_sorted_disjoint_and_on_character_boundaries() {
    let goose = issue(9, "Zażółć gęślą jaźń", Some("ana"), &["błąd", "żółty"]);
    let row = goose.row_text().to_string();
    let title_line = goose.title_line();
    let details_line = goose.details_line();

    // Out of order, across both lines.
    let in_order = goose.emphasis(&[range_of(&row, "żółty"), range_of(&row, "gęślą")]);
    assert_eq!(
        in_order.title,
        [range_of(&title_line, "gęślą")],
        "a multi-byte title match maps to its bytes on the title line"
    );
    assert_eq!(
        in_order.details,
        [range_of(&details_line, "żółty")],
        "a multi-byte label match maps to its bytes on the details line"
    );

    // Overlapping and unsorted spans become sorted, disjoint ranges that cover the same text.
    let zazolc = range_of(&row, "Zażółć");
    let overlapping = goose.emphasis(&[
        range_of(&row, "żółć gęś"),
        zazolc.clone(),
        range_of(&row, "błąd"),
        range_of(&row, "łąd, żó"),
    ]);
    assert_well_formed(&title_line, &overlapping.title);
    assert_well_formed(&details_line, &overlapping.details);
    assert_eq!(
        overlapping.title,
        [range_of(&title_line, "Zażółć gęś")],
        "overlapping title spans are one range"
    );
    assert_eq!(
        overlapping.details,
        [range_of(&details_line, "błąd, żó")],
        "overlapping label spans are one range"
    );

    // A span that cuts through a character is widened to whole characters.
    let cut = zazolc.start + 3..zazolc.start + 4;
    assert!(
        !row.is_char_boundary(cut.start),
        "the fixture span starts inside `ż`"
    );
    let widened = goose.emphasis(&[cut]);
    assert_well_formed(&title_line, &widened.title);
    assert_eq!(
        widened.title,
        [range_of(&title_line, "ż")],
        "a span inside a character covers that character"
    );
}
