//! T033b [US1] — scrollback-by-range: a range request returns contiguous lines by `LineId`, a
//! request past the retained watermark clamps rather than errors, and a line's identity is immutable
//! so a selection anchored to line ids is never corrupted by new output arriving mid-scroll
//! (FR-017, FR-018).

mod support;

use micold_core::protocol::grid::LineId;
use micold_core::session::SessionId;
use micold_daemon::framer::Framer;
use support::DrivenTerm;

#[test]
fn a_range_returns_contiguous_lines_by_id() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let mut framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 300); // 300 lines: well into history
    let frame = framer.frame(&vt.term, true, None);

    let (lines, _, _, more) = framer.scrollback_range(&vt.term, frame.oldest_available, 10);
    assert_eq!(lines.len(), 10, "asked for 10 retained lines, got 10");
    for pair in lines.windows(2) {
        assert_eq!(
            pair[1].id.0,
            pair[0].id.0 + 1,
            "range lines are contiguous by id"
        );
    }
    assert_eq!(
        lines[0].id, frame.oldest_available,
        "range starts at the watermark"
    );
    assert!(!more, "nothing older than the oldest retained line");
}

#[test]
fn a_request_past_the_watermark_clamps_instead_of_erroring() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let mut framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 300);
    let frame = framer.frame(&vt.term, true, None);

    // Ask starting far below the oldest retained line: clamp up to the watermark, don't error.
    let (lines, _, _, _) =
        framer.scrollback_range(&vt.term, LineId(frame.oldest_available.0 - 1000), 5);
    assert_eq!(lines.len(), 5);
    assert_eq!(
        lines[0].id, frame.oldest_available,
        "clamped to the oldest retained line"
    );

    // Ask starting past the newest line: empty, not a panic.
    let (empty, _, _, _) =
        framer.scrollback_range(&vt.term, LineId(frame.viewport_top.0 + 10_000), 5);
    assert!(
        empty.is_empty(),
        "a request beyond the live edge returns nothing"
    );
}

#[test]
fn a_lines_identity_is_immutable_under_new_output() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let mut framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 100);
    let frame = framer.frame(&vt.term, true, None);

    // Anchor a "selection" to a specific line id well inside history.
    let anchor = LineId(frame.oldest_available.0 + 5);
    let (before, _, _, _) = framer.scrollback_range(&vt.term, anchor, 1);
    let anchored_text = before[0].text.clone();
    assert_eq!(before[0].id, anchor);

    // New output arrives mid-scroll (the session keeps producing).
    vt.feed_lines(100, 50);
    let _ = framer.frame(&vt.term, false, None);

    // The same id still resolves to the same content — a history line is immutable once scrolled off
    // (invariant I2), so a selection anchored to it is never corrupted.
    let (after, _, _, _) = framer.scrollback_range(&vt.term, anchor, 1);
    assert_eq!(after[0].id, anchor);
    assert_eq!(
        after[0].text, anchored_text,
        "an anchored line's text must not change under new output"
    );
}

// ---------------------------------------------------------------------------------------
// Feature 034, T063 — `Framer::plain_tail`: what `read_session_output` returns (FR-012,
// research R11; U194–U197)
// ---------------------------------------------------------------------------------------

fn numbered(range: std::ops::Range<usize>) -> Vec<String> {
    range.map(|i| format!("line{i}")).collect()
}

/// U194: the tail spans scrollback and screen, oldest first, and ends at the last line written —
/// the rows of the screen the process has not reached yet are not output.
#[test]
fn plain_tail_returns_the_last_n_lines_of_scrollback_plus_screen() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 300); // far more than one screen: most of it is scrollback

    let (lines, truncated) = framer.plain_tail(&vt.term, 5);
    assert_eq!(lines, numbered(295..300));
    assert!(truncated, "295 older lines exist");

    // 40 lines cross the boundary between the 24-row screen and the scrollback above it.
    let (lines, _) = framer.plain_tail(&vt.term, 40);
    assert_eq!(lines, numbered(260..300));
}

