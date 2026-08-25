# 006 T042 — the deferred GUI pass, partly run

> **Superseded 2026-08-25** by [`gui-pass-2026-08-25.md`](./gui-pass-2026-08-25.md), which ran
> the remaining steps and closed T042. The results below stand — this file is the record of
> what steps 1 and 9's validation clause showed on 2026-08-20 and is kept unaltered.

**Date**: 2026-08-20
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own client + daemon, one invocation, pinned inside the build lock.

T042 was deferred as "needs a manual GUI run on a display". Part of it does not: rendering and
validation are answerable from a frame. Steps that turn on *perceived* latency or on a real GPU are
still out of reach and are listed as unrun rather than passed.

## Step 1 — coloured, faithful rendering (SC-002) — **PASS**

A session switched to **Regular Terminal** from the status bar's mode control (its tooltip reads
"AI CLI — switch to Regular Terminal"), then a single `printf` exercising every attribute the step
names. `step1-ansi-dark.png`:

| Attribute | Observed |
|---|---|
| basic ANSI fg | `RED` in red |
| bold + colour | `BOLDGREEN` heavier and green |
| italic + colour | `ITALICBLUE` in blue |
| underline | `UNDER` underlined |
| reverse | `REV` drawn as dark-on-cyan — fg/bg swapped |
| 256-colour | `256-ORANGE` in xterm 208 |
| truecolor | `TRUECOLOR-PINK` in `#FF69B4` |
| cursor | visible block at the prompt |

The shell's own coloured prompt renders correctly beside it.

**Theme clause — PASS.** Cycling the app theme from the overflow menu to light
(`step1-ansi-light.png`) swapped the terminal's default foreground and background with the app,
while **every ANSI colour above is unchanged** — same hues, same reverse-video pairing, cursor now
dark-on-light. That is exactly the split the step asks for: defaults follow the scheme, palette
does not.

The setting persisted: `settings.json` carries `"theme": "light"` afterwards.

## Step 9 — configurable scrollback (SC-007) — **validation clause PASS**

Overflow → Settings shows `Scrollback lines 10000` ("Lines kept per terminal"), alongside feature
011's environment-include block. Replacing it with `0` and pressing Save produced a clear inline
message in the error colour — **"Enter a number between 100 and 1000000."** — and the dialog stayed
open. `step9-validation.png`.

Nothing was written: `settings.json` still reads `"scrollback_lines": 10000`. The out-of-range value
was rejected, not clamped and not persisted.

The other half of step 9 — that a session started *after* a valid change honours the new limit, and
that the value survives a relaunch — was not run.

## Not run

| Step | Why |
|---|---|
| 2 — focus gate | not attempted this run |
| 3 — interactive `claude` (arrows, Tab autocomplete, multi-line, Ctrl+C) | not attempted this run |
| 4 — Ctrl+Shift+E releases focus | not attempted this run |
| 5 — copy/paste chords, middle-click, right-click menu | not attempted this run |
| 6 — resize reflow, scrollback scrolling | not attempted this run |
| 7 — input discarded while not Running | not attempted this run |
| 8 — responsiveness under flood (SC-008) | **out of reach here.** The claim is "≤~100 ms perceived" on a real GPU; lavapipe is a software rasteriser and its frame pacing says nothing about a user's machine. Measuring it here would produce a number that means nothing. |
| 10 — BUG-001 auto-focus / BUG-002 scrolling | already verified by the user on Wayland/GNOME, recorded in T042's own note |

So T042 stays open. What this run removes from it is steps 1 and the validation half of 9 —
including the theme clause, which was the one most likely to have rotted quietly across 018's
visual-system rework.
