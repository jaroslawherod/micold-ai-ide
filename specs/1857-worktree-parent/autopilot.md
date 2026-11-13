# Autopilot ledger — #1858 worktree-parent M1

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Milestone M1 of specs/1857-worktree-parent/tasks.md (T001-T014, T020-T022); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1858
- **Worktree branch**: feat/1858_worktree-parent-m1-persist-and-expose-the-parent
- **Started**: 2026-11-13
- **Phase**: milestones
- **Next step**: full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1861 | Design (spec, plan, tasks; merged content, treated as reviewed) | not merged, content on this branch | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T020–T022 | full | Stable worktree ids and parent links in per-project state; create_worktree records the parent; list_worktrees/list_sessions/get_session report it; removed tombstone; user guide | #… | in-progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | milestones | Where do the new daemon tests live? | In `crates/micold-daemon/tests/mcp/` as module `mcp_worktree_parent` (one binary, #1175), not new top-level files; catalog tests are in the same module | agent-resolved | tests/mcp/main.rs |
| D2 | milestones | Pure id/link rules | In `micold-core/src/worktree_parent.rs` (unit-tested), applied by `Catalog` | agent-resolved | plan.md |
| D3 | milestones | Relocation test (T011) | No relocation fixture exists for per-project state (state is keyed by project path); covered by the store round trip and daemon restart tests | agent-resolved | store.rs `project_state_path` |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|
| A | 1 | origin/main...HEAD | 11d635f5 | d7c7f1d4eb34:11d635f5ac23 | 9 findings, none fixed (declined, see below; F1/F2 are real narrow races) |
| B | 1 | origin/main...HEAD | 11d635f5 | d7c7f1d4eb34:11d635f5ac23 | CHANGES: T011 no test (task text updated, drift-M1.md), T004/T007 path drift (drift-M1.md), mutant evidence (daemon mutants added to cycle log) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | record after create leaves a window with no parent (F1) | Design in plan.md records after the create returns; a window of milliseconds; passing parent into the create path is a follow-up |
| M1 | A | record retires a refresh-minted id, a concurrent child of the same parent turns removed (F2) | Same window; needs a concurrent create of a child within it; follow-up with F1 |
| M1 | A | caller's worktree missing from the cached snapshot gives no parent (F3) | Spec'd: an unresolvable caller creates an unparented worktree (test `an_unresolvable_caller_creates_an_unparented_worktree`) |
| M1 | A | store save does not merge on-disk ids/links (F4) | Only the daemon saves the workspace with these maps; follow-up if a client writer appears |
| M1 | A | transient absence tombstones children (F7) | The documented R3 design; guarded by directory-exists check |
| M1 | A | perf of views per read, clone in refresh, parent_choice ordering, 8-arg fn (F5, F6, F8, F9) | MINOR, taste |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Record the parent inside the create path (review A F1/F2): closes the window where a new worktree is visible without its link and a concurrent child of it can be tombstoned.
