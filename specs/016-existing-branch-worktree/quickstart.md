# Quickstart: Reuse or Overwrite an Existing Branch When Creating a Worktree

Validation guide for feature 016. Scenarios 1–7 are manual GUI procedures covering the thin iced
wiring that Constitution Principle I's GUI-exception permits to be validated this way (features
006, 010, 013 set the precedent). Everything with decision logic is covered by automated tests —
see the test obligations in each contract.

## Prerequisites

- A trusted `mise.toml` (`mise trust` once per worktree)
- A scratch git repository to open as a project

## Automated gate — run first

```bash
mise run test                                  # render-free core, matches CI
cargo test --features gui                      # GUI-feature build
cargo clippy --features gui --all-targets      # lints
```

All three must pass before the manual scenarios mean anything. Expect coverage in:
`tests/branch_conflict.rs`, `tests/branch_candidates.rs`, `tests/worktree_create.rs`,
`tests/worktree_rollback.rs`, `tests/git_fake.rs`, `tests/naming.rs`, `tests/app_state.rs`.

## Fixture

```bash
mkdir -p /tmp/wt-016 && cd /tmp/wt-016
git init -b main . && git commit --allow-empty -m "base"

# A branch started "outside the IDE", with a distinctive commit (Scenarios 1, 3)
git branch feat/reporting
git switch feat/reporting && git commit --allow-empty -m "OUTSIDE-WORK" && git switch main

# A stale branch to overwrite (Scenario 2)
git branch feat/stale

# A bare remote with a branch that has no local counterpart (Scenario 4)
git init --bare /tmp/wt-016-remote.git
git remote add origin /tmp/wt-016-remote.git
git push origin main:refs/heads/feat/from-elsewhere
git fetch origin           # the ONLY fetch — the app must never run one
```

Launch with `mise run run`, then open `/tmp/wt-016` as a project.

---

## Scenario 1 — Reuse an existing branch (US1, FR-001/002/004)

1. Sidebar → **+** to open **New worktree**.
2. Type `feat`, name `reporting` (derives branch `feat/reporting`). Press **Create**.
3. **Expect**: creation pauses; a panel names `feat/reporting` as already existing and offers
   **Reuse**, **Overwrite**, **Cancel** — *not* the old "A branch with that name already exists."
4. Press **Cancel**. **Expect**: back on the form with `feat` and `reporting` still filled in
   (FR-007).
5. Press **Create** again, then **Reuse**.
6. **Expect**: the worktree appears in the sidebar. Verify the history survived:
   ```bash
   git -C /tmp/wt-016/.claude/worktrees/feat-reporting log --oneline
   # must contain OUTSIDE-WORK
   ```
7. Start a session on it. **Expect**: behaves like any other worktree (FR-023).

## Scenario 2 — Overwrite a stale branch (US2, FR-005/006)

1. New worktree → type `feat`, name `stale` → **Create** → **Overwrite**.
2. **Expect**: a second, explicit warning naming the branch and stating its commits will be
   discarded. Press **Back**. **Expect**: returned to the reuse/overwrite choice, nothing changed
   (US2 AS3).
3. **Overwrite** → **Confirm**.
4. **Expect**: the worktree is created at `main`'s tip:
   ```bash
   git -C /tmp/wt-016 rev-parse feat/stale main   # same commit
   ```

## Scenario 3 — Reuse rollback must not delete the branch (FR-008, SC-003)

The most important manual check; its automated counterpart is `tests/worktree_rollback.rs` #11.

1. Pre-create the target directory with content so creation fails after pre-flight:
   ```bash
   mkdir -p /tmp/wt-016/.claude/worktrees/feat-reporting-2 && touch /tmp/wt-016/.claude/worktrees/feat-reporting-2/x
   git -C /tmp/wt-016 branch feat/reporting-2 feat/reporting
   ```
2. Create a worktree deriving `feat/reporting-2` and choose **Reuse**.
3. **Expect**: creation fails with a directory-clash message.
4. **Expect — critical**: `git -C /tmp/wt-016 branch --list feat/reporting-2` still lists it, and
   `git log feat/reporting-2` still contains `OUTSIDE-WORK`.

## Scenario 4 — Continue from a remote-only branch (US4, FR-016/017/020)

1. New worktree → type `feat`, name `from-elsewhere` → **Create**.
2. **Expect**: the panel identifies it as a branch on **origin** and offers **Continue from
   origin** and **Start fresh at HEAD**, with the divergence warning on the latter (FR-018). The
   remote-staleness note is visible.
3. **Continue from origin**.
4. **Expect**: local branch created at the remote tip, tracking it:
   ```bash
   git -C /tmp/wt-016 rev-parse feat/from-elsewhere origin/feat/from-elsewhere   # same
   git -C /tmp/wt-016 rev-parse --abbrev-ref feat/from-elsewhere@{upstream}      # origin/feat/from-elsewhere
   ```
5. **Offline check (FR-020, Principle IV)**: `mv /tmp/wt-016-remote.git /tmp/wt-016-remote.off`,
   restart the app, and repeat steps 1–2 for another remote-only branch.
   **Expect**: the panel still appears, sourced from local `refs/remotes` — no hang, no network
   error.

## Scenario 5 — Pick a branch from the list (US2, FR-010–FR-015)

1. New worktree → switch to **Existing branch**.
2. **Expect**: local and remote branches listed; remote rows show their remote; branches already
   checked out show `· in use by …`; the staleness note is present.
3. Select a blocked branch (e.g. `main`). **Expect**: the reason is explained and **Create** is
   disabled — no operation starts.
4. Select an available branch. **Expect**: the explanation clears, Create re-enables, and the
   directory preview shows the derived `.claude/worktrees/<name>` (FR-014).
5. Press **Create**. **Expect**: the worktree is created on exactly that branch, history intact.
6. Switch back to **New branch**. **Expect**: the new-branch inputs return with no leftover
   selection (FR-015).

## Scenario 6 — Branch already checked out (US5, FR-021)

1. New worktree → type/name deriving `main` (the repository's own current branch).
2. **Expect**: an explanation that the branch is in use by the project's checkout, with **no**
   reuse or overwrite offered, and nothing changed on disk.
3. Repeat for the branch bound to the worktree created in Scenario 1.
4. **Expect**: the explanation names that worktree.

## Scenario 7 — No regression for a free name (FR-025, SC-008)

1. New worktree → type `chore`, name `something-new` → **Create**.
2. **Expect**: created immediately, with the same steps and prompts as before this feature — no
   extra dialog.

---

## Documentation gate (Principle VII, FR-026)

`docs/user-guide/worktrees-and-sessions.md` must, in the same change, cover: the reuse/overwrite
choice and what each does; the existing-branch picker; continuing from a remote branch and that
the list reflects the last fetch; and when neither reuse nor overwrite is available. Overwrite's
irreversibility should be stated plainly — git's reflog is not an undo feature this app offers.

## Cleanup

```bash
rm -rf /tmp/wt-016 /tmp/wt-016-remote.git /tmp/wt-016-remote.off
```
