# T058 — performance pass: redraw coalescing and the per-session scrollback cap

**Run**: 2026-08-25, Linux, this worktree.
**Claims**: SC-004 / SC-005, via T058 — "verify redraw coalescing (≤1/frame) and per-session
scrollback cap under chatty output".
**Instrument**: a new integration test, `crates/micold-daemon/tests/frame_coalescing.rs`
(two tests, both flooding a real PTY). Not a screenshot pass.

## Why this is measurable where SC-008 was not

T058 was left open on 2026-08-21 with a specific and correct objection: the headless harness renders
through Mesa lavapipe, a software rasteriser, so a frame-pacing figure taken there says nothing
about the user's GPU. That objection is fatal to a *perceived-latency* claim. It is not fatal to
this one, because the thing T058 asks about is no longer decided by the renderer.

The task names `src/ui/terminal.rs`, which is where the coalescing lived when the client polled the
PTY itself. It does not any more. Since the daemon split, terminal output reaches the client as
`Frame::Grid` messages over the connection, and the rate is fixed on the *daemon* side:

- `crates/micold-daemon/src/server.rs:1540-1594` — `stream_view` drives a
  `tokio::time::interval(FRAME_INTERVAL)` with `FRAME_INTERVAL = 16 ms` and
  `MissedTickBehavior::Delay`, and each tick sends at most one frame, gated on
  `if pty.signals().take_dirty()`. A clean tick sends nothing.
- `crates/micold-daemon/src/terminal.rs:55-56, 160-163` — `take_dirty` is a
  `swap(false, AcqRel)`, and every `Event::Wakeup` from the VT parser just re-raises the same
  boolean. A thousand writes between two ticks collapse into one flag, hence one frame.
- `crates/micold-client/src/shell/subscriptions.rs` — the client has no redraw driver of its own
  for terminal output. There is no animation clock and no output poll (its own comment: *"The
  terminal output poll is gone — the daemon streams grid frames over the connection"*); the only
  unconditional timer is the 500 ms / 1 s OS-theme poll. The single `shell.request_redraw()` in the
  whole rendering layer is in `ui/cdk/motion.rs:245`, behind an `if self.animating()` guard, and
  belongs to widget transitions rather than to terminal output.

So one `Frame::Grid` is one `Message::DaemonGridFrame` is at most one redraw, and bounding the
frames on the wire bounds the redraws. That bound is a **protocol-layer count**: it owes nothing to
the renderer, which is exactly what makes it answerable on this machine.

## Measurement 1 — coalescing under a flood

`a_flood_is_coalesced_to_at_most_one_frame_per_frame_interval`: spawn a real `sh` child that prints
20,000 lines as fast as it can, view the session through `serve_connection` over a real socket, and
count `Frame::Grid` messages from the first frame until the flood's last line appears on screen.

```
coalescing: 20000 lines streamed in 48 frames over 1.025511068s (budget 66; 416.7 lines per frame)
coalescing: 20000 lines streamed in 62 frames over 1.277989778s (budget 81; 322.6 lines per frame)
coalescing: 20000 lines streamed in 51 frames over 1.047363426s (budget 67; 392.2 lines per frame)
```

Three consecutive runs, each well under its ceiling: 48/66, 62/81, 51/67. Between 320 and 420 lines
of output are folded into every frame, under a child that never pauses. An uncoalesced stream would
have framed per write and landed in the thousands.

The budget is `elapsed / 16 ms + 2`. It is a true ceiling rather than an estimate: `Delay` lets a
tick slip late but never early, and the `+2` covers the initial full snapshot plus the partial
interval at each end. The test also asserts non-vacuity — the last line of the flood did arrive, and
more than one frame was seen — so a low count cannot be a stalled child masquerading as coalescing.

## Measurement 2 — the scrollback cap is per session, under chatty output

`each_session_keeps_its_own_capped_history_under_a_flood`: two real PTY sessions, both spawned with
a 100-line cap. One is flooded with 5,000 lines; the other prints three lines and idles.

```
cap: 5000 lines flooded into a 100-line cap retained 124 lines (cap+screen = 124),
     oldest surviving id 0; the untouched neighbour's oldest is still 0
```

Exactly `cap + screen`. At least 4,876 lines were discarded oldest-first while the session stayed
live and its newest output stayed correct. The neighbour keeps its own `quiet_3`, contains none of
the flood, and has evicted nothing — the cap is per `Term`, not a shared pool. That is structural:
`supervisor.rs:200-206` builds each session's `Term` with its own
`Config { scrolling_history: scrollback_lines, .. }`.

One assertion was deliberately **not** made, and the test says so at the point where it would go:
that the framer's `oldest_available` watermark has advanced past 0. The watermark only moves as
evictions are *observed between frames*, and a 5,000-line burst can land entirely inside one 16 ms
window — in which case every eviction happened before the first frame and is unobservable by
construction (`slow_client.rs` records the same limitation). Asserting it made the test pass under
`cargo test`'s default parallelism, which slowed the flood enough to spread it across frames, and
fail under `--test-threads=1`. Retention is the property; the watermark is bookkeeping about it.
The three verification runs demonstrate it directly: the watermark read `0`, `0` and `100` across
them while the retained figure was `124` every time.

### Corroboration from the GUI

The same cap was measured through the real UI during 006's quickstart pass on the same day: with the
limit set to 170, a session created afterwards was flooded with 600 lines and retained exactly 170
rows, the oldest surviving flood line being `CAP 432` (600 − 432 + 1 = 169, the flood's share).
See [006 `evidence/gui-pass-2026-08-25.md` §10g](../../006-real-terminal-emulator/evidence/gui-pass-2026-08-25.md)
and `step10g-scrollback-cap.png`. The framer's own arithmetic is covered independently by
`crates/micold-daemon/tests/slow_client.rs`.

## Verification

```
cargo fmt --check -p micold-daemon
cargo clippy -p micold-daemon --test frame_coalescing -- -D warnings
cargo test -p micold-daemon --test frame_coalescing -- --test-threads=1   # ×3, stable
```

## What this run does not claim

- **Not** a statement about frame pacing on the user's GPU. It bounds how many frames the client is
  *asked* to draw, not how long any one of them takes to draw. SC-008's perceived latency stays out
  of reach here for the reason recorded on 2026-08-21.
- The figures are from this machine under this load. The assertion is the invariant; the numbers are
  an illustration of it.
