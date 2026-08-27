# 001 — the manual walkthrough (quickstart steps 1–9), run for the first time

**Date**: 2026-08-20
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own `micold-ai-ide` + `micold-daemon`, built in one invocation and copied
out of the shared target directory **inside** the build lock (`~/vp83/bin`).
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp83`, `XDG_DATA_HOME` in a scratch dir seeded with a
throwaway git repo. Everything started here was stopped by PID afterwards.
**Platform**: Linux only. The quickstart asks for steps 1–9 "on each platform"; macOS and Windows
are not reachable from here, and CI runs no GUI walkthrough on any platform.

## Result

| # | Step | Result |
|---|------|--------|
| 1 | Launch → one window, top toolbar | **stale, effectively pass** — one window, one top bar. The step says "only Help visible"; the shell has since gained a project chip and a `⋮` overflow, and About moved under it. The shipped shell is not what this line describes and has not been for a long time. |
| 2 | Select "Help" → an "About" action appears | **stale, effectively pass** — the overflow menu holds Theme, Settings, Session service diagnostics, Keep sessions after logout, About. There is no "Help" entry and About is not alone under it. |
| 3 | Activate About → modal opens, background non-interactive | **PASS** — pressing the scrim directly over the *Forget* button of the project row beneath dismissed the dialog and **did not** activate Forget; the project was still listed afterwards. |
| 4 | Read dialog → name, version, license, one-line description | **FAIL** — see below. |
| 5 | Version matches `Cargo.toml` | **PASS** — dialog reads `Version 0.8.0`; `Cargo.toml` has `version = "0.8.0"`. |
| 6 | Activate About again → still exactly one dialog | **not reachable** — as the step's own "(if reachable)" anticipates: the overflow menu is behind the scrim, and a press there dismisses. Idempotence is covered by `tests/about_flow.rs`. |
| 7 | Click Close → closes, window unchanged | **PASS** |
| 8 | Reopen, press Esc → closes, window unchanged | **PASS** |
| 9 | Esc with no dialog → nothing happens | **PASS** |

`about-dialog.png` is step 4/5's frame. `close-esc-noop.png` stacks steps 7–9 in order:
after-close, reopened, after-esc, esc-with-nothing-open.

## Step 4 — the failure

The dialog shows name, version and license correctly, and then this description:

> Render-free shared domain model for Micold AI IDE (state, persistence, session/worktree logic,
> wire protocol).

That is `micold-core`'s package description. The About dialog is describing the internal library
rather than the application. Filed as [BUG-001](../bugs/BUG-001.md), with the cause: `env!` in
`micold-core/src/metadata.rs` expands against *that* crate's manifest, and the workspace split
moved the manifest out from under a line written when there was only one crate.

Version and license are right only because both are `workspace = true` and therefore identical in
every crate. The description is the one field that differs per crate, and it is the one that broke.

## On steps 1 and 2

Neither is a defect and neither should be recorded as a pass without saying why. They describe a
toolbar that no longer exists — the feature's own UI was superseded by 003, 017 and 018. Rewriting
them is a documentation change, not a fix; they are marked stale here rather than silently ticked.
