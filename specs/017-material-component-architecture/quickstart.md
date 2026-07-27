# Quickstart: Validating the Component Architecture

**Feature**: `specs/017-material-component-architecture` | **Date**: 2026-07-27

This feature's acceptance test is unusual and simple: **nothing may look different.** Part A is
automated and gates the build. Part B is the recorded parity walkthrough — the constitution's
Principle I GUI-wiring exception requires it to exist and be followed rather than improvised.

Run Part B **twice**: once in the light scheme, once in dark.

---

## Prerequisites

```sh
mise trust          # first time in a fresh worktree only
mise run test       # cargo test --workspace
mise run run        # cargo run -p micold-client
```

**Baseline.** Parity has two halves.

*Style* parity is captured automatically in `crates/micold-client/tests/fixtures/style_snapshot.txt`
— 116 resolved styles across every widget status and both schemes, asserted byte-for-byte. It runs
in CI and names the component and status that drifted.

*Layout* parity still needs eyes, because the snapshot cannot see spacing or widget-tree structure.
Capture before Phase 4: main shell (sidebar expanded and collapsed), the add-worktree dialog in
both branch-source modes, one open menu, and the sidebar's visible worktree count **at a recorded
window size**.

---

## Part A — automated gates

```sh
mise run test
```

| Gate | Test | Proves |
|------|------|--------|
| No feature module styles anything | `micold-client/tests/material_boundary.rs` | SC-001, FR-001, FR-002, FR-004 |
| Every component exposes the mandated builder | `micold-client/tests/material_builder_api.rs` | Principle VIII |
| **Resolved styles match the baseline** | `micold-client/tests/style_snapshot.rs` | **FR-005, FR-023, SC-002** |
| Token values survive the move unchanged | `micold-core/tests/tokens_move.rs` | FR-021, SC-009 |
| Unified dismissal rules are total | `micold-core/tests/overlay_dismissal.rs` | FR-009 |

**Expected**: all pass, and the total is **781 or more** — the pre-change baseline. A drop means a
test was lost in the move, not that the suite got faster.

Verify the boundary is structural, not conventional:

```sh
grep -c iced crates/micold-core/Cargo.toml     # comments only — no dependency line
cargo test -p micold-core                      # tokens exercised with no renderer present
```

Verify the counts reached zero:

```sh
# Feature modules importing rendering widgets — expect 0 (baseline 13)
grep -rl "use iced::widget::" crates/micold-client/src/ui/*.rs | wc -l
# Style applications outside the library — expect 0 (baseline 119)
grep -rho "style::" crates/micold-client/src/ui/*.rs | wc -l
```

---

## Part B — parity walkthrough

### B1. The headline check

- [ ] `mise run test` is green, including the style snapshot — that covers colour, border, radius
      and status behavior exhaustively (FR-023, SC-002).
- [ ] Launch and compare **layout** against the captured shots. Spacing and structure unchanged.
- [ ] Sidebar shows the same number of worktrees without scrolling as the baseline.

### B2. Every surface, both schemes

Walk each and compare to its baseline screenshot:

- [ ] Main shell, sidebar expanded and collapsed
- [ ] Worktree rows: resting, hovered, selected
- [ ] Known-projects list and the project switcher
- [ ] Every dialog: create worktree (both branch sources), rename, delete, forget, settings, about
- [ ] Overflow menu, context menus, the tag-filter panel
- [ ] Terminal pane and its chrome
- [ ] Notification surface
- [ ] Connection banner, if reachable

### B3. Floating surfaces — the one sanctioned change

- [ ] Every non-modal surface dismisses on **outside click** (FR-009)
- [ ] Every non-modal surface dismisses on **Escape**
- [ ] Every non-modal surface dismisses when content beneath it **scrolls**
- [ ] Dialogs dismiss on Escape and scrim click
- [ ] A dialog holding unsaved input does **not** dismiss accidentally — confirm it is declared
      non-dismissible
- [ ] Two open surfaces stack in a consistent order (FR-010)

This is the only place behavior may differ from the baseline. Note each surface whose dismissal
changed, so the change is recorded rather than discovered later.

### B4. Component-owned state

- [ ] Hover one worktree row while another is still fading — both animate independently (FR-011)
- [ ] Open a menu and press an item that closes it — nothing continues animating afterwards
- [ ] Resize the sidebar by dragging — the handle behaves as before
- [ ] Collapse and expand the sidebar — the slide is unchanged

### B5. Nothing else changed

- [ ] Create, rename, delete a worktree; both branch sources, reuse and overwrite
- [ ] Start, switch, remove a session; confirm a Default (project-root) session works
- [ ] Open, filter, switch projects; forget a project
- [ ] Every keyboard shortcut behaves as before — arrows, Tab and PageUp/Down still reach the
      **terminal**
- [ ] Terminal input, output, scrollback and selection unaffected
- [ ] Quit and relaunch: sidebar width, expanded nodes, filters and theme all restore identically
      (SC-006)

### B6. Performance

- [ ] Leave the app idle for a sustained period: no frames requested, CPU indistinguishable from
      the baseline build (FR-025, SC-008)
- [ ] Press every interactive element in turn, then idle: no animation state remains held

---

## Recording the result

Record in the PR: date, platform, schemes exercised, and the two count checks from Part A. List
every surface whose dismissal changed (B3) — that list is the complete, sanctioned behavior delta.

**An unchecked box in B1, B2 or B5 blocks merge.** This feature's entire value is that it changed
nothing visible; a visible change means a wrapper is not at parity and the foundation is not
trustworthy.
