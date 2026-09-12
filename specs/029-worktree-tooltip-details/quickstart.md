# Quickstart: Worktree tooltip shows the full name and its details

Two parts. **§A** is what the machine checks. **§B** is what has to be looked at, because this
feature's whole claim is about a hover surface: no test in this repository can open a tooltip, and
the two things most likely to go wrong — a panel that grows wider than the sidebar, and a long path
that refuses to break — are visible only once it is on screen.

§B is runnable without a human: drive it with the repo's `visual-pass` skill on a private display.

---

## §A — The automated suite

```bash
mise run test-core   # the core half: WorktreeStatus::label
mise run test        # whole workspace, matching CI
```

Green is the gate. What each gate is watching, for this feature:

| Gate | Watching |
|---|---|
| `micold-core/tests/worktree_model.rs` | `WorktreeStatus::label()` — a word for `Missing` and `Invalid`, `None` for `Valid` (contract §4.1). The `None` is the assertion that matters: it is what lets both call sites drop their own emptiness check |
| `micold-client/tests/features_sidebar.rs` | `worktree_tooltip` line **order** and labels — `Name`, `Branch`, `Folder`, `Location`, `Status`, in that sequence (§2) |
| `micold-client/tests/features_sidebar.rs` | every **omission**: no branch → no `Branch:` line; folder equal to the display name → no `Folder:` line; `Valid` → no `Status:` line; `project_root: None` → no `Location:` line and everything else intact (§2.2, §3.3, §4.3) |
| `micold-client/tests/features_sidebar.rs` | the name line is **first** and carries `display_name` verbatim, including a name long enough that the row would ellipsize it (§2.3, FR-001, FR-002) |
| `micold-client/tests/features_sidebar.rs` | an `included` worktree's location carries `" (outside this app)"` after an absolute path (§3.2) |
| `micold-client/tests/sidebar_tree.rs` | the location value itself is unchanged from feature 010 — relative under the project root, absolute outside it (§3.1). These are the existing assertions, re-pointed at the new function rather than rewritten |
| `micold-client/tests/features_sidebar.rs` | `DEFAULT_LOCATION_LABEL` is still `"Project root"` and still the Default entry's tooltip (§1.3, FR-011) |
| `micold-client/tests/features_are_render_free.rs` | the builder stayed out of the rendering layer — the check that keeps Principle I's exception honest |
| `micold-client/tests/material_builder_api.rs` | `Tooltip` and `Text` still construct with required inputs and terminate in `.into()`; the new `wrapping` is chainable, not positional (Principle VIII) |
| `micold-client/tests/material_boundary.rs` | the sidebar still names no rendering-stack `tooltip` of its own — the multi-line change stayed inside the library |
| `micold-client/tests/showcase_completeness.rs` | the gallery's `Tooltip` entry still matches the component, with its posed states declared |
| `micold-client/tests/showcase_captions.rs` | the new posed instance is named in `catalogue.rs`, so the gallery cannot show a state it does not label |

**What §A cannot tell you**: whether the panel is bounded, whether a 60-character path wraps or
overflows, and whether five labelled lines are legible at `body_small`. All three are §B.

---

## §B — The manual pass

```bash
mise run run     # the application; it spawns/attaches the daemon itself
```

Prerequisites: one project with at least three worktrees — one with a name long enough that the row
ellipsizes it, one bound to a branch, and (for B4) one that is missing from disk. `mise run
showcase` covers B5 on its own.

### B1 — The reported bug is gone (US1, FR-001, SC-001)

Narrow the sidebar until a worktree's name ends in an ellipsis. Hover the row.

**Expect**: the tooltip's first line reads `Name: <the whole name>`, with no ellipsis. Widen the
sidebar so the same name fits; hover again — the tooltip is unchanged (FR-001, acceptance scenario
1.2).

### B2 — The details are there and are labelled (US2, FR-004, FR-005, FR-008)

Hover a worktree whose folder name carries a type token and a ticket — the row shows a `feat` chip
and an issue chip, and its label shows neither.

**Expect**: `Branch:` names the full branch, `Folder:` names the directory as it is on disk, and
each is on its own line. Hover a worktree with no bound branch: **no** `Branch:` line, and no blank
line where it would have been (§2.2).

### B3 — It stays inside its ceiling (FR-009, SC-005)

Hover the worktree with the longest path, then shrink the window to its narrowest usable size and
hover it again.

**Expect**: the panel stops at its ceiling and the path **wraps** — this is the one that fails
loudly if `Wrapping::WordOrGlyph` did not land, because a path has no spaces to break at. Nothing
extends past the window edge in either direction (the rendering stack snaps it inward).

