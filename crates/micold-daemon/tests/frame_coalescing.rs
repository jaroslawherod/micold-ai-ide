//! T058 [005] — the two halves of the performance claim, measured where they are actually decided:
//! redraw coalescing (≤ 1 frame per frame-interval, SC-004/SC-005) and the per-session scrollback
//! cap, both under genuinely chatty output.
//!
//! The task was written against `src/ui/terminal.rs`, back when the client polled the PTY itself.
//! It does not any more: the daemon streams grid frames, and `stream_view` in `server.rs` is what
//! bounds the rate — a fixed 16 ms ticker gated on the VT dirty flag. The client has no redraw
//! driver of its own for terminal output (`shell/subscriptions.rs`: no animation clock, no output
//! poll), so one `Frame::Grid` is one `Message::DaemonGridFrame` is at most one redraw. Bounding
//! the frames on the wire therefore bounds the redraws — and, unlike a frame-pacing figure, it is a
//! protocol-layer count that owes nothing to the renderer. That is why this is measurable headless
//! where SC-008's *perceived* latency is not.
//!
//! Both tests flood a real PTY. A synthetic feed (`support::DrivenTerm`, used by `slow_client.rs`)
//! proves the framer's arithmetic; only a real child writing as fast as it can proves the tick
//! survives contact with a process that never pauses.

#![cfg(unix)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::SessionId;
use micold_daemon::catalog::Catalog;
use micold_daemon::framer::Framer;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use tokio_util::codec::Framed;

/// `server::stream_view`'s tick. Private there, restated here — if it changes, this test is
/// supposed to be re-derived rather than to silently keep passing against the old number.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

const ROWS: u16 = 24;
const COLS: u16 = 80;

/// A shell that prints `<marker>_1 … <marker>_<count>` as fast as it can, then idles so the PTY
/// stays open (EOF would end the stream and stop the clock early).
fn flood(marker: &str, count: usize) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(format!(
        "i=1; while [ $i -le {count} ]; do echo {marker}_$i; i=$((i+1)); done; sleep 60"
    ));
    cmd.cwd(std::env::temp_dir());
    cmd
}

