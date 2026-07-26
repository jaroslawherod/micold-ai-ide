# Phase 0 Research: Reuse or Overwrite an Existing Branch When Creating a Worktree

All unknowns from the Technical Context are resolved below. No `NEEDS CLARIFICATION` remains.

---

## R1 — How to detect that a branch is already checked out, and where

**Decision**: Reuse `git worktree list --porcelain`, which the app already calls, and match on the
`branch` field of the parsed `WorktreeRecord`s. Report the holder as the record's `path`.

**Rationale**: `git worktree list --porcelain` lists **the main checkout as its first record**, not
only linked worktrees. That single call therefore answers both halves of FR-021 — "in another of
the app's worktrees" and "the repository's own current checkout" — with no extra command and no
`rev-parse --abbrev-ref HEAD`. `parse_worktrees()` already extracts `branch` per record; today
`create_worktree` throws that information away by only comparing `record.path` against the target
directory. Note that `reconcile()` cannot be used here: it deliberately filters to records whose
parent is `.claude/worktrees/`, which discards the main checkout — exactly the record FR-021's
second case needs. Pre-flight must read the raw records.

**Alternatives considered**:
- `git rev-parse --abbrev-ref HEAD` for the main checkout plus the porcelain list for the rest —
  two commands, two code paths, and a race between them. Rejected.
- `git branch --list --format='%(worktreepath)'` — gives the same fact but adds a second ref-walk
  and a second parser for no benefit. Rejected.

---

## R2 — Creating a worktree on a branch that already exists

**Decision**: `git worktree add <path> <branch>` — positional branch, no `-b`.

**Rationale**: This is git's own "check out this existing branch into a new worktree" form. It is
a single command, so it keeps the existing shape of `worktree_add_new_branch` (one call that both
binds the branch and materializes the directory) and therefore the existing rollback plan applies
unchanged apart from the branch-deletion step (R5). Git itself enforces the "already checked out"
refusal, so R1's check is a *better error message*, not the only guard — a race between pre-flight
and execution still fails safely.

**Alternatives considered**: `git worktree add --detach <path> <branch>` then `git switch` inside
the new worktree — two steps, a transient detached state, and a wider failure window. Rejected.

---

## R3 — Overwriting an existing branch

**Decision**: `git worktree add -B <branch> <path> HEAD`.

**Rationale**: `-B` is `-b`'s force variant: create the branch, or reset it to the start point if
it already exists, then check it out — in one command. Because pre-flight has already established
that the branch is not checked out anywhere (R1), the reset cannot be refused for that reason. The
start point stays the literal `HEAD` used by today's `worktree_add_new_branch`, so "overwrite"
lands at exactly the same commit a conflict-free create would have (FR-006). Using one command
rather than `git branch -f` + `git worktree add` removes the window in which the old branch is
already destroyed but no worktree exists yet.

**Alternatives considered**:
- `git branch -D <branch>` then `git worktree add -b <branch> …` — delete-then-create widens the
  destructive window and, if the add fails, leaves the user with neither the old branch nor a
  worktree, having run *two* mutations to get there. Rejected. (`-B`'s single failure mode is
  narrower, and US2 AS4 already covers reporting it.)
- Preserving the old tip somewhere (a backup ref or the reflog) — beyond the spec, which states
  plainly that overwrite discards commits after an explicit confirmation. Note that git's own
  reflog still records the pre-reset tip; this is worth mentioning in the docs but is not a
  product feature and must not be presented as an undo.

---

## R4 — Continuing from a branch that exists only on a remote

**Decision**: `git worktree add --track -b <branch> <path> <remote>/<branch>`, with the remote
named explicitly by the user's selection.

**Rationale**: Creates the local branch at the remote branch's tip and sets its upstream in one
command (FR-017). Naming the remote explicitly is what makes the multi-remote edge case behave —
the spec requires the user to pick when the same name exists on more than one remote, and this
form has no ambiguity to resolve. It is also strictly offline: `--track` writes local config and
positions a local ref from a ref already on disk; it does not contact the remote.

**Alternatives considered**:
- Relying on git's DWIM (`git worktree add <path> <branch>` inventing a tracking branch when the
  name exists on exactly one remote) — behavior depends on `worktree.guessRemote`/git version and
  is silent when it fires. Explicit beats implicit here, especially for a step that decides where
  a later `push` goes. Rejected.
- `--guess-remote` — same objection, and its name advertises the guessing. Rejected.
- Fetching first so the tip is current — **forbidden by FR-020**; disclosed staleness instead
  (R7). Rejected.

---

## R5 — Rollback must not delete a branch the operation did not create

