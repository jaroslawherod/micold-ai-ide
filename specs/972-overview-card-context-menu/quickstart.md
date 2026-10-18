# Quickstart: Overview Card Context Menu

**Feature**: 972 | **Plan**: [plan.md](./plan.md) | Contract: [ui](./contracts/ui.md) |
Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree (in the cloud environment: `scripts/gate.sh`).
- For §B: the `visual-pass` skill's private display, client and daemon built together, a private
  data directory, and project P with Default, worktree `feat-42_alpha` (one session, an open pull
  request), an included worktree `inc`, worktree `misc` (no session) and a missing folder.

## §A — Automated

```sh
mise run test-core   # unchanged core; quick check nothing regressed
mise run gate        # client state (tests/overview_card_menu.rs), registry pairs, keys, card/menu pixel and geometry gates, icons, docs
```

Expected: green. Each test in [plan.md](./plan.md#test-strategy-by-layer) exists and fails without
the change it covers.

## §B — Recorded pass on Linux (visual-pass)

Screenshot each step that names a look, in light and dark.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Open the Overview; right-click `feat-42_alpha`'s card. | Menu at the pointer: *Open*, then the same entries as its sidebar row menu, same order and icons. The card shows the target mark. | US1.1, US2.1, FR-001, FR-002, FR-006 |
| B2 | Move the pointer over the menu and onto another card. | Mark stays on `feat-42_alpha` only; the other card shows only hover. | US2.1 |
| B3 | Choose *Rename*; rename to `feat-42_beta`. | Dialog over the Overview; after confirming the card shows the new name; Overview still open. | US1 Independent Test, US1.5, FR-005 |
| B4 | Right-click; choose *Rename*; cancel. | Overview unchanged. | Edge: dialog cancelled |
| B5 | Right-click the Default card. | *Open* then the Default row menu's entries. | US1.2 |
| B6 | Right-click a card; press Escape. Then right-click and click empty Overview space; then right-click and click another card. | Each time the menu closes, no entry runs, the mark goes, the Overview stays; the clicked card does not activate. | US2.2, US2.3, FR-007, FR-008, SC-005 |
| B7 | Right-click card A, then right-click card B. | A's menu closes; B's opens at the new point with B marked. | US2.4 |
| B8 | Right-click a card near the right and bottom edges; then at the minimum window size. | Panel fully inside the window. | Edge: window edge, small window |
| B9 | Right-click a card; scroll the grid. | Menu closes. | FR-012, Edge: grid scrolls |
| B10 | Tab to `inc`; press Shift+F10; Down; Enter. | Menu opens inside the card with *Open* highlighted; the second entry runs for `inc`. | US3 Independent Test, US3.1, US3.2, FR-009, FR-010 |
| B11 | Tab to a card; press the Menu key (if present); press Escape. | Menu opens; Escape closes it and the ring is on the same card. | US3.3, FR-011 |
| B12 | Tab to `misc`, Shift+F10, choose *Delete*, confirm. | `misc`'s card disappears; focus moves to the first card. | FR-011, US1.5 |
| B13 | With no card focused (click empty space), press Shift+F10. | No menu. | US3.5 |
| B14 | Choose *Open* on `feat-42_alpha`; then *Review changes* on another card. | Each closes the Overview and shows the session / the Changes view. | US1.3, US1.6, FR-003 |
| B15 | Open a card menu, then delete that worktree from a second window. | Menu closes; nothing runs. | FR-012, Edge: target disappears |
| B16 | Open a card menu; press Ctrl+Shift+O. | Overview and menu close together. | FR-012, Edge: Overview closes |
| B17 | Showcase: `OverviewCard` targeted, `MenuOverlay` highlighted. | Mark and highlight visible in both schemes and in a greyscale screenshot. | FR-006 |

Note: the keyboard-open chain (`CardMenuRequested` → `focused_bounds_task` → `CardMenuOpened`) and
the focus-return drain live in `main.rs`; their decisions (`focus_return_target`,
`card_menu_entry`, the anchor inset) are unit-tested, the chains themselves only here (B10–B12).

## §C — macOS and Windows

Repeat B1, B6, B10 (and Ctrl+click / two-finger click for B1 on macOS) on macOS and Windows.
Expected: the same results (Principle VI).
