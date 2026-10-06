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

## Results (T020, 2026-10-05)

Linux container, Xvfb 1600x1400 with Mesa lavapipe and no window manager; client and daemon
built together from `7303f78e` and pinned (`client attached to daemon` in the daemon log). Project
P seeded through a pre-split `projects.json`: D1 unread on Default, X1 and X2 unread on
`feature-x`, Y1 and S read on a worktree named `fix-y` instead of `feature-y`, so that the `fix`
type tag can hide `feature-x` in B9. Light scheme only for these steps; B10 covered both.
Screenshots are in [visual/](./visual/).

| # | Result | Evidence |
|---|---|---|
| §A | PASS. The full gate passed (`cargo test --workspace` failed only on the six root-only baseline tests the ledger lists). Eight deliberate mutants of `counts_as_unread`, `SidebarEntry::unread_count`, `TreeItem::unread_count` and `count_tint` were each caught ([tdd/verification.md](./tdd/verification.md)). | gate log, `tdd/verification.md` |
| B1 | PASS (state seeded at launch, not turns finishing live): Default `● 1`, `feature-x` `● 2`, `fix-y` no indicator, all rows collapsed. The 1 s timing was not measured. | `visual/close-b1-b2-b3-sidebar.png`, left |
| B2 | PASS: expanded `feature-x` still `● 2`; X1 and X2 each carry their own mark. | same image, right |
| B3 | PASS: on hover, the row actions (`+` and delete) sit after `● 2` and do not cover it; the tooltip ends `2 unread sessions`. | same image, middle |
| B4 | NOT RUN: the sidebar was not narrowed. Covered by the `a_long_name_is_cut_short_before_the_count_is` geometry test. | — |
| B5 | PASS once the window had focus: selecting X1 cleared `feature-x`'s indicator, and P's switcher row went to `1 unread`. Before `xdotool windowfocus` the indicator stayed, because a session comes into view only in a focused window (039 FR-019). Xvfb has no window manager to give the window focus. | `visual/close-b5-into-view.png` |
| B6 | PASS: closing X2 from its row lowered `feature-x` from `● 2` to `● 1` (X1 was still unread, so the row kept an indicator), and P's switcher count fell from `3 unread` to `2 unread`. | `visual/close-b6-close-unread.png` |
| B7 | NOT RUN: there was no second window. Rests on 039's one session service (R7). | — |
| B8 | PASS: after both processes were stopped and the client was relaunched, the rows showed the same counts (Default `● 1`, `feature-x` `● 1`, `fix-y` none). The closed X2 stayed uncounted. | `visual/close-b8-restart.png` (before left, after right) |
| B9 | PASS: with the `fix` tag filter on, the `feature-x` row was gone and P's switcher row still said `2 unread`, which includes X1 on the hidden `feature-x`. | `visual/close-b9-tag-filter.png` |
| B10 | PASS in both schemes (M1, `visual/evidence.md`). | `visual/b10-*.png` |
| B11 | PASS: from the collapsed sidebar alone (B1 image), the worktrees holding sessions waiting were `feature-x` (2) and Default (1), not `fix-y`. That matches the seeded unread sessions. This was one trial by the agent, not the 5 user trials SC-002 asks for. | `visual/close-b1-b2-b3-sidebar.png`, left |
| §C | NOT RUN: there is no macOS or Windows host here. The container has a `docker` client but no daemon it can reach, so no sandboxed run was possible. CI runs the suite on all three OSes and its sandbox job runs the real-runtime tests. FR-015 and SC-006 still need a person on macOS and Windows. | — |
