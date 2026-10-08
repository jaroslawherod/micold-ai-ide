# Feature Specification: Run One Prompt Across Several Agents and Pick the Best Result

**Feature Branch**: `claude/project-thread-wysm57`

**Created**: 2026-10-08

**Status**: Draft

**Input**: User description: "Implement GitHub issue #483: Run one prompt across several agents in
parallel worktrees and pick the best result. Problem: trying the same task with different AI CLIs
(or the same CLI several times) and picking the best result takes a lot of manual setup: create
several worktrees, start a session in each, paste the same prompt into each, then compare the
results by hand. Proposal: add a Run in parallel action. The user enters one prompt, picks a base
branch and chooses N runs, each with a provider (`claude`, `copilot`, `pi`). The app creates N
worktrees from the base with related branch names (for example `<name>-1..N`), starts a session in
each and sends the prompt. A Compare view lists the runs with their status, their changed files and
line counts, and opens each run's diff (see #482). Pick this one merges or rebases the chosen
branch into the base branch and offers to remove the other worktrees and branches. Acceptance
criteria: all N worktrees and sessions are created from one dialog, and a failure in one run does
not stop the others; the runs are grouped together in the sidebar; picking a winner never deletes a
worktree with uncommitted changes without asking. Depends on #482 for the compare step."

## Terms

- **Provider**: an AI CLI a session runs: Claude Code (`claude`), GitHub Copilot CLI (`copilot`) or
  Pi Coding Agent (`pi`).
- **Parallel run group** (group): the set of runs started together from one Run in parallel
  dialog, sharing one prompt, one base branch and one name.
- **Run**: one member of a group: its own worktree, its own branch, its own session and its
  provider.
- **Base branch**: the existing branch every run's branch starts from, and the branch the picked
  run is integrated into.
- **Changes view**: the per-worktree view of feature 482 (Reviewing a worktree's changes), which
  lists the files a worktree changed against its base branch and shows their diffs.
- **Pick**: choosing one run as the winner: integrating its branch into the base branch, then
  optionally removing the other runs.
- **Loser**: every run of a group other than the picked one.

## Clarifications

### Session 2026-10-08

- Q: When the picked run has uncommitted changes, what does the pick do? → A: It refuses the pick,
  naming the uncommitted files, until the user commits them; nothing changes. _(default, pending
  user confirmation)_
- Q: How does Pick this one integrate the run's branch into the base branch? → A: Always a merge:
  a fast-forward when the base branch has not moved since the run's branch left it, otherwise a
  merge commit; the run's branch is never rewritten. _(decided by user)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start N runs from one dialog (Priority: P1)

A user opens **Run in parallel**, types a prompt and a name, picks the base branch, and sets up N
runs, choosing a provider for each (for example two `claude` and one `copilot`). On confirming, the
app creates N worktrees from the base branch with related branch names, starts a session with the
chosen provider in each, and sends the prompt to every session. A run that fails at any step is
marked failed with its reason; the others carry on.

**Why this priority**: It removes the manual setup the issue describes and is the precondition of
every other story.

**Independent Test**: With two providers installed, start a group of 3 runs from the dialog and
check that 3 worktrees exist on branches derived from the one name, each starting at the base
branch's commit, each with one session of its chosen provider that received the prompt exactly
once. Then repeat with one run set to a provider that is not installed, and check the other runs
still complete.

**Acceptance Scenarios**:

1. **Given** a project with base branch `main`, **When** the user confirms a group named
   `login page` with 3 runs, **Then** 3 worktrees are created on 3 distinct branches derived from
   `login page` and numbered 1 to 3, each starting at `main`'s current commit.
2. **Given** the 3 worktrees exist, **When** the group is started, **Then** each worktree has one
   session of the provider chosen for its run, and each session receives the prompt as its first
   input exactly once.
3. **Given** a group of 3 runs where run 2's worktree cannot be created (for example its branch name
   is already taken), **When** the group is started, **Then** runs 1 and 3 are created, started and
   prompted, and run 2 is marked failed with the reason, leaving no half-created branch or folder.
