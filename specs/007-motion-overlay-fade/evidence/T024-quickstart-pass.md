# 007 — manual quickstart pass

**Date**: 2026-08-25
**Covers**: T024 (§3, §5), T028 (§4), T030 (§3 perceptibility), T032 (end-to-end)
**Result**: entrances pass; **every exit fails** — filed as [BUG-001](../bugs/BUG-001.md)
**Ran on**: Xvfb `:83` at 1600×1400, Mesa lavapipe software Vulkan, no window manager —
not a real display. See "What this does and does not transfer" at the end.

## The instrument

The `visual-pass` skill says mid-flight animation is out of reach: *"a screenshot pipeline cannot
reliably catch a chosen frame of a 150 ms transition."* That is true of `import -window root` —
a six-shot burst fired immediately after Esc returned six byte-identical frames — but it is not
true of this machine. GStreamer records the X server continuously:

```bash
gst-launch-1.0 -q ximagesrc display-name=:83 startx=350 starty=250 endx=1249 endy=1149 \
  use-damage=false ! video/x-raw,framerate=60/1 ! videoconvert ! pngenc snapshot=false \
  ! multifilesink location=cap/f%04d.png
```

Verified at **119 frames in 2.0 s ≈ 60 fps sustained**, so one capture frame is 16.7 ms and a
transition is a count, not an impression. Frames are read back with PIL: mean luminance per frame
(`analyze.py`), the bounding box of everything differing from a chosen base frame (`bbox.py`, which
is what makes the dialog's `scale` visible), and fixed interior/exterior patches (`patches.py`,
which separates the dialog fill from the scrim).

One thing this cannot see: **`fade()` composites toward the element's own surface tone**, so a patch
inside a fading dialog never changes colour. Only the `scale` and the scrim are observable from
outside. Every "frames" count below is therefore a lower bound on what is being animated, and an
exact count of what is *visible*.

Two harness properties matter for reading the numbers:

- **`step_for` is frame-count stepped, not wall-clock interpolated** (`ui/cdk/motion.rs:37`,
  `FRAME = 16`). A stated 300 ms lasts 300 ms only while frames arrive at 60 Hz. Under lavapipe the
  app renders at ~30 Hz, so its 300 ms enter takes ~16 app frames ≈ 500 ms of wall clock. This is a
  property of the software rasteriser, not a timing bug — but it means the wall-clock column below
  reads long by roughly 5/3 for anything the *app* draws, while the sidebar and ripple numbers,
  which came out at a clean 60 Hz, do not need the correction.
- **Compound dwell-less clicks do not register.** `xdotool mousemove X Y mousedown 1 mouseup 1`
  silently does nothing here; a 0.22 s dwell between mousedown and mouseup is required. Two results
  in this pass were invalid until that was found, and were re-run.

## §3 — overlay fade in and out

| Clause | Result |
|---|---|
| Open About: fades/grows in, perceptibly, 0.15–0.5 s | **PASS** |
| Open Settings: same | **PASS** |
| Close via **Cancel** — fades out over ~0.2 s | **FAIL — 1 frame** |
| Close via **Esc** — same fade-out | **FAIL — 1 frame** |
| Submit **successfully** — fades out, does not blink away | **FAIL — 1 frame** |
| Invalid submit — overlay **stays open**, error shown | **PASS** |

**The About entrance, in detail.** Its bounding box grows **530×256 → 560×278** about its centre —
94.6% → 100% of final size — over 30 capture frames (500 ms wall, ~16 app frames), while the scrim
patch over the app content dims from grey 20 to grey 14. The steps are large first and one pixel
last: that is the `emphasized` decelerate curve, drawn. Settings enters the same way, 408×456 →
434×482 over ~23 capture frames (383 ms).

That is exactly the transition §3 asks to see in reverse, and it is what makes the exit numbers
unambiguous rather than a measurement failure: the same instrument, pointed at the same dialog,
resolves sixteen steps going in and one coming out.

**The exits.** About/Esc `f36 → f37`; About/Close-button `f44 → f45`; Settings/Cancel
`f153 → f154`; Settings/Save `f205 → f206`. One capture frame each — dialog and scrim both fully
present, then both fully absent, with nothing between. The Save case needed a 6 s capture window
rather than 2.5 s, because Settings stays open for the daemon round-trip (~3.4 s) before dismissing;
the first attempt missed the dismissal entirely and was re-run.

