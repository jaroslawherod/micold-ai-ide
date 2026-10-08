# Research: Run One Prompt Across Several Agents and Pick the Best Result (#483)

Each decision below resolves a technical unknown of [spec.md](./spec.md). Nothing here contacts a
remote (Principle IV); every git read and write is local.

## R1 — How a pick integrates the winner's branch into the base branch

**Decision**: never check anything out on the user's behalf and never rewrite history. The pick runs
in three steps, all through a new `micold_core::runs::integrate` module over the `Git` trait:

1. **Conflict pre-check, mutating nothing**: `git merge-tree --write-tree <base> <run branch>`.
   Exit 0 gives the merged tree id; a non-zero exit lists the conflicted paths on stdout
   (`parse_merge_tree_conflicts`). On conflict the pick refuses with those paths and stops before
   anything is written (FR-013, SC-006).
2. **Fast-forward when the base tip is an ancestor of the run tip** (`git merge-base --is-ancestor`):
   the base branch gains no commit of its own.
3. Otherwise a **merge commit** `git commit-tree <tree> -p <base tip> -p <run tip> -m "<message>"`.

How the new tip reaches `refs/heads/<base>` depends on whether the base branch is checked out:

- **Not checked out anywhere** (the common case for a group whose base is `main` while the user
  works in worktrees): `git update-ref refs/heads/<base> <new tip> <observed old tip>`. The old value
  makes it a compare-and-swap: a base branch that moved between the pre-check and the write fails the
  update and the pick refuses rather than clobbering the move.