#[tokio::test]
async fn a_flood_is_coalesced_to_at_most_one_frame_per_frame_interval() {
    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
    let state = std::sync::Arc::new(DaemonState::new(Catalog::ephemeral()));

    // 20k lines: far more writes than ticks, whatever the machine. An uncoalesced stream would
    // frame per write and land in the thousands.
    const LINES: usize = 20_000;
    let sid = SessionId::new();
    let session =
        PtySession::spawn(sid, flood("flood", LINES), 1_000, Some((COLS, ROWS))).expect("spawn");
    state.register_session(session);

    let _server = tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(&state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());

    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }

    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: PathBuf::from("/repo/demo"),
            session: Some(sid),
        }))
        .await
        .unwrap();

    // Count grid frames from the first one until the flood's last line is on screen. The clock
    // starts at the first frame, not at the request, so connection setup is not counted against
    // the budget.
    let mut frames = 0usize;
    let mut started: Option<Instant> = None;
    let mut saw_last = false;
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), client.next()).await {
            Ok(Some(Ok(Frame::Grid(f)))) => {
                frames += 1;
                started.get_or_insert_with(Instant::now);
                if f.lines
                    .iter()
                    .any(|l| l.text.contains(&format!("flood_{LINES}")))
                {
                    saw_last = true;
                    break;
                }
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    let elapsed = started.expect("at least one frame arrived").elapsed();

    // Non-vacuity: the flood really ran and really streamed, so a low frame count means coalescing
    // and not a stalled child.
    assert!(saw_last, "the flood's last line ({LINES}) never arrived");
    assert!(frames >= 2, "only {frames} frame(s) — nothing was streamed");

    // The budget. `MissedTickBehavior::Delay` means a tick can slip late but never early, so the
    // ceiling is exact; +2 covers the initial full snapshot and the partial interval at each end.
    let budget = elapsed.as_nanos() / FRAME_INTERVAL.as_nanos() + 2;
    // Printed so the evidence note quotes measurements rather than the assertion's mere absence.
    println!(
        "coalescing: {LINES} lines streamed in {frames} frames over {elapsed:?} \
         (budget {budget} @ {FRAME_INTERVAL:?}/frame; {:.1} lines per frame)",
        LINES as f64 / frames as f64
    );
    assert!(
        frames as u128 <= budget,
        "{frames} frames in {elapsed:?} exceeds one per {FRAME_INTERVAL:?} (budget {budget}) — \
         output is not being coalesced"
    );
}

#[test]
fn each_session_keeps_its_own_capped_history_under_a_flood() {
    // A small cap so the flood blows past it by two orders of magnitude, and a second session that
    // does nothing but sit on its own output while the first is hammered.
    const CAP: usize = 100;
    const LINES: usize = 5_000;

    let noisy = SessionId::new();
    let quiet = SessionId::new();
    let noisy = PtySession::spawn(noisy, flood("noisy", LINES), CAP, Some((COLS, ROWS)))
        .expect("spawn noisy");
    let quiet =
        PtySession::spawn(quiet, flood("quiet", 3), CAP, Some((COLS, ROWS))).expect("spawn quiet");

    // Wait for the flood to finish (its last line on screen), framing as we go so evictions are
    // observed the way a viewing client would observe them.
    let mut noisy_framer = Framer::new(SessionId::new());
    let mut quiet_framer = Framer::new(SessionId::new());
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut done = false;
    while Instant::now() < deadline && !done {
        std::thread::sleep(FRAME_INTERVAL);
        let f = noisy_framer.frame(noisy.term(), false, None);
        done = f
            .lines
            .iter()
            .any(|l| l.text.contains(&format!("noisy_{LINES}")));
        let _ = quiet_framer.frame(quiet.term(), false, None);
    }
    assert!(done, "the flood never reached its last line");

    let n = noisy_framer.frame(noisy.term(), true, None);
    let retained = n.viewport_top.0 - n.oldest_available.0 + ROWS as i64;
    assert!(
        retained <= (CAP + ROWS as usize) as i64,
        "the flooded session retained {retained} lines, past its cap+screen ({})",
        CAP + ROWS as usize
    );
    // The flood really did overrun the cap: its last line is on screen (asserted above, so all
    // `LINES` of them were emitted, in order) while the retained window is two orders of magnitude
    // smaller — at least `LINES - retained` lines were discarded oldest-first.
    assert!(
        (retained as usize) < LINES,
        "retained {retained} of {LINES} — the flood never exceeded the cap"
    );
    // Deliberately *not* asserted: that `oldest_available` has advanced past 0. The watermark only
    // moves as evictions are observed between frames, and a 5k-line burst can land entirely inside
    // one 16 ms window — in which case every eviction happened before the first frame and is
    // unobservable by construction (`slow_client.rs` says the same). Asserting it made this test
    // pass under `cargo test`'s default parallelism, which slowed the flood enough to spread it
    // over several frames, and fail under `--test-threads=1`. Retention is the property; the
    // watermark is the framer's bookkeeping about it.

    // The quiet session is untouched by its neighbour: its own three lines, none of the flood, and
    // a history nowhere near the cap.
    let q = quiet_framer.frame(quiet.term(), true, None);
    let text: String = q.lines.iter().map(|l| l.text.as_str()).collect::<String>();
    // Printed so the evidence note quotes measurements rather than the assertion's mere absence.
    println!(
        "cap: {LINES} lines flooded into a {CAP}-line cap retained {retained} lines \
         (cap+screen = {}), oldest surviving id {}; the untouched neighbour's oldest is still {}",
        CAP + ROWS as usize,
        n.oldest_available.0,
        q.oldest_available.0
    );
    assert!(
        text.contains("quiet_3"),
        "the quiet session lost its output"
    );
    assert!(
        !text.contains("noisy_"),
        "the flood leaked into the other session"
    );
    assert_eq!(
        q.oldest_available.0, 0,
        "the quiet session evicted history it never produced — the cap is not per-session"
    );

    let _ = noisy.kill();
    let _ = quiet.kill();
}
