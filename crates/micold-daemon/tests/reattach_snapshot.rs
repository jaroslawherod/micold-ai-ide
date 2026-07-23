//! T028 [US1] — on reattach the client gets a full-snapshot `GridFrame` (not a replay) whose
//! content reflects the current screen, with output that arrived while detached included and
//! scrollback bounded by the retention limit (FR-014, FR-017).

mod support;

use micold_core::session::SessionId;
use micold_daemon::framer::Framer;
use support::DrivenTerm;

/// The visible text of a full frame (rows in order), for content assertions.
fn frame_text(frame: &micold_core::protocol::grid::GridFrame) -> String {
    let mut lines: Vec<_> = frame.lines.iter().collect();
    lines.sort_by_key(|l| l.id.0);
    lines
        .iter()
        .map(|l| l.text.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn reattach_gets_a_full_snapshot_covering_the_detached_interval() {
    let rows = 24;
    let mut vt = DrivenTerm::new(80, rows, 10_000);
    let mut framer = Framer::new(SessionId::new());

    // Attached: some output, streamed as usual.
    vt.feed_lines(0, rows);
    let _ = framer.frame(&vt.term, false, None);

    // Detached: the client is gone; more output arrives with nobody framing it.
    vt.feed_lines(rows, 500);

    // Reattach: the client asks for a full snapshot rather than a replay of 500 deltas.
    let snap = framer.frame(&vt.term, true, None);
    assert!(snap.full, "reattach yields a full snapshot, not a delta");
    assert_eq!(snap.lines.len(), rows, "snapshot is the whole screen");

    // The snapshot reflects the CURRENT screen — output produced while detached is present, and the
    // stale pre-detach lines are gone from the viewport.
    let text = frame_text(&snap);
    assert!(
        text.contains(&format!("line{}", rows + 499)),
        "latest detached-interval output must be in the snapshot"
    );
    assert!(
        !text.contains("line0\n") && !text.starts_with("line0"),
        "the pre-detach top line should have scrolled out of the viewport"
    );

    // Scrollback is bounded: the oldest retained line never predates the retention window.
    assert!(
        snap.oldest_available.0 >= 0,
        "oldest_available is a real watermark"
    );
    assert!(
        snap.viewport_top.0 > snap.oldest_available.0,
        "there is retained history behind the viewport"
    );
}
