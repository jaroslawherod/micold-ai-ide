# T085 — Retargeted `TerminalPane` repaint cost at 60 Hz (client side)

**Date**: 2026-08-25 · **Risk 2** · Measured on Xvfb `:83` (1600×1400) with Mesa **lavapipe**
(software Vulkan), pinned client+daemon `0.8.0` built 2026-08-20 from this branch.

The task was recorded as *"Blocked in this environment: needs a running GUI + a frame profiler;
can't be measured headlessly here."* Both halves turned out to be reachable:

- the **frame profiler already ships in the client** — `MICOLD_FRAME_PROBE=<frames>[:<warm-up>]`
  (`micold_core::frame_probe`, feature 018 FR-039b) subscribes `iced::window::frames()`, times
  `render(app)` per frame, prints `mean / p95 / max` and exits;
- **CPU** is `/proc/<pid>/task/*/stat` (`utime+stime`) sampled over a fixed window, which separates
  the application's own main thread from lavapipe's rasteriser pool.

## Method

One project (`r10`), one displayed session in **Regular** terminal mode (a real bash PTY — no API
tokens), window 1400×1300, 140×64 cells. Two conditions, everything else identical:

| Condition | What the session was doing |
|---|---|
| **idle** | bash at a prompt, zero output |
| **flood** | `seq 1 6000000 \| sed "s/$/ ....../"` — ~23k lines/s, saturating the daemon's 16 ms frame interval |

## View-composition cost (the frame probe, 400 counted frames after 60 warm-up)

| Condition | mean | p95 | max |
|---|---|---|---|
| idle | **0.31 ms** | 0.40 ms | 1.20 ms |
| flood | **0.42 ms** | 0.58 ms | 1.39 ms |

Streaming a saturating flood adds **+0.11 ms mean / +0.18 ms p95** to composing a frame — 2.5 % of a
16.7 ms budget at 60 Hz, against 1.9 % at rest. The pane itself is not the cost.

## Whole-process CPU (20 s windows, ticks → % of one core)

| | client, all threads | client, **main thread** | daemon |
|---|---|---|---|
| idle | 15.8 % | **1.6 %** | 1.0 % |
| flood | 529 % | **51.7 %** | 101.9 % |

Read the three columns separately:

- **client, all threads** is dominated by lavapipe: at idle, 14 `llvmpipe-*` worker threads accrue
  ~280 of the 316 ticks and the main thread 32. A software rasteriser drawing 60 fps of dense text
  is what 529 % is; it says nothing about the user's GPU.
- **client main thread** is the figure T085 is actually about: **51.7 % of one core under a
  saturating flood**, of which view composition is only 2.5 % (0.42 ms × 60 Hz). The remaining ~49 %
  is everything else on that thread — decoding `GridFrame`s off the connection, updating
  `GridCache`, and driving the renderer.
- **daemon at 101.9 %** — one full core — is the vte parse of 23k lines/s, not the client's problem.

## Is the tick rate the right knob?

**No — the client has no terminal tick to turn.** `shell/subscriptions.rs` states it: *"The terminal
output poll is gone — the daemon streams grid frames over the connection"*, and *"the idle window
schedules nothing at all, rather than ticking 60 times a second."* The only per-frame subscription in
the client is the measurement run's own `window::frames()`, guarded on `probe_config().is_some()` and
pinned by `tests/idle_subscriptions.rs`.

The knob that exists is **daemon-side**: `FRAME_INTERVAL = 16 ms` in `micold-daemon/src/server.rs:1543`,
the view stream's wake interval, between which output is coalesced into a single delta.
`crates/micold-daemon/tests/frame_coalescing.rs` pins the property — 20 000 lines arrived in **30
frames over 615 ms** (667 lines per frame). Raising it trades latency for client CPU roughly linearly,
since per-frame cost is nearly flat in how much changed (0.31 → 0.42 ms from nothing to a full screen
of new text).

So the honest recommendation is: leave 16 ms alone. At 60 Hz the retargeted pane costs half a
millisecond a frame; if a slow client ever needs relief, `FRAME_INTERVAL` is where to find it, and it
is one constant in one place.

## What this does not measure

- **GPU submit and present.** The probe times `render(app)` — the view-tree composition — not the
  wgpu draw. On lavapipe the draw is what the `llvmpipe-*` threads are doing, and its cost here is a
  property of the rasteriser, not of the client.
- **Frame pacing.** Nothing here says a frame was *presented* every 16.7 ms, only what composing one
  cost.
- **A real GPU.** Every figure above is from a software Vulkan device.
