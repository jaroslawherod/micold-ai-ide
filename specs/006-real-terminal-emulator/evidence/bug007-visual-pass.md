# BUG-007 (A3) visual pass — T080

Date: 2026-09-16. Branch `fix/black-on-black`, worktree
`.claude/worktrees/fix-black-on-black`. Ran on **Xvfb + lavapipe** (software Vulkan
rasteriser), not a real display — see caveats at the bottom.

## Setup

- Built `micold-client` (`micold-ai-ide` bin) and `micold-daemon` from this branch in one
  `cargo build` invocation through `scripts/build-lock.sh` (waited briefly behind
  `feat-links-in-terminal-should-be-clickable`'s build, then ran; ~11s once the lock was free).
  Compiler log named all three crates: `micold-core`, `micold-client`, `micold-daemon`.
- Copied both binaries to a private pin dir (`~/vp/bin`), verified they are a matched pair by
  grepping both for the new wire type: `TerminalColorScheme` present in both (client: 1 hit,
  daemon: 2 hits).
- Private X server: `Xvfb :77 -screen 0 1600x1400x24 -nolisten tcp`.
- Private runtime/data dirs: `XDG_RUNTIME_DIR=/tmp/vp77/rt`, `XDG_DATA_HOME=/tmp/vp77/data`,
  `XDG_CONFIG_HOME=/tmp/vp77/config`.
- Launched with `env -u WAYLAND_DISPLAY DISPLAY=:77 WGPU_BACKEND=vulkan
  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`.
- Seeded `/tmp/vp77/data/micold-ai-ide/projects.json` with this worktree as the one known
  project (the client binary has no CLI to open a project headlessly), then restarted the
  client so the daemon's `Catalog` picked it up. Confirmed in the daemon log:
  `client attached to daemon client_build=micold-ai-ide/0.15.0` and
  `project attached client=1 project=.../fix-black-on-black`.
- Cleaned up afterwards by matching process name + `XDG_RUNTIME_DIR=/tmp/vp77/rt` in
  `/proc/<pid>/environ` before killing (never `pkill -f`); did not touch the unrelated
  `micold-ai-ide`/`micold-daemon` pair already running under the user's own runtime dir.

## Check 1 — a Regular terminal answers OSC 11 with the window's scheme

Command run inside a plain shell pane (opened via the pane toolbar's "Open a new terminal
instance", Ctrl+Shift+T):

```
stty raw -echo; printf '\033]11;?\007'; dd bs=1 count=24 2>/dev/null | tr '\033\007' EB; stty sane
```

- **Dark scheme (app's default/current theme at the time):** printed
  `E]11;rgb:1414/1313/1616B` — i.e. `rgb:1414/1313/1616`. **Matches expected exactly.**
  Screenshot: `bug007-osc11-dark.png` (full frame; the answer is on the third terminal line).
- Switched the app to light via **Settings → Appearance → Theme → Light → Save** (reached
  through the ⋮ menu at the top right of the window, not the project-picker ⋮ — that menu only
  has "Settings / Session service diagnostics / About").
- **Light scheme, same command, same pane:** printed `E]11;rgb:fdfd/f8f8/fdfdB` — i.e.
  `rgb:fdfd/f8f8/fdfd`. **Matches expected exactly.**
  Screenshot: `bug007-osc11-light.png` (full frame; the answer is on the fifth terminal line,
  appended below the dark-scheme run in the same pane's scrollback).

**Check 1: PASS.** The window's reported scheme flows through to the session's OSC 11 answer,
both at connect (dark, the scheme in effect when the session started) and after a live change to
light while the pane stayed open — matching BUG-007 U12–U15 / A1 exactly.

## Check 2 — `claude` (theme auto) draws legible body text in the dark scheme

Switched the app back to dark (Settings → Appearance → Theme → Dark → Save) before this check,
since it must run "in the dark scheme". Selected the AI-CLI session pane (Claude Code, theme
`auto`) that had been running throughout — it happened to be actively narrating work on this
same bug/task, which is incidental to the check but is what appears in the screenshot.

Screenshots: `bug007-claude-dark-full.png` (full 1600×1400 frame) and
`bug007-claude-dark-crop.png` (cropped/zoomed body-text region).

Observed: body text renders as light gray/white on the dark background, bullet markers and
bold headings in a slightly brighter white, a green-highlighted diff block with black text on
green (correct for a diff addition, not a scheme mismatch), tool-call labels in green
(`Skill(visual-pass)`, `Agent(...)`), and dimmed gray for secondary lines (`Ran 1 shell
command`). Nothing is black-on-black or otherwise illegible anywhere in the pane.

**Check 2: PASS.** `claude` (auto theme) in the dark scheme draws fully legible body text — the
original BUG-007 symptom (black-on-black) is not reproducible.

## What this pass did and did not cover

- Covered: static appearance of both checks' end states, in both colour schemes, on a real
  client+daemon pair built from this branch.
- Not covered: the transition frame at the moment of a live theme flip (mid-flight animation is
  outside what a screenshot pipeline can catch, per the visual-pass skill's own caveat) — the
  before/after states were captured, not the swap itself. This doesn't affect the verdicts above,
  which only need the settled OSC 11 answer and the settled dark-mode render.
- Ran on Xvfb + lavapipe (software rasteriser), not the user's real GPU/display — colour and text
  legibility are unaffected by that, but frame-pacing/smoothness claims are out of scope
  regardless.