**Decision**: `rollback_plan()` becomes `rollback_plan(mode: CreateMode) -> Vec<CleanupStep>` and
omits `CleanupStep::BranchDelete` for `CreateMode::ReuseLocal`. All other steps, and their order,
are unchanged for every mode.

**Rationale**: This is the single most dangerous interaction in the feature. Today's unconditional
plan (`WorktreeRemove → WorktreePrune → BranchDelete → RemoveDir`) exists because the branch was
always freshly created by the failed attempt. Under reuse that same plan would destroy the user's
pre-existing commits *as a consequence of a failure they did not cause* — the exact outcome
FR-008 and SC-003 forbid. Making the plan a function of the mode keeps the decision in one pure,
directly-testable place rather than scattering `if` guards through `run_rollback`.

Per-mode rationale:

| Mode | `BranchDelete` in plan? | Why |
|---|---|---|
| `NewBranch` | yes | Unchanged from today; the attempt created the branch. |
| `ReuseLocal` | **no** | The branch predates the attempt and must survive it (FR-008). |
| `Overwrite` | yes | The branch at this point is the *new* one the attempt created; the old tip is already gone and deleting the new branch leaves no junk behind. |
| `TrackRemote` | yes | The attempt created the local branch. The remote branch is untouched by `branch -D`. |

**Alternatives considered**: keeping one plan and making `branch_delete` refuse when the branch
predates the call — requires `create_worktree` to remember a pre-image and pushes a policy
decision into the I/O boundary, where it cannot be unit-tested as cleanly. Rejected.

---

## R6 — Enumerating branch candidates for the picker

**Decision**: one call — `git for-each-ref --format=%(refname) refs/heads refs/remotes` — parsed
by a new pure function. Drop any `refs/remotes/<remote>/HEAD` (a symbolic alias, not a branch).
Local refs become `BranchOrigin::Local`; remote refs split on the first path component after
`refs/remotes/` into `BranchOrigin::Remote { remote }` plus the branch name.

**Rationale**: One command, one parse, stable machine-readable output (unlike `git branch -a`,
whose output carries decoration such as the `*` marker and `->` alias arrows and is explicitly
not for scripts). Keeping the parser pure mirrors `parse_worktrees()` and makes the whole listing
testable from canned strings. `for-each-ref` reads local ref storage only — no network (FR-020).

A branch present both locally and on a remote collapses to a single **local** candidate, which is
what FR-019 demands: reuse and overwrite act on the local branch.

**Alternatives considered**:
- `git branch -a --format=…` — same data, but the `-a` listing needs extra filtering for the
  `HEAD ->` alias and offers nothing `for-each-ref` doesn't. Rejected.
- Two calls (`refs/heads` then `refs/remotes`) — needless second process spawn. Rejected.

---

## R7 — Remote staleness disclosure

**Decision**: no fetch, ever, in this flow. Remote-origin candidates render with their remote name
and the list carries a persistent one-line note: remote branches reflect the last fetch.

**Rationale**: FR-020 and Constitution Principle IV (local-first, fully functional offline). A
fetch would also make opening a form block on the network — a UX regression on top of a principle
violation. Disclosure is the honest alternative to silently showing possibly-stale data, and it
sets the user's expectation before they act on it (FR-020 second clause).

**Alternatives considered**: an explicit "Refresh from remotes" button — defensible as an explicit
user-initiated action under Principle IV, but it is scope the spec placed out of bounds
("Fetching or refreshing remotes from within this flow is out of scope"). Deferred, not rejected
on principle.

---

## R8 — Presenting unavailable branches in the picker

**Decision**: unavailable candidates stay in the list and stay *selectable*, labelled with their
reason (e.g. `main · in use by the project checkout`). Selecting one shows the blocked explanation
and disables Create; it never starts an operation.

**Rationale**: This is the one place Principle VIII was actively traded against a requirement.
FR-012 wants unavailable branches visible with a reason; the natural reading is a disabled list
row, but `Select` wraps iced's `pick_list`, which has no per-item disabling. The three ways out
were: (a) fork a bespoke list widget, (b) extend `Select` with a per-item disabled predicate, or
(c) allow selection and block at the point of action.

(c) wins. It satisfies FR-012 (visible, reasoned) *and* FR-021/US5 (the explanation is shown
exactly when the user reaches for the branch, which is when they need it) with zero new widget
surface — no fork, no `pick_list` reimplementation. It also degenerates gracefully if a branch
becomes checked out between listing and submit, since the same block is re-derived at pre-flight
(FR-009).

