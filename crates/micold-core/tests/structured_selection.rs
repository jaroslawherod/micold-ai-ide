//! Feature 1550 — the selection model at the core: positions in reading order, grapheme snapping,
//! word and line granularity and stability under streaming
//! (`specs/1550-transcript-text-selection/contracts/selection-model.md` S-1, S-4, S-5, S-11).

use std::ops::Range;

use micold_core::structured::selection::{
    snap, source_line_at, word_at, Granularity, RowPos, RowSelection, Selection, TextPath, TextPos,
    Under,
};
use micold_core::structured::transcript::{EntryId, EntryKey, EntryKind, Transcript};
use micold_core::structured::{Delta, StructuredEvent};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{built, events_of, lines_of, replayed, RECORDED};

fn pos(row: RowPos, path: &[u16], offset: usize) -> TextPos {
    TextPos {
        row,
        path: TextPath::new(path),
        offset,
    }
}

fn body(id: EntryId) -> RowPos {
    RowPos::Entry(id, Under::Body)
}

/// The first two entries' ids of a recorded transcript: `a < b`.
fn two_ids() -> (EntryId, EntryId) {
    let transcript = built(&events_of(&lines_of("tool-turn")));
    let mut ids = transcript.entries().map(|e| e.id());
    let (a, b) = (
        ids.next().expect("an entry"),
        ids.next().expect("two entries"),
    );
    assert!(a < b);
    (a, b)
}

fn selection(from: TextPos, to: TextPos) -> Selection {
    let mut selection = Selection::start(from, Granularity::Char, 0..0);
    selection.extend(to, 0..0);
    selection
}

// S-1 — positions order as they are drawn.

#[test]
fn rows_order_as_they_are_drawn() {
    let (a, b) = two_ids();
    let rows = [
        RowPos::Omitted,
        body(a),
        RowPos::Entry(a, Under::Prompt(0)),
        RowPos::Entry(a, Under::Prompt(1)),
        body(b),
        RowPos::End(0),
        RowPos::End(1),
    ];
    for pair in rows.windows(2) {
        assert!(pair[0] < pair[1], "{:?} before {:?}", pair[0], pair[1]);
    }
}

#[test]
fn text_paths_order_lexicographically() {
    let p = |segments: &[u16]| TextPath::new(segments);
    assert!(p(&[1]) < p(&[1, 0]));
    assert!(p(&[1, 0]) < p(&[2]));
    assert!(p(&[1, 9]) < p(&[2, 0]));
    assert!(p(&[]) < p(&[0]));
}

#[test]
fn positions_order_by_row_then_path_then_offset() {
    let (a, b) = two_ids();
    let drawn = vec![
        pos(RowPos::Omitted, &[0], 5),
        pos(body(a), &[0], 0),
        pos(body(a), &[0], 3),
        pos(body(a), &[1], 0),
        pos(body(a), &[1, 0], 0),
        pos(body(a), &[2], 0),
        pos(RowPos::Entry(a, Under::Prompt(0)), &[0], 9),
        pos(RowPos::Entry(a, Under::Prompt(1)), &[0], 0),
        pos(body(b), &[0], 0),
        pos(RowPos::End(0), &[0], 0),
        pos(RowPos::End(1), &[0], 0),
    ];
    let mut sorted = drawn.clone();
    sorted.reverse();
    sorted.sort();
    assert_eq!(sorted, drawn, "reading order is drawing order");
}

#[test]
fn a_range_from_the_omitted_notice_into_an_entry_covers_both_rows() {
    let (a, b) = two_ids();
    let range = selection(pos(body(a), &[1], 2), pos(RowPos::Omitted, &[0], 4));
    assert_eq!(
        range.in_row(&RowPos::Omitted),
        Some(RowSelection::From(TextPath::new([0]), 4))
    );
    assert_eq!(
        range.in_row(&body(a)),
        Some(RowSelection::To(TextPath::new([1]), 2))
    );
    assert_eq!(range.in_row(&body(b)), None);
}

#[test]
fn a_range_across_request_rows_lists_the_rows_in_drawing_order() {
    let (a, b) = two_ids();
    let range = selection(pos(body(a), &[0], 1), pos(RowPos::End(0), &[0], 3));
    let rows = [
        RowPos::Omitted,
        body(a),
        RowPos::Entry(a, Under::Prompt(0)),
        body(b),
        RowPos::End(0),
        RowPos::End(1),
    ];
    let parts: Vec<Option<RowSelection>> = rows.iter().map(|row| range.in_row(row)).collect();
    assert_eq!(
        parts,
        [
            None,
            Some(RowSelection::From(TextPath::new([0]), 1)),
            Some(RowSelection::Whole),
            Some(RowSelection::Whole),
            Some(RowSelection::To(TextPath::new([0]), 3)),
            None,
        ]
    );
}

// S-4 — offsets are grapheme boundaries.

/// Every grapheme boundary of `text`, its end included.
fn boundaries(text: &str) -> Vec<usize> {
    use unicode_segmentation::UnicodeSegmentation;
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect()
}