**Invalid submit passes cleanly**: typing a non-numeric scrollback and pressing Save leaves the
dialog up with *"Enter a whole number of lines."* beneath the field. Cropped frames recorded as
`007-invalid-typed-crop.png` / `007-invalid-submit-crop.png` in the pass scratch.

## §4 — the four migrated animations (T028)

| Animation | Frames @60 fps | Wall | Verdict |
|---|---|---|---|
| Sidebar collapse | 25 | 417 ms | **PASS** — a new value every capture frame |
| Sidebar expand | 26 | 433 ms | **PASS** |
| Resize-handle hover ramp | ≥34 | ≥567 ms above the measurement floor | **PASS** |
| Button press ripple | 8 | 133 ms | **PASS** |
| Overflow menu **open** | 3 | 50 ms | PASS (fast, but stepped) |
| Overflow menu **close** (Esc) | **1** | 17 ms | **FAIL** |
| Main view switch (session ⇄ session) | **1** | 17 ms | **FAIL** — 20.8395 → 31.2524 mean luminance in one frame |

So §4 splits along the same seam as §3: the three animations that move a widget that stays in the
tree are intact and smooth; the two that remove something are instantaneous. That is the observation
BUG-001 is built on — it is not a dialog problem or an Escape problem.

## §5 — the awkward cases

| Clause | Result |
|---|---|
| **Rapid toggling** never leaves an overlay stuck part-way | **PASS** — 6 open/close cycles, final frame clean, nothing wedged (`007-rapid-toggle.png`) |
| **Reopen immediately after dismissing** | **PASS** — Settings re-renders correctly, scrollback still 170 (`007-reopen.png`) |
| A reversal **resumes from where it is** rather than snapping | **PASS**, by test rather than by eye — `an_interrupted_transition_resumes_from_where_it_is` and `rapid_toggling_never_sticks_part_way` in `crates/micold-client/src/ui/cdk/motion.rs` (5 passed, 0 failed). This also settles the identical clause left open by 022 §B2. |
| **Reveal-beneath** — app content progressively reappears | **UNANSWERABLE** while BUG-001 stands: there is no interval in which it could happen |
| **Reopen during the exit** | **UNANSWERABLE**, same reason |
| **Quit mid-animation** | **UNANSWERABLE on this harness** — see below |

**Quit mid-animation, and why it is not a second bug.** `xdotool windowclose` during the enter
panicked: `wgpu error: Validation Error / In Surface::configure / Surface does not support the
adapter's queue family`. Before reporting that, I relaunched a fresh instance and closed it **at
rest, with nothing animating** — *identical panic*. With no window manager on `:83` there is no
WM_DELETE_WINDOW path, so the only close gesture available destroys the X window underneath
lavapipe. The clause is unreachable here and the control run is the proof; it needs a real display.

## §6 — idle cost

§6 asks that CPU sit near zero when the app is untouched. Measured over 15 s with a plain shell in
the main view, no overlay, and the mouse parked: the client sustains **28.9–31.0%** of one core.

That number needed two retractions before it was worth anything. The first reading (~32%) was taken
with a **live Claude Code agent session** in the main view, spinner running. The second attempt
typed `tput civis` to stop the cursor blinking — the keystrokes went to that agent *as a prompt*,
which then worked and pushed CPU to 133.5%. Recovered with ctrl+u then Escape, switched the main
view to a plain shell, and confirmed by capture that **the terminal cursor is not blinking**
(0 pixel transitions over 4 s at 60 fps) before re-measuring.

The per-thread breakdown is what makes the figure interpretable:

| | CPU |
|---|---|
| **llvmpipe software-rasteriser workers** | 29.3% |
| app main thread | 1.0% |
| `micold-daemon` | <1% |

So the application is idle and the cost is the software rasteriser presenting frames; the wall-clock
number does not transfer to GPU hardware and I am not filing it as a defect. The
renderer-independent form of §6's claim — *the animation clock subscription is not running at
rest* — is already carried by the green `idle_requests_no_frames.rs` and `idle_subscriptions.rs`.

## §3 timing constants (T029/T030)

