# Contract: existing-branch picker

Covers User Story 2 (FR-010–FR-015) — choosing an existing branch directly instead of deriving its
name from type/ticket/name inputs.

---

## 1. Form shape

The add-worktree modal gains a `BranchSource` switch above its fields, rendered as a pair of
`material::ToggleChip`s (existing shared primitive — Principle VIII, no new widget):

```
┌ New worktree ───────────────────────────────────┐
│  ( New branch ) ( Existing branch )             │  ← ToggleChip pair
│                                                 │
│  … source-specific fields …                     │
│  Directory: .claude/worktrees/<derived>         │  ← preview, both sources
└─────────────────────────────────────────────────┘
```

| `source` | Fields shown |
|---|---|
| `New` (default) | type `Select`, ticket, name — today's form, unchanged |
| `Existing` | candidate `Select` + the staleness note (§4) |

Switching **to** `Existing` triggers the candidate listing (§2). Switching **back to** `New`
clears `selected_branch` and restores the new-branch inputs with no residual state (FR-015).

---

## 2. Candidate listing

Produced by `parse_branch_refs(list_branch_refs(repo))` (see `git-trait-branches.md`), then
annotated with `blocked_by` from the same `worktree list --porcelain` records pre-flight uses
(`branch-conflict.md` §1). One pass, no second git call.

**Ordering** — deterministic, so rendering and assertions are stable:

1. available before blocked
2. `Local` before `Remote`
3. by remote name (ASCII)
4. by branch name (ASCII)

**Row label**

| Candidate | Label |
|---|---|
| local, available | `feat/login` |
| remote-only, available | `feat/reporting · origin` |
| blocked by a worktree | `feat/login · in use by feat-login` |
| blocked by the project checkout | `main · in use by the project checkout` |

The label is the `Display` impl of `BranchCandidate` — `Select` requires
`Clone + ToString + PartialEq`, so no widget change is needed.

**Empty list** (FR-013): when no candidate at all is available, the picker is replaced by an
explanatory line — never an empty `Select`. "No branches available to reuse" when the repository's
only branches are checked out; "This repository has no other branches" when there are none.

---

## 3. Selection behavior

Blocked candidates remain **selectable** (research R8). On selecting one:

- the blocked explanation is shown inline, naming the holder (FR-012, FR-021)
- **Create is disabled** — no operation is ever started for a blocked candidate
- selecting an available candidate afterwards clears the explanation and re-enables Create

This is the deliberate trade recorded in research R8: `Select` wraps iced's `pick_list`, which has
no per-item disabling, and forking a bespoke list widget is exactly what the Component-reuse gate
rejects. Blocking at the point of action satisfies both FR-012 (visible with a reason) and US5
(the explanation appears when the user reaches for the branch).

**Directory preview** (FR-014): with a candidate selected, the preview shows
`.claude/worktrees/<dir_name_from_branch(candidate.name)>` before the user commits. A candidate
whose derived directory is empty, or clashes with an existing directory, is reported the same way
as any directory clash (FR-022) — at submit, via `BranchSituation::DirectoryTaken`.

---

## 4. Remote staleness disclosure (FR-020)

Whenever remote-origin candidates are present, a persistent one-line note sits under the picker:

> Remote branches reflect your last fetch. Nothing is downloaded here.

**No command in this flow contacts a remote** — the picker reads `refs/remotes` from local ref
storage (research R6/R7, Constitution Principle IV).

---

## 5. Submit path

Selecting an available candidate and pressing Create runs the same two-phase flow as the typed
path: `preflight()` on the selected branch name → `BranchSituation` → resolution → `create_worktree`
with a `CreateMode`. In the common case the situation is `LocalAvailable` or `RemoteOnly` and the
user has, in effect, already answered by picking the branch — so the prompt for a picked candidate
may be skipped **only** when:

- the situation is `LocalAvailable` ⇒ `CreateMode::ReuseLocal` (picking a branch *is* the intent
  to reuse it), or
- the situation is `RemoteOnly` with the picked candidate's remote ⇒ `CreateMode::TrackRemote`

Any other situation — `Blocked`, `DirectoryTaken`, or a situation that no longer matches the
candidate (the branch was deleted, or became checked out, since listing) — raises the normal
prompt or explanation. `Overwrite` is **never** reachable without the explicit confirmation
(`branch-conflict.md` §3 invariant 1); picking a branch is never construed as consent to destroy
it.

---

## 6. Test obligations

**`tests/branch_candidates.rs`** (new)

1. Listing maps local and remote refs to candidates with the right `origin`; `origin/HEAD` is
   dropped; local+remote duplicates collapse to `Local` (FR-011, FR-019).
2. `blocked_by` is set for branches held by a `.claude/worktrees/` worktree and for the branch held
   by the project's own checkout, with the right `BlockReason` variant each (FR-012).
3. Ordering matches §2 exactly for a mixed fixture.
4. Labels match §2's table for all four row shapes.
5. Empty/all-blocked repositories produce the two distinct explanations, never an empty list
   (FR-013).

**`tests/app_state.rs`** (extended)

6. Switching to `Existing` and back clears `selected_branch` and leaves the new-branch inputs
   intact (FR-015).
7. `preview()` under `Existing` derives from the selected candidate; under `New` it is unchanged
   (FR-014).
8. Selecting a blocked candidate disables submit; selecting an available one re-enables it.
9. The §5 skip rule: a picked `LocalAvailable` submits `ReuseLocal` directly, a picked `RemoteOnly`
   submits `TrackRemote`, and neither path can reach `Overwrite`.

**`tests/naming.rs`** (extended)

10. `dir_name_from_branch()` per `data-model.md` — multi-segment branches, uppercase, punctuation,
    Windows reserved names, and the empty-result case.
