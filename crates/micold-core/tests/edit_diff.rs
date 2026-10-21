//! Feature 940 — Edit, MultiEdit and Write tool calls as line diffs (contracts/edit-diff.md E-1–E-3;
//! FR-013, FR-014, FR-016–FR-019 at the model, SC-003).

use micold_core::review::diff::LineKind;
use micold_core::structured::edit_diff::{edit_diff, EditDiff, EditHunk, EditKind, HunkNote};
use micold_core::structured::{ContentBlock, StructuredEvent};
use serde_json::{json, Value};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{events_of, lines_of, without_stream_events};

fn diff(tool: &str, input: Value) -> EditDiff {
    let text = input.to_string();
    edit_diff(tool, &text).unwrap_or_else(|| panic!("{tool} {text} has a diff"))
}

/// The lines of `text` as the contract splits them: `\n` and `\r\n` end a line, a trailing newline
/// starts no extra one.
fn lines(text: &str) -> Vec<String> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() {
        return Vec::new();
    }
    body.split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
        .collect()
}

/// The hunk's lines of the given kinds, in order.
fn side(hunk: &EditHunk, other: LineKind) -> Vec<String> {
    hunk.lines
        .iter()
        .filter(|l| l.kind == LineKind::Context || l.kind == other)
        .map(|l| l.text.clone())
        .collect()
}

/// A hunk as the multi-edit test compares it: ordinal, of, note and lines.
type Shape<'a> = (u32, u32, Option<HunkNote>, Vec<(LineKind, &'a str)>);

fn kinds(hunk: &EditHunk) -> Vec<(LineKind, &str)> {
    hunk.lines
        .iter()
        .map(|l| (l.kind, l.text.as_str()))
        .collect()
}

/// The old and new text of each edit the input names, for the round trip.
fn texts_of(tool: &str, input: &Value) -> Vec<(String, String)> {
    let s = |v: &Value, k: &str| v[k].as_str().unwrap().to_string();
    match tool {
        "Edit" => vec![(s(input, "old_string"), s(input, "new_string"))],
        "MultiEdit" => input["edits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (s(e, "old_string"), s(e, "new_string")))
            .collect(),
        "Write" => vec![(String::new(), s(input, "content"))],
        other => panic!("not an edit tool: {other}"),
    }
}

/// E-1 and E-3 for one call: each hunk's `Context`+`Removed` lines are the old text's lines and its
/// `Context`+`Added` lines the new text's; the counts sum the hunks' lines.
fn assert_round_trip(tool: &str, input_json: &str) {
    let d = edit_diff(tool, input_json).unwrap_or_else(|| panic!("{tool} {input_json} has a diff"));
    let input: Value = serde_json::from_str(input_json).unwrap();
    let texts = texts_of(tool, &input);
    assert_eq!(
        d.hunks.len(),
        texts.len(),
        "{input_json}: one hunk per edit"
    );
    for (hunk, (old, new)) in d.hunks.iter().zip(&texts) {
        if hunk.note == Some(HunkNote::NoChange) {
            assert_eq!(old, new, "NoChange only for equal texts: {input_json}");
            assert!(hunk.lines.is_empty(), "NoChange has no lines: {input_json}");
            continue;
        }
        assert_eq!(
            side(hunk, LineKind::Removed),
            lines(old),
            "old side of {input_json}"
        );
        assert_eq!(
            side(hunk, LineKind::Added),
            lines(new),
            "new side of {input_json}"
        );
    }
    let count = |k: LineKind| {
        d.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == k)
            .count() as u32
    };
    assert_eq!(d.added, count(LineKind::Added), "E-3 added of {input_json}");
    assert_eq!(
        d.removed,
        count(LineKind::Removed),
        "E-3 removed of {input_json}"
    );
    assert_eq!(d.path, input["file_path"].as_str().unwrap());
}

/// Every Edit, MultiEdit and Write call in every stream-json fixture.
fn recorded_calls() -> Vec<(String, String, String)> {
    let dir = format!("{}/tests/fixtures/stream-json", env!("CARGO_MANIFEST_DIR"));
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.strip_suffix(".jsonl").map(str::to_string)
        })
        .collect();
    names.sort();
    let mut calls = Vec::new();
    for name in names {
        for event in events_of(&without_stream_events(&lines_of(&name))) {
            let StructuredEvent::Assistant { blocks, .. } = event.event else {
                continue;
            };
            for block in blocks {
                if let ContentBlock::ToolUse {
                    name: tool, input, ..
                } = block
                {
                    if matches!(tool.as_str(), "Edit" | "MultiEdit" | "Write") {
                        calls.push((name.clone(), tool, input));
                    }
                }
            }
        }
    }
    calls
}