`OVERLAY_ENTER` / `OVERLAY_EXIT` no longer exist in `main.rs`; the timings moved into
`ui/material/modal.rs` when 017 introduced `material::Modal`, and now read
`ENTER = duration::MEDIUM_2` (300 ms) and `EXIT = duration::SHORT_4` (200 ms) from the shared token
scale in `micold-core/src/tokens/motion.rs`, with `SCRIM_ALPHA = 0.32`. T029's substance holds —
legible named `Duration`s in one place — but its wording is stale twice: the file is no longer
`main.rs`, and the exit is 200 ms, not the 240 ms the task records. The enter sits inside SC-002's
0.15–0.5 s band and is plainly visible. The exit's nominal 200 ms is also in band; it is simply
never drawn.

## What this does and does not transfer

- **Transfers**: that entrances animate and exits do not. That is a difference in *rendered frame
  count* between two transitions measured by one instrument on one machine in one session, and no
  property of lavapipe produces sixteen steps in one direction and one in the other.
- **Does not transfer**: wall-clock durations, and anything about perceived smoothness or frame
  pacing. lavapipe is a software rasteriser; the ~30 Hz app frame rate stretches every app-drawn
  duration by roughly 5/3.
- **Not covered**: rename, add-worktree and project-switcher overlays were not individually
  exercised — About and Settings were, on all three dismissal paths, and all five go through the
  same `material::Modal`. The three untested ones are expected to behave identically and should be
  re-checked when BUG-001 is fixed.
- **Not covered**: quit-mid-animation, reveal-beneath and reopen-during-exit, for the reasons above.

## Why the suite did not catch this

`tests/overlay_transition_identity.rs` asserts a closing dialog keeps a stable identity — total,
faithful, injective — so the renderer does not restart the transition.
`tests/overlay_dismissal_delta.rs` asserts which gestures dismiss which surface. **Neither asserts
that an exit renders an intermediate frame.** The suite is green and the headline feature does not
work; that gap is the reason this pass exists, and a regression test for it belongs with the fix.

---

## Postscript, same day — the exit is *truncated*, not absent

Running 022 §B2 in the showcase a few hours later sharpened this considerably, and BUG-001 has been
rewritten around it. The showcase renders faster than the client under lavapipe, so the head of an
exit survives long enough to photograph:

- The gallery poses `Fade` and `Scale` with their own **Replay** / **Reverse (play it out)**
  buttons — no overlay, no dialog, no dismissal gesture. `Fade`'s Reverse renders **6 evenly-spaced
  steps of ~4.6% alpha each — 23% of the visible range — and then jumps the remaining ~77% in one
  frame.** `Scale`'s Reverse shrinks 246 → 236 px over 6 steps and then vanishes.
- The Select list closes over 3 frames covering ~1% of its range; the Typeahead list over 3 frames
  covering ~7%.

So the exits above are not *missing*; they **start on a smooth ramp and are then cut off**, and the
client's "1 frame" is the same thing sampled at ~30 Hz. That relocates the fault from the overlay
snapshot machinery — which this note had reported as reading correctly, and which is now exonerated
— to whatever decides an exiting element is done.

**One hypothesis tested and refuted.** Because `step_for` is frame-count stepped, the natural guess
was that an exit only advances while something *else* is asking for redraws (the pressed button's
ripple) and dies when that demand stops. The `Scale` Reverse was repeated with the pointer driven
continuously across two buttons, forcing a redraw every frame: **it cut off at exactly the same
236 px, in the same number of steps.** Frame supply is not the mechanism.

**One thing seen once and not reproduced.** During that session the showcase wedged — the window
froze at a stale frame and stopped responding to hover and to scroll for at least 8 s each, while
the main thread ran at ~24% and every llvmpipe worker was hot, i.e. it was rendering flat out and
presenting nothing. It was killed and relaunched. The obvious suspect — ~120 scroll notches over the
posed `TerminalPane`, which is what the pointer was over — was tried deliberately on the fresh
instance and did **not** reproduce it. Recorded because a reader should know it happened, not as a
defect: with no trigger and no second occurrence there is nothing for a fixer to act on. Note also
that the gallery legitimately sits near 8 cores of software rasterisation at rest, because it poses
permanently-animating components (`StageProgress`'s live line, `ActivityBadge`'s spinners), so a
high idle CPU figure there is expected and is not evidence of a runaway loop.