/// U194: never more than N, whatever N is.
#[test]
fn plain_tail_never_returns_more_than_n_lines() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 3_000);
    for n in [1, 2, 23, 24, 25, 200, 2_000] {
        let (lines, truncated) = framer.plain_tail(&vt.term, n);
        assert_eq!(lines.len(), n, "n = {n}");
        assert_eq!(
            lines.last().map(String::as_str),
            Some("line2999"),
            "n = {n}"
        );
        assert!(truncated, "n = {n}");
    }
}

/// U195: the grid holds cells, not bytes, so colour and cursor sequences leave no trace; padding
/// to the right edge is dropped, and a blank line between two lines is kept.
#[test]
fn plain_tail_lines_carry_no_escape_sequences_and_are_right_trimmed() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let framer = Framer::new(SessionId::new());
    vt.feed("\x1b[1;31mred\x1b[0m and plain   \r\n");
    vt.feed("\r\n");
    vt.feed("\x1b]8;;https://example.test\x1b\\link\x1b]8;;\x1b\\\r\n");
    vt.feed("  indented\x1b[K\r\n");
    vt.feed("a\tb\r\n");
    vt.feed("日本語 ok\r\n");

    let (lines, truncated) = framer.plain_tail(&vt.term, 200);
    assert_eq!(
        lines,
        [
            "red and plain",
            "",
            "link",
            "  indented",
            "a       b",
            "日本語 ok",
        ]
    );
    assert!(!truncated);
    for line in &lines {
        assert!(
            !line.chars().any(char::is_control),
            "a control character reached the text: {line:?}"
        );
    }
}

/// U195: a cursor-addressed redraw is read as it is shown, not as the bytes that drew it.
#[test]
fn plain_tail_reads_what_the_screen_shows_after_a_redraw() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let framer = Framer::new(SessionId::new());
    vt.feed("working...\r\n");
    // Back to the first row, erase it, write the final text.
    vt.feed("\x1b[1;1H\x1b[2Kdone\r\n");

    let (lines, truncated) = framer.plain_tail(&vt.term, 200);
    assert_eq!(lines, ["done"]);
    assert!(!truncated);
}

/// U196 (EC-16): `truncated` says older lines exist beyond the ones returned.
#[test]
fn plain_tail_is_truncated_when_older_lines_exist_beyond_n() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 3);

    let (lines, truncated) = framer.plain_tail(&vt.term, 2);
    assert_eq!(lines, numbered(1..3));
    assert!(truncated, "line0 is older than what was returned");
}

/// U196: lines the scrollback limit already discarded count as older lines too, once the framer
/// has seen them leave.
#[test]
fn plain_tail_is_truncated_when_the_scrollback_limit_discarded_older_lines() {
    let mut vt = DrivenTerm::new(80, 24, 100);
    let mut framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 50);
    let _ = framer.frame(&vt.term, true, None);
    vt.feed_lines(50, 250);
    let _ = framer.frame(&vt.term, false, None);

    let (lines, truncated) = framer.plain_tail(&vt.term, 2_000);
    assert!(
        lines.len() < 300,
        "the limit discarded lines: {}",
        lines.len()
    );
    assert_eq!(lines.last().map(String::as_str), Some("line299"));
    assert!(
        truncated,
        "everything retained was returned, but older lines existed and were discarded"
    );
}

/// U197: when the whole content fits, nothing older exists.
#[test]
fn plain_tail_is_not_truncated_when_the_whole_content_fits_in_n() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 3);

    for n in [3, 4, 200] {
        let (lines, truncated) = framer.plain_tail(&vt.term, n);
        assert_eq!(lines, numbered(0..3), "n = {n}");
        assert!(!truncated, "n = {n}: exactly the whole content");
    }
}

/// A session that has shown nothing has no lines, and nothing older than none.
#[test]
fn plain_tail_of_an_empty_terminal_is_empty() {
    let vt = DrivenTerm::new(80, 24, 10_000);
    let framer = Framer::new(SessionId::new());

    assert_eq!(framer.plain_tail(&vt.term, 200), (Vec::new(), false));
}