- **Checked out** (the project root, or another worktree; found with the existing
  `worktree_list_porcelain` + `worktree::parse_worktrees`): the ref cannot move behind a working
  tree, so the merge runs in that checkout: `git -C <checkout> merge --no-edit --no-stat
  <run branch>`. git itself refuses when local changes or untracked files would be overwritten and
  changes nothing; its message is reported verbatim (FR-013, Edge "Base branch checked out with
  uncommitted changes"). A merge left in progress by an unforeseen failure is undone with
  `git -C <checkout> merge --abort`, so the refusal still changes nothing.

**Rationale**: `merge-tree --write-tree` is git's own merge, evaluated without a working tree, so
the "nothing changes on conflict" guarantee (SC-006) is a property of the algorithm rather than of
cleanup code. `update-ref` with an expected old value is the only atomic branch write git offers.
Running `git merge` in an existing checkout is what makes the result identical to the merge the user
would have run by hand, including the index and working tree they are looking at.

**Minimum git**: `merge-tree --write-tree` needs git ≥ 2.38 (2022-10). The project already assumes a
modern git (`micold-core::review` was written against 2.43, CI runs 2.43+). A git that does not
support it fails the pre-check; the pick is then refused with "this git cannot check the merge
(needs git 2.38 or newer)" and nothing is changed. Recorded in the user guide.

**Alternatives rejected**:

- *A temporary worktree on the base branch, `git merge` there, then remove it*: git refuses a second
  worktree on a branch already checked out, so it needs the checked-out path anyway; it also writes
  a whole tree to disk per pick and leaves a worktree behind when the process dies.
- *`git merge` in the run's own worktree, then move the base ref*: moves the run's branch, which
  FR-012 forbids.
- *Rebase or `pull --rebase`*: rewrites the run's branch (FR-012 forbids), and the user asked for a
  merge (Clarifications).
- *`git fetch . <run>:<base>`*: fast-forward only, so it cannot satisfy the merge-commit half, and
  its refusal message is about refspecs rather than about the user's branches.

## R2 — Where a group lives and who owns it

**Decision**: the daemon owns groups, one record per project, persisted at
`runs/<project id>.json` beside `reviews/` and `projects/` through new `ProjectStore` methods
(`load_runs`, `save_runs`, `remove_runs`), exactly as feature 482's review file is. Every accepted
change is written before memory changes, then pushed to every attached window as
`DaemonMsg::RunGroupsChanged { project, groups }`.

**Rationale**: FR-008 wants one truth shown alike in every window and restored after a restart, and
FR-018 wants at most one pick across windows — both need a single writer, which is the daemon
(the pattern feature 482 established and its `Reviews` state proves out). A per-project file keeps a
forgotten project's state removable in one unlink (`remove_runs`, as `remove_reviews`).

**Alternatives rejected**: client-local state (two windows would disagree, nothing survives a
restart); the catalog file (the catalog is refreshed from git discovery, and a group is not
discoverable from git); a run marker inside each worktree (a dismissed group, a deleted worktree or a
half-created run leaves the grouping inconsistent, and it writes inside the user's repository).

## R3 — Starting N runs without one failure stopping another

**Decision**: the daemon spawns one tokio task per run, each running the existing pipeline
(`ops::create_worktree` → `ops::create_session_with_prompt`), and reports each run's status
transition through `RunGroupsChanged`. `RunGroupCreate` is answered `OperationOk(RunGroupCreated)`
as soon as the group record exists, before the runs finish; the runs then report themselves.

**Rationale**: FR-005 requires independence "whatever order the runs finish in", and the prompt step
waits up to the first-prompt bound (`DaemonState::first_prompt_bound`, tens of seconds), so a
sequential loop would make run N wait for run 1's provider. Worktree creation is already serialized
per project by `DaemonState::worktree_gate` inside `ops::create_worktree`, so concurrent tasks cannot
interleave two `git worktree add` calls — the per-run concurrency costs no new locking.

**Alternatives rejected**: one task for the whole group (couples the runs); creating all worktrees
first and then starting sessions (a late session failure still leaves the others unprompted longer,
and nothing is gained since the gate serializes creation anyway).

## R4 — Delivering the prompt

**Decision**: reuse `ops::create_session_with_prompt(state, NewSession { … }, Some(FirstPrompt {
text, asked, require_bracketed: true }))` unchanged. Its `FirstPromptUndelivered` variants are
exactly the spec's failure vocabulary: `NotStarted`, `AsksTrust` (Edge "Provider not installed" /
folder trust), `NotReady` (US1 s4), `Typing`.

`require_bracketed: true` because a group prompt is one text the user typed into a multi-line field
and a non-bracketed terminal would split it into several submissions; the run is then marked "prompt
not delivered" with that reason and keeps its session, which is what US1 s4 asks for.

**Rationale**: FR-004 wants the prompt delivered exactly once as a new session's first input, and
that path (feature 034, hardened by 482 W7–W9) already enforces readiness, the trust check and the
single submission. Reusing it is also what keeps the behaviour identical to Send to session
(Assumptions).

**Alternatives rejected**: writing to the pty directly (duplicates the trust and readiness rules);
`type_submission` after a plain start (it does not wait for the CLI to be ready, so the prompt would
land in a provider that is not listening).

## R5 — Deriving the runs' branch and folder names

**Decision**: a pure `runs::naming::derive_group(naming: &WorktreeNaming, count: u8) ->
Result<Vec<DerivedNames>, NamingError>` that, for run `n`, calls the existing `naming::derive` with
`name = format!("{name} {n}")`. `slugify` turns the space into `-`, so `feat` + `login page` gives
`feat/login-page-1` … `feat/login-page-N` with dir names `feat-login-page-1` … (Assumptions, FR-003).

Case-insensitive collisions need no extra rule: `slugify` lowercases, so two names differing only in
case derive the same branch and folder and collide identically on every platform (Edge "Name
collisions").

**Rationale**: one derivation function keeps the group's names subject to the same validation,
reserved-name and cross-platform checks the New worktree form already passes (`naming.rs`,
`is_valid_branch`), which is what FR-003 and the cross-platform edge case require. Appending to the
*name* part rather than to the branch keeps the ticket boundary (`TICKET_SEP`) intact, so
`dir_name_from_branch` still recovers the ticket of a re-picked branch.

**Alternatives rejected**: appending `-<n>` to the derived branch string (bypasses validation and can
break the ticket boundary); a random suffix per run (the issue asks for visibly related names).

## R6 — How many runs

**Decision**: `runs::MAX_RUNS: u8 = 8`, `DEFAULT_RUNS: u8 = 2`, with `MIN_RUNS: u8 = 2`. Validated in
the pure dialog state (so the user is told in the dialog, FR-002) and again in the daemon on
`RunGroupCreate` (so a stale or hand-built frame cannot exceed it).

**Rationale**: the spec fixes the numbers (Assumptions). Validating twice follows the project's
existing shape: the form decides what the user may ask for, the daemon decides what it will do.

**Alternatives rejected**: no upper cap (a slip of the keyboard could start dozens of agents and
worktrees at once); a cap read from a setting (a new setting for a number the spec fixes, with no
user asking for it); validating in the daemon only (the user would learn of a bad count only after
submitting, against FR-002).

## R7 — Compare's file and line counts

**Decision**: reuse feature 482's core reader. For each run, `GitCli::change_list(run root,
ReviewScope::Worktree { base: MergeBase { branch: <group base>, commit: merge-base(HEAD, group base)
} }, Toggles { committed: true, uncommitted: true })`, and sum `ChangedFile` rows into
`RunSummary { files, added, removed, uncommitted }`. Binary files count as changed files and
contribute no lines (`ChangedFile::lines == None`), which is the Changes view's own rule (Edge "Large
or binary changes").

**Spec tension, recorded rather than resolved in code**: FR-009 says the counts are "against the base
branch" (the group's base), while FR-010 says they equal the run's Changes view totals with both
toggles on. The Changes view's base is the merge-base with the repository's *default* branch
(feature 482, R7). The two agree whenever the group's base branch is the default branch — the
dialog's own default and the case US3 s2 describes. When the user picks another base branch, Compare
follows FR-009 (the group's base), because counts against a branch the run never left would include
the base branch's own commits and misreport the run's work. The user guide says which base Compare
compares with; a follow-up may align FR-010's wording.

**Alternatives rejected**: a second, independent counter (two ways to count the same files is how
FR-010 gets broken); reading the counts from the Changes view's client state (Compare must work for
a run whose Changes view was never opened).

## R8 — Deciding whether a run holds uncommitted changes

**Decision**: one reader, `GitCli::change_list(run root, scope, Toggles { committed: false,
uncommitted: true })`; a non-empty list means uncommitted changes. Used for three separate rules: the
pick refusal (FR-012), the loser marking in the cleanup offer (FR-015), and `RunSummary.uncommitted`
in Compare.

**Rationale**: the pick's refusal and the cleanup's second confirmation must agree with what the user
sees in Compare, so they come from the same answer. It also already handles untracked-not-ignored
files, which `git status --porcelain` would too but `diff --quiet` would not.

**Alternatives rejected**: `git status --porcelain` parsed separately (a second parser for a question
the review module already answers); trusting the last Compare refresh (racy: the user commits in the
run's own terminal between the refresh and the pick, which US4 s9 explicitly expects to work).

## R9 — One pick per group, across windows

**Decision**: the pick is serialized per project by the existing `DaemonState::worktree_gate(project)`
and refuses when `group.winner.is_some()` after taking the gate. The winner is written to the runs
file before the integration's ref write is reported, and `RunGroupsChanged` tells every window.

**Rationale**: FR-018 and US4 s6. The worktree gate is the lock the project's worktree mutations
already take, and a pick is one of those (it can be followed by loser deletes), so reusing it also
keeps a pick from interleaving with a create of the same project.

**Alternatives rejected**: an advisory flag in the client (loses the race the scenario names); a new
per-group mutex (a second lock ordering against the worktree gate, for no gain).

## R10 — Runs interrupted by the app closing

**Decision**: a run's status is persisted. On the daemon's first read of a project's runs file, any
run persisted as `Creating` or `Starting` becomes `Failed { step, reason: "interrupted" }`, and a
`Creating` run is reconciled on disk: when its worktree directory exists, is registered as
app-created provenance and hosts no session, it is removed exactly as a rolled-back create would
remove it (`worktree::remove_worktree` + directory removal + branch delete), and its branch is
deleted when it exists and the removal succeeded. Anything else is left alone and reported in the
run's reason.

**Rationale**: FR-019 and US2 s5. `ops::create_worktree` rolls back within the process; only a kill
mid-create can leave a half-created pair behind, and the provenance record plus "no sessions" is what
distinguishes it from a worktree the user means to keep.

**Alternatives rejected**: cleaning up at shutdown (a kill has no shutdown); leaving the leftovers
and only marking the run failed (the spec requires no half-created branch or folder remains).

## R11 — Keeping Compare fresh within 2 seconds

**Decision**: reuse feature 482's watcher shape. While a group's Compare view is open the client runs
one subscription per run root (`shell/runs_watch.rs`, built on the existing
`review::watch::{relevant_paths, Debouncer}` and `notify::RecommendedWatcher`, 300 ms debounce), and
each wake re-reads only that run's summary in `spawn_blocking`. Status changes arrive through the
catalog and `RunGroupsChanged`, not the watcher.

**Rationale**: FR-010's 2 s is already met by this exact mechanism in the Changes view, and reusing
it keeps one filter (the one that ignores `.git/objects` churn) rather than two.

**Alternatives rejected**: polling every run every second (N git reads per second per window);
watching the project root once (a run's worktree lives under `.claude/worktrees/`, and the filter
would have to re-derive which run each event belongs to).

## R12 — UI: no new shared primitive

**Decision**: build the three surfaces from the existing shared components (Principle VIII):

- **Run in parallel dialog**: `Modal` + `dialog::body`, the existing worktree-form fields
  (type `Select`, ticket and name `TextField`), a `TextArea` (482) for the prompt, a base-branch
  `Select` fed by the existing `ClientMsg::BranchList`, and one row per run with a provider `Select`
  and an `IconButton` to remove it, plus an **Add run** `Button`.
- **Sidebar group**: a new `SidebarEntry::Group(GroupNode)` rendered through the existing `TreeView`
  with the runs as its children and their sessions one level deeper; the group row carries its name,
  a run-count `Tag` and a failed-count `Tag`, and uses `TreeItem::expandable`.
- **Compare**: a main-area view beside the Changes view, rows built from `Text`, `Tag` and `Button`;
  at most 8 rows, so no virtualisation.

**Rationale**: every element has a shared primitive already; adding a bespoke "run row" widget would
be exactly what the component-reuse gate rejects. The sidebar's third level is the one structural
change, and it lands in `features/sidebar.rs`'s pure tree and `row_heights`, both already tested.

**Alternatives rejected**: a new `ComparisonTable` component (one call site, no second consumer); a
separate window for Compare (the app is single-window-per-project by convention and the spec puts
Compare in the group).

## R13 — Wire shape

**Decision**: protocol 35 → 36. New `ClientMsg::{RunGroupCreate, RunGroupPick, RunGroupDismiss}`,
`DaemonMsg::RunGroupsChanged`, `OperationResult::{RunGroupCreated, RunPicked}`, and the group types
in `micold_core::runs`. Cleanup of the losers is **not** a new message: the client sends the existing
`ClientMsg::WorktreeDelete { stop_sessions: true, delete_branch }` per loser the user left selected.

**Rationale**: FR-016 asks that removing a loser behave exactly as the existing Delete does, and the
existing Delete already stops sessions, archives them, drops provenance and review comments and
broadcasts. Reusing the message is how that stays true without a second implementation. A group whose
run loses its worktree drops that run through the same catalog refresh (FR-007, US2 s4).

**Alternatives rejected**: a `RunGroupCleanup` message that deletes the selected losers in the daemon
(duplicates delete semantics and its confirmations; the second uncommitted-changes confirmation is a
client-side dialog either way).
