
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