**Alternatives considered**:
- (a) Fork a list widget — squarely what the Component-reuse gate exists to reject. Rejected.
- (b) Extend `Select` with disabling — would mean reimplementing `pick_list`'s overlay to control
  row interactivity, i.e. (a) wearing a builder API. Rejected for this feature; if a second call
  site ever needs it, it belongs in `Select` as a proper builder method.
- Hiding unavailable branches entirely — explicitly forbidden by FR-012, and it produces the
  "where did my branch go?" confusion US5 exists to remove. Rejected.

---

## R9 — Where the conflict decision lives in application state

**Decision**: inside `WorktreeForm`, as a `resolution: ResolutionState` sub-state machine
(`Idle → Choosing(BranchSituation) → ConfirmingOverwrite(BranchSituation) → Idle`), not as a new
`Overlay` variant.

**Rationale**: FR-007 requires that cancelling at either step return the user to the create form
**with their entered values preserved**. `Overlay` is a single-slot enum — the app can show one
modal at a time — so routing the choice through a new overlay variant would tear down the
`AddWorktree` overlay and, with it, the form state that must survive. Keeping the decision inside
the form makes "cancel restores everything" the default rather than something to reconstruct, and
keeps the whole state machine in the render-free core where it is unit-testable (Principle I).

**Alternatives considered**: a nested/stacked overlay stack — a structural change to `Overlay`
affecting every dialog in the app, to serve one flow. Rejected as unjustified complexity.

---

## R10 — Deriving the worktree directory for a selected existing branch

**Decision**: a new pure `naming::dir_name_from_branch(branch) -> String`: split on `/`, `slugify`
each segment, drop empties, join with `-`.

**Rationale**: It is the inverse of the mapping `derive()` already produces — `feat/abc-123-login`
→ `feat-abc-123-login` — so a worktree created from a *selected* branch lands in the same
directory it would have had if the user had typed the inputs (FR-014), and the sidebar's existing
`parse_tags`/`display_name` derivation keeps working with no special case. Routing through the
existing `slugify()` inherits its guarantees for free: `[a-z0-9-]` only, collapsed separators, and
the Windows reserved-device-name guard (Principle VI).

Two branches can in principle collapse to the same directory name (`feat/a-b` and `feat/a/b`).
That is not a new hazard: the existing duplicate-directory pre-flight catches it and reports the
clash before anything is created (FR-022).

**Alternatives considered**: storing the branch on the `Worktree` and skipping derived directory
naming — would break the established convention that `dir_name` is the identity used by sessions,
renames, and tag parsing. Rejected.

---

## R11 — Splitting pre-flight from execution without a TOCTOU hole

**Decision**: extract `preflight()` as a pure-over-`Git` function returning `BranchSituation`.
`create_worktree` calls it **again** as its own first step and aborts with
`CreateError::SituationChanged` when the fresh situation is incompatible with the `CreateMode` it
was given.

**Rationale**: The user's answer is necessarily separated from the act by human think-time, during
which a terminal in another window can create, delete, or check out the branch. FR-009 requires
re-verification at the moment of action. Re-running the same function — rather than writing a
second, subtly different check — means the two can never disagree, and the compatibility rule
(`ReuseLocal` needs an available local branch, `TrackRemote` needs that same remote-only branch,
`Overwrite` needs an existing non-checked-out local branch, `NewBranch` needs a free name) is one
small pure predicate to test exhaustively.

This does not make the operation atomic — nothing short of a repository lock would — but the
failure mode is a clean abort before mutation, plus git's own refusal as the backstop (R2).

**Alternatives considered**: passing the pre-flight result forward as a token and trusting it —
precisely the stale-information behavior FR-009 forbids. Rejected.

---

## R12 — Progress stages for the new paths

**Decision**: extend `CreateStage` with `CheckingBranch` is *not* needed — the existing
`PreflightCheck` covers it — but `CreatingWorktree`'s label becomes mode-dependent
("Creating branch and worktree" / "Checking out existing branch" / "Replacing branch and creating
worktree" / "Creating tracking branch and worktree").

**Rationale**: FR-024 asks that reporting name the step being performed. The stage *set* is
unchanged (same four phases in the same order), so the existing progress plumbing — the
`CreateProgressEvent` channel, the 150ms poll, `StageProgress` — needs no structural change; only
`CreateStage::label()` grows a parameter. Keeping the enum closed and the label a pure function of
`(stage, mode)` preserves the Principle V guarantee that an unreachable stage cannot be displayed.

**Alternatives considered**: new stage variants per mode — multiplies a closed enum by four for
labels alone, and every consumer would have to treat the variants as equivalent anyway. Rejected.
