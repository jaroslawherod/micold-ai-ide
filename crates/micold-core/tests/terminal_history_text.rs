//! T004 (feature 041): the separator printed between restored history and a restarted session's
//! new output (data-model §7) — the rules go first, then the text is cut, and it is always one row
//! no wider than the terminal (FR-009).

use micold_core::terminal_history::text::{notice_line, separator_line};

/// A start time as the daemon formats it: local date and time with the UTC offset.
const AT: &str = "2026-10-02 14:31 +02:00";
const FULL: &str = "── session restarted at 2026-10-02 14:31 +02:00 ──";
const WITHOUT_RULES: &str = "session restarted at 2026-10-02 14:31 +02:00";

fn width(text: &str) -> usize {
    text.chars().count()
}

// U8: a terminal wide enough shows the whole separator, rules and all.
#[test]
fn at_80_columns_the_separator_is_the_full_text() {
    const COLUMNS: usize = 80;

    assert_eq!(separator_line(AT, COLUMNS), FULL);
}

// U9: the rules are decoration, so they are the first thing a narrow terminal loses.
#[test]
fn narrower_than_the_full_text_the_rules_are_dropped() {
    for columns in [width(FULL) - 1, width(WITHOUT_RULES)] {
        assert_eq!(
            separator_line(AT, columns),
            WITHOUT_RULES,
            "at {columns} columns the text fits only without its rules"
        );
    }
}

// U10: narrower still, the text keeps its start and loses its end.
#[test]
fn narrower_than_the_text_without_rules_the_text_is_cut_to_the_width() {
    let columns = width(WITHOUT_RULES) - 1;

    assert_eq!(
        separator_line(AT, columns),
        WITHOUT_RULES.chars().take(columns).collect::<String>(),
        "at {columns} columns the text is cut to the width"
    );
}

// U11: a separator wider than the terminal, or with a line break, would wrap into a second row.
#[test]
fn the_separator_is_never_wider_than_the_columns_and_never_breaks_the_line() {
    for columns in [1, 2, width(FULL), width(FULL) - 1] {
        let separator = separator_line(AT, columns);

        assert!(
            width(&separator) <= columns,
            "{separator:?} is wider than {columns} columns"
        );
        assert!(
            !separator.contains(['\n', '\r']),
            "{separator:?} holds a line break"
        );
    }
}

const NOTICE: &str = "── earlier output could not be restored ──";
const NOTICE_WITHOUT_RULES: &str = "earlier output could not be restored";

// U12: the notice has its rules at 80 columns and loses them first when narrower.
#[test]
fn the_notice_is_ruled_at_80_columns_and_loses_its_rules_first() {
    assert_eq!(notice_line(80), NOTICE);
    for columns in [width(NOTICE) - 1, width(NOTICE_WITHOUT_RULES)] {
        assert_eq!(notice_line(columns), NOTICE_WITHOUT_RULES, "at {columns}");
    }
}

// U13: narrower still it is cut, and it is always one row.
#[test]
fn the_notice_is_cut_to_the_width_and_never_wider_or_broken() {
    for columns in [1, 2, 10, width(NOTICE_WITHOUT_RULES) - 1] {
        let notice = notice_line(columns);
        assert_eq!(
            notice,
            NOTICE_WITHOUT_RULES
                .chars()
                .take(columns)
                .collect::<String>()
        );
        assert!(width(&notice) <= columns && !notice.contains(['\n', '\r']));
    }
}