/// Inputs the recordings do not hold (Claude Code no longer offers MultiEdit; research R13).
fn synthetic() -> Vec<(&'static str, Value)> {
    vec![
        (
            "Edit",
            json!({"file_path": "/r/a.rs", "old_string": "fn a() {\n    1\n}\n", "new_string": "fn a() {\n    2\n}\n"}),
        ),
        (
            "Edit",
            json!({"file_path": "/r/a.rs", "old_string": "x", "new_string": "x", "replace_all": true}),
        ),
        (
            "Edit",
            json!({"file_path": "/r/a.rs", "old_string": "a\r\nb\r\n", "new_string": "a\nb\nc\n"}),
        ),
        (
            "Edit",
            json!({"file_path": "/r/a.rs", "old_string": "", "new_string": "added\n"}),
        ),
        (
            "Edit",
            json!({"file_path": "/r/a.rs", "old_string": "gone\nkept", "new_string": "kept"}),
        ),
        (
            "MultiEdit",
            json!({"file_path": "/r/b.md", "edits": [
                {"old_string": "one\ntwo\nthree", "new_string": "one\n2\nthree"},
                {"old_string": "four", "new_string": "four\nfive", "replace_all": true},
                {"old_string": "six\n", "new_string": ""}
            ]}),
        ),
        (
            "Write",
            json!({"file_path": "/r/c.txt", "content": "first\nsecond\n\nlast"}),
        ),
        ("Write", json!({"file_path": "/r/c.txt", "content": ""})),
        (
            "Write",
            json!({"file_path": "/r/c.txt", "content": "ünïcödé ✓\r\n\r\n"}),
        ),
    ]
}

#[test]
fn an_edit_is_one_hunk_of_removed_added_and_shared_lines() {
    let d = diff(
        "Edit",
        json!({"file_path": "/r/src/main.rs", "old_string": "a\nb\nc\n", "new_string": "a\nB\nc\n"}),
    );
    assert_eq!(d.path, "/r/src/main.rs");
    assert_eq!(d.kind, EditKind::Edit);
    assert_eq!(d.hunks.len(), 1);
    let hunk = &d.hunks[0];
    assert_eq!((hunk.ordinal, hunk.of, hunk.note), (1, 1, None));
    assert_eq!(
        kinds(hunk),
        vec![
            (LineKind::Context, "a"),
            (LineKind::Removed, "b"),
            (LineKind::Added, "B"),
            (LineKind::Context, "c"),
        ]
    );
    assert_eq!((d.added, d.removed), (1, 1));
    assert!(!hunk.old_no_newline && !hunk.new_no_newline);
}

#[test]
fn a_multi_edit_has_one_hunk_per_edit_in_order() {
    let d = diff(
        "MultiEdit",
        json!({"file_path": "/r/b.md", "edits": [
            {"old_string": "first", "new_string": "1st"},
            {"old_string": "second", "new_string": "2nd", "replace_all": true},
            {"old_string": "third", "new_string": "3rd", "replace_all": false}
        ]}),
    );
    assert_eq!(d.kind, EditKind::MultiEdit);
    let order: Vec<Shape> = d
        .hunks
        .iter()
        .map(|h| (h.ordinal, h.of, h.note, kinds(h)))
        .collect();
    assert_eq!(
        order,
        vec![
            (
                1,
                3,
                None,
                vec![(LineKind::Removed, "first"), (LineKind::Added, "1st")]
            ),
            (
                2,
                3,
                Some(HunkNote::ReplaceAll),
                vec![(LineKind::Removed, "second"), (LineKind::Added, "2nd")]
            ),
            (
                3,
                3,
                None,
                vec![(LineKind::Removed, "third"), (LineKind::Added, "3rd")]
            ),
        ]
    );
    assert_eq!((d.added, d.removed), (3, 3));
}

#[test]
fn a_write_is_its_content_as_added_lines_of_new_content() {
    let d = diff(
        "Write",
        json!({"file_path": "/r/new.txt", "content": "one\ntwo\n"}),
    );
    assert_eq!(d.kind, EditKind::Write);
    assert_eq!(d.hunks.len(), 1);
    let hunk = &d.hunks[0];
    assert_eq!((hunk.ordinal, hunk.of), (1, 1));
    assert_eq!(hunk.note, Some(HunkNote::NewContent));
    assert_eq!(
        kinds(hunk),
        vec![(LineKind::Added, "one"), (LineKind::Added, "two")]
    );
    assert_eq!((d.added, d.removed), (2, 0));
}

