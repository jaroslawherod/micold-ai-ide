//! T002 (feature 041): the rules a `HistorySnapshot` must hold before it is seeded or saved
//! (data-model §1) — runs cover the text exactly, the text carries no control character, and a
//! colour index stays inside its palette.

use micold_core::terminal_history::{
    HistoryColor, HistorySnapshot, HistoryStyle, LogicalLine, SnapshotError, StyleFlags, StyleRun,
};

/// A line of `text` covered by default-style runs of the given lengths.
fn line(text: &str, run_chars: &[u32]) -> LogicalLine {
    LogicalLine {
        text: text.to_string(),
        runs: run_chars
            .iter()
            .map(|&chars| StyleRun {
                chars,
                style: HistoryStyle::default(),
            })
            .collect(),
    }
}

/// A one-character snapshot in `style`.
fn styled(style: HistoryStyle) -> HistorySnapshot {
    snapshot_of(LogicalLine {
        text: "x".to_string(),
        runs: vec![StyleRun { chars: 1, style }],
    })
}

/// The two places a colour sits in a style: as foreground and as background.
fn as_fg_and_bg(color: HistoryColor) -> [HistoryStyle; 2] {
    [
        HistoryStyle {
            fg: color,
            ..HistoryStyle::default()
        },
        HistoryStyle {
            bg: color,
            ..HistoryStyle::default()
        },
    ]
}

fn snapshot_of(line: LogicalLine) -> HistorySnapshot {
    HistorySnapshot { lines: vec![line] }
}

// U1: runs are counted in characters, not bytes — "héllo" is five characters in six bytes.
#[test]
fn validate_accepts_a_line_whose_runs_sum_to_its_character_count() {
    let snapshot = snapshot_of(line("héllo", &[2, 3]));

    assert_eq!(
        snapshot.validate(),
        Ok(()),
        "runs whose `chars` sum to the line's number of characters cover it exactly"
    );
}

// U2: a run sum on either side of the character count leaves text unstyled or styles text that is
// not there.
#[test]
fn validate_rejects_a_line_whose_run_sum_is_one_more_or_one_less_than_its_character_count() {
    const CHARACTERS: u32 = 5;

    for run_sum in [CHARACTERS + 1, CHARACTERS - 1] {
        let snapshot = snapshot_of(line("héllo", &[2, run_sum - 2]));

        assert_eq!(
            snapshot.validate(),
            Err(SnapshotError::RunsDoNotCoverText { line: 0 }),
            "runs summing to {run_sum} do not cover a line of {CHARACTERS} characters"
        );
    }
}

// U3: a restored line is printed back into a terminal, so a control character in it would be
// interpreted instead of shown (FR-016).
#[test]
fn validate_rejects_a_line_holding_a_c0_a_c1_or_an_escape_character() {
    const BELL: char = '\u{7}';
    const CSI_C1: char = '\u{9b}';
    const ESCAPE: char = '\u{1b}';

    for control in [BELL, CSI_C1, ESCAPE] {
        let snapshot = snapshot_of(line(&format!("a{control}b"), &[3]));

        assert_eq!(
            snapshot.validate(),
            Err(SnapshotError::ControlCharacter { line: 0 }),
            "{:?} is a control character, which a line's text never holds",
            control
        );
    }
}

// U4: the 16 basic colours are numbered 0 to 15; 16 names no basic colour.
#[test]
fn basic_color_accepts_0_and_15_and_rejects_16() {
    const FIRST: u8 = 0;
    const LAST: u8 = 15;

    for index in [FIRST, LAST] {
        for style in as_fg_and_bg(HistoryColor::Basic(index)) {
            assert_eq!(
                styled(style).validate(),
                Ok(()),
                "Basic({index}) is one of the 16 basic colours"
            );
        }
    }
    for style in as_fg_and_bg(HistoryColor::Basic(LAST + 1)) {
        assert_eq!(
            styled(style).validate(),
            Err(SnapshotError::ColorOutOfRange { line: 0 }),
            "Basic({}) is past the 16 basic colours",
            LAST + 1
        );
    }
}

// U5: the dim variants exist for the first 8 basic colours only, numbered 0 to 7.
#[test]
fn dim_color_accepts_0_and_7_and_rejects_8() {
    const FIRST: u8 = 0;
    const LAST: u8 = 7;

    for index in [FIRST, LAST] {
        for style in as_fg_and_bg(HistoryColor::Dim(index)) {
            assert_eq!(
                styled(style).validate(),
                Ok(()),
                "Dim({index}) is the dim variant of one of the first 8 basic colours"
            );
        }
    }
    for style in as_fg_and_bg(HistoryColor::Dim(LAST + 1)) {
        assert_eq!(
            styled(style).validate(),
            Err(SnapshotError::ColorOutOfRange { line: 0 }),
            "Dim({}) is past the 8 colours that have a dim variant",
            LAST + 1
        );
    }
}

// U6: an empty snapshot means there is nothing to show, so nothing is seeded (FR-010).
#[test]
fn an_empty_snapshot_is_empty_and_one_with_a_line_is_not() {
    assert!(
        HistorySnapshot::default().is_empty(),
        "a snapshot without lines has nothing to show"
    );
    assert!(
        !snapshot_of(line("", &[])).is_empty(),
        "a snapshot with a line, even a blank one, has something to show"
    );
}

// U7: each attribute set on a style is read back, and it sets no other attribute.
#[test]
fn style_flags_round_trip_each_attribute() {
    let attributes = [
        ("bold", StyleFlags::BOLD),
        ("dim", StyleFlags::DIM),
        ("italic", StyleFlags::ITALIC),
        ("underline", StyleFlags::UNDERLINE),
        ("inverse", StyleFlags::INVERSE),
        ("strikethrough", StyleFlags::STRIKETHROUGH),
        ("hidden", StyleFlags::HIDDEN),
    ];

    for (name, attribute) in attributes {
        let flags = StyleFlags::default().with(attribute);

        assert!(flags.contains(attribute), "{name} set is read back");
        for (other_name, other) in attributes {
            if other_name != name {
                assert!(
                    !flags.contains(other),
                    "setting {name} does not set {other_name}"
                );
            }
        }
    }
}
