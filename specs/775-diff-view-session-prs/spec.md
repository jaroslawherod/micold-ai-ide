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
2. **Given** a worktree whose branch has merged, closed and draft PRs, **When** the diff view is open, **Then** each is listed with its own state, merged distinct from closed and draft distinct from open.
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
3. **Given** a merged PR whose head branch was deleted, **When** the user selects it, **Then** its base...head diff is shown from GitHub's data, the same as for a PR whose branch still exists.

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
- A PR with a very large diff: a file above the view's large-file limit is handled as the existing view handles large files; a PR GitHub itself refuses to diff (over 300 files or 20 000 changed lines) shows one "too large to show here" line with the PR address, never a notice.
- Detached HEAD, a worktree whose branch was renamed, or the project root (no branch of its own in the Changes pane): no branch to match, so no PR entries.
- PR status setting off: no PR section (FR-014).
- Base branch changed or force-pushed head: the shown diff matches the PR's current base...head.
- Windows, macOS and Linux behave the same, including where `gh` is found (Principle VI).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The diff view MUST list the pull requests whose head branch is the branch of the selected session's worktree.
- **FR-002**: The diff view MUST list the same pull requests for a selected worktree that has no active session.
- **FR-003**: Each entry MUST show the PR number, title and state, where state is one of open, draft, merged or closed (a draft PR is shown as draft, as the sidebar does).
- **FR-004**: Selecting an entry MUST show that PR's diff as its base compared with its head, taken from GitHub's data so it also works when the head branch was deleted, using the view's existing file list, layouts and per-file display.
- **FR-005**: The user MUST be able to return from a PR's diff to the working-tree/branch changes.
- **FR-006**: The view MUST show the working-tree/branch changes by default, unchanged from today, when no PR is selected.
- **FR-007**: The list MUST update when a PR for the branch is opened, merged or closed while the view is open.
- **FR-008**: When PR information is unavailable (no GitHub remote, no `gh`, no authentication, offline), the diff view MUST keep working and MUST NOT raise an error notice for this alone.
- **FR-009**: Reading PRs MUST NOT block or slow the display of working-tree changes.
- **FR-010**: The PR list MUST NOT cross projects: a worktree only ever shows PRs of its own repository.
- **FR-011**: PRs MUST be listed newest first.
- **FR-012**: When a refresh of the PR list fails, the last list MUST stay shown and, once older than the app's existing stale threshold for PR status, be marked as possibly stale.
- **FR-013**: The selected PR is per selected session or worktree: switching selection MUST show that selection's own view, and a selected PR that disappears from the list MUST return the view to the working-tree/branch changes.
- **FR-014**: When the user's PR status setting is off, the diff view MUST show no PR section and raise no notice; turning it on makes the list appear.

### Key Entities

- **Session PR**: a pull request whose head branch equals a worktree's branch; has a number, title, state, base and head.
- **Worktree**: the checkout whose branch is the association key; a session reaches PRs only through its worktree.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a worktree with an open PR, a user reaches that PR's diff from the diff view in at most 2 interactions.
- **SC-002**: 100% of PRs whose head is the worktree's branch are listed, with the correct state, in the acceptance fixtures.
- **SC-003**: Working-tree changes render before any PR read completes, with no added delay, whether or not PR information is available.
- **SC-004**: A PR opened, merged or closed while the view is open shows its new state within the existing PR status refresh interval.
- **SC-005**: With PR information unavailable, zero new error notices appear and every pre-existing diff view behaviour is unchanged.

## Assumptions

- "Diff view" is the app's Changes pane; the PR list lives there, not in a new window.
- PR data is read through the app's existing GitHub access (`gh` discovery and authentication, and the PR status setting); no new credential is introduced. The list needs every PR of a branch, so it may need its own read beyond the sidebar status one.
- Association is by head branch name equal to the worktree's branch, in the repository of the worktree's project; forks are out of scope.
- Read-only: creating, merging or commenting on PRs from the diff view is out of scope.
- Review comments on a PR's diff are out of scope; the PR diff is for reading.

## Clarifications

### Session 2026-10-12

- Q: Should selecting a PR whose head branch no longer exists locally or on the remote (merged and deleted) show its diff from GitHub, or be listed but not selectable? → A: Shown from GitHub's base...head data, selectable like any other entry; every listed entry is selectable. _(agent-resolved: specs/775-diff-view-session-prs/spec.md#Functional Requirements FR-004 "Selecting an entry MUST show that PR's diff", and #Assumptions: PR data comes from the existing GitHub access)_
