# Feature Specification: Show the full file and browse the repo's code while reviewing a diff

**Feature Branch**: `feat/1150_show-the-full-file-while-reviewing-a-diff-in-the`

**Created**: 2026-10-20

**Status**: Closed 2026-10-23 — shipped in PRs #1151, #1156, #1162, #1215, #1221, #1253, #1282, #1291, #1293 (client visual pass of the browse flow outstanding, T066)

**Input**: User description: "Show the full file while reviewing a diff in the Changes view (including a pull request's) (issue #1150). The Changes view shows only the changed hunks with a few lines of context, so a reviewer cannot read the surrounding code or look at files the change does not touch. Wanted: navigate the repository's code: open any file of the repo, including files the change does not affect (e.g. a file tree or path picker, and following a reference from the diff into another file), and go back to the diff; files are read at the PR head version for a PR's diff, the worktree's version otherwise. Also: while reviewing a diff, including a pull request's, view the file's full code: expand hidden unchanged lines (per gap or a whole-file toggle), with syntax colouring and line numbers as in the diff. Works for the worktree's own changes and for a PR's diff (the PR head version). Lines revealed this way can be picked for review comments wherever picking is allowed (see #1149). Relates to #482, #1003, #1149."

## Background

The Changes view (482, 775, 1003) lists the changed files of the worktree, or of one of its pull requests, and shows each file as hunks with a few unchanged context lines. Everything between hunks, and before the first and after the last hunk, is hidden. A reviewer who needs to understand a change must leave the view to read the rest of the file. Line picking for review comments is specified by #1149; this feature also lets the reviewer open any file of the repository, and makes revealed lines pickable where that feature allows picking.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Expand a hidden gap between hunks (Priority: P1)

A reviewer reading a file's diff sees a marker where unchanged lines are hidden. They activate the marker and the hidden lines appear in place, with line numbers and syntax colouring identical to the diff's own context lines.

**Why this priority**: The smallest slice that lets a reviewer read surrounding code; deliverable alone.

**Independent Test**: Open a file with two distant hunks, expand the gap, and check the hidden lines show with correct numbers and colouring and the diff's changed lines are unchanged.

**Acceptance Scenarios**:

1. **Given** a file diff with hidden unchanged lines between two hunks, **When** the reviewer expands that gap, **Then** all lines of the gap appear between the hunks, numbered consecutively with the neighbouring lines, in the diff's syntax colouring.
2. **Given** hidden lines before the first hunk or after the last hunk, **When** the reviewer expands them, **Then** those lines appear at the start or end of the file's diff.
3. **Given** an expanded gap, **When** the reviewer collapses it, **Then** the diff returns to its hunk-with-context form.

---

### User Story 2 - Show the whole file with one toggle (Priority: P1)

A reviewer toggles a whole-file mode for a file and sees every line of it, changed lines still marked as added or removed, instead of expanding gap by gap.

**Why this priority**: The "read the entire file" case named in the issue.

**Independent Test**: Toggle whole-file mode on a file with several hunks; verify every line of the file appears once, in order, with changed lines marked; toggle off and verify the hunk form returns.

**Acceptance Scenarios**:

1. **Given** a file diff, **When** the reviewer turns whole-file mode on, **Then** every line of the source's version of the file (worktree, or PR head for a pull request) appears in order with the diff's line numbers and colouring, and removed lines stay shown in their place.
2. **Given** whole-file mode on, **When** the reviewer turns it off, **Then** the file returns to hunks with context.

---

### User Story 3 - Works for a pull request's diff (Priority: P1)

A reviewer selecting one of the worktree's pull requests (1003) can expand gaps and use whole-file mode on that pull request's files; the lines shown are the PR head version of the file, not the worktree's current content.

**Why this priority**: Explicit requirement; the worktree may have moved on from the PR head.

**Independent Test**: Select a PR whose head differs from the worktree files; expand a gap; verify the lines match the PR head.

**Acceptance Scenarios**:

1. **Given** a pull request selected in the Changes view whose head differs from the worktree's files, **When** the reviewer expands a gap, **Then** the revealed lines are the PR head's lines.
2. **Given** a worktree-changes diff (no PR selected), **When** the reviewer expands a gap, **Then** the revealed lines are the worktree's current file content.

---

### User Story 4 - Pick revealed lines for review comments (Priority: P2)

A reviewer picks revealed lines exactly as they pick lines of the diff, wherever #1149 allows picking, and comments on them.

**Why this priority**: Makes the revealed code useful to the review workflow; depends on #1149.

**Independent Test**: Expand a gap, pick a revealed line where picking is allowed, and confirm the pick carries the same file and line identity as a diff line would; where picking is not allowed, confirm revealed lines are not pickable either.