#[test]
fn snap_returns_the_nearest_grapheme_boundary() {
    let family = "a\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}b";
    let combining = "e\u{301}x";
    let cjk = "日本語";
    for text in [family, combining, cjk, "plain", ""] {
        let marks = boundaries(text);
        for offset in 0..=text.len() + 2 {
            let snapped = snap(text, offset);
            assert!(marks.contains(&snapped), "{text:?} @ {offset} → {snapped}");
        }
        for &mark in &marks {
            assert_eq!(snap(text, mark), mark, "{text:?}: a boundary is kept");
        }
        assert_eq!(snap(text, text.len() + 5), text.len(), "{text:?}: clamped");
    }
    assert_eq!(snap(cjk, 1), 0);
    assert_eq!(snap(cjk, 2), 3);
    assert_eq!(
        snap(family, 5),
        1,
        "inside the ZWJ sequence, nearer its start"
    );
    assert_eq!(snap(family, 15), 19, "nearer its end");
    assert_eq!(snap(combining, 1), 0);
    assert_eq!(snap(combining, 2), 3, "never between a letter and its mark");
}

#[test]
fn a_position_built_in_a_text_is_snapped() {
    let at = TextPos::in_text(RowPos::End(0), TextPath::new([0]), "日本語", 4);
    assert_eq!(at.offset, 3);
}

// S-5 — streaming never moves a selection.

/// Each entry's key, id and revision, in order.
type Snapshot = Vec<(EntryKey, EntryId, u64)>;

fn ids_by_key(transcript: &Transcript) -> Snapshot {
    transcript
        .entries()
        .map(|e| (e.key().clone(), e.id(), e.revision()))
        .collect()
}

#[test]
fn appending_and_changing_entries_never_moves_a_selection() {
    let (mut appended, mut changed_outside) = (0, 0);
    for name in RECORDED {
        let events = events_of(&lines_of(name));
        let mut transcript = Transcript::new();
        let mut held: Option<(Selection, Snapshot)> = None;
        for event in &events {
            transcript.apply(0, std::slice::from_ref(event), true);
            let now = ids_by_key(&transcript);
            let Some((selection, before)) = &held else {
                if now.len() >= 2 {
                    let anchor = pos(body(now[0].1), &[0], 0);
                    let focus = pos(body(now[1].1), &[0], 1);
                    held = Some((selection_of(anchor, focus), now));
                }
                continue;
            };
            let (start, end) = selection.bounds();
            assert_eq!(&start, selection.anchor(), "{name}");
            assert_eq!(&end, selection.focus(), "{name}");
            for (key, id, revision) in before {
                let (_, now_id, now_revision) = now
                    .iter()
                    .find(|(k, ..)| k == key)
                    .unwrap_or_else(|| panic!("{name}: {key:?} stays"));
                assert_eq!(now_id, id, "{name}: an entry keeps its id, so its rows");
                let in_range = body(*id) <= end.row;
                if now_revision != revision && !in_range {
                    changed_outside += 1;
                }
            }
            for (_, id, _) in &now[before.len()..] {
                appended += 1;
                assert!(
                    body(*id) > end.row,
                    "{name}: an appended entry orders after the focus"
                );
                assert_eq!(selection.in_row(&body(*id)), None);
            }
            let parts = |ids: &[(EntryKey, EntryId, u64)]| -> Vec<Option<RowSelection>> {
                ids.iter()
                    .map(|(_, id, _)| selection.in_row(&body(*id)))
                    .collect()
            };
            assert_eq!(
                parts(&now[..before.len()]),
                parts(before),
                "{name}: each row's part is unchanged"
            );
            held = Some((selection.clone(), now));
        }
    }
    assert!(appended > 0, "the fixtures append entries");
    assert!(
        changed_outside > 0,
        "the fixtures change entries outside the range"
    );
}

fn selection_of(anchor: TextPos, focus: TextPos) -> Selection {
    let mut selection = selection(anchor, focus);
    selection.release();
    selection
}

fn reply_text(transcript: &Transcript) -> (EntryId, String) {
    let last = transcript.entries().last().expect("an entry");
    match last.kind() {
        EntryKind::AssistantText { text, .. } => (last.id(), text.clone()),
        other => panic!("the last entry is the reply: {other:?}"),
    }
}

#[test]
fn growing_the_focus_entry_past_the_focus_keeps_the_selected_text() {
    let events = events_of(&lines_of("partial-deltas"));
    let is_text_delta = |e: &StructuredEvent| {
        matches!(
            e,
            StructuredEvent::StreamDelta {
                delta: Some(Delta::Text { .. }),
                ..
            }
        )
    };
    let first = events
        .iter()
        .position(|e| is_text_delta(&e.event))
        .expect("the fixture streams text");
    let mut transcript = replayed(&events[..=first]);
    let (id, text) = reply_text(&transcript);
    let focus = snap(&text, 3);
    let selected = text[..focus].to_string();
    let anchor_entry = transcript.entries().next().expect("an entry").id();
    let held = selection_of(pos(body(anchor_entry), &[0], 0), pos(body(id), &[0], focus));
    let parts = held.in_row(&body(id));
    let mut grew = 0;
    for event in events[first + 1..]
        .iter()
        .take_while(|e| is_text_delta(&e.event))
    {
        transcript.apply(0, std::slice::from_ref(event), true);
        let (now_id, now_text) = reply_text(&transcript);
        assert_eq!(now_id, id, "the reply grows in place");
        assert!(now_text.len() > focus);
        assert_eq!(
            &now_text[..focus],
            selected,
            "the selected text is unchanged"
        );
        assert_eq!(held.in_row(&body(id)), parts);
        grew += 1;
    }
    assert!(grew > 0, "the reply grew past the focus");
}

