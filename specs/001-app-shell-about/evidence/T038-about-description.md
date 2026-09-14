# 001 T038 — the About dialog on a rendered frame, after BUG-001

**Date**: 2026-09-13
**Run by**: an agent, not a person at a display. Xvfb `:96` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.

**Builds**: two `micold-ai-ide` + `micold-daemon` pairs. Each was built in one locked invocation and
copied out inside the lock.

- **after**: `~/vp/bin-about-layout/after` (`aa57e8db`). `strings` finds the new description once
  in the client.
- **before**: `~/vp/bin-about-layout/before`. This is the same tree with `ui/about.rs`,
  `ui/toolbar.rs` and `micold-core/src/metadata.rs` checked out from `2b584b7f`. The new
  description appears 0 times.

For both builds, the daemon log showed `client attached to daemon` before the capture.

**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp96`, with scratch XDG data, config and state dirs. Everything
started here was stopped by PID afterwards.

## Result: **PASS**

The dialog was opened from the `⋮` overflow menu → **About**, in a 1200×900 window. The
comparison is in `BUG-001-about-before-after.png`: the red frame is before the fix, the blue frame
after, and both crops share the same geometry.

- **Before**: "Render-free shared domain model for Micold AI IDE (state, persistence,
  session/worktree logic, wire protocol)." This is `micold-core`'s description.
- **After**: "A local-first, AI-assisted desktop IDE for managing git worktrees and AI coding
  sessions in an embedded terminal."

In both builds the title reads "Micold AI IDE", the version 0.13.1 and the license Apache-2.0.

## Not checked

- Keyboard dismissal. Close and Esc behaviour is unchanged by this fix and was recorded in
  `B-about-flow-pass.md`.
- The light scheme.
- macOS and Windows.