4. **Given** a run whose session starts but whose provider never becomes ready for input, or would
   ask about trusting the folder, **When** the group is started, **Then** that run keeps its
   worktree and session, is marked "prompt not delivered" with the reason, and the other runs are
   unaffected.
5. **Given** the dialog, **When** the user leaves the prompt or name empty, sets fewer than 2 or
   more than the maximum number of runs, or picks a provider not available where sessions run,
   **Then** the dialog says what is wrong and nothing is created.

---

### User Story 2 - See the runs as one group (Priority: P1)

The runs of a group appear together in the sidebar under one group row carrying the group's name,
so the user can tell at a glance which worktrees belong to the same experiment and how each is
doing.

**Why this priority**: Grouping is an acceptance criterion of the issue; without it N similar
worktrees are mixed in with everything else.

**Independent Test**: Start a group of 3 runs next to 2 unrelated worktrees, and check the sidebar
shows one group row with the 3 runs under it and the 2 others outside it; restart the app and check
the grouping is unchanged.

**Acceptance Scenarios**:

1. **Given** a group of 3 runs and 2 unrelated worktrees, **When** the user looks at the sidebar,
   **Then** the 3 runs are listed together under one group row named after the group, each run
   showing its number and its provider, and the 2 unrelated worktrees are outside the group.
2. **Given** a group, **When** the user collapses its row, **Then** its runs are hidden and the row
   still shows how many runs it has and how many have failed.
3. **Given** a group, **When** the app restarts, **Then** the group, its runs and their order are
   restored.
4. **Given** a run's worktree is deleted through the normal Delete action, **When** the sidebar
   updates, **Then** that run leaves the group and the group keeps the rest; a group with no runs
   left disappears.
5. **Given** the app is closed while a group's runs are still being created, **When** it starts
   again, **Then** the runs that finished are intact, every unfinished run shows as failed
   ("interrupted"), and no half-created branch or folder of those runs remains (a run whose
   worktree was fully created before the app closed keeps it, like a run whose prompt was not
   delivered).
6. **Given** a group, **When** the user dismisses it, **Then** the group row goes away and its
   worktrees, branches and sessions stay as ordinary sidebar entries, unchanged.

---

### User Story 3 - Compare the runs (Priority: P2)

From the group the user opens **Compare**, which lists every run with its provider, its status, the
number of files it changed and its added and removed line counts against the base branch. From any
run the user opens that run's Changes view to read its diff.

**Why this priority**: Comparing is how the user decides which run to pick; it builds on the
Changes view of feature 482, which already exists.

**Independent Test**: In a group of 3 runs, make different changes in each worktree (committed and
uncommitted), open Compare, and check each run's file count and line counts equal those the Changes
view shows for that worktree, and that opening a run opens its Changes view.

**Acceptance Scenarios**:

1. **Given** a group of 3 runs, **When** the user opens Compare, **Then** it lists the 3 runs in
   order, each with its number, provider, status, changed-file count, added lines and removed lines.
2. **Given** a run changed 4 files with 120 added and 30 removed lines against the base branch,
   counting committed and uncommitted changes, **When** Compare shows it, **Then** it reads 4 files,
   +120, −30, the same totals the run's Changes view shows with both toggles on when the base
   branch is the repository's default branch.
3. **Given** Compare is open, **When** a run's files change, **Then** that run's counts update
   within 2 seconds without reopening Compare, and its status follows its session as the sidebar's
   does.
4. **Given** Compare is open, **When** the user opens a run's diff, **Then** that run's Changes view
   opens.
5. **Given** a failed run, **When** Compare shows it, **Then** it shows the failure reason instead
   of counts, and offers no diff when the run has no worktree.

---

