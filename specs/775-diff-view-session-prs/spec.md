# Feature Specification: Diff view shows PRs created within sessions and worktrees

**Feature Branch**: `feat/775_diff-view-should-show-prs-created-within-sessions`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Diff view should show PRs created within sessions and worktrees (issue #775). When a session (e.g. an autopilot run) opens a PR from its worktree, the diff view only shows the working-tree/branch changes; there is no way to see the PR itself or its diff from there. The diff view lists PRs associated with the current session and its worktree (PRs whose head branch is the worktree's branch); selecting a PR shows its diff (base...head); PR state (open / merged / closed) is visible alongside the entry; a worktree with no active session still shows its PRs."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See the PRs of the worktree I am looking at (Priority: P1)

A user has a session or a worktree selected and opens the diff view. Next to the working-tree and branch changes, the view lists every pull request whose head branch is the worktree's branch, each with its title, number and state (open, merged or closed).

**Why this priority**: Without the list nothing else is reachable; it alone answers "did this session open a PR, and what became of it".

**Independent Test**: Select a worktree whose branch has two pull requests (one open, one merged), open the diff view, and check both entries appear with the right state.

**Acceptance Scenarios**:

1. **Given** a worktree whose branch has an open PR, **When** the user opens the diff view for its session, **Then** the PR is listed with its number, title and state "open".
2. **Given** a worktree whose branch has merged and closed PRs, **When** the diff view is open, **Then** each is listed with its own state, merged distinct from closed.
3. **Given** a worktree with no active session, **When** the user selects the worktree and opens the diff view, **Then** its PRs are listed the same as for a session in that worktree.
4. **Given** a worktree whose branch has no PRs, **When** the diff view is open, **Then** no PR entries are shown and the existing working-tree and branch changes are unaffected.

---

### User Story 2 - Review a PR's diff without leaving the app (Priority: P1)

The user selects a PR entry and the diff view shows the PR's changes (its base compared with its head), file by file, in the same layouts the view already offers.

**Why this priority**: It is the point of the feature: reviewing a session's PR in the app.

**Independent Test**: Select a listed PR and check the changed files shown equal the files of that PR's base...head comparison.

**Acceptance Scenarios**:

1. **Given** a listed PR, **When** the user selects it, **Then** the diff view shows the files and hunks of the PR's base...head comparison.
2. **Given** a PR is selected, **When** the user selects the working-tree/branch changes again, **Then** the view returns to them.
3. **Given** a merged PR whose head branch was deleted, **When** the user selects it, **Then** its diff is still shown, or a clear message says why it cannot be.

---

### User Story 3 - Entries stay current (Priority: P2)

When a session opens, merges or closes a PR while the diff view is open, the list reflects it without the user reopening the view.

**Why this priority**: A stale state misleads, but the list is useful without live updates.

**Independent Test**: Open a PR from the session's worktree while the view is open and check an entry appears within the refresh interval.

**Acceptance Scenarios**:

1. **Given** the diff view is open, **When** the session opens a PR from its worktree, **Then** the PR appears in the list.
2. **Given** a listed open PR, **When** it is merged, **Then** its entry shows "merged".

---

### Edge Cases

- No GitHub remote, `gh` missing or not signed in: the PR section is absent or states why it is empty; the rest of the diff view works and no error notice is raised.
- GitHub unreachable or rate limited: the last known list stays shown, marked as possibly stale; it never blocks the existing changes.
- Several sessions share one worktree: each shows the same PR list; one read serves them (Principle II).
- Several PRs for one branch (reopened, re-created after close): all are listed, newest first.
- A PR with a very large diff: it is handled as the existing view handles large diffs.
- Detached HEAD, or a worktree whose branch was renamed: no branch to match, so no PR entries.
- Base branch changed or force-pushed head: the shown diff matches the PR's current base...head.
- Windows, macOS and Linux behave the same, including where `gh` is found (Principle VI).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The diff view MUST list the pull requests whose head branch is the branch of the selected session's worktree.
- **FR-002**: The diff view MUST list the same pull requests for a selected worktree that has no active session.
- **FR-003**: Each entry MUST show the PR number, title and state, where state is one of open, merged or closed.
- **FR-004**: Selecting an entry MUST show that PR's diff as its base compared with its head, using the view's existing file list, layouts and per-file display.
- **FR-005**: The user MUST be able to return from a PR's diff to the working-tree/branch changes.
- **FR-006**: The view MUST show the working-tree/branch changes by default, unchanged from today, when no PR is selected.
- **FR-007**: The list MUST update when a PR for the branch is opened, merged or closed while the view is open.
- **FR-008**: When PR information is unavailable (no GitHub remote, no `gh`, no authentication, offline), the diff view MUST keep working and MUST NOT raise an error notice for this alone.
- **FR-009**: Reading PRs MUST NOT block or slow the display of working-tree changes.
- **FR-010**: The PR list MUST NOT cross projects: a worktree only ever shows PRs of its own repository.

### Key Entities

- **Session PR**: a pull request whose head branch equals a worktree's branch; has a number, title, state, base and head.
- **Worktree**: the checkout whose branch is the association key; a session reaches PRs only through its worktree.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a worktree with an open PR, a user reaches that PR's diff from the diff view in at most 2 interactions.
- **SC-002**: 100% of PRs whose head is the worktree's branch are listed, with the correct state, in the acceptance fixtures.
- **SC-003**: Working-tree changes appear as quickly as before the feature, whether or not PR information is available.
- **SC-004**: A PR opened, merged or closed while the view is open shows its new state within the existing PR status refresh interval.
- **SC-005**: With PR information unavailable, zero new error notices appear and every pre-existing diff view behaviour is unchanged.

## Assumptions

- "Diff view" is the app's Changes pane; the PR list lives there, not in a new window.
- PR data comes from the same GitHub reading the app already does for PR status; no new credential is introduced.
- Association is by head branch name equal to the worktree's branch, in the repository of the worktree's project; forks are out of scope.
- Read-only: creating, merging or commenting on PRs from the diff view is out of scope.
- Review comments on a PR's diff are out of scope; the PR diff is for reading.

## Open Questions

- [NEEDS CLARIFICATION: Should selecting a PR whose head branch no longer exists locally or on the remote (merged and deleted) show its diff from GitHub, or is it listed but not selectable?]
