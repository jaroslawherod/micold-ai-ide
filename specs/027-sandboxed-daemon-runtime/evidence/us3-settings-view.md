# US3 verification: Settings as a full-surface view

**Date**: 2026-08-25 · **Runtime**: Xvfb `:92` (1600×1400) + Mesa lavapipe, **not a real display**
· **Binaries**: `micold-ai-ide` + `micold-daemon` built from `feat/run-daemon-inside-an-container-sandbox`
at d350e69 and pinned to `~/vp/bin-027s/` so no other worktree's build could be under test
**Covers**: quickstart.md §B.6

Run with the repo's `visual-pass` skill. Everything below was driven with `xdotool` against that
server; nothing was inspected on the user's own display.

## What passed

**Full-surface view with a rail, not a 420-point modal (FR-026).** Settings opens over the whole
window with a four-item navigation rail on the left — Appearance, Terminal, Environment, Session
service — and its own action bar. See `us3-themes-and-width.png`.

**Every daemon setting is in the Daemon section (FR-027).** Session service holds *Where sessions
run*, *Container runtime*, *Image source*, *Image reference*, *Image file*, the four credential
opt-ins, and *Keep sessions running after I sign out*. Nothing daemon-shaped appears under
Appearance, Terminal or Environment. The overflow menu still offers *Keep sessions after logout*
and *Session service diagnostics*, which are shortcuts to the same setting and the same
diagnostics view rather than settings kept elsewhere — the same relationship the app bar's theme
button already has to Appearance, which the section's own helper text describes.

**Every pre-existing setting still exists and still works (FR-028).** Appearance keeps the theme
picker, Terminal keeps *Scrollback lines*, Environment keeps the source-a-script toggle with its
script path and timeout. Verified as a round trip rather than by sight: scrollback edited from
10000 to 12000, Save, reopen — the field reads 12000 and
`$XDG_DATA_HOME/micold-ai-ide/settings.json` has `"scrollback_lines": 12000`.

**Active credential opt-ins are each individually visible (FR-004c, N-2).** Git configuration, SSH
agent and Git credentials each show their own checked box, and the summary line under them names
exactly those three.

**Keyboard navigation reaches every section and every control; focus order is sane.** Tab walks
Appearance → Terminal → Environment → Session service → the page's own controls in reading order →
Cancel → Save, and each one is visibly marked: an outline ring on a rail row and on a button, a
thickened accent indicator on a select, a state-layer disc on a checkbox. See
`us3-keyboard-focus.png`.

This is what T075a/T075b fixed. On the first attempt at this pass, eight Tab presses on Appearance
changed zero pixels — `Button` and `Select` were not focusable at all in this rendering stack, so
the theme picker, every rail row, Cancel and Save were unreachable by keyboard.

**The focused control is visible (FR-030, second clause).** T075c. With the window at 900×600 the
Session service page scrolls, and each Tab now brings the newly focused control into the panel with
a margin rather than focusing it off-screen. `us3-scroll-into-view.png` is three consecutive Tab
presses: *Image reference*, then *Git configuration*, then *Keep sessions running after I sign out*,
each one visible at the moment it takes the keyboard.

**Both themes.** Dark renders the whole view correctly, and the focus ring is if anything clearer
against it (light outline on a dark row). See `us3-themes-and-width.png`, dark on the left.

## What passed with a caveat

**No truncated labels at the narrowest supported width.** No minimum window width is declared
anywhere in the spec or set on the window, so this was measured rather than checked against a
number. At 1000, 800 and 640 points wide nothing truncates — labels and helper text wrap. Below
that the rail keeps its fixed width and the page is squeezed:

- **600**: the *Image reference* value clips at the right edge (the field scrolls its own text, so
  this is the field behaving as a field, but the value is no longer readable at a glance)
- **560**: *Keep sessions running after I sign out* runs past the right edge
- **480**: several checkbox labels are cut mid-word and Save collapses to an unlabelled circle

So: clean to 640, degrading below it. Whether 640 is the floor is a decision nobody has recorded.

## What was not answered

**Idle with the view open: no repainting.** Inconclusive here, by the skill's own warning — lavapipe
rasterises every frame on the CPU, so this machine's numbers say nothing about the user's. What
*can* be said is that the settings view is not a regression: over 20-second samples the process used
157 ticks with the view open and idle, and 151 with it closed on the main surface. Identical within
noise, so the view adds no idle cost of its own. The absolute claim rests on
`crates/micold-client/tests/idle_requests_no_frames.rs`, which passes.

**Mid-flight animation and perceived smoothness.** Out of reach of a screenshot pipeline on a
software rasteriser; not attempted.

## Known gap, outside this surface

`IconButton` is still not focusable, so the app bar's own controls — *Select project* and the
overflow — cannot be reached by keyboard. FR-030 is about the settings surface, and every control
on it is reachable; the app bar is not, and no task covers it.

## Observation

Toggling the theme from the overflow menu **while the Settings view is open** and then pressing Save
reverts the theme: the draft the view is holding still says what it said when the view opened, and
Save writes the draft. Reproduced twice. Not in scope for any FR here, and not filed.
