# Contract: the three user-facing surfaces

Everything below is composed from existing shared components (research R12, Principle VIII). Each
labelled item is what the quickstart's visual pass checks.

## D — Run in parallel dialog (FR-001 to FR-003)

Opened from the sidebar header's **Run in parallel** action, beside **New worktree** (no project menu offers New worktree, so the header action is the placement). A `Modal` + `dialog::body`.

- **D1** Heading "Run in parallel"; fields in order: prompt (`TextArea`, multi-line, required),
  type (`Select`, the Conventional types, required), ticket (`TextField`, optional), name
  (`TextField`, required), base branch (`Select`, local branches from `ClientMsg::BranchList`,
  defaulting to the project root's current branch).
- **D2** A **Runs** section listing one row per run: `#<n>`, a provider `Select` offering only the
  CLIs available where sessions run (`State::offered_providers`), and a remove `IconButton`
  (disabled while the count is `MIN_RUNS`). An **Add run** `Button`, disabled at `MAX_RUNS`. A new
  row defaults to the user's default AI CLI (FR-002).
- **D3** Under the list, the derived names each run will get, in order, as read-only text:
  `feat/login-page-1 → .claude/worktrees/feat-login-page-1` … (FR-003).
- **D4** Confirm is **Start runs**; it is disabled while `validate()` fails, and the first failing
  rule is shown as the field's error text: empty prompt, missing type, empty name, a run count
  outside `MIN_RUNS..=MAX_RUNS`, an unavailable provider, an invalid derived name (US1 s5).
- **D5** Confirming closes the dialog at once; the group appears in the sidebar with its runs
  `Creating`. Nothing is created when the dialog is dismissed.

## G — The group in the sidebar (FR-007, FR-008, FR-020)

- **G1** One group row above its runs, at the worktree level, showing the group's name, a run-count
  tag (`3 runs`) and, when any run failed, a failed-count tag (`1 failed`). The runs sit one level
  in, each row showing `#<n>`, its provider and its worktree's usual name and tags; their sessions
  sit one level further in, unchanged.
- **G1a** A run with no worktree (failed while creating it) is still listed under its group, as a
  row reading `#<n>`, its provider and a `failed` tag, with the failure reason as its tooltip and no
  worktree menu; it leaves the group only with the group (FR-005, US1 s3).
- **G2** The group row is expandable (`TreeItem::expandable`); collapsed, it still shows both counts
  (US2 s2).
- **G3** Runs of a group never also appear as ungrouped worktree rows; unrelated worktrees are
  unaffected and keep their position (US2 s1).
- **G4** The group row's menu has **Compare**, **Dismiss group** and nothing else; a run row keeps
  the ordinary worktree menu (Review changes, Delete, …).
- **G5** **Dismiss group** asks for confirmation naming that the worktrees, branches and sessions
  stay, then removes only the grouping (FR-020, US2 s6).
- **G6** After a restart the group, its runs and their order are as they were (FR-008, US2 s3).

## C — Compare (FR-009 to FR-011, FR-014 to FR-017)

A main-area view, opened from the group row, in the place the Changes view occupies.

- **C1** Header: the group's name, its base branch, its prompt (clamped, with the full text as the
  row tooltip), and a close action.
- **C2** One row per run in run order: `#<n>`, the provider's display name, the status, `<files>
  files`, `+<added>`, `−<removed>`. Status text: Creating, Starting, Working, Waiting for input,
  Prompt not delivered, Failed, Picked — the first two from the run status, Working and Waiting for
  input from the run's session (Assumptions), the rest from the run status.
- **C3** A failed run shows its reason in place of the counts, and offers no **Open diff** when it
  has no worktree (US3 s5).
- **C4** Each row has **Open diff**, which opens that run's Changes view (FR-011, US3 s4).
- **C5** Counts refresh within 2 s of a change in a run's worktree, without reopening Compare
  (FR-010, US3 s3); a run with uncommitted changes carries an `uncommitted` tag.
- **C6** Each eligible row has **Pick this one**. It is absent for a run with no branch and disabled
  for every row while any run is `Creating` or `Starting`, with the reason as its tooltip (FR-017,
  US4 s7). After a pick, no row offers it and the winner's row says Picked (Edge "Picked group").
- **C7** Pressing **Pick this one** on a run whose session is still working first asks for
  confirmation saying so (FR-017, US4 s8).
- **C8** A refused pick is reported as a snackbar or dialog carrying the refusal's own text: the
  uncommitted files (US4 s9), the conflicting files (US4 s2), git's message when the base branch is
  busy, or "this group already has a winner" (US4 s6). Nothing in the view changes.

## K — The cleanup offer (FR-014 to FR-016)

Shown once, right after a successful pick, as a `Modal`.

- **K1** Heading names what happened: "Run 2 was merged into main" or "… fast-forwarded main".
- **K2** One `Checkbox` row per loser: its name, and what removing it deletes — its worktree folder,
  its sessions (with their count), and its branch. A per-loser **Delete the branch too** checkbox,
  on by default.
- **K3** A loser with uncommitted changes carries an `uncommitted changes` tag and starts
  **unselected** (FR-015, US4 s4). Each loser's state is read fresh when the offer opens; until it is
  known, or when the read fails, the loser is treated as having uncommitted changes.
- **K4** Confirm first re-reads the selected losers, then removes exactly the losers left selected, by sending the existing
  `WorktreeDelete { stop_sessions: true, delete_branch }` per loser (FR-016, research R13).
- **K5** A selected loser with uncommitted changes triggers a second confirmation before anything is
  removed, naming it and that it holds uncommitted changes; declining that leaves it alone and
  removes the rest (FR-015, SC-005).
- **K6** Dismissing the offer removes nothing and leaves every loser in the group (US4 s5). The offer
  does not come back; the user deletes a loser from its own row afterwards.
