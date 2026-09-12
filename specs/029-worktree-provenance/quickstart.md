# Quickstart: Worktree Provenance

Validation guide for [plan.md](./plan.md). Part 1 is the automated suite — the primary gate, and
where every decision in this feature is covered. Part 2 is the recorded manual procedure for the
render-only pieces the render-free suite structurally cannot reach (Constitution Principle I,
GUI-wiring exception): the count on the reveal chip and the claim entry in the row menu. Part 3 is
the migration rehearsal, which needs a store on disk that predates the feature.

## Prerequisites

- The repo's `mise.toml` trusted once in this worktree: `mise trust`.
- A throwaway git repo to open in the app (do **not** run the fixture steps against this
  repository — they create real worktrees and branches).
- For Part 3, a copy of a pre-upgrade store to restore from; the steps below make one.

## Part 1 — Automated (the gate)

```bash
mise run test-core     # fast loop: classification, migration, store, protocol
mise run test          # the gate: whole workspace, matching CI
```

Expected: green, including the new coverage.

| Test file | Covers |
|---|---|
| `crates/micold-core/tests/worktree_provenance.rs` | Every row of the truth table in [contracts/worktree-classification.md](./contracts/worktree-classification.md) §5 — recorded/unrecorded, both `agent-*` rows (FR-007/FR-007b), both location rows (FR-005), both `state_unreadable` rows (FR-011), health-blindness. Plus §4 invariant 6: two worktrees identical but for a reserved-convention name classify by their record, not their name. |
| `crates/micold-core/tests/reserved_convention.rs` | 014's fourteen-row truth table, carried over unchanged as the veto's corpus — including the 16-vs-15 boundary pair and the case-sensitivity pair (FR-007a). |
| `crates/micold-core/tests/provenance_migration.rs` | `plan_backfill()`: evidence from an override, evidence from an archived session, no evidence ⇒ not backfilled, the reserved-convention veto, out-of-root skipped, idempotence (FR-006b), `already_migrated ⇒ None` (FR-006c), `state_unreadable ⇒ None` and no marker (FR-011), `Some(empty)` still marks (data-model §3). |
| `crates/micold-core/tests/store_roundtrip.rs` | `created_worktrees` + `provenance_migrated` round-trip; both absent from an old file default cleanly; a corrupt per-project file marks the project unreadable rather than silently emptying it (FR-011). |
| `crates/micold-core/tests/workspace.rs` | `forget()` drops provenance and the marker with the project's other records (FR-009). |
| `crates/micold-core/tests/protocol_roundtrip.rs` | `WorktreeClaim` and `WorktreeSnapshot::user_created` survive both wire encodings. |
| `crates/micold-daemon/tests/worktree_provenance_rpc.rs` | A record is written for each of the four `CreateMode`s and not for a rolled-back create (FR-001); delete removes it (FR-009); the migration runs on the first `refresh_worktrees()` and never again (FR-006c); a session started afterwards in a revealed worktree does not promote it (FR-006d). |
| `crates/micold-daemon/tests/worktree_claim.rs` | The RPC acks, persists, and broadcasts; a reserved-convention name is honoured (FR-023); a second claim is idempotent; there is no un-claim (FR-024). |
| `crates/micold-client/tests/worktree_visibility.rs` | The hidden set, `hidden_worktree_count()` (zero while revealed, zero when nothing is hidden, equal to what reveal adds — FR-025/025a/025b), out-of-root worktrees never hidden, "Default" untouched (FR-017). |
| `crates/micold-client/tests/worktree_claim.rs` | The reducer's immediacy (FR-022), its sole mutation, and that the row survives the reveal control being switched off. |

Regression, not new: `crates/micold-client/tests/app_state.rs` and `sidebar_tree.rs` must still pass
014's assertions — default-off, the toggle's sole mutation, the project-switch reset, the `agent`
chip on a revealed row, and tag filters applying to revealed rows (SC-003). They need edits only
where they *construct* an agent worktree by name; those fixtures now withhold a record instead.

**Also confirm nothing on disk moved.** `crates/micold-client/tests/worktree_delete.rs` and the
daemon's git-touching suites should need no edits: this feature adds no git call anywhere (FR-003,
SC-005).

## Part 2 — Manual (GUI wiring)

### Fixture

In a scratch repo (`$REPO`), build the mix this feature exists for: worktrees made *through the app*,
and worktrees made behind its back under ordinary names.