#[test]
fn an_empty_write_says_so_and_has_no_lines() {
    let d = diff("Write", json!({"file_path": "/r/empty.txt", "content": ""}));
    let hunk = &d.hunks[0];
    assert_eq!(hunk.note, Some(HunkNote::EmptyWrite));
    assert!(hunk.lines.is_empty());
    assert_eq!((d.added, d.removed), (0, 0));
}

#[test]
fn replace_all_is_noted_on_its_hunk() {
    let d = diff(
        "Edit",
        json!({"file_path": "/r/a", "old_string": "apple", "new_string": "pear", "replace_all": true}),
    );
    assert_eq!(d.hunks[0].note, Some(HunkNote::ReplaceAll));
    let d = diff(
        "Edit",
        json!({"file_path": "/r/a", "old_string": "apple", "new_string": "pear", "replace_all": false}),
    );
    assert_eq!(d.hunks[0].note, None);
}

#[test]
fn equal_old_and_new_text_is_no_change_without_lines() {
    let d = diff(
        "Edit",
        json!({"file_path": "/r/a", "old_string": "same\ntext\n", "new_string": "same\ntext\n"}),
    );
    assert_eq!(d.hunks[0].note, Some(HunkNote::NoChange));
    assert!(d.hunks[0].lines.is_empty());
    assert_eq!((d.added, d.removed), (0, 0));
}

#[test]
fn a_line_ending_only_difference_is_no_change() {
    let d = diff(
        "Edit",
        json!({"file_path": "/r/a", "old_string": "a\r\nb\r\n", "new_string": "a\nb\n"}),
    );
    let hunk = &d.hunks[0];
    assert_eq!(
        kinds(hunk),
        vec![(LineKind::Context, "a"), (LineKind::Context, "b")]
    );
    assert_eq!((d.added, d.removed), (0, 0));
    assert_eq!(hunk.old_no_newline, hunk.new_no_newline);
    assert_eq!(
        hunk.note, None,
        "the texts differ, so the lines are shown, all unchanged"
    );
}

#[test]
fn a_final_newline_only_change_shows_in_the_flags_with_equal_lines() {
    let d = diff(
        "Edit",
        json!({"file_path": "/r/a", "old_string": "a\nb\n", "new_string": "a\nb"}),
    );
    let hunk = &d.hunks[0];
    assert_eq!(
        kinds(hunk),
        vec![(LineKind::Context, "a"), (LineKind::Context, "b")]
    );
    assert_ne!(hunk.old_no_newline, hunk.new_no_newline);
    assert!(!hunk.old_no_newline && hunk.new_no_newline);
    assert_eq!(hunk.note, None, "the texts differ, so it is a change");
}

#[test]
fn bytes_is_the_input_text_size() {
    let input = json!({"file_path": "/r/a", "old_string": "x", "new_string": "yz"}).to_string();
    let d = edit_diff("Edit", &input).unwrap();
    assert_eq!(d.bytes, input.len() as u64);
}

#[test]
fn unknown_extra_fields_are_ignored() {
    let d = diff(
        "Edit",
        json!({"file_path": "/r/a", "old_string": "x", "new_string": "y", "future": [1, 2], "caller": {}}),
    );
    assert_eq!((d.added, d.removed), (1, 1));
    let d = diff(
        "MultiEdit",
        json!({"file_path": "/r/a", "edits": [{"old_string": "x", "new_string": "y", "extra": null}], "more": 1}),
    );
    assert_eq!(d.hunks.len(), 1);
}

