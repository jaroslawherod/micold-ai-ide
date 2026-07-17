# Quickstart & Validation: Worktree Sidebar Refinement

How to prove the feature works. Headless logic first (fast, CI-covered), then a short manual
GUI pass for the purely visual acceptance criteria (padding, 80% font, tag colors).

## Prerequisites

- Rust stable toolchain (via `mise`).
- Repo builds: `cargo build`.

## 1. Headless logic validation (primary — Principle I)

Run the full suite:

```bash
cargo test
```

Contrast test must be runnable without the GUI feature (as today):

```bash
cargo test --no-default-features --test tokens
```

Expected — these tests exist and pass after implementation:

| Area | Test file | Proves |
|------|-----------|--------|
| Friendly name + tags | `tests/naming.rs` | `display_name`/`parse_tags` match the table in contracts/naming-tags.md (incl. untyped, issue-key, fallback) |
| Tag AA contrast | `tests/tokens.rs` | every type/issue tag `(on_fill, fill)` pair ≥ AA in light AND dark; sidebar sizes == round(0.8 × base) |
| Filter predicate | `tests/sidebar_tree.rs` | each `TagFilter` and OR-combined sets select the right worktrees; empty filter shows all |
| Menu / filter / rename state | `tests/sidebar_state.rs` | one menu open at a time; filter toggle/clear; rename draft + validation error path |
| Override persistence | `tests/store_roundtrip.rs` | override round-trips; old file w/o field loads (no schema bump); survives reload |
| Delete orchestration | `tests/worktree_delete.rs` | confirm ⇒ `FakeGit` removed worktree + deleted branch, matching sessions `killed`, others untouched; cancel ⇒ nothing called |
| Reducers / escape | `tests/app_state.rs` | rename updates display name via `Workspace`; delete-confirm drops session records + clears active; Esc maps both new overlays |

Guidance from repo memory: prefer these headless VT/logic tests over launching the GUI.

## 2. Manual GUI validation (visual-only acceptance)

Launch:

```bash
cargo run
```

Open a git project that has worktrees created via the app's naming convention (create a few
with the Add-Worktree form using different types, e.g. `feat` + `ABC-123`, `fix` no ticket,
`chore`, plus one non-conforming branch like `main`).

Checklist mapped to acceptance scenarios:

- [ ] **US1 / FR-001..008**: each worktree shows a friendly name (e.g. "Login page") with a
      color-coded type tag and, when a Jira key is present, an issue tag. Same type ⇒ same
      color across rows. Non-conforming worktree shows no type tag. Branch/dir on disk unchanged
      (`git worktree list`).
- [ ] **US5 / FR-009,010,012**: no git icon next to worktrees; left/right padding is minimal;
      sidebar text is visibly smaller (80%). Verify legibility in BOTH light and dark themes
      (toggle theme).
- [ ] **US5 / FR-011**: a missing/invalid worktree is still distinguishable (name in error color
      + missing/invalid status tag), without the old icon.
- [ ] **US4 / FR-024..028**: activate a type filter ⇒ only those worktrees list; activate a
      second ⇒ union (OR); "untyped" filter surfaces non-conforming ones; clear restores all;
      a no-match filter shows the empty state with one-tap clear.
- [ ] **US3 / FR-013..017**: right-click a worktree ⇒ menu with Rename + Delete. Rename ⇒ name
      changes in the sidebar only; tags unchanged. Quit and relaunch ⇒ custom name persists;
      `git worktree list` and the branch are unchanged.
- [ ] **US2 / FR-018..023**: with a running session in a worktree, right-click ⇒ Delete ⇒
      confirmation names the directory, its sessions, and the branch. Confirm ⇒ running session
      terminates, worktree + sessions + branch gone (`git worktree list`, `git branch --list`),
      row disappears; if it held the active session the app settles cleanly. Cancel ⇒ nothing
      removed.

## 3. Docs & constitution gates

- [ ] User guide updated with tags, filtering, right-click rename/delete (Principle VII); docs
      build passes in CI.
- [ ] `cargo test` green on Linux/macOS/Windows in CI (Principles I, VI).

## References

- Data shapes: [data-model.md](./data-model.md)
- Behavior contracts: [contracts/](./contracts/)
- Decisions & rationale: [research.md](./research.md)
