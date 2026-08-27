# 021 T076 — §B8's open/close rule, run for the first time

**Date**: 2026-08-21
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: `~/vp83/bin/micold-showcase`, pinned inside the build lock on 2026-08-20 21:03 from this
checkout — `strings` resolves its source path to
`micold-ai-ide/.claude/worktrees/docs-cleanup-completed-features`, and it carries both
`TypeaheadFocused` and `TypeaheadDismissed`, so it is the rewired entry (T073/T074) and not a
pre-BUG-001 binary.

T076 asked for two things: that §B8 spell the rule out, and that someone run it. The four steps have
been in the quickstart since 2026-08-07; this is the run.

## Result — all four steps **PASS**, in both schemes

`b8-open-close-rule.png` is the three states at identical geometry.

| # | Step | Observed |
|---|------|----------|
| 1 | On launch the list is **closed** — a search field and nothing beneath it | **PASS** — top panel. The field sits directly above the **FilterTrigger** entry with no list between them. Nothing had touched the entry: the binary was launched and the page scrolled to it, and scrolling is not a message. |
| 2 | Press the field → the list opens **before anything is typed**, floating over the page without moving the entries around it (FR-001b) | **PASS** — middle panel. Four rows appear with an empty query. |
| 3 | Press a row → the list closes and the marker stays on the row that was chosen | **PASS** — the list closed on the press; reopening shows `feat/login-page` carrying the selection marker and the selected-row tint (bottom panel). |
| 4 | Reopen, then press elsewhere on the page — or press Escape. The list closes; the search text and the choice both survive | **PASS on both routes** — see below. |

### Step 2 — that it really floats

Not eyeballed. The frames before and after the press are **pixel-identical** outside the list:

```
$ compare -metric AE <(crop y 0..300 of closed) <(crop y 0..300 of open) null:
0 (0)
$ compare -metric AE <(crop y 1300..1400 of closed) <(crop y 1300..1400 of open) null:
0 (0)
```

Zero differing pixels above the field and at the far bottom of the window. The list is drawn over
the **FilterTrigger** and **ResizeHandle** entries — both visibly clipped behind it in the middle
panel, at the same y they occupy in the top panel — rather than pushing them down. That is FR-001b.

### Step 4 — both routes, and what survives

Typed `login` into the open list first, so there was something to survive. The list narrowed to two
rows with the matched characters emphasised (`feat/`**`login`**`-page`, `fix/`**`log`**`out-redirect
— `**`in`**` use in ../review`), the clear ✕ appeared, and the marker stayed put.

| Route | Observed |
|---|---|
| Press elsewhere on the page | list closed; field still reads `login`, label still floated, ✕ still offered |
| Reopen, press Escape | same — closed, `login` intact |
| Reopen after either | the marker is still on `feat/login-page` |

So the search text and the choice both survive a dismissal, by either route.

### Both schemes

Toggled to dark from the control at the top of the page and returned to the entry
(`b8-dark.png`). The entry was **closed** on arrival — the rest state again, and the `login` query
had survived the scheme change. Pressing the field opened it; emphasis on the matched characters is
legible against the dark surface, the selection marker and the selected-row tint both read, and the
dimmed unavailable row (`fix/logout-redirect — in use in ../review`) stays distinguishable from the
enabled one. Nothing in the rule behaves differently by scheme.

## What this closes

§B8's record table listed the rule as test-driven and the four on-screen steps as *"not yet
recorded — needs a human at the display"*. They are recorded now, and the split the table describes
still holds: `crates/micold-client/tests/showcase_state.rs` owns the rule, and what this run
confirms is the glue in `showcase/sections/controls.rs` that applies it (FR-020a, SC-007a).