/// E-2: every malformed input of the contract's table is `None`, never a panic.
#[test]
fn malformed_inputs_fall_back_to_the_plain_input() {
    let malformed: Vec<(&str, String)> = vec![
        (
            "Read",
            json!({"file_path": "/r/a", "old_string": "x", "new_string": "y"}).to_string(),
        ),
        ("edit", json!({"file_path": "/r/a", "old_string": "x", "new_string": "y"}).to_string()),
        ("Edit", "not json".into()),
        ("Edit", r#"{"file_path": "/r/a", "old_string": "x""#.into()),
        ("Edit", "".into()),
        ("Edit", "[]".into()),
        ("Edit", "\"text\"".into()),
        ("Edit", "null".into()),
        ("Edit", json!({"old_string": "x", "new_string": "y"}).to_string()),
        ("Edit", json!({"file_path": "/r/a", "new_string": "y"}).to_string()),
        ("Edit", json!({"file_path": "/r/a", "old_string": "x"}).to_string()),
        ("Edit", json!({"file_path": 3, "old_string": "x", "new_string": "y"}).to_string()),
        ("Edit", json!({"file_path": "/r/a", "old_string": ["x"], "new_string": "y"}).to_string()),
        ("Edit", json!({"file_path": "/r/a", "old_string": "x", "new_string": null}).to_string()),
        (
            "Edit",
            json!({"file_path": "/r/a", "old_string": "x", "new_string": "y", "replace_all": "yes"})
                .to_string(),
        ),
        ("MultiEdit", json!({"file_path": "/r/a"}).to_string()),
        ("MultiEdit", json!({"file_path": "/r/a", "edits": []}).to_string()),
        ("MultiEdit", json!({"file_path": "/r/a", "edits": {}}).to_string()),
        ("MultiEdit", json!({"file_path": "/r/a", "edits": ["x"]}).to_string()),
        (
            "MultiEdit",
            json!({"file_path": "/r/a", "edits": [{"old_string": "x"}]}).to_string(),
        ),
        (
            "MultiEdit",
            json!({"file_path": "/r/a", "edits": [{"old_string": "x", "new_string": "y"}, {"old_string": 1, "new_string": "y"}]})
                .to_string(),
        ),
        (
            "MultiEdit",
            json!({"file_path": "/r/a", "edits": [{"old_string": "x", "new_string": "y", "replace_all": 1}]})
                .to_string(),
        ),
        (
            "MultiEdit",
            json!({"edits": [{"old_string": "x", "new_string": "y"}]}).to_string(),
        ),
        ("Write", json!({"file_path": "/r/a"}).to_string()),
        ("Write", json!({"content": "x"}).to_string()),
        ("Write", json!({"file_path": "/r/a", "content": 7}).to_string()),
        ("Write", json!({"file_path": ["/r/a"], "content": "x"}).to_string()),
    ];
    for (tool, input) in malformed {
        assert_eq!(edit_diff(tool, &input), None, "{tool} {input}");
    }
}

#[test]
fn no_line_numbers_are_part_of_a_hunk() {
    // FR-016: a hunk is named by its ordinal; a line is only its kind and text.
    let d = diff(
        "Edit",
        json!({"file_path": "/r/a", "old_string": "@@ -1,2 +1,2 @@", "new_string": "x"}),
    );
    assert_eq!(
        kinds(&d.hunks[0]),
        vec![
            (LineKind::Removed, "@@ -1,2 +1,2 @@"),
            (LineKind::Added, "x")
        ]
    );
}

/// E-1 over the recordings (SC-003): every recorded Edit, MultiEdit and Write round-trips.
#[test]
fn every_recorded_edit_and_write_round_trips() {
    let calls = recorded_calls();
    let from_edit_turn: Vec<&str> = calls
        .iter()
        .filter(|(f, ..)| f == "edit-turn")
        .map(|(_, tool, _)| tool.as_str())
        .collect();
    assert_eq!(
        from_edit_turn,
        ["Write", "Edit", "Edit"],
        "edit-turn records a Write and two Edits"
    );
    let edit_turn: Vec<EditDiff> = calls
        .iter()
        .filter(|(f, ..)| f == "edit-turn")
        .map(|(_, tool, input)| edit_diff(tool, input).unwrap())
        .collect();
    assert_eq!(edit_turn[0].hunks[0].note, Some(HunkNote::NewContent));
    assert_eq!(edit_turn[1].hunks[0].note, Some(HunkNote::ReplaceAll));
    assert!(
        edit_turn[2].hunks[0]
            .lines
            .iter()
            .any(|l| l.kind == LineKind::Context),
        "the multi-line Edit shares a line"
    );
    for (_, tool, input) in &calls {
        assert_round_trip(tool, input);
    }
}

/// E-1 over the synthetic set.
#[test]
fn every_synthetic_edit_and_write_round_trips() {
    for (tool, input) in synthetic() {
        assert_round_trip(tool, &input.to_string());
    }
}

#[test]
fn a_large_edit_finishes_and_round_trips() {
    // The 50 ms deadline may coarsen the diff, never break it.
    let old: String = (0..4000)
        .map(|i| format!("line {}\n", i * 7 % 4001))
        .collect();
    let new: String = (0..4000)
        .map(|i| format!("line {}\n", i * 11 % 4003))
        .collect();
    let input = json!({"file_path": "/r/big", "old_string": old, "new_string": new}).to_string();
    assert_round_trip("Edit", &input);
}