### User Story 4 - Pick the winner and clean up (Priority: P2)

In Compare the user presses **Pick this one** on the best run. The app integrates that run's branch
into the base branch, then offers to remove the losing runs' worktrees and branches. A loser with
uncommitted changes is never removed unless the user explicitly confirms removing it.

**Why this priority**: It closes the loop the issue describes; without it the user still merges and
cleans up by hand.

**Independent Test**: In a group of 3 runs where run 2 has committed changes and run 3 has
uncommitted changes, pick run 2, accept cleanup, and check the base branch now contains run 2's
commits, run 1's worktree and branch are removed, and run 3 is kept unless its removal was
confirmed separately.

**Acceptance Scenarios**:

1. **Given** run 2 has committed changes and the base branch has not moved, **When** the user picks
   run 2, **Then** the base branch contains run 2's commits and the group records run 2 as the
   winner.
2. **Given** the integration would conflict, **When** the user picks a run, **Then** nothing is
   changed (base branch, run branch and every worktree are as before) and the user is told which
   files conflict.
3. **Given** a successful pick, **When** the cleanup offer appears, **Then** it lists every loser
   with what removing it deletes (worktree folder, its sessions and, optionally, its branch), and
   confirming removes exactly the losers left selected.
4. **Given** a loser with uncommitted changes, **When** the cleanup offer appears, **Then** that
   loser is marked as having uncommitted changes, is not selected, and is removed only if the user
   selects it and confirms a second time naming that it holds uncommitted changes.
5. **Given** the user declines the cleanup offer, **When** it closes, **Then** no worktree, branch
   or session is removed and the losers stay in the group.
6. **Given** two windows open on the same group, **When** both pick a run at the same time,
   **Then** exactly one run is integrated, and the other window is told the group already has a
   winner and changes nothing.
7. **Given** a run that is failed with no branch, or any run of the group still being created or
   started, **When** the user looks at Compare, **Then** Pick this one is unavailable for that run
   (or, while runs are still being created or started, for every run) with the reason shown.
8. **Given** a run whose session is still working, **When** the user picks it, **Then** the app
   asks for confirmation, saying the session is still working, before integrating anything.
