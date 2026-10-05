# Quickstart: Attention Indicator on the Sidebar's Worktree Rows

**Feature**: 575 | **Plan**: [plan.md](./plan.md) | Contract:
[attention-indicator](./contracts/attention-indicator.md) | Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree.
- For §B: the `visual-pass` skill's private display, pinned binaries and private data directory.
- For §C: a macOS bundle from `mise run app` and an installed Windows build.

## §A — Automated

```sh
mise run test-core     # counts_as_unread, unread_session_count skipping closed sessions
mise run gate          # client state tests, tree_view geometry, contrast gate, fmt, clippy, docs
```

Expected: both green. Each test named in [plan.md](./plan.md#test-strategy-by-layer) exists and
fails without the change it covers.

## §B — Recorded pass on Linux (visual-pass)

One project P with the Default location and worktrees `feature-x` and `feature-y`. Sessions: D1
on Default; X1, X2 on `feature-x`; Y1 on `feature-y`; S selected in view (on `feature-y`). Record
a screenshot for each step that names a look, in the light and the dark scheme.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Collapse every row. Let X1, X2 and D1 finish a turn. | `feature-x` row `● 2`, Default row `● 1`, `feature-y` row nothing, within 1 s each. | US1.1, US1.2, US2.1, SC-001 |
| B2 | Expand `feature-x`. | Row still `● 2`; X1 and X2 each carry their own mark and emphasised name. Collapse it again: still `● 2`. | US1.3, FR-003 |
| B3 | Hover `feature-x`. | Row actions fade in beside the indicator; the indicator neither moves nor is covered. Tooltip ends `2 unread sessions`. | FR-004, FR-005 |
| B4 | Narrow the sidebar to its minimum. | The name shortens with an ellipsis; `● 2` stays whole; row height unchanged. | FR-004, U8 |
| B5 | Select X1. | `feature-x` shows `● 1` within 1 s; the switcher's P row drops by one. | US2.3, SC-003 |
| B6 | Close X2 (unread) from its row. | `feature-x` indicator gone; P's switcher count no longer includes X2. | US1.5, FR-010 |
| B7 | Open a second window on P. Select D1 in the first. | Default row's indicator gone in both windows within 1 s. | US2.4, FR-008 |
| B8 | With some rows counted, quit the application, reopen it. | Same counts on the same rows; the rows of P add up to P's switcher count. | US2.5, SC-005 |
| B9 | Turn on a tag filter that hides `feature-x` while it holds an unread session. | No `feature-x` row; P's switcher count still includes it. | Edge case, FR-010 |
| B10 | Showcase (`mise run showcase`): the `UnreadMark` entry, both schemes. | Location rows with `● 2` collapsed and expanded beside one without. | FR-012, FR-013 |
| B11 | Without expanding any row, say which worktrees hold sessions waiting for you. | The answer read from the indicators matches the unread sessions. | SC-002 |

## §C — macOS, Windows and a sandboxed project

Repeat B1, B5 and B8 on macOS and Windows, and once on Linux with P run in a container (sandbox).
Expected: the same results as §B (FR-015, SC-006).