**Acceptance Scenarios**:

1. **Given** picking is allowed for the shown diff, **When** the reviewer picks revealed lines (alone or together with diff lines), **Then** the pick is accepted and addresses the lines by the same file and line numbers as picked diff lines.
2. **Given** picking is not allowed for the shown diff, **When** lines are revealed, **Then** they are not pickable.
3. **Given** a browsed file the change does not touch, **When** the reviewer tries to pick lines, **Then** picking works only where #1149 allows picking in the shown source and context (#1149 is open and has no spec yet, so this feature defines no anchor of its own); otherwise the lines are not pickable.

### User Story 5 - Browse any file of the repository and return to the diff (Priority: P2)

A reviewer opens a file the change does not touch, from a file tree or path picker, or by following a reference in the diff into another file (a path or import string that names an existing repository file). The file shows with line numbers and syntax colouring. A back action returns to the diff exactly where they left it.

**Why this priority**: Extends reading beyond the changed files; builds on the same file-reading path as stories 1 to 3.

**Independent Test**: Open an untouched file from the tree, then follow a reference from a diff line into another file, go back twice, and verify the diff shows with the same file, scroll position and expansion state.

**Acceptance Scenarios**:

1. **Given** the Changes view, **When** the reviewer picks any repository file in the file tree or path picker, **Then** its full content shows with line numbers and syntax colouring.
2. **Given** a path or import string in a diff line that names an existing repository file, **When** the reviewer activates it (click or tap on the text), **Then** that file opens, at the referenced line when the text gives one.
3. **Given** a browsed file, **When** the reviewer goes back, **Then** the previous view (the diff, or the file they came from) returns with its selected file, scroll position and expansion state.
4. **Given** a pull request selected, **When** the reviewer browses to any file, **Then** it is shown at the PR head version, including files the PR does not change; with worktree changes it is the worktree's version.
5. **Given** a browsed file the change also modifies, **When** it opens, **Then** the reviewer can switch to that file's diff.

### Edge Cases

