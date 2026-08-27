# 007 — the exits, re-measured after BUG-001

**Date**: 2026-08-26
**Covers**: [BUG-001](../bugs/BUG-001.md) verification, and the two clauses the bug left
unanswerable — §3's *reveal-beneath* and the spec's *reopen-during-exit* edge case
**Result**: both clauses **PASS**; the exits now run to their stated duration
**Ran on**: Xvfb `:85` at 1600×1400, Mesa lavapipe software Vulkan, no window manager —
not a real display. See "What this does and does not transfer" at the end.

## The instrument, and the one correction to it

Same GStreamer recorder as the [first pass](T024-quickstart-pass.md), reading the X server
continuously:

```bash
gst-launch-1.0 -q ximagesrc display-name=:85 use-damage=false \
  ! 'video/x-raw,framerate=60/1' ! videoconvert ! pngenc ! multifilesink location=cap/f%05d.png
```

**The requested 60 fps is not what you get, and the first pass's arithmetic assumed it was.** The
real cadence has to be read back from the frames themselves — `os.stat(f).st_mtime` deltas — and it
depends on the scene: **19.1 ms** per frame for the client with a dialog open, **25.5 ms** for the
showcase. Every wall-clock number below is a count of frames multiplied by that measured cadence,
not by 16.7 ms.

Two further measurement notes:

- **Pick a high-contrast patch.** The first probe here sat on a flat dark background and resolved
  seven integer grey levels across an entire transition — useless. The numbers below come from the
  purple *Open a project* button (client) and the interior of the demo card (showcase).
- **Frame numbers are capture frames, not app frames.** The app draws faster than the recorder
  samples, so a count of capture frames is a *lower* bound on rendered intermediate values.

## The About dialog's exit — FR-002, SC-002

Client on `:85`, About opened, then dismissed. Mean luminance of the *Open a project* button behind
the scrim, one row per capture frame at 19.1 ms:

```
109.39  109.43  112.71  117.35  122.18  128.50  134.91  142.19  148.62  156.51  160.58
        ^ last still frame                                                 settled ^
```

| Clause | Result |
|---|---|
| More than one intermediate value | **PASS** — 8–9 monotone intermediate capture frames |
| Reaches its target before the element is dropped | **PASS** — plateaus at 160.58 and stays |
| Spans its stated `SHORT_4` (200 ms) | **PASS** — bounded to 163–186 ms of wall clock |

The nominal is 200 ms and the *visible* span is shorter by construction: the scrim exits on
`STANDARD_ACCELERATE`, which leaves only ~6 % of its range in the last frame before the end, below
this measurement's quantisation. Curve arithmetic predicts ~181 ms visible; 163–186 ms brackets it.

Before the fix the same exit measured 124–163 ms with its first drawn frame already a third of the
way out — which is what
[BUG-001's second defect](../bugs/BUG-001.md#a-second-defect-in-the-fix--found-by-the-pixels-not-by-the-suite)
turned out to be.

### Reveal-beneath (§3, FR-003) — **PASS**

The same trace answers it: the application behind the dialog reappears *progressively*, over nine
capture frames, rather than at once when the dialog is dropped. It holds at the pixel level and not
only in the mean — the button's fill and its label move together and monotonically:

| frame | 206 | 208 | 210 | 212 | 214 | 216 |
|---|---|---|---|---|---|---|
| fill | (140,128,173) | (145,132,179) | (157,143,194) | (174,158,214) | (192,174,236) | (207,188,255) |
| label | (19,18,20) | (20,19,21) | (21,21,23) | (23,23,25) | (26,25,28) | (28,27,30) |

![the dialog's exit revealing the app beneath](img/dialog-exit-reveal.png)

*Capture frames 206, 208, 210, 212, 214, 216 at identical geometry — 19.1 ms apart, so ~190 ms
left to right.*

## The showcase `Fade` — Reverse

Motion section, **Reverse (play it out)**, nominal 600 ms, cadence 25.5 ms:

| | rendered steps visible to the capture | wall clock |
|---|---|---|
| As reported (BUG-001) | 6, then the remaining ~77 % in a single frame | 245 ms |
| Now | ~21, monotone | ~561 ms |

## Reopen during exit — the spec's edge case — **PASS**

**Reverse**, then **Replay** roughly 230 ms into the 600 ms exit. Mean luminance of the card
interior; at rest the card reads 233.69 and the page behind it reads 250.00:

```
frame  285     289     293     295  │  296     298     300     302     309
       234.56  234.57  235.45  238.07│ 237.19  235.45  234.58  234.57  233.69
       └──────── exiting ───────────┘│ └──────── returning ───────────┘
                                     ^ Replay pressed
```

It **resumes from where it is**. The exit had travelled to 238.07 — about 27 % of its range — and
the return starts from 238.07, not from the fully-hidden 250.00 and not from a snap back to 233.69.
There is no discontinuity at the reversal in either direction.

![an exit interrupted by a reopen](img/reopen-during-exit.png)

*Capture frames 285/289/293 (exiting, red), 295 (the frame Replay was pressed, amber), 298/302/308
(returning, blue). The signal is the card's fill and border against the page — see the observation
below for why its label is not part of it.*

The primitive-level gate `an_interrupted_transition_resumes_from_where_it_is` already asserted this;
what the first pass could not do, and this can, is confirm it composited on screen.

## An observation this pass was not looking for: the veil does not dim text

`Fade` approximates opacity by compositing a quad of the backdrop tone over its own content
(`ui/material/animation.rs`, module header — "iced exposes no element opacity"). Measured across the
showcase card's whole exit, the card's **background** converges correctly — (248,242,247) →
(253,248,253), which is exactly the page tone behind it — while its **glyphs** stay at (28,27,30)
for every frame and then disappear with the widget. The veil is a quad drawn in the same layer as
the text it is meant to cover, and the renderer draws that layer's text above its quads.

This is a property of the approximation, not of BUG-001, and it does not affect this feature's
headline claim: an overlay's scrim is a *separate* layer in `cdk::overlay`'s stack, so it does dim
the text beneath it — which is exactly what the reveal-beneath table above measures. What it means
is that `Fade` used on its own (the showcase demo, `ViewFade`, `HoverReveal`) fades a panel's fill
and leaves its labels at full contrast until the panel is dropped.

## What this does and does not transfer

**Covered here**: exit frame counts, exit wall clock against the stated durations, progressive
reveal of the content beneath, and resumption from mid-flight on a reversal — all as pixel
measurements, on this rasteriser.

**Not covered**:

- **Perceived smoothness on real hardware.** lavapipe is a software rasteriser; frame pacing here
  says nothing about frame pacing on the user's GPU. The *durations* transfer — the clock is now
  wall-clock elapsed time, which is the point of the fix — the *cadence* does not.
- **The `Fade` veil's one-frame label removal**, above. Recorded, not fixed.
