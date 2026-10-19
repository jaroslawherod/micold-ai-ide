# Feature Specification: Review changes reaches every pull request of the worktree

**Feature Branch**: `claude/project-thread-c3olht`

**Created**: 2026-10-19

**Status**: Draft

**Input**: User description: "Review changes: allow navigating between all PRs of the worktree (only the first PR's diff is reachable) (issue #1003). A worktree can produce several PRs, but review changes only shows the diff of the first PR. The other PRs are out of reach: there is no way to switch to them. Related: #775 (closed). Expected: Review changes lists every PR created from the worktree (open, merged and closed) and lets the user navigate between them, showing the selected PR's diff, status and checks. Acceptance criteria: Review changes shows all PRs whose head branch(es) were created in the worktree, with title, number, state and check status. The user can switch between PRs (list, tabs or prev/next) and the diff view shows the selected PR's diff, not always the first. The selection persists while the review view stays open and the list updates when a new PR is created from the worktree. A worktree with no PRs shows a clear empty state. Test covers a worktree with multiple PRs: selecting the second and third shows their own diffs."

## Background

Feature 775 added a **Pull requests** section to the Review changes view (the Changes pane). It lists the pull requests whose head branch equals the worktree's **current** branch, and selecting one shows its base...head diff. Three gaps remain, and together they make later pull requests of a worktree unreachable:

- A worktree that has moved to a new branch (one branch per milestone, or a branch renamed or recreated after a merge) only lists the pull requests of the branch it is on now; pull requests opened from its earlier branches disappear.
- An entry shows number, title and state, but not whether its checks pass, so the user cannot tell which pull request needs attention without opening each one.
- When the worktree has no pull requests the section is simply absent, so "none yet" looks the same as "not loaded" or "feature off".

This feature closes those gaps. It keeps everything 775 shipped that this spec does not change (read-only, GitHub data, per-entry selection, no error notice when PR information is unavailable). One 775 behaviour changes on purpose: with no GitHub access the section is no longer absent but shows one muted line (FR-011), so the user can tell it apart from "no pull requests".

## Clarifications

### Session 2026-10-19

