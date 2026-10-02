//! T002 (feature 041): the rules a `HistorySnapshot` must hold before it is seeded or saved
//! (data-model §1) — runs cover the text exactly, the text carries no control character, and a
//! colour index stays inside its palette.

use micold_core::terminal_history::{
    HistorySnapshot, HistoryStyle, LogicalLine, SnapshotError, StyleRun,
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
