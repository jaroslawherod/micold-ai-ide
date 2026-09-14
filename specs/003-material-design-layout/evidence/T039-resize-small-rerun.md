# 003 T039 — §3 "Resize small" re-run after BUG-002

**Date**: 2026-09-13
**Run by**: an agent, not a person at a display. Xvfb `:96` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Builds**: two `micold-ai-ide` + `micold-daemon` pairs, each built in one locked invocation (all three
crates named in `Compiling`) and copied out of the shared target directory inside the lock:

- **after**: `~/vp/bin-about-layout/after`, the branch at `aa57e8db`. `strings` finds `cdk::reflow`
  11 times in the client.
- **before**: `~/vp/bin-about-layout/before`, the same tree with the fix's sources (`shell.rs`,
  `cdk/mod.rs`, `showcase/catalogue.rs`) checked out from `2b584b7f`. `cdk::reflow` appears 0 times.

For both, the daemon log showed `client attached to daemon` for the client's PID before any capture,
and the daemon process ran from the pinned directory.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp96`, with scratch `XDG_DATA_HOME`, `XDG_CONFIG_HOME` and
`XDG_STATE_HOME`. Everything started here was stopped by PID afterwards.
**Fixture**: a catalog seeded with three entries: `a-project-whose-folder-name-is-much-longer-than-the-row-can-show`,
a git repo; `notes`, a plain folder; and `gone-away-and-renamed-elsewhere-on-disk`, a missing path, so
the unavailable row renders too. No project was open.

## Result: **PASS**

`BUG-002-reflow-before-after.png` shows the Known-projects list at window widths of 400, 560 and
760 px. The red-framed strip is before the fix and the blue-framed strip is after. Every crop has
the same geometry.

| Width | Before | After |
|---|---|---|
| 760 | The long name wraps onto two lines beside the actions. | The long name is elided to one line (`…much-longe…`), and the actions stay beside it. |
| 560 | The long name wraps to seven lines. The unavailable row's name is cut to `gone-away-and-renamed-`. | The actions move beneath the name, and all three names are painted whole on one line. |
| 400 | The long name is gone. Forget is an empty pill, and the second row's buttons are squashed into slivers. | The actions move beneath the elided name and wrap onto a second line. Every label is painted. |

At 1200 px, and at the default window, every row is unchanged: the name, the badge and all three
actions share one line.

## Checked

- Placement, and whether each name and label is painted, elided or clipped, at each width.
- The unavailable row: at every width it keeps its disabled **Unavailable** button, Rename and
  Forget.

## Not checked

- **Scrolling at 400 px.** The list runs past the bottom of a 700 px window both before and after.
  Whether the page scrolls to the rest of it was not exercised.
- **Hover and press states** in the moved layout. Those states are drawn by the same buttons, whose
  boxes the layout snapshot shows unchanged.
- **Light scheme.** Only the dark scheme was captured.
- **macOS and Windows.**