9. **Given** the picked run itself has uncommitted changes, **When** the user picks it, **Then**
   the pick is refused with the reason and the list of uncommitted files, and nothing changes
   until the user commits them (for example in the run's session or terminal) and picks again.

---

### Edge Cases

- **Empty input**: an empty prompt, empty name, or a group of fewer than 2 runs cannot be
  confirmed; the dialog says which field is wrong.
- **Too many runs**: the dialog does not allow more runs than its maximum (Assumptions).
- **Name collisions**: a derived branch name or worktree folder that already exists fails only that
  run, with the same message the New worktree form gives; the dialog shows the derived names before
  confirming so the user can rename first. Collision checks treat names that differ only in letter
  case as the same name on platforms whose file systems do.
- **Base branch missing**: a base branch deleted between opening the dialog and confirming refuses
  the whole group with that reason; no group, run, worktree or branch is created.
- **Provider not installed**: a run whose provider is not available where sessions run is refused
  in the dialog; if it disappears between confirming and starting, only that run fails.
- **Prompt not delivered**: a run whose provider asks about trusting the folder or is not ready in
  time is marked "prompt not delivered" with the reason; its session stays, and the user can type
  into it.
- **App closed during creation**: on the next start, runs that were not finished are shown as failed
  ("interrupted"), no half-created branch or folder remains (a worktree whose creation completed is
  kept), and the finished runs are intact.
- **Two windows**: every window open on the project shows the same group as runs are created and
  change; two windows picking in the same group at once integrate exactly one run, and the other
  window is told the group already has a winner.
- **Session isolation (Principle II)**: each run's session runs only in its own worktree; nothing
  one run's session does is visible to another run's session except through the shared repository
  history once picked.
- **Base branch moved**: if the base branch gained commits since the group started, the pick still
  integrates the winner onto the current base branch tip, or reports conflicts and changes nothing.
- **Base branch checked out with uncommitted changes**: if the base branch is checked out somewhere
  (for example the project root) with uncommitted changes that integrating would overwrite, the
  pick is refused with that reason and nothing changes.
- **Winner with uncommitted changes**: the pick is refused, naming the uncommitted files, and
  nothing changes; the user commits them and picks again.
- **Run with no changes**: Compare shows 0 files, +0, −0; it can still be picked, and integrating it
  leaves the base branch unchanged.
- **Large or binary changes**: counts come from the same rules as the Changes view; binary files
  count as changed files with no line counts, and a large diff never freezes Compare.
- **Picked group**: after a pick, Compare shows the winner and offers no further pick; the group
  row stays until its runs are removed or the user dismisses the group, which removes only the
  grouping, never a worktree.
- **Cross-platform (Principle VI)**: derived branch names and folder names are valid on Linux,
  macOS and Windows (no characters or reserved names a platform rejects), and every behaviour above
  is the same on the three platforms.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The app MUST offer a **Run in parallel** action that opens one dialog taking a prompt,
  a name (with the New worktree form's type and optional ticket), a base branch, and a list of runs, each with a provider.
- **FR-002**: The dialog MUST offer only providers available where sessions run, MUST default each
  new run to the user's default AI CLI, and MUST let the user add and remove runs between 2 and the
  maximum (Assumptions).
- **FR-003**: The dialog MUST show, before confirming, the branch name and worktree folder each run
  will get; names MUST be derived from the group name with the run's number (1 to N) so they are
  visibly related and distinct.
- **FR-004**: Confirming MUST create one worktree per run on a new branch starting at the base
  branch's current commit, start one session of the run's provider in it, and deliver the prompt to
  that session as its first input exactly once.
- **FR-005**: A failure in one run (worktree creation, session start, or prompt delivery) MUST NOT
  stop, roll back, or keep from completing any other run, whatever order the runs finish in; the failed run MUST show its failed
  step and reason.
- **FR-006**: A run whose worktree creation fails MUST leave no branch or folder of its own behind.
- **FR-007**: The runs of a group MUST be shown together in the sidebar under one group row named
  after the group; each run row MUST show its number and provider; the group row MUST be
  collapsible and show its run count and failed count.
- **FR-008**: A group, its runs, their order, providers, prompt, base branch, status and winner MUST
  be kept on the local filesystem and restored after a restart, and shown alike in every window
  open on the project.
- **FR-009**: The app MUST offer a **Compare** view per group listing every run with its number,
  provider, status (creating, starting, working, waiting for input, failed with reason, prompt not
  delivered, picked), changed-file count, added-line count and removed-line count against the base
  branch, counting committed and uncommitted changes.
- **FR-010**: Compare's counts MUST come from the same reader as the Changes view's, so they equal
  the totals the run's Changes view shows with both toggles on whenever the group's base branch is
  the repository's default branch (the Changes view's own base); for another base branch they are
  counted against the group's base branch, and the user guide says so. They MUST refresh within 2
  seconds of a change in the run's worktree.
- **FR-011**: Compare MUST open a run's Changes view from that run's row.
- **FR-012**: **Pick this one** MUST integrate the chosen run's branch into the base branch using
  a merge: a fast-forward when the base branch's tip is an ancestor of the run's branch, otherwise
  a merge commit on the base branch; the run's branch MUST NOT be rewritten. A run whose worktree
  holds uncommitted changes MUST NOT be picked: the pick MUST be refused, naming the uncommitted
  files, and change nothing.
- **FR-013**: When integration cannot complete (conflicts, or the base branch is checked out with
  uncommitted changes it would overwrite), the pick MUST change nothing and MUST tell the user why,
  naming the conflicting files when there are conflicts.
- **FR-014**: After a successful pick the app MUST offer to remove the losers: it lists each loser
  with what removal deletes (its worktree folder, its sessions, and its branch, which the user can
  keep), and removes only the losers the user leaves selected.
- **FR-015**: A loser with uncommitted changes MUST be marked as such, MUST start unselected, and
  MUST be removed only after a second confirmation that names it as holding uncommitted changes.
  No pick path may delete a worktree with uncommitted changes without that confirmation.
- **FR-016**: Removing a loser MUST stop its running sessions first, as the existing Delete does;
  declining the offer MUST remove nothing.
- **FR-017**: Pick this one MUST be available only for a run that has a branch (working, waiting
  for input, or prompt not delivered), and for no run while any run of the group is still being
  created or started; an unavailable pick MUST show its reason. Picking a run whose session is
  still working MUST first ask for confirmation saying so.
- **FR-018**: A group MUST accept at most one pick; a second pick, from any window, MUST be refused
  with the reason that the group already has a winner.
- **FR-019**: Runs interrupted by the app closing during creation MUST be shown as failed
  ("interrupted") after restart, with no half-created branch or folder left behind; a run whose
  worktree creation had completed keeps its worktree and branch.
- **FR-020**: Dismissing a group MUST remove only the grouping; its worktrees, branches and sessions
  stay as ordinary sidebar entries.
- **FR-021**: Each run's session MUST be an ordinary session of its worktree (same terminal,
  restart, resume and close behaviour), isolated from the other runs' sessions.