- Q: Should branches merely checked out in the worktree (created elsewhere) count toward its pull requests, or only branches created in it? → A: Only branches created in the worktree, plus its current branch; a branch only checked out there does not count. _(agent-resolved: issue #1003 acceptance criteria "PRs whose head branch(es) were created in the worktree"; spec.md#Edge Cases, default/other branch it did not create)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See every pull request the worktree produced (Priority: P1)

A user opens Review changes for a worktree that has produced several pull requests over its life, from one branch or from several. The view lists all of them, open, draft, merged and closed, each with its number, title, state and check status, newest first.

**Why this priority**: A pull request that is not listed cannot be reviewed; listing them all is what the issue asks first.

**Independent Test**: Give a worktree three pull requests whose head branches are three different branches it has had (only the last is its current branch), open Review changes, and check all three entries appear with their own state and check status.

**Acceptance Scenarios**:

1. **Given** a worktree that had branch A (one merged pull request), then branch B (one closed pull request), and is now on branch C (one open pull request), **When** the user opens Review changes, **Then** all three pull requests are listed, newest first, each with its number, title and state.
2. **Given** a worktree whose single branch has three pull requests (opened, merged, then re-opened from the same branch name), **When** the view is open, **Then** all three are listed.
3. **Given** a listed pull request whose checks are passing, failing, still running, or absent, **When** the view is open, **Then** its entry shows that check status, using the same marks the sidebar uses for PR checks.
4. **Given** another worktree of the same project that had its own branches, **When** the user opens Review changes for this worktree, **Then** none of the other worktree's pull requests are listed.

---

### User Story 2 - Switch between the pull requests and review each one's diff (Priority: P1)

The user moves between the listed pull requests, by pressing an entry or with previous/next, and the diff area always shows the selected pull request's own files and hunks, together with its state and check status.

**Why this priority**: The issue's core defect: only the first pull request's diff is reachable.

**Independent Test**: With three listed pull requests whose diffs touch different files, select the second, then the third, and check the file list and hunks each time equal that pull request's own base...head comparison.

**Acceptance Scenarios**:

1. **Given** three listed pull requests with different diffs, **When** the user selects the second, **Then** the diff area shows the second pull request's files and hunks, not the first's.
2. **Given** the second pull request is shown, **When** the user selects the third, **Then** the diff area shows the third's files and hunks.
3. **Given** a pull request is shown, **When** the user presses next (or previous), **Then** the adjacent pull request in the list is selected and shown; next on the last entry and previous on the first do nothing.
4. **Given** a pull request is shown, **When** the user looks at the diff area's header, **Then** it names that pull request (number and title) with its state and check status.
5. **Given** a pull request is shown, **When** the user chooses the working tree again, **Then** the view returns to the worktree's own changes (unchanged from 775).

---

### User Story 3 - Selection holds and the list stays current (Priority: P2)

While Review changes stays open, the chosen pull request stays chosen across list refreshes, and a pull request newly opened from the worktree appears without reopening the view.

**Why this priority**: A selection that jumps back, or a list that misses a new pull request, misleads; but the list is useful without it.

**Independent Test**: Select the second of three pull requests, open a fourth from the worktree, wait one refresh interval, and check the fourth is listed while the second is still the one shown.

**Acceptance Scenarios**:

1. **Given** the second pull request is selected, **When** the list refreshes and a new pull request appears at the top, **Then** the second stays selected and shown.
2. **Given** the view is open, **When** a pull request is opened from any branch the worktree has had, **Then** it appears in the list within the PR status refresh interval.
3. **Given** a selected pull request whose state or checks change, **When** the list refreshes, **Then** its entry and the diff header show the new state and check status, and the shown diff stays selected.
4. **Given** the selected pull request disappears from the list, **When** the list refreshes, **Then** the view returns to the working-tree changes (unchanged from 775).

---

### User Story 4 - A clear empty state (Priority: P2)

A worktree with no pull requests shows a short line saying so, so "none yet" is distinguishable from "still loading" and from "PR information unavailable".

**Why this priority**: Removes ambiguity; small.

**Independent Test**: Open Review changes for a worktree none of whose branches has a pull request, and check the section reads "No pull requests from this worktree yet".

**Acceptance Scenarios**:

1. **Given** a worktree none of whose branches has a pull request and PR information is available, **When** the list has been read, **Then** the section shows "No pull requests from this worktree yet" and the working-tree changes are shown as before.
2. **Given** the first read has not finished, **When** the view is open, **Then** the section shows that it is loading, not the empty-state line.
3. **Given** PR information is unavailable (no GitHub remote, `gh` missing, not signed in, offline) and no list was read yet, **When** the view is open, **Then** the section shows one muted line saying pull requests cannot be shown, and no error notice is raised.

---

### Edge Cases

- Branch deleted locally and on the remote after its pull request merged: its pull requests stay listed and their diffs come from GitHub's data (as 775).
- Worktree checked out the repository's default branch (or another branch it did not create) for a while: that branch's pull requests are not listed (FR-002).
- Two worktrees that both had a branch of the same name: each lists that branch's pull requests (association is by branch name in the project's repository); the app cannot tell whose they are.
- The worktree's branch history cannot be read (history pruned, worktree re-added under the same path): the list falls back to the current branch's pull requests, as 775 did, with no notice.
- Many branches (more than the app asks about in one GitHub request): every branch is still covered; reads are batched, not truncated silently.
- More pull requests than fit the section: the section scrolls; previous/next still reach every entry.
- The project root (Default row) has no branch of its own: no pull request entries, and no empty-state line either (unchanged from 775).
- PR status setting off: no section and no empty-state line (775 FR-014).
- A fork pull request whose head branch has the same name: excluded, as the sidebar excludes them.
- The selected pull request's diff fails to read, is too large for GitHub to diff, or is still loading: the diff area shows that (as 775 does), its header still names the pull request, and previous/next and the entries still work.
- The user presses several entries, or next, quickly: the last one pressed is shown; a diff that arrives for an earlier choice is never shown under another pull request's header.
- Windows, macOS and Linux behave the same (Principle VI).

### Section states

