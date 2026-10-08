
## Pass 2026-10-08 (quickstart Part B, checks 1-6 PASS, check 7 NOT JUDGED)

Ran on headless sway + pixman + lavapipe, not a real display. Showcase built from HEAD 1db21a4e
(M1 #600, M2 #607). The pointer was a persistent `zwlr_virtual_pointer_v1` client on a private sway
(wlrctl cannot hover: its virtual pointer lives only for the command, so the app sees capabilities
appear and vanish within microseconds). Screenshots via grim; crops in `visual-pass/`.

| Check | Verdict | Evidence |
|---|---|---|
| 1 Follow | PASS | [c1](visual-pass/c1.png): panel ~16 px right/below the pointer, moved 150,470 -> 250,500, unchanged after a 1.5 s rest (pixel-identical) |
| 2 Edge flip | PASS | [c2](visual-pass/c2.png) bottom edge: panel above pointer; [c2r](visual-pass/c2r.png) 520 px floating window: right edge -> panel left of pointer, corner -> left and above |
| 3 Leave | PASS | [c3](visual-pass/c3.png): panel gone after the pointer left |
| 4 Delay | PASS | [c4](visual-pass/c4.png): none at 0.45 s, shown at 1.65 s, leave at 0.4 s shows none; [c4b](visual-pass/c4b.png): jiggle until 0.75 s, none at 0.96 s, shown by 1.19 s (a restarting timer would show at >= 1.75 s) |
| 5 Delay + follow | PASS | [c5](visual-pass/c5.png): entered at 700,400, moved to 900,470 during the wait; panel appeared beside 900,470 and tracked to 1000,430 |
| 6 Existing tooltips | PASS | [c6a](visual-pass/c6a.png) below/left/above/right and multi-line; [c6b](visual-pass/c6b.png) 3 s delay: none at 1.2 s with jitter, shown after ~3.6 s rest |
| 7 Idle CPU | NOT JUDGED | The showcase idles at ~170% CPU and ~60 wl_surface commits/s with no tooltip open (page top and tooltip section). Source not attributed, no pre-430 baseline built; likely a showcase animation or the software compositor. SC-005 is not claimed from this pass; it stays covered by the `idle_requests_no_frames` test. |

Not judgeable by screenshots: timing precision (+-100 ms), smoothness of the follow, real-GPU and
idle-power behaviour, and the pointer on the cursor image (grim does not draw it).

## Pass 2, 2026-10-08 (steps 4, 5 and 7 PASS)

Headless sway + pixman + lavapipe, not a real display. HEAD b905b1c3 (M1 #600, M2 #607), pointer by
the committed `vptr.py`, screenshots by grim.

| Step | Verdict | Evidence |
|---|---|---|
| 4 Delay | PASS | [pass2-c4](visual-pass/pass2-c4.png): no panel at 0.95 s (only the 56 px hover state layer), panel at 1.87 s. Leaving at 0.3 s showed nothing (diff 0). Jiggling inside did not restart the wait: absent at ~0.85 s, present at ~1.4 s (a restarting timer would show it at >= 1.7 s). |
| 5 Delay + follow | PASS | [pass2-c5](visual-pass/pass2-c5.png): entered 700,580, moved to 900,650 during the wait; at 1.5 s the panel is beside 900,650 (none before: diff 0), then followed to 1000,620. |
| 7 Idle | PASS relative to a pre-#430 baseline | Table below; raw output in [pass2-idle-results.txt](visual-pass/pass2-idle-results.txt). |

Step 7 method: baseline = commit 47ae6297 (pre-#430), built from `git archive` in the shared target dir
under the build lock. 30 s samples after a 10 s settle, no hover, same scroll offset from the page
bottom, 1600x1200 window. CPU % = utime+stime of the process over 30 s; commits from
`WAYLAND_DEBUG=client`.

| Build | CPU % (4 samples) | Mean | wl_surface commits/s |
|---|---|---|---|
| Baseline 47ae6297 | 159.7, 178.6, 171.5, 170.6 | 170.1 | 60.2, 60.2 |
| Current | 185.3, 189.8, 166.9, 177.2 | 179.8 | 60.4, 60.3 |

+5.7% against a baseline run-to-run spread of ~11%: within noise.

Caveats:
- The showcase redraws continuously at ~60 Hz in BOTH builds with nothing open. This shows the feature
  adds no idle work; it is not true idleness. The real-client idle claim (SC-005) stays covered by
  `idle_requests_no_frames`.
- The two builds' pages differ in content (more tooltip poses in current): ~7% of pixels differ at the
  same offset from the bottom. Same-build runs are pixel-identical.
- Not judged: timing beyond +-100 ms, smoothness, the cursor image, real-GPU power. lavapipe/pixman CPU
  says nothing about a real GPU.

## Earlier attempts (history)

### M1 (steps 1-3, 6)

Date 2026-10-05. NOT RUN (blocked, no verdict). The showcase built from this worktree
(strings check: "follows the pointer" present), but this machine has no Xvfb, no xdotool
and no xwininfo, so no private X display could be started and no pointer could be driven.
Steps 1, 2, 3 and 6 are unverified. Needs `Xvfb` and `xdotool` installed, then rerun.

### M2 (steps 4, 5, 7)

Date 2026-10-06. NOT RUN (blocked, no verdict). Same cause as M1: no Xvfb, xdotool or xwininfo on
this machine. Steps 4, 5 and 7 (the "show delay" and "show delay and follow" poses) are
unverified. Needs `Xvfb` and `xdotool` installed, then rerun.
