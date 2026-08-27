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

**Idle with the view open: no repainting.** Left open here, on the grounds that lavapipe's numbers
say nothing about the user's GPU. Settled on the third pass by measuring a different process — see
*Idle with the view open* at the end of this file.

**Mid-flight animation and perceived smoothness.** Out of reach of a screenshot pipeline on a
software rasteriser; not attempted.

## Known gap, outside this surface

`IconButton` is still not focusable, so the app bar's own controls — *Select project* and the
overflow — cannot be reached by keyboard. FR-030 is about the settings surface, and every control
on it is reachable; the app bar is not, and no task covers it.

## Observation, since filed and fixed

Toggling the theme from the overflow menu **while the Settings view is open** and then pressing Save
reverts the theme: the draft the view is holding still says what it said when the view opened, and
Save writes the draft. Reproduced twice.

Recorded here as out of scope for any FR, which was the wrong call: it *is* a regression from this
feature. The two writers were harmless while Settings was a modal covering the app bar, and FR-026
is what put them both on screen at once. Filed 2026-08-27 as `bugs/BUG-001.md` and fixed there —
`features/settings.rs::apply_theme` carries an app-bar choice into an open draft, gated by
`crates/micold-client/tests/settings_draft_tracks_the_live_theme.rs`.

## The rail's icons and its collapsed state (§B.6, second pass)

Run 2026-08-27 on Xvfb `:78` at 1600×1400 with Mesa's lavapipe, not on a real display, from binaries
pinned in `~/vp/bin27` rather than launched out of the shared target directory. Dark scheme
throughout: the four boxes below are about placement and glyph shape, and `style_snapshot` owns
colour in both schemes.

**Every section carries an icon, and the icons are distinguishable at the rail's own size
(FR-026b).** Sun (Appearance), a prompt inside a window (Terminal), a sparkle (Environment), stacked
racks (Session service). At 5× magnification each is a different silhouette; none overlaps its own
label, which is the collision the T075 pass found elsewhere in this application.

**Collapsed, the rail is its icons (FR-026c).** 288dp → 80dp, labels gone, the current section still
drawn as a filled pill, and the width goes to the section: the Theme field's leading edge moves from
312dp to 104dp in the same window. The fixture records both, as `settings-view-with-validation-
error` and the new `settings-view-rail-collapsed`.

**…and it still navigates, by pointer and by keyboard.** Pointer: pressing the fourth icon opens
Session service. Keyboard: Tab walks Appearance → Terminal → Environment → Session service → the
collapse control → the page → Cancel → Save and back round, each rail row taking a visible outline
ring while collapsed (`us3-rail-collapsed-focus.png`); Enter on the focused Environment row switched
the page and left the rail collapsed.

**The collapsed state survives (FR-026d).** Collapse, Cancel, reopen — collapsed. Collapse, Save,
reopen — collapsed. It is `State`, not `SettingsDraft`, so neither action has an opinion about it,
and nothing is written to `settings.json`: a fresh process opens expanded.

**The overflow menu offers no control a section owns (FR-026e).** Three items: Settings, Session
service diagnostics, About. No theme, no session survival.

**Session survival still works from its section, under both placements (FR-014d).** With "On this
computer" the checkbox reads *"Your systemd user manager keeps the service running after you sign
out."*; switching to "In a container" changes it to *"The container is created with a restart
policy, so this takes effect the next time the sandbox starts."* — the same control, saying what the
placement it is configured for will actually do.

### What this pass found

**The current row's icon sat 12dp right of the other three.** `SectionList` draws the current row
`Filled` and the rest `Text`, and the two variants are inset by different padding. Expanded, the
labels are what the eye tracks and the pill's boundary explains the extra inset. Collapsed, a row is
its glyph and nothing else, and the rail stopped reading as a column —
`us3-rail-icons-align.png`, before on the left, after on the right.

Every gate was green over it, and the geometry fixture had *recorded* it: a snapshot compares
against what it was shown. The fix is to centre a row's content when the row is nothing but an
icon — padding is symmetric, so both variants then land on the same axis. The assertion is
`gates/rail_icons_align.rs`, which fails on the old geometry naming the drift ("row 1 at 27.0 … the
axis at 39.0").

### What this pass could not answer

Nothing new. Frame *pacing* stays out of reach — a software rasteriser says nothing about how
smooth this looks on the user's GPU. The idle-repaint box was still open after this pass; the
section below closes it.

## Idle with the view open (§B.6, third pass)

**2026-08-27, Xvfb `:78` at 1600×1400 with lavapipe** — the same rig as the second pass.

The two earlier passes both declined this box for the same reason: lavapipe rasterises every frame
on the CPU, so a cost measured here is not a cost the user pays. That reasoning answers *"how
expensive is a frame?"*, which is not what the box asks. The box asks *"are there any frames?"* —
and for that question a software rasteriser is an **advantage**, because a frame that is presented
has to be composited and blitted to the X server, and the X server is a separate process whose CPU
can be read on its own.

So the instrument is `/proc/<pid>/stat` fields 14+15 (utime+stime, `CLK_TCK=100`) sampled for the
**client and for `Xvfb` together**, over the same window:

| 20s window | client | Xvfb |
|---|---|---|
| Main surface, idle, pointer parked | 101 ticks | **6 ticks** |
| Settings open, idle, pointer parked | 133 ticks | **6 ticks** |
| Settings open, pointer moved continuously (control) | 13045 ticks | **741 ticks** |

The third row is the part that makes the first two mean anything. Without it, "6 ticks" is a number
with nothing to compare it to and the reading would be an assumption. With it, the instrument is
shown to register real repainting at **123× the idle figure** — so 6 ticks is not the floor of a
blunt instrument, it is the absence of the thing being measured. A 30-second capture-compare over
the idle view agrees independently: `compare -metric AE` reports **0 pixels differ** between the
first and last frame.

The X server therefore does *identical* work whether the settings view is open or the main surface
is — and two orders of magnitude more the moment something actually repaints. **The settings view
presents no frames at rest.** That is the box, and it is now closed on a measurement rather than on
the automated test alone.

The residual 32 ticks (0.32s of CPU over 20s, ~1.6% of a core) the client spends with Settings open
and not with it is the client's *own* wakeups — the reconnect subscription and the OS-theme probe
clock, both of which tick without producing a frame. Xvfb seeing nothing is what distinguishes a
wakeup from a repaint; that distinction is the whole reason both processes are sampled.

`crates/micold-client/tests/idle_requests_no_frames.rs` still asserts the same claim structurally,
and still passes. It is now corroborated rather than solely relied upon.

### What this still does not establish

Frame pacing and perceived smoothness on a real GPU — unchanged, and out of reach of this rig.
