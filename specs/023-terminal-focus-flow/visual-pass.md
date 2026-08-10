# Visual pass — Natural Terminal Focus Flow

Run with the repo's `visual-pass` skill: Xvfb `:78` at 1600×1400, Mesa lavapipe (software Vulkan),
`xdotool` to drive and `import` to capture. An isolated instance throughout — its own
`XDG_RUNTIME_DIR` (`/tmp/vp78`), `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, seeded with one project — so
it spawned its own daemon and never touched the session already running on this machine.

Compare against [`visual-pass-baseline.md`](./visual-pass-baseline.md), recorded on `f546b60`
before any of this feature's code landed.

**Every press below is held for 350 ms.** That is not decoration: the baseline established that
`xdotool click`'s ~12 ms press/release is too fast for a frame to be rendered between them, and the
defect only exists in that gap (research R1). A pass driven with instant clicks would have been
green before the fix.

---

## §B1 — One press, one outcome (US1, SC-001, SC-002) — **PASS**

**Date**: 2026-08-10, at `c1f2b04`.

Set up exactly as the baseline: a Default-location session in the worktree, its terminal holding the
keyboard.

**The reported case.** From AI CLI mode, focused, one press on the mode toggle at (1567, 1367):

| | Title | Trailing controls |
|---|---|---|
| Before | `Claude Code` | release affordance (bright), mode toggle |
| After **one** press | the shell's prompt | release affordance (bright), "+", mode toggle |

The mode switched **on that press**, and the terminal still held the keyboard. In the baseline the
same press only released focus and left the mode alone; the mode changed on the second press.

**Typing straight afterwards.** With no click in between, `echo ONE_PRESS_THEN_TYPED` was typed and
appeared at the shell prompt with the caret after it (`v-b1-typed.png`). The keystrokes reached the
attached process, which is the half of SC-001 a bar screenshot cannot show.

## §B2 — No shape change, no blink (FR-008a, SC-007) — **PASS on the structure**

The release affordance is now always in the bar. Cropped at identical geometry
(`260x60+1400+1340`), focused vs released:

- **Focused**: keyboard glyph drawn in the full-strength role.
- **Released**: keyboard glyph **still present**, drawn muted — and the "+" and the mode toggle sit
  at **exactly the same x** as in the focused frame.

That is the whole finding. In the baseline those two controls moved 56 px left the instant focus was
released, which is what handed the pressed control its left neighbour's tree node and swallowed the
click. `v-b2-compare.png`.

**Not claimed**: that no focus ring blinks for one frame mid-press. A screenshot pipeline cannot
reliably catch a chosen frame of a transition, and this was recorded as unrun in the baseline for
the same reason. What replaces it is stronger than a photograph would have been: nothing publishes a
release during a press any more (`terminal_pane.rs`'s inline `a_press_outside_the_pane_does_not_
touch_focus`), the bar cannot change shape as a function of focus
(`tests/terminal_bar_stability.rs::the_bar_does_not_branch_on_focus`), and the two
`Task::done(Message::TerminalFocused)` re-assertions that produced the blink are deleted. There is
no code path left that could draw an intermediate holder.

## §B6 — Pressing into the terminal (FR-008b, SC-009) — **PASS on the keyboard half**

With the terminal **not** holding the keyboard (released via the affordance), one press inside the
pane at (800, 400):

- the terminal took the keyboard on that press — the affordance returned to full strength, with the
  controls again unmoved;
- `echo PRESSED_IN_THEN_TYPED` typed immediately afterwards reached the shell, appended to the
  prompt line (`v-b6.png`).

**Not shown in pixels**: that the same press is reported to a *mouse-aware* program. Driving `vim`
to prove it needs a program whose reaction to a click at a chosen cell is unambiguous on a
screenshot, and that was not reached in this run. It is covered mechanically instead, at the two
places the decision actually lives: `press_grants_focus`'s truth table and
`the_granting_press_is_routed_as_if_focused`, which asserts that the press granting focus routes as
`MouseReport` under mouse mode — the exact thing that was `HandleLocally` before, and the reason a
TUI needed a second press.

## §B1 — rapid alternation (spec Edge Cases) — not run

Left for T034's full sitting.

---

## Sections still to run

§B3 (US2), §B4 (US3), §B5 (US4) — each with its own story, plus the whole of §B in one sitting at
T034.

## Frames

In the run's scratchpad, not committed:

- `v-b1-compare.png` — the headline: one press, mode switched, focus kept
- `v-b1-typed.png` — the characters that followed with no click
- `v-b2-compare.png` — focused vs released, controls unmoved
- `v-b6.png` — the granting press and what it let through
