# Visual pass M3 (feature 1792): quickstart Part B steps 2, 3, 5, 6 and decision D1

Date 2026-11-14. Branch claude/project-thread-q9zjol, head f11b1b9df. Release builds of `micold-ai-ide` and `micold-daemon` built from this tree and pinned out of `target-shared/`. Run on Xvfb :88 (1400x900) in a cloud container, tiny-skia renderer (no Vulkan ICD here), not a real display. Window is 1024x768, no window manager, dark/light not varied (light scheme only). Input by Xlib XTest (button held ~100 ms); frames by `root.get_image` of the composer region about every 22 ms (`+-` a frame; timing is wall clock and approximate). The "claude" CLI is a 20-line stand-in script first on PATH that emits the recorded `init` line and answers `initialize` with the recorded model/command list (`crates/micold-core/tests/fixtures/stream-json/`). Fixture repo `/tmp/vprepo`, private data dir. The session was made structured through the top-bar menu "Structured". Steps 1, 4 and 7 were not in scope.

Measure: the y of the topmost non-background pixel of the options row, left 210 px of the composer, per frame. Row fully open: y=548 (window y), closed: no pixels.

| Step | Result |
|---|---|
| 2 Expand and collapse | PASS |
| 3 Toggle twice quickly | PASS (visual, coarse) |
| 5 Reduce motion on | FAIL |
| 6 Two sessions | PASS for animation, with an observation (see below) |
| D1 `/model` with row collapsed | PASS |

## Step 2: PASS

Press of the settings icon: the row grew from 0 to full height in 8 to 13 frames after the release, top edge y: 622, 600, 589, 579, 572, 564, 560, 557, 554, 551, 550, 549, 548 at roughly +0.29 s to +0.55 s, i.e. about 270-300 ms, decelerating (large steps first). SC-002 / `MEDIUM_2` = 300 ms: consistent within frame timing. The first frame with any row pixel is 0.29 s after press start (press held 0.1 s), so the start is within one frame of the release. Content is clipped, not squashed: the Model/Effort row shows first, the Permissions row is cut at the bottom while it grows.

Collapse (press again): shrinks continuously over about 0.3 s (frames 0.29 s to 0.57 s after press start) to nothing, content clipped from the top with the controls sliding down, no jump.

Images: `visual-pass-M3-expand-frames.png` (closed, then four growing frames, left to right at +0.29, +0.33, +0.42, +0.55 s), `visual-pass-M3-collapse-frames.png` (full, then eight shrinking frames to empty).

## Step 3: PASS (visual, coarse)

Opened, then pressed again about 0.1 s after the first release (mid-growth: both toggles inside the 300 ms). The row grew to about two thirds, then shrank continuously from that height to nothing with no snap to full height or to zero (12 frames, `visual-pass-M3-reversal-frames.png`, left to right, top row then bottom row). Limit: my per-frame height measure plateaued while the Model row slid, so I judged the reversal by looking at the frames, not by a height trace; a single-frame jump smaller than the frame spacing is not excluded.

## Step 5: FAIL

Setting Settings > Appearance > Reduce motion on (saved; `settings.json` shows `"reduce_motion": true`; also repeated after restarting the client so the value was loaded at boot) did not make the row switch in one step.

- Expected (FR-013): expand and collapse in one frame.
- Seen: expand with reduce motion on still grew over about 0.26 s (top edge y: 626, 610, 592, 581, 574, 567, 562, 557, 554, 552, 550, 549, 548), the same curve as step 2, three runs, plus once after a client restart. Collapse with reduce motion on, in the first run, held full height and vanished in one frame about 0.3 s after the press, but after the restart it shrank over about 0.1 s (548, 550, 554, 561, 569 then gone), so it is not reliably instant either.
- The source builds `Duration::ZERO` when `state.settings.reduce_motion` (`crates/micold-client/src/ui/composer.rs`, `options_motion`), so the value reaching the animation, or the animation's handling of a zero duration, is the place to look; I did not debug it.
- Also not an error but worth a look: the Reduce motion setting's own description in the Settings page still names only the Overview ("Switch the Overview's full-screen zoom, card focus and expansion in one step").

Image: `visual-pass-M3-reduce-motion-expand-frames.png` (reduce motion on, expand, frames at +0.13, +0.17, +0.22, +0.28, +0.39 s: still a gradual grow).

## Step 6: PASS for the animation, with an observation

Two structured sessions (the second made structured through the same menu; the daemon asks "Start beside a running session?" when the other tab is selected, answered Start). I collapsed and expanded the row in one session, selected the other tab, and recorded frames around the switch: the other session's composer appeared at its resting state (the same open or closed state as the one just left) with no growth or shrink animation after the switch, and no animation ran in the session that was not showing.

Observation (not a visual defect, and consistent with the ledger's F4 note): the open or closed state is one window-wide setting (`composer_options_open`), so expanding the row in one session also shows it expanded in the other once selected. If FR-008 is read as "the other session's row stays collapsed" it does not hold here; if it is read as "separate animation tracks" nothing contradicted it. The two composers could not be seen side by side.

## D1: PASS

With the row collapsed, `/model` typed in the composer opened the slash-command popup ("/model, Set the AI model for Claude Code"); Enter chose it, and a second Enter sent it. The row expanded (`composer_options_open` turned true) and the model list opened above the Model trigger with its filter field and the entries Default, Sonnet, Fable, Opus, Haiku, selected row marked. The view was not clipped. The first screenshots after typing show the sidebar darkening and a blank white box above the composer: that is a frame-damage artefact of this software renderer (it is not there once redrawn), I did not count it.

Image: `visual-pass-M3-d1-model-list.png` (cropped).

## Not covered

Step 1 and 4 (M1), step 7 (narrow window, light and dark) and the focus and pressed states. The recorded frames are not committed (only the strips above); frame times are wall-clock samples with about 22 ms spacing and a software renderer, so they say nothing about smoothness on a real GPU.

## Step 5 re-check (unit, after the pass): PASS, the failure was a mutated binary

Root cause: the release client the pass ran was built while review B round 1's mutation check
had `options_motion` mutated to `if false && state.settings.reduce_motion` (recorded in
`tdd/cycle-log.md`, *Review B round 1 F2*). The binary is stamped 21:36:03, `ui/composer.rs` was
restored by `git checkout` at 21:36:15, and the mutation commit landed at 21:36:42: the binary
carried the mutant, which ignores Reduce motion. The committed code has no fault.

Evidence: the same binary reproduced the grow (top edge 624, 606, 591 ... 548 over about 0.25 s,
`reduce_motion: true` loaded at boot). A release client rebuilt from the committed tree, same data
dir and flow (Xvfb, session made structured from the top-bar menu, settings button pressed), with
a temporary trace in `Expand::update`: 14 toggles, every one logged `over=0ns` and a single step
`0 -> 1` or `1 -> 0` on the first frame; the frame trace showed the row at its end height on the
first changed frame. Rebuilt again without the trace: 4 toggles, each in one frame. A visual
re-check of step 5 by the orchestrator is still due.

Follow-up for the process: a mutation check and a build for a visual pass must not share a
worktree at the same time.
