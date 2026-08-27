# 022 §B2 — the transition, measured

**Date**: 2026-08-25
**Covers**: T011 (§B2 against the type-ahead), T028 (§B2 against both pickers), and the §B2 row of
T040's table
**Platform**: Xvfb `:83` at 1600×1400, Mesa lavapipe software Vulkan, no window manager — not a real
display. `micold-showcase` from `~/vp83/bin`, built 2026-08-20.
**Result**: the **grow-in** clause passes for both pickers; the **fade-out** clause fails for both,
and it is the same defect 007 found — [007 BUG-001](../../007-motion-overlay-fade/bugs/BUG-001.md)

**Superseded reading (2026-08-26)**: the lists were not cut. Both faded the whole way out and
reached their targets — in about a third of the stated duration, so at 60 fps only two or three
frames of each exit were captured, and on the back-loaded `accelerate` curve those frames are its
flat head. That is the "~1%" and "~7%" below. See
[BUG-001's root cause](../../007-motion-overlay-fade/bugs/BUG-001.md#root-cause); the clock is now
elapsed-time, so §B2's "in noticeably less time than it took to arrive" now holds by the durations
the components state (`SHORT_4` out against `MEDIUM_2` in).

## The instrument that made this answerable

[B-gallery-pass.md](./B-gallery-pass.md) left §B2 **PARTIAL** with a specific reason: *"A screenshot
pipeline cannot reliably catch a chosen frame of a 150 ms transition — the `visual-pass` skill says
so and this pass did not beat it. T011 and T028 stay open on exactly this."*

That is true of `import -window root`. It is not true of this machine, which has GStreamer:

```bash
gst-launch-1.0 -q ximagesrc display-name=:83 startx=30 starty=742 endx=329 endy=914 \
  use-damage=false ! video/x-raw,framerate=60/1 ! videoconvert ! pngenc snapshot=false \
  ! multifilesink location=cap/f%04d.png
```

60 fps sustained, verified at 119 frames in 2.0 s. One capture frame is 16.7 ms, so a transition is
a frame count. Frames are read back with PIL — mean luminance per frame, and the bounding box of
everything differing from a chosen base frame, which is what makes the list's growth measurable.

Two harness notes carried over from the 007 pass: **compound dwell-less clicks do not register**
here (`xdotool mousemove X Y mousedown 1 mouseup 1` silently does nothing; a dwell between down and
up is required), and the app renders below 60 Hz under lavapipe, so a count of *distinct* values is
the honest measure and wall-clock figures read long.

## The clauses

> - It **grows** from slightly compressed to full while fading in, and settles rather than snapping.

**PASS, both pickers.**

The **Select** list is first drawn at 165 px tall with its rows indented ~21 px further left than
their resting position, and over the next 14 capture frames the list widens outward and the rows
walk back to their final offset — row text at x≈48, then 32, then 30, then 27 — settling by f63.
Large steps first, one-pixel steps at the end.

The **Typeahead** list does the same over ~5 distinct values: rows at x≈42 (f50), 32 (f52), 30
(f54), 27 (f58).

Neither snaps. Both arrive compressed and finish full.

> - It **fades out** on the way, in noticeably less time than it took to arrive.

**FAIL, both pickers.** It does not fade out. It plays a few percent of the exit and is then cut.

| | Frames | Range covered before it vanished |
|---|---|---|
| Select — close (second press on the trigger) | 3 | 166 → 165 → 164 px, then gone — **~1%** |
| Typeahead — close (Escape) | 3 | 300 → 290 → 278 px, then gone — **~7%** |

Set against a 14-frame entrance, "noticeably less time than it took to arrive" is technically
satisfied and entirely beside the point: there is no fade-out to be shorter.

**This is not a picker defect.** The same session measured the gallery's own `Fade` and `Scale`
entries — posed with their own **Replay** and **Reverse (play it out)** buttons, with no picker,
overlay or dismissal gesture anywhere near them — and `Fade`'s Reverse renders **6 evenly-spaced
steps of ~4.6% alpha each, 23% of the visible range, and then jumps the remaining ~77% in a single
frame**. `Scale`'s Reverse shrinks 246 → 236 px over 6 steps and then vanishes. The pickers are
inheriting a defect in the shared exit path, and it is filed against 007, where the primitives live.
It is also not frame starvation: repeating `Scale`'s Reverse while driving the pointer continuously
across two buttons — forcing a redraw every frame — cut off at exactly the same 236 px.

> - Reverse one mid-flight: it continues from where it is, it does not jump to either end.

**PASS, with a caveat about how it was shown.**

A Select interrupted by a second press went 167 → 165 → 164 → gone: it resumed downward from its
current value rather than jumping to a compressed start or snapping to full first. But the enter had
already reached 167 by the time the second press landed, so this was a reversal *at* the end of the
enter rather than genuinely mid-flight — the enter here is only about four app frames, and posing a
press inside that window with `xdotool` was not reliable.

The rigorous answer is renderer-independent and already exists:
`an_interrupted_transition_resumes_from_where_it_is` and `rapid_toggling_never_sticks_part_way` in
`crates/micold-client/src/ui/cdk/motion.rs` (5 passed, 0 failed). The observation above is consistent
with them. This is recorded as passing on the strength of the tests plus a consistent observation,
not on the observation alone.

> - Press where a row used to be while the list is fading out: **nothing is chosen.**

**PASS** — already established, by a different route, in
[B2-press-during-exit.md](./B2-press-during-exit.md). Worth noting that BUG-001 makes this clause
much easier to satisfy than intended: the window in which a press could land on a departing row is
about one frame wide.

> - Watch the rest of the page throughout. **Nothing outside the list moves, at any point.**

**PASS** — established in [B-gallery-pass.md](./B-gallery-pass.md) as 0 differing pixels above the
list between an open and a closed frame with the pointer controlled, and re-confirmed here: across
every capture above, the surrounding page (the `FilterTrigger` and `ResizeHandle` headings sitting
under the lists) is byte-identical except where the list overlaps it.

## Summary for T040's table

| §B2 clause | |
|---|---|
| grows from compressed to full, settles | **PASS** — 14 frames (select), ~5 (type-ahead) |
| fades out on the way | **FAIL** — 3 frames, ~1% / ~7% of range, then cut. [007 BUG-001](../../007-motion-overlay-fade/bugs/BUG-001.md) |
| a reversal resumes rather than jumping | **PASS** — by unit test, with a consistent observation |
| a press during the exit chooses nothing | **PASS** — [B2-press-during-exit.md](./B2-press-during-exit.md) |
| nothing outside the list moves | **PASS** — 0 differing pixels |

## What transfers and what does not

- **Transfers**: that entrances run to completion and exits are truncated. That is a difference in
  rendered frame count between two directions of the same transition, measured by one instrument in
  one session; no property of a software rasteriser produces fourteen steps one way and three the
  other.
- **Does not transfer**: wall-clock durations and perceived smoothness. lavapipe is a software
  rasteriser and the gallery renders well below 60 Hz on it.
- **Not covered**: the dark scheme for §B2 specifically. §B1's eight properties were checked in both
  schemes in the earlier pass; the transition was measured in light only, and the scheme cannot
  plausibly change a frame count.