- Empty file, or a file with one line: no gap marker is offered; whole-file mode shows what exists.
- Added file: the diff already shows every line; no markers and the whole-file toggle is hidden.
- Deleted file: there is no current version; expansion is not offered and the removed lines show as they do today.
- Binary or non-text file: expansion is not offered.
- Very large file: the reveal does not freeze the interface; lines show progressively or the reveal is bounded with a clear note; a file whose shown version exceeds 2 MB (the limit the Changes view already uses, 482 R8) is not expanded or browsed automatically: a clear note says it is large and offers a per-file "Show anyway" that lasts while the view is open.
- File content cannot be obtained (read error, PR head unreachable offline, file missing on the PR head): the reviewer sees a clear error on that file; the diff stays usable.
- The file changes on disk (agent sessions edit the worktree concurrently, Principle II) while a gap is expanded: the diff refreshes, and expansion state is kept for gaps that still exist; no stale lines are shown against new numbering.
- Switching between files, between worktree changes and a PR, or between PRs: expansion state of one file never leaks into another source's diff of the same path.
- Untouched files: browsing works for any file, with no diff markers; the file tree lists files that exist in the source being shown (a file added by the PR is absent from the worktree's tree and vice versa).
- Browsing a binary file, a file over the size limit, or a path missing at the PR head (deleted, or a reference that resolves nowhere): a clear message replaces the content; the reviewer can still go back; a reference with no target does nothing visible beyond a short notice.
- Back-to-diff state: after browsing, going back restores file, scroll position and expansion state; if the diff refreshed meanwhile, the restored view shows the refreshed diff.
- Switching PR or source while browsing: the browsed path is re-read at the new source, or shows the missing-file message.
- Symbolic links and files outside the repository: not followed outside the repository root.
- Pull request head not obtainable (head branch deleted, offline, fetch fails) while the PR diff still works from GitHub's data: expansion and browsing show the FR-009 message for that file; the diff stays usable.
- Renamed files: lines come from the file's new path.
- Files with Windows line endings or without a trailing newline show like the diff's own lines.
- Behaviour is identical on Linux, macOS and Windows (Principle VI).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each hidden range of unchanged lines in a file's diff (between hunks, before the first, after the last) MUST be marked by an activatable control that states how many lines are hidden.
- **FR-002**: Activating a control MUST reveal exactly that range in place, with line numbers continuous with its neighbours and syntax colouring the same as the diff's context lines.
- **FR-003**: A revealed range MUST be collapsible again, restoring the hunk-with-context form.
- **FR-004**: Each file's diff MUST offer a whole-file toggle (except added files, where every line is already shown and the toggle is hidden) that shows every line of the file, keeping added and removed lines marked, and toggles back to the hunk form.
- **FR-005**: For worktree changes, revealed lines MUST come from the worktree's current file content; for a selected pull request (1003) they MUST come from the pull request's head version of the file.
- **FR-006**: Revealed lines MUST be pickable for review comments exactly where #1149 allows picking diff lines, and addressed by the same file and line numbers; where picking is not allowed they MUST NOT be pickable.
- **FR-007**: Expansion MUST work in every layout the Changes view offers, as in the diff's own display.
- **FR-008**: Expansion state MUST be kept per file and source while the view stays open, survive a diff refresh for ranges that still exist, and be dropped for ranges that no longer exist.
- **FR-009**: Files for which no text version exists (binary, deleted, unreadable) MUST NOT offer expansion; an unobtainable version MUST show a clear message on that file without breaking the rest of the diff.
- **FR-010**: Expanding MUST NOT modify any file, the diff, or any stored review state, and MUST work offline for worktree changes.
- **FR-011**: The reviewer MUST be able to open any text file of the repository (tracked files of the shown source; untracked and git-ignored files only for worktree changes when they exist on disk, never for a PR), including files the change does not affect, from a file tree or path picker, and see it with line numbers and syntax colouring as in the diff.
- **FR-012**: A path or import string in a diff line that names an existing repository file MUST be activatable (click or tap) and open that file, at the referenced line when the text gives one; text that names no existing file MUST NOT be activatable. Symbol definition lookup is out of scope.
- **FR-013**: A back action MUST return from a browsed file to the previous view with its file, scroll position and expansion state; browsing MUST NOT lose diff state.
- **FR-014**: Browsed files MUST be read at the PR head version when a pull request is selected, and at the worktree's current version otherwise; paths MUST NOT resolve outside the repository.
- **FR-015**: Browsed files MUST be pickable for review comments only where #1149 allows picking in the shown source and context (see Story 4 scenario 3), and offer a way to open that file's diff when the change modifies it.

### Key Entities

- **Hidden range**: a run of unchanged lines not shown in a file's diff, identified by file, source (worktree or PR) and line span.
- **Expansion state**: which hidden ranges, or whole-file mode, are open for a file within a source.
- **Browse history**: the stack of views (diff or browsed file, with position) the back action returns through.
- **Revealed line**: an unchanged line shown through expansion; has file path and line number like any diff line.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A reviewer can read any file's complete content from the Changes view without leaving it, in at most one action per hidden range or one action for the whole file.
- **SC-002**: 100% of lines revealed (when the version is obtainable) carry line numbers and colouring equal to the diff's own context lines, in all layouts.
- **SC-003**: For a pull request, 100% of revealed lines equal the PR head version whenever it is obtainable, even when the worktree differs.
- **SC-004**: Expanding a typical source file (up to 5,000 lines) shows its lines in under 1 second without freezing the interface.
- **SC-005**: Wherever picking is allowed, a revealed line can be picked and commented in the same number of steps as a diff line.

- **SC-006**: A reviewer can open any file of the repository, whether or not the change touches it, in at most 3 actions from the diff, and return to the diff state in 1.
- **SC-007**: 100% of files browsed under a selected pull request equal the PR head version whenever obtainable.

## Assumptions

- Large-file limit reuses the Changes view's 2 MB per-version limit (`review::limits::MAX_VERSION_BYTES`).

- Line picking, comments, and where picking is allowed are defined by #1149. This feature adds no picking rules of its own; it depends on #1149 being delivered, and until then revealed and browsed lines are simply not pickable.
- The pull request's head content is obtainable the way the PR diff itself is (1003); no new credentials.
- "A few lines of context" stays as today when nothing is expanded.
- Expansion state is view state only and is not persisted across app restarts.

## Out of scope

- Editing files; semantic code intelligence beyond path and import references (find-references, search across files; symbol definition lookup).
- Changing which lines the diff marks as changed, or the context size.
- Expanding to show a file's version at the PR base or other revisions.

## Clarifications

### Session 2026-10-20

- Q: Size limit above which a file is not fully expanded? → A: 2 MB per version, with a per-file "Show anyway", as the Changes view does. _(agent-resolved: specs/482-worktree-changes-review/research.md#R8)_
- Q: Is symbol definition lookup in scope? → A: No; the issue asks only for opening files and following references from the diff. _(agent-resolved: issue #1150 Wanted; spec.md#Out of scope)_
- Q: Can lines of files outside the change be picked for comments? → A: Only where #1149 allows; this feature adds no picking rule or anchor, and until #1149 delivers they are not pickable. _(agent-resolved: spec.md#Assumptions; issue #1150 Wanted)_