// S-11 — words and lines.

fn range_of(text: &str, needle: &str) -> Range<usize> {
    let start = text.find(needle).expect(needle);
    start..start + needle.len()
}

#[test]
fn word_at_selects_a_word_a_space_run_or_a_punctuation_run() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    let text = format!("hello,  world!! テスト {family} 안녕하세요");
    let text = text.as_str();
    let world = range_of(text, "world");
    for offset in world.clone() {
        assert_eq!(word_at(text, offset), world, "inside a word");
    }
    let spaces = range_of(text, "  ");
    assert_eq!(word_at(text, spaces.start + 1), spaces, "a whitespace run");
    let bangs = range_of(text, "!!");
    assert_eq!(word_at(text, bangs.start), bangs, "a punctuation run");
    assert_eq!(word_at(text, bangs.start + 1), bangs);
    let katakana = range_of(text, "テスト");
    assert_eq!(word_at(text, katakana.start + 3), katakana, "a CJK word");
    let emoji = range_of(text, family);
    assert_eq!(word_at(text, emoji.start + 4), emoji, "an emoji whole");
    let hangul = range_of(text, "안녕하세요");
    assert_eq!(word_at(text, hangul.start), hangul);
    assert_eq!(word_at(text, 0), range_of(text, "hello"), "at offset 0");
    assert_eq!(
        word_at(text, text.len()),
        hangul,
        "at the end, the last run"
    );
    assert_eq!(word_at("", 0), 0..0);
    let full_width = "テスト！！";
    let bangs = range_of(full_width, "！！");
    assert_eq!(word_at(full_width, bangs.start), bangs, "a full-width run");
}

#[test]
fn word_at_never_crosses_a_line_break() {
    let code = "foo  \n    bar\n\nbaz";
    assert_eq!(
        word_at(code, 4),
        3..5,
        "trailing spaces, not the break after them"
    );
    assert_eq!(word_at(code, 7), 6..10, "the next line's indentation alone");
    assert_eq!(word_at(code, 5), 3..5, "at a line's end, its last run");
    assert_eq!(word_at(code, 14), 14..14, "an empty line selects nothing");
    assert_eq!(word_at(code, code.len()), 15..18);
}

#[test]
fn source_line_at_selects_the_line_without_its_break() {
    let text = "one\ntwo\n\nlast";
    assert_eq!(source_line_at(text, 0), 0..3);
    assert_eq!(source_line_at(text, 5), 4..7);
    assert_eq!(source_line_at(text, 7), 4..7, "at the line's break");
    assert_eq!(source_line_at(text, 8), 8..8, "an empty line");
    assert_eq!(
        source_line_at(text, 11),
        9..13,
        "the last line, no break after it"
    );
    assert_eq!(source_line_at(text, text.len()), 9..13);
    assert_eq!(source_line_at("", 0), 0..0);
}

#[test]
fn a_selection_by_word_or_line_extends_by_whole_units() {
    let (a, _) = two_ids();
    let at = |offset| pos(body(a), &[0], offset);
    let words = "alpha beta gamma delta";
    let mut by_word = Selection::start(at(7), Granularity::Word, word_at(words, 7));
    assert_eq!(
        by_word.bounds(),
        (at(6), at(10)),
        "the word under the click"
    );
    by_word.extend(at(13), word_at(words, 13));
    assert_eq!(
        by_word.bounds(),
        (at(6), at(16)),
        "later: whole words at both ends"
    );
    by_word.extend(at(2), word_at(words, 2));
    assert_eq!(
        by_word.bounds(),
        (at(0), at(10)),
        "earlier: whole words at both ends"
    );
    let lines = "one\ntwo\nthree";
    let mut by_line = Selection::start(at(5), Granularity::Line, source_line_at(lines, 5));
    assert_eq!(by_line.bounds(), (at(4), at(7)));
    by_line.extend(at(10), source_line_at(lines, 10));
    assert_eq!(by_line.bounds(), (at(4), at(13)));
    by_line.extend(at(1), source_line_at(lines, 1));
    assert_eq!(by_line.bounds(), (at(0), at(7)));
    let in_next_unit = pos(body(a), &[1], 2);
    by_word.extend(in_next_unit.clone(), 0..5);
    assert_eq!(
        by_word.bounds(),
        (at(6), pos(body(a), &[1], 5)),
        "across units, each end in its own unit"
    );
}