### B4 — A flagged row explains itself (US3, FR-006, FR-007)

Delete a worktree's directory from disk outside the app, let the list re-discover, and hover the row
now tinted with the error colour.

**Expect**: `Status: missing`, matching the row's own chip word for word. Then hover a worktree
included from outside `.claude/worktrees/`: its `Location:` is absolute and ends
`(outside this app)`, matching its chip.

### B5 — The gallery shows the shape (Principle VIII)

```bash
mise run showcase
```

**Expect**: the `Tooltip` entry offers three posed instances, the third multi-line, and hovering it
shows the same bounded, wrapped panel B3 describes.

### B6 — Nothing else moved

Hover the **Default** entry, a session row, and a header action icon.

**Expect**: Default still reads `Project root` and nothing else (FR-011); session rows still have no
row tooltip; action-icon tooltips are unchanged single lines. The rows themselves still ellipsize —
this feature added a way to read a name, not a way to stop shortening it (§5.4).

---

## Recording the pass

Append the result below, with the date, the display it ran on, and a screenshot per step that
changed something visible (B1, B3, B4, B5). A step that could not be run is recorded as *not run*
with the reason — not as a pass.

---

## The pass — 2026-09-12

**Where**: not a real display. A private `Xvfb :93` at 1600×1400×24, rendered by Mesa's lavapipe
(`WGPU_BACKEND=vulkan`, `VK_ICD_FILENAMES=…/lvp_icd.json`), driven with `xdotool` — the repo's
`visual-pass` skill. Client, daemon and showcase were built in one locked invocation and pinned to a
per-task directory, and the daemon log was checked for `client attached to daemon` before anything on
screen was read as evidence.

**Fixture**: a throwaway repo with five worktrees — one named long enough to ellipsize, one carrying
a type token, one detached (no branch), one deleted from disk after registration, and one checked out
outside `.claude/worktrees/` and then included through the form's *Include that worktree* action.

| Step | Result | What was seen |
|---|---|---|
| B1 | **pass** | Row read `Tooltip of worktree should show full…`; the tooltip's first line read `Name: Tooltip of worktree should show full worktree name and details`, no ellipsis. Widening the sidebar until the row showed the whole name left the tooltip byte-for-byte the same, and the panel did not grow with the sidebar. |
| B2 | **pass** | `Branch: feat/1234-payment-gateway-retry` and `Folder: feat-1234-payment-gateway-retry` on their own labelled lines, while the row showed the `feat` chip and neither string. The detached worktree's tooltip went `Name` → `Folder` → `Location` with no `Branch:` line and no blank line in its place. |
| B3 | **pass** | At a 360 px-wide window the panel stopped at ~318 px — the 320 dp ceiling — stayed inside the window edge, and broke the path mid-token (`…feat-tooltip-of-worktree-` / `should-show-full-…`). This is the step that would have failed loudly without `Wrapping::WordOrGlyph`; it did not. |
| B4 | **pass** | The deleted worktree's tooltip ended `Status: missing`, the same word as its chip. The included one's `Location:` was absolute and ended `(outside this app)`, matching its chip. |
| B5 | **pass** | The gallery's `Tooltip` entry offered three posed instances captioned `below (the default), to the left, multi-line, wrapped at the ceiling`; hovering the third showed the same bounded, wrapped four-line panel. |
| B6 | **pass** | Default still read `Project root` and nothing else. A session row hovered with no tooltip at all. The header's action icons still showed single-line tooltips (`Add a worktree (new git bra…`, `Hide sidebar`, `Filter worktrees`). Rows kept ellipsizing throughout. |

**Screenshots** (`evidence/`):

- B1 — [`b1-full-name-on-an-ellipsized-row.png`](evidence/b1-full-name-on-an-ellipsized-row.png)
- B3 — [`b3-bounded-and-wrapped-at-the-narrowest-window.png`](evidence/b3-bounded-and-wrapped-at-the-narrowest-window.png)
- B4 — [`b4-status-missing-and-outside-this-app.png`](evidence/b4-status-missing-and-outside-this-app.png) (missing above, included below)
- B5 — [`b5-showcase-multi-line-posed-instance.png`](evidence/b5-showcase-multi-line-posed-instance.png)

**Not covered, and not claimed**: the tooltip's *appearance* over time. A screenshot pipeline cannot
catch a chosen frame of the show/dismiss transition, and lavapipe is a software rasteriser, so
nothing here says anything about how the fade looks or how it paces on the user's GPU. Every claim
above is about a static frame. The tooltip also ran in the dark scheme in the application and the
light scheme in the showcase, which is how both appear above; neither scheme was compared against a
real display's colour.
