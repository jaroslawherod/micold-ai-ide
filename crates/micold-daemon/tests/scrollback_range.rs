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