| Situation | Section shows |
|---|---|
| PR status setting off | nothing (no section) |
| Entry has no branch of its own (project root) | nothing (no section) |
| First read pending | the loading line |
| No GitHub access, or the first read failed (no list ever read) | one muted line: "Pull requests can't be shown for this worktree" |
| Read succeeded, no pull requests | "No pull requests from this worktree yet" |
| Read succeeded, pull requests | the list |
| A refresh failed after a list was read | the last list, the selection kept, marked possibly stale past the threshold |

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Review changes MUST list the pull requests whose head branch is any branch of the worktree's branch set (FR-002), in the worktree's project repository, of every state: open, draft, merged and closed.
- **FR-002**: A worktree's branch set MUST be its current branch plus every branch that was created in that worktree, known from the worktree's own record so it survives app and daemon restarts without new stored app state. A branch only checked out in the worktree but created elsewhere (another worktree, the remote, the main checkout) is not part of the set unless it is the current branch. The repository's default branch is never part of the set.
- **FR-003**: Each entry MUST show the pull request's number, title, state and check status. Check status is one of passing, failing, pending or none, computed and marked the same way as the sidebar's PR checks.
- **FR-004**: Entries MUST be ordered newest first by creation time, across all branches of the set; equal times put the higher number first.
- **FR-005**: Pressing an entry MUST select it and show that pull request's base...head diff from GitHub's data, in the view's existing file list, layouts and per-file display (775 FR-004).
- **FR-006**: The view MUST offer previous and next controls while a pull request is shown, moving the selection to the adjacent entry in list order; they are disabled at the ends of the list.
- **FR-007**: While a pull request is shown, the diff area MUST name it (number and title) and show its state and check status.
- **FR-008**: The selection MUST be kept per worktree while the app runs (775 FR-013): a list refresh, a new entry, or a state or check change of any entry MUST NOT change which pull request is shown. Only the selected pull request leaving the list returns the view to the working-tree changes.
- **FR-009**: The list MUST refresh on the existing PR status triggers and interval, so a pull request opened, merged or closed from any branch of the set appears or changes state within that interval.
- **FR-010**: When the list has been read and is empty, the section MUST show "No pull requests from this worktree yet". While the first read is pending it MUST show a loading line instead.
- **FR-011**: When PR information is unavailable and no list has been read for this view (no GitHub access, or the first read failed), the section MUST show the one muted line "Pull requests can't be shown for this worktree" and MUST NOT raise an error notice (775 FR-008). This replaces 775's absent section for that case. The section states table above is normative.
- **FR-012**: When the PR status setting is off, or the entry has no branch of its own (the project root), the section MUST be absent, with no empty-state line (775 FR-014).
- **FR-013**: Reading the pull requests MUST NOT block the working-tree changes (775 FR-009): they render before the pull request read finishes. Every branch of the set MUST be covered however many there are; no branch is dropped.
- **FR-014**: The list MUST NOT cross worktrees or projects: only branches of this worktree's set, in this project's repository; fork pull requests are excluded.
- **FR-015**: When a refresh fails after a list was read, the last list and the selection MUST stay shown, marked possibly stale once past the existing stale threshold (775 FR-012); the unavailable line is not shown then.
- **FR-016**: When the user changes the selection while a diff is loading, the last choice MUST win: the diff area MUST only ever show the diff of the pull request its header names. A failed or too-large diff MUST be shown as 775 shows it, with previous/next and the entries still usable.

### Key Entities

- **Worktree branch set**: the branches a worktree has been on that count for its pull requests: its current branch and the branches created in it, never the default branch. Derived from the worktree's git history, not stored.
- **Worktree pull request**: a pull request whose head branch is in the branch set; has number, title, state, check status, creation time, base and head.
- **Selection**: which pull request (or the working tree) the view shows for a worktree; in memory only.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the acceptance fixtures, 100% of pull requests whose head branch is in the worktree's branch set are listed, with the correct state and check status, and 0 pull requests of other worktrees are listed.
- **SC-002**: For a worktree with three pull requests, selecting the second and then the third shows each one's own files and hunks, in one interaction each.
- **SC-003**: From any shown pull request, every other listed pull request is reachable in at most one interaction (pressing it) or by repeated next/previous.
- **SC-004**: A pull request opened from the worktree while the view is open is listed within the existing PR status refresh interval, and the selection shown before the refresh is still shown after it.
- **SC-005**: A worktree with no pull requests shows the empty-state line once its read finishes, and never shows it while the read is pending or when PR information is unavailable.
- **SC-006**: With PR information unavailable, zero new error notices appear, and in every case the working-tree changes render before the pull request read finishes.
- **SC-007**: For a worktree whose branch set has 25 branches, each with a pull request, all 25 pull requests are listed.

## Assumptions

- "Review changes" is the Changes pane opened by **Review changes** on a worktree row (feature 482), whose **Pull requests** section 775 added; this feature extends that section, not a new window.
- PR data is read through the app's existing GitHub access and PR status setting; no new credential, no new stored state (feature 040 FR-032: nothing about pull requests is stored).
- "Created in the worktree" is read from the worktree's own git history (branch creation and checkout records); where that history is gone, the current branch alone is used.
- Association is by head branch name in the worktree's project repository, as in 775; forks are out of scope.
- Read-only: creating, merging, or commenting on pull requests from this view stays out of scope.
- Keyboard shortcuts for previous/next are not required; the controls are buttons.
