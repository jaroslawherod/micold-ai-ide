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

## §B3 — Away and back (US2, SC-003) + the undisturbed process (FR-025) — **PASS**

**Date**: 2026-08-10, at `f777128`.

OS focus was moved off the application and back with a second X client (`xterm`), since `:78` has no
window manager and nothing else sets input focus.

**1. Typing resumes with no click.** Focused terminal → focus to the xterm → focus back → typed. The
characters appeared at the shell prompt (`v-b3-final.png`). No click in between.

**2. A release survives the round trip.** Terminal released via the affordance → focus to the xterm
→ focus back → typed `MUST_NOT_REACH_THE_SHELL`. The prompt line stayed **empty**
(`v-b3-released-rt.png`); not one character reached the process, and the release affordance was
still drawn muted in the bar with its siblings unmoved. Returning to a window is not a request to
undo a decision the user made before leaving.

**3. A dialog field survives.** Not driven here — covered by
`a_field_still_holds_the_keyboard_after_a_window_round_trip`, which asserts `focused_field` is
unchanged across the round trip and that the terminal has not taken the keyboard back.

**4. The process is undisturbed (FR-025).** `yes | nl | head -100000` was left printing while focus
was released and re-acquired **six** times and the window focus round trip above was performed. The
line numbers ran from 43869 to **100000** — consecutive, no gap, no restart from 1, no reflow
(`v-b3-running.png` → `v-b3-final.png`). Focus is not something the attached process can notice.

## §B4 — Landing ready to type (US3, SC-004) — **PASS**

**Date**: 2026-08-10, at `e6ca347` + US3.

From a **released** terminal — affordance drawn muted, confirmed in `w-b4-released.png` — the mode
was toggled twice (to AI CLI and back to Regular) and `echo NAVIGATED_THEN_TYPED` typed immediately.
It appeared at the shell prompt with the caret after it (`w-b4-typed.png`).

Zero presses into the pane. Before this feature the release would have outlived the navigation and
those keystrokes would have gone to the application: navigating to a terminal is a request for it,
so it clears the release (FR-011, FR-021a).

Session start and select were exercised incidentally throughout the run — every session here was
started from the sidebar's "+", and each came up focused. Project switch and relaunch are covered by
`a_switch_lands_on_a_terminal_ready_to_type` and `a_restored_session_holds_the_keyboard_at_launch`;
this instance has one project, so a switch is not drivable in it.

## §B5 — Never taken mid-word (US4, SC-005, SC-006) — **PASS**

`yes | nl | head -200000` was left flooding the pane throughout, so every claim below was made while
the terminal had output arriving.

**A surface takes the keyboard while it is open.** Opening the application's overflow menu
(top-right) with the terminal focused: the release affordance in the bar went **muted** in the same
frame the menu appeared (`w-b5-menu.png`). The terminal had yielded — which is the registry doing
the work, since nothing in this feature names that menu.

**And gives it back.** Escape closed the menu; the affordance returned to full strength
(`w-b5-closed-bar.png`) and `echo BACK_AFTER_MENU` typed straight afterwards reached the shell
(`w-b5-final.png`). No restore stack — the predicate simply reads true again (FR-010).

**Output changed nothing.** The flood ran to 200000 unbroken across the menu opening, the Escape,
and the typing. It never moved the keyboard and was never disturbed by it (FR-019, FR-025).

**Not driven here**: typing into the Add Worktree form specifically. This project has no worktrees,
so the form was not reachable without creating one. The claim it stands for — that a text field
keeps the keyboard while a background session produces output and reaches `Running` — is asserted
directly by `output_and_lifecycle_never_change_the_holder`, which checks `focused_field` is
untouched across exactly those events.

## §B1 — rapid alternation (spec Edge Cases) — **PASS**

Six alternating 250 ms presses — release affordance, into the pane, release affordance, into the
pane, … — ending on a press **into** the pane. `echo SETTLED_ON_LAST_PRESS` typed straight
afterwards reached the shell (`w-alt.png`), and the affordance was drawn bright.

The state settled on exactly what the last press asked for. Nothing lingered from an earlier one,
and nothing was restored behind the user's back — which is what the derived holder buys: there is no
saved state to come back, and after T015 no follow-up message that could arrive late.

## Final verification (T033/T034)

`mise run test` — 184 test targets, all green. `cargo fmt --check` clean, `clippy -D warnings`
clean. CI covers Linux, macOS and Windows; no `cfg(target_os)` was added by this feature, so a
platform-only failure would be a real finding rather than an expected difference.

Every §B section has now run against a build containing the whole feature.

---

## Sections still to run