- **FR-022**: The user guide MUST describe Run in parallel, the group in the sidebar, Compare, Pick
  this one and the cleanup offer, in the same change.

### Key Entities

- **Parallel run group**: name, prompt, base branch (and its commit when the group started),
  creation time, ordered runs, winner (none until picked).
- **Run**: number within the group, provider, branch, worktree, session, status and, when failed,
  the failed step and reason.
- **Run summary** (shown in Compare): changed-file count, added lines, removed lines, whether the
  worktree holds uncommitted changes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user starts a group of 4 runs with mixed providers from one dialog in under 1
  minute of their own time, against several minutes of manual setup per run today.
- **SC-002**: In 100% of groups where one run fails, every other run still gets its worktree, its
  session and its prompt.
- **SC-003**: Every run of a group is shown under its group row in 100% of sidebar views, including
  after a restart.
- **SC-004**: For groups based on the repository's default branch, Compare's file and line counts
  match the run's Changes view totals in 100% of runs.
- **SC-005**: 0 worktrees with uncommitted changes are removed by a pick without the user's explicit
  second confirmation.
- **SC-006**: A conflicting pick leaves the base branch and every run's branch and files
  byte-for-byte unchanged.

## Out of Scope

- Pushing the base branch, opening a pull request, or any remote operation as part of a pick.
- Sending a different prompt per run, or per-run model or CLI flags.
- Scoring, ranking or automatically choosing a winner.
- Starting a group from an assistant through the app's agent tools.
- Combining changes from several runs into the winner.
- Re-running a group's prompt in its existing runs.

## Assumptions

- The maximum number of runs in a group is 8; the default for a new dialog is 2.
- The name follows the New worktree form's naming rules (type, optional ticket, name), and each
  run's branch and folder append `-<number>` to the name part, so `feat` + `login page` gives
  `feat/login-page-1` … `feat/login-page-N`.
- The base branch defaults to the project root's current branch and can be any local branch.
- The prompt is delivered the way a new session's first prompt (docs/user-guide/agent-tools.md,
  "The first prompt") and Send to session already are: typed once, as one submission, after the
  provider is ready for input.
- Run worktrees are app-created worktrees, so they are never hidden as agent worktrees.
- Run status for working and waiting for input comes from the session's existing busy/idle state.
- The winner's worktree is kept after a pick; the user deletes it with the normal Delete action if
  they want to.
- Integration is local only (Principle IV); nothing is pushed.
