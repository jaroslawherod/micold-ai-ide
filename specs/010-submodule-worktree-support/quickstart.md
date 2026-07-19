# Quickstart / Validation Guide: Git Submodule Support for Worktree Creation

Proves the feature end-to-end. See
[contracts/git-trait-submodules.md](./contracts/git-trait-submodules.md) and
[data-model.md](./data-model.md) for the interfaces referenced here.

## 1. Headless logic-core suites (CI parity, no GUI needed)

```bash
cargo test --no-default-features --all-targets
cargo test --features gui
cargo clippy --features gui --all-targets
```

Expected: all green, including new/updated tests in `tests/worktree_create.rs` and
`tests/worktree_rollback.rs` covering:
- A `FakeGit` primed with submodules → `create_worktree` calls
  `submodule_update_init_recursive` and returns `Ok`.
- A `FakeGit` with no submodules → `create_worktree` never calls
  `submodule_update_init_recursive` (FR-003 — zero overhead path unchanged).
- A `FakeGit` primed to fail the submodule step → `create_worktree` runs the full rollback
  plan (`worktree_remove → worktree_prune → branch_delete`, same order as an add-failure) and
  returns `CreateError::RolledBack` (FR-005).

## 2. Manual check: repository with submodules (US1: FR-001/002/003, SC-001)

Set up a local throwaway superproject + submodule (no network required — both are local repos):

```bash
d=$(mktemp -d) && cd "$d"

git init -q sub && (cd sub && git commit -q --allow-empty -m init)

git init -q nested && (cd nested && git commit -q --allow-empty -m init)
(cd sub && git submodule add -q "$d/nested" nested-dep && git commit -q -m "add nested submodule")

git init -q super
(cd super && git submodule add -q "$d/sub" vendor/sub && git commit -q -m "add submodule")

cargo run --features gui -- # open "$d/super" as the project
```

| Action | Expected |
|--------|----------|
| Create a worktree on `super` | Form shows a "Creating worktree…" state (US2), then closes on success. |
| Inspect `.claude/worktrees/<new-dir>/vendor/sub/` | Populated with `sub`'s files (not empty) — submodule was fetched (FR-002). |
| Inspect `.claude/worktrees/<new-dir>/vendor/sub/nested-dep/` | Populated with `nested`'s files — the *nested* submodule was also fetched (FR-002). |
| Create a second worktree, non-submodule repo (any plain git repo) | Creation completes with no "Creating…" delay beyond today's baseline, no submodule-related UI at all (FR-003, SC-004). |

## 3. Manual check: submodule fetch failure rolls back (US3: FR-005/006, SC-003)

```bash
(cd super && git submodule add -q "$d/nested" vendor/broken && git commit -q -m "add submodule with bad url")
git -C super config --file .gitmodules submodule.vendor/broken.url "$d/does-not-exist" && \
  git -C super config -f .git/config submodule.vendor/broken.url "$d/does-not-exist"
```

| Action | Expected |
|--------|----------|
| Create a worktree on `super` (now with the broken submodule URL) | Creation fails; the error names the failing submodule path (`vendor/broken`) and the underlying git error (FR-006). |
| Run `git worktree list` / inspect `.claude/worktrees/` afterward | No worktree directory, no registered worktree, no branch left behind — full rollback occurred (FR-005), not a partially-populated worktree. |
| Retry after fixing the submodule URL (or removing it) | Worktree creation with the same name now succeeds. |

## 4. Cross-platform note

Steps 1–3 rely only on the `git` binary already required by every existing worktree operation
(research R5) — no platform-specific setup. CI's existing Linux/macOS/Windows matrix covers this
without a new lane.