```bash
cd "$REPO"
mkdir -p .claude/worktrees

# Made outside the app — what an assistant session leaves behind. MUST be hidden.
git worktree add .claude/worktrees/feat-login-refactor -b feat/login-refactor
git worktree add .claude/worktrees/spike-caching       -b spike/caching

# Made outside the app, machine-named — hidden under 014 and still hidden here.
git worktree add .claude/worktrees/agent-a885b42dc521fbda1 -b worktree-agent-a885b42dc521fbda1

# Made by hand by the user — indistinguishable from the above, and hidden by design.
git worktree add .claude/worktrees/my-hand-made        -b chore/hand-made
```

Record the pre-run state so SC-005 can be checked afterwards:

```bash
git -C "$REPO" worktree list --porcelain > /tmp/before.txt
```

### Steps

1. `mise run run`, open `$REPO`.
2. **US1 / SC-001** — the sidebar lists **no** worktrees: every one of the four was made outside the
   app. The empty state reads "No worktrees yet", not "No worktrees match the filter".
3. **FR-025 / SC-010** — open the filter accordion. The reveal control reads
   `Show agent worktrees · 4`. This is the only thing on screen that says something is being
   withheld, and it must be legible without opening the docs.
4. Switch it on. Exactly **four** rows appear (FR-025b), each carrying the `agent` chip, and the
   count disappears from the control while it is on.
5. **FR-015** — right-click one. Every ordinary action is present and enabled — start session,
   rename, delete — plus `Claim as mine`. Hover it: the hover cluster is unchanged.
6. **US2 scenario 2 / FR-007b** — create a worktree *through the app* named
   `agent-deadbeefdeadbeef`. It appears immediately (FR-010), with **no** `agent` chip, and the
   hidden count stays at 4. Under 014 this row would have vanished; it is the clearest single proof
   the rule inverted.
7. **US5 / SC-008** — claim `my-hand-made`. Its chip disappears at once. Switch the reveal control
   **off**: it is still listed, and the count reads `· 3`.
8. **FR-014** — switch reveal on, then switch to another project and back. It is off again, and
   switching back does not restore it.
9. Restart the app. The created worktree and the claimed one are both still listed with reveal off
   (US2 scenario 1, US5 scenario 2); the other three are still hidden.
10. **SC-005 / FR-003 / FR-018** — nothing on disk moved:

    ```bash
    git -C "$REPO" worktree list --porcelain > /tmp/after.txt
    diff /tmp/before.txt /tmp/after.txt        # only the worktree created in step 6
    find "$REPO/.claude/worktrees" -name '*provenance*' -o -name '*.micold*'   # must print nothing
    ```

11. **US1 scenario 3** — with the project still open, create a worktree from a terminal under
    `.claude/worktrees/`. On the next refresh **no new row appears**, and the count goes up by one.

## Part 3 — The migration (FR-006, SC-009)

The migration only ever runs once per project, so rehearse it against a store copied aside first.

```bash
STORE="${XDG_DATA_HOME:-$HOME/.local/share}/micold"     # or the path `micold-daemon` logs at start
cp -r "$STORE" /tmp/store-backup
```

1. With the pre-feature build, open `$REPO`, rename one hidden worktree's display label, and start a
   session in another. Quit. These are the two evidence forms FR-006 recognises.
2. Copy the store aside again (`cp -r "$STORE" /tmp/store-premigration`) — this is your "day before
   the upgrade" state, and you will want it more than once.
3. Run the new build and open `$REPO`.
   - **SC-009** — the renamed one and the one with a session are **listed**; the rest are gone.
   - The machine-named `agent-*` worktree is **not** backfilled even if you gave it a label, because
     of the FR-007a veto.
4. Quit and reopen. Nothing further is backfilled, and the count does not change (FR-006c).
5. **FR-006d** — switch reveal on, start a session in a hidden worktree, quit, reopen. It is still
   hidden. Claiming it is the only way to keep it.
6. **FR-011 / SC-007** — restore `/tmp/store-premigration`, then corrupt the project's per-project
   state file (`printf '{' > <project-state>.json`) and open the project. **Every** worktree is
   listed, nothing is hidden, and the reveal control shows no count. Quit, restore the good file, and
   confirm the migration still runs — the failed read must not have consumed it.

Restore your real store when finished: `rm -rf "$STORE" && mv /tmp/store-backup "$STORE"`.

## What "done" looks like

- `mise run test` green.
- Parts 2 and 3 walked once, on one platform, with step 6 of Part 2 and step 6 of Part 3 recorded —
  they are the two inversions of 014's behaviour and the two that a regression would silently undo.
- `docs/user-guide/worktrees-and-sessions.md` updated (FR-019) and the docs check green.