All six sections have now run at least once. T034 repeats them in one sitting against the final
build, and adds the rapid-alternation sequence.

## Frames

In the run's scratchpad, not committed:

- `v-b1-compare.png` — the headline: one press, mode switched, focus kept
- `v-b1-typed.png` — the characters that followed with no click
- `v-b2-compare.png` — focused vs released, controls unmoved
- `v-b6.png` — the granting press and what it let through
- `v-b3-final.png` — 100000 unbroken lines, and typing after the window round trip
- `v-b3-released-rt.png` — the empty prompt after a released terminal came back
- `w-b4-released.png` / `w-b4-typed.png` — released, navigated, typed with no press
- `w-b5-menu.png` / `w-b5-closed-bar.png` / `w-b5-final.png` — the keyboard lent to a menu and returned

---

## 2026-08-14 — §B3 re-run after BUG-001 removed the release affordance

**Ran on**: Xvfb `:77` (1600×1400) + lavapipe (Mesa's software Vulkan), not a physical display.
Client and daemon both `debug`, built from `fix/small-visual-improvements` and **copied out of
`target-shared/` before launching** — see the hazard note below. Isolated `XDG_DATA_HOME` and a
private `XDG_RUNTIME_DIR=/tmp/vp77`; the user's own app, daemon and project catalog were untouched
throughout, and only processes whose `XDG_RUNTIME_DIR` read `/tmp/vp77` were ever stopped.

**Why re-run**: `012-multiple-regular-terminals` BUG-001 removes the bottom bar's release-focus
affordance (FR-021b). That leaves the reserved `Ctrl+Shift+E` chord as the only explicit release, so
the question is whether the release path still works when the button that used to sit beside it is
gone — and whether removing a bar child reintroduced the swallowed press FR-008a exists to prevent.

### Passed

- **Press-to-focus, then type.** Pressed the pane, typed `echo VP_FOCUSED_OK` — it reached the shell
  and ran. The granting press still both takes the keyboard and lets the keys through (FR-008b).
- **The chord still releases (FR-021, 006 FR-011).** Pressed `Ctrl+Shift+E`, then typed
  `echo VP_SHOULD_NOT_APPEAR`. **Nothing reached the process** — the pane was byte-for-byte
  unchanged, still showing the previous prompt. With the affordance gone, the chord alone carries
  the "never trapped" guarantee, and it does.
- **Re-acquisition.** Pressed the pane again, typed `echo VP_REACQUIRED` — it ran. The full
  release → re-acquire cycle works with one control fewer in the bar.
- **The process was undisturbed across the cycle (FR-025).** Scrollback stayed continuous over all
  three steps: no gap, no restart, no reflow. Focus is still not something the shell notices.
- **One press does what you pressed (FR-008a, the regression this risked).** Every control pressed
  during the session acted on its *first* press while the terminal held the keyboard: the mode
  toggle, the "+" that opens an instance, and a switcher tab. Removing the affordance
  unconditionally kept the bar's child list focus-independent, exactly as
  `terminal_bar_stability.rs` requires — and that gate plus its new sibling
  `the_bar_has_no_release_focus_control` now pass together, which only an unconditional removal can
  do.
- **The bar no longer shows the control.** Confirmed by eye in both modes: in AI CLI mode the bar is
  session name · status · mode toggle; in Regular mode it gains the switcher and the "+". No
  keyboard-hide glyph in either.

### Not run — and why

- **B3.1 / B3.3, the window round trips** (switch to another application and back; a dialog field
  keeping its caret). There is no window manager on `:77` and no second application: `xterm` was
  installed but would not start on the private display, so nothing could take X input focus away
  from the app and give it back. These paths are untouched by BUG-001 — the removed button plays no
  part in window focus or field focus — but untouched is not tested, and they are recorded here as
  unrun rather than passed.
- **Mid-flight animation and perceived smoothness**, as always with this harness.

### Hazard worth carrying forward

The first bar screenshot of this run still showed the release-focus button, which looked like the
change having failed. It had not: `ui/terminal.rs` contains no `Icon::ReleaseFocus` and its gate
passes. The binary was another worktree's. `target-shared/` is shared by every checkout on this
machine (CLAUDE.md), so `target-shared/debug/micold-ai-ide` is whatever branch built last, and four
other worktrees were building throughout. A second trap sits behind it: the client refuses to talk to
a daemon whose protocol **schema hash** differs (`handshake::evaluate`), and the log reports that as
"contract or build mismatch" while printing matching v5/0.8.0 versions on both sides — so a client
and daemon picked up from `target-shared/` at different moments will not connect, and the message
does not say why. Build both in one invocation and copy them aside; run the copies. The skill has
been updated with this.
