//! T035 [US1] — stable-`LineId` diffing keeps a scrolling workload at ~1 line/frame, not a whole
//! screen. This 11× reduction is load-bearing (it is why streaming is affordable) and must not
//! silently regress.

mod support;

use micold_core::session::SessionId;
use micold_daemon::framer::Framer;
use support::DrivenTerm;

#[test]
fn scrolling_sends_about_one_line_per_frame_not_the_whole_screen() {
    let rows = 24;
    let cols = 80;
    let mut vt = DrivenTerm::new(cols, rows, 10_000);
    let mut framer = Framer::new(SessionId::new());

    // Fill the screen, then take the initial full snapshot.
    vt.feed_lines(0, rows);
    let first = framer.frame(&vt.term, false, None);
    assert!(first.full, "first frame is a full snapshot");
    assert_eq!(
        first.lines.len(),
        rows,
        "full frame carries every visible row"
    );

    // Now scroll one line at a time. Each scroll shifts the whole viewport up by one, but with
    // stable ids every surviving line keeps its id — so the delta is only the NEW bottom line.
    let mut worst = 0usize;
    for i in 0..200 {
        vt.feed_lines(rows + i, 1);
        let frame = framer.frame(&vt.term, false, None);
        assert!(!frame.full, "steady scrolling must not force full frames");
        worst = worst.max(frame.lines.len());
    }
    assert!(
        worst <= 2,
        "a one-line scroll must diff to ~1 line, got up to {worst} lines/frame"
    );
}

#[test]
fn an_unchanged_screen_diffs_to_zero_lines() {
    let mut vt = DrivenTerm::new(80, 24, 10_000);
    let mut framer = Framer::new(SessionId::new());
    vt.feed_lines(0, 24);
    let _ = framer.frame(&vt.term, false, None);

    // No new output: the next frame is a delta carrying nothing.
    let frame = framer.frame(&vt.term, false, None);
    assert!(!frame.full);
    assert_eq!(frame.lines.len(), 0, "an idle screen sends no line data");
}
