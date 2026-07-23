//! T036 [US1] — a client that stops reading causes no unbounded daemon growth and converges to the
//! true screen on resume (SC-006; Edge: slow consumer).
//!
//! At the framer layer this means: the framer holds only a bounded shadow (≤ one viewport), a single
//! frame after a long unframed burst carries at most one screen (no backlog replay), and once caught
//! up the next frame is empty (converged). The full slow-consumer channel back-pressure test lands
//! with the server streaming path.

mod support;

use micold_core::session::SessionId;
use micold_daemon::framer::Framer;
use support::DrivenTerm;

#[test]
fn a_long_unframed_burst_converges_to_one_screen_then_settles() {
    let rows = 24usize;
    let mut vt = DrivenTerm::new(80, rows, 10_000);
    let mut framer = Framer::new(SessionId::new());

    // The client never framed a thing; meanwhile a huge burst of output arrives.
    vt.feed_lines(0, 20_000);

    // One frame converges to the CURRENT screen — bounded by the viewport, not 20k lines of backlog.
    let frame = framer.frame(&vt.term, true, None);
    assert!(
        frame.lines.len() <= rows,
        "a frame after a burst carries at most one screen, got {}",
        frame.lines.len()
    );
    // It reflects the latest output, not stale history.
    let newest = frame
        .lines
        .iter()
        .map(|l| l.id.0)
        .max()
        .expect("frame has lines");
    assert_eq!(
        newest,
        frame.viewport_top.0 + rows as i64 - 1,
        "the frame's newest line is the live bottom of the screen"
    );

    // With no further output, the next frame is a delta carrying nothing — fully converged.
    let settled = framer.frame(&vt.term, false, None);
    assert!(!settled.full);
    assert_eq!(settled.lines.len(), 0, "converged: nothing left to send");
}

#[test]
fn scrollback_retention_is_bounded_even_while_unframed() {
    // A small retention cap; a big burst with NO framing during it. The retained window must stay
    // bounded (oldest-first discard happens in the emulator, applied even with no client) — no
    // unbounded growth. (Evictions that happened before the first frame are unobservable, so the
    // watermark simply bases at 0 here; that is correct — a not-yet-attached client has no cache.)
    let rows = 24usize;
    let cap = 100usize;
    let mut vt = DrivenTerm::new(80, rows, cap);
    let mut framer = Framer::new(SessionId::new());

    vt.feed_lines(0, 5_000);
    let frame = framer.frame(&vt.term, true, None);

    let retained = frame.viewport_top.0 - frame.oldest_available.0 + rows as i64;
    assert!(
        retained <= (cap + rows) as i64,
        "retained lines ({retained}) must stay within cap+screen ({})",
        cap + rows
    );
}

#[test]
fn the_watermark_advances_as_evictions_are_observed() {
    // When the framer ticks across the cap boundary, it observes evictions and advances
    // `oldest_available` — proving discard is tracked, not silently accumulating.
    let rows = 24usize;
    let cap = 100usize;
    let mut vt = DrivenTerm::new(80, rows, cap);
    let mut framer = Framer::new(SessionId::new());

    // Fill below cap first and take a baseline frame (watermark at 0, nothing evicted yet).
    vt.feed_lines(0, rows);
    let base = framer.frame(&vt.term, true, None);
    assert_eq!(
        base.oldest_available.0, 0,
        "no evictions before the cap is reached"
    );

    // Now drive well past the cap, framing each step so evictions are observed.
    for i in 0..(cap + 200) {
        vt.feed_lines(rows + i, 1);
        let _ = framer.frame(&vt.term, false, None);
    }
    let frame = framer.frame(&vt.term, false, None);
    assert!(
        frame.oldest_available.0 > 0,
        "watermark must advance once history fills and lines are discarded"
    );
    let retained = frame.viewport_top.0 - frame.oldest_available.0 + rows as i64;
    assert!(
        retained <= (cap + rows) as i64,
        "retention stays bounded ({retained} <= {})",
        cap + rows
    );
}
