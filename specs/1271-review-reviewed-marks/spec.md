# Feature Specification: Review UI — reviewed marks and progress

**Feature Branch**: `claude/project-thread-umhor9`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1271: Review UI: reviewed marks and progress.
Blocked by: #1269. Per-file reviewed state (unseen / has drafts / reviewed), 'n of m reviewed' with
a progress bar, a file whose diff changes after the mark returns to unseen. Stored by the session
service next to the comments (protocol field) so every window agrees. Part of the review UI
redesign: `docs/development/review-redesign.md`. Mockup: Review tab redesign. Acceptance also: pure
logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes)
updated." Labels: enhancement, flow:feature. This is slice R4 (item 1) of
`docs/development/review-redesign.md`.

## Context and dependencies

- **Builds on #1269 (R0, not merged when this spec was written)**: the review of an entry becomes a
  **Review tab** beside the entry's session tabs (the tab strip of feature 1126), and the view's
  state is kept per entry while you switch tab, session, worktree or project. This feature adds to
  that view; it does not change how the view opens, closes or is kept. Where this spec says
  **review view** it means the Review tab once #1269 has merged (today's Changes view before it).
  Nothing here depends on the view being a tab: marks belong to the entry, not to the view.
- **Builds on features 482, 946 and 1149**: the session service keeps an entry's review comments,
  pushes every change to every window attached to the project, keeps them across restarts, and
  drops them when the worktree or project is removed. Reviewed marks are kept the same way,
  beside the comments.
- **Leaves to later slices**: the `v` key and the "next unreviewed" jump (`g`) are #1274 (R5);
  "*n* files changed since you looked" while the diff stays put is #1275 (R6); the file tree, its
  sort (including *unreviewed first*) and filter are #1273 (R3); the two-column layout and source
  dropdown are #1272 (R2). This feature adds the marks, the per-file state shown on today's file
  rows, the mark control in the file's diff header, and the progress in the view's header.

## Terms

- **Entry**: the Default entry (the project root) or one worktree, as in feature 482.
- **Source**: what the review view lists: the **working tree** (the entry's changes against its
  base, as chosen by the Committed / Uncommitted toggles) or one **pull request** (feature 1149).
  The repository browser (**Files**, feature 1150) is not a source of changes and has no marks.
- **File's change**: the whole change of one file in one source, as the view shows it with every
  change kind of that source included — for the working tree, both committed and uncommitted
  changes together; for a pull request, its diff at its head commit. Two file's changes are the
  **same** when the file's path, its kind (Added, Modified, Deleted, Renamed from, Mode) and every
  line of its diff on both sides are identical.
- **Reviewed mark**: the user's statement "I have looked at this file's change", recorded for one
  file of one source of one entry, together with the file's change it was made on.
- **Review state** of a listed file, exactly one of:
  - **Unseen** (`○`): no mark, or a mark whose file's change is no longer the same, and no drafts;
  - **Has drafts** (`●`): not reviewed, and at least one Draft comment (feature 946) on the file
    in this source;
  - **Reviewed** (`✓`): a mark whose file's change is still the same.
- **Changed since reviewed**: a file that has a mark whose file's change is no longer the same.
  Its review state is Unseen or Has drafts; its row also shows a small *changed* dot.
- **Progress**: *n of m reviewed*, where *m* is the number of files the list shows now and *n* the
  number of those whose review state is Reviewed.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Mark a file reviewed and see my progress (Priority: P1)

After an agent run that changed many files, I open the entry's review view. Every file row shows
its review state. I read a file's diff and press **Mark reviewed** in its diff header; the row
turns to ✓ and the header's *n of m reviewed* and its progress bar go up by one. I can take the
mark back with the same control.

**Why this priority**: it is the feature: without marks there is no sense of progress through a
large change (redesign problem 2).

**Independent Test**: open a review of an entry with three changed files, mark one reviewed, and
check that its row shows ✓, the header reads *1 of 3 reviewed* with a bar one third full, and
unmarking returns both to their previous look.

**Acceptance Scenarios**:

1. **Given** a review view listing 3 files, none marked and none with drafts, **When** it opens,
   **Then** every row shows ○ and the header reads *0 of 3 reviewed* with an empty progress bar.
2. **Given** that view with `src/a.rs` selected, **When** I press **Mark reviewed** in the diff
   header, **Then** the row of `src/a.rs` shows ✓, the control reads **Reviewed** (pressed), and
   the header reads *1 of 3 reviewed* with the bar one third full.
3. **Given** `src/a.rs` is Reviewed, **When** I press its control again, **Then** the mark is
   removed, the row shows ○ (or ● if it has drafts) and the header reads *0 of 3 reviewed*.
4. **Given** `src/b.rs` has one Draft comment and no mark, **When** the view shows it, **Then** its
   row shows ● and it is not counted in *n*.
5. **Given** `src/b.rs` has a Draft and I mark it reviewed, **Then** its row shows ✓, its draft
   count stays visible on the row, and the draft is unchanged.
6. **Given** all 3 files are Reviewed, **Then** the header reads *3 of 3 reviewed* with a full bar.

---

### User Story 2 - A file that changes after I marked it asks for another look (Priority: P1)

While I review, the agent keeps working. A file I marked is edited again. When the view reads the
new change, that file is no longer counted as reviewed: its row drops back to ○ (or ● with
drafts) with a *changed* dot, and *n* goes down, so I never mistake new code for code I read.

**Why this priority**: a mark that survives a change would claim a review that did not happen;
without this the progress figure cannot be trusted.

**Independent Test**: mark a file, change one line of it on disk, wait for the view to re-read, and
check that the row shows ○ with the changed dot and *n* went down by one; revert the line and check
the file is Reviewed again.

**Acceptance Scenarios**:

1. **Given** `src/a.rs` is Reviewed, **When** a line of it changes on disk and the view re-reads,
   **Then** its row shows ○ with the changed dot and *n* goes down by one.
2. **Given** that file, **When** I mark it reviewed again, **Then** the changed dot goes and the
   mark now holds the new change.
3. **Given** `src/a.rs` is Changed since reviewed, **When** the change is undone so the file's
   change is the same as when I marked it, **Then** it is Reviewed again without a new mark.
4. **Given** `src/a.rs` is Reviewed, **When** a different file changes, a new file appears, or a
   new commit is made that leaves `src/a.rs`'s change the same, **Then** `src/a.rs` stays Reviewed
   (the new file is Unseen and *m* goes up).
5. **Given** `src/a.rs` is Reviewed in the working tree with only its uncommitted change, **When**
   that change is committed unaltered, **Then** it stays Reviewed (committing does not alter the
   file's change against the base).
6. **Given** the Review tab is not shown (#1269) while a marked file changes, **When** I show the
   tab and it re-reads, **Then** the file is shown Changed since reviewed.

---

### User Story 3 - Every window, and the next start, agrees (Priority: P2)

I have two windows open on the same project. A mark I set or remove in one shows in the other's
review of that entry within a moment, and after I quit and restart the app the marks are still
there.

**Why this priority**: the issue requires it and comments already behave so; without it the marks
would disagree between windows and vanish on restart, but a single window is still usable.

**Independent Test**: open the same entry's review in two windows, mark a file in one, and check
the other shows it within 2 seconds; restart the app and check the mark is still shown.

**Acceptance Scenarios**:

1. **Given** two windows show the review of the same entry, **When** I mark a file in window A,
   **Then** window B shows it Reviewed and its header's *n* goes up, within 2 seconds, without any
   action in window B.
2. **Given** a marked file, **When** I quit and restart the app, **Then** the file is still
   Reviewed (if its change is still the same).
3. **Given** both windows mark and unmark the same file at nearly the same time, **Then** both
   windows end up showing the same state.
4. **Given** a worktree with marks, **When** the worktree is removed (from the app or outside it),
   **Then** its marks are removed with its comments and do not appear for any other entry; removing
   the project removes the marks of all its entries.

---

### User Story 4 - Pull requests have their own marks (Priority: P3)

When the review view shows one of the entry's pull requests (feature 1149), its files have their
own marks and progress, separate from the working tree's, so reviewing a pull request and reviewing
the local work do not mix.

**Why this priority**: the pull request source is used less than the working tree; the working
tree alone delivers the value.

**Independent Test**: mark a file in pull request #12's diff, switch to the working tree and check
the same path there is not marked; switch back and check it still is.

**Acceptance Scenarios**:

1. **Given** the view shows pull request #12, **When** I mark `src/a.rs`, **Then** the header reads
   *1 of m reviewed* for #12, and the working tree's `src/a.rs` is not marked by it.
2. **Given** that mark, **When** pull request #12 gets a new head commit that changes `src/a.rs`'s
   diff, **Then** `src/a.rs` is Changed since reviewed in #12; a new head commit that leaves that
   diff the same keeps it Reviewed.

### Edge Cases

- **Empty list**: with no changed files (or both change kinds toggled off) there is no progress
  figure and no bar; the view shows its existing empty-list message.
- **Toggles**: turning Committed or Uncommitted off changes which files are listed (*m*) but not
  which are reviewed: the mark is judged on the file's change with both kinds included, so a file
  stays Reviewed whichever toggles are on. The Default entry has only uncommitted changes; that is
  its whole file's change.
- **Renamed file**: a rename is listed under its new path; a mark on the old path does not carry
  over, so the renamed file is Unseen. A file whose rename is undone gets its old mark back if its
  change is the same.
- **File no longer changed**: a marked file that leaves the list (reverted, or deleted when it was
  added) is not counted; if it comes back with the same change it is Reviewed again. Marks of files
  that are no longer listed may be dropped by the session service when it next stores the entry's
  marks; nothing visible depends on keeping them.
- **Binary, not-text and large files** (shown without a line diff): they can be marked; their file's
  change is the same while the file's kind and its contents on both sides are unchanged.
- **The diff cannot be read** (error, or still *Reading changes…*): files keep the state last
  worked out; marking is unavailable for a file whose change has not been read.
- **The session service is unreachable** (disconnected, or an older service that does not keep
  marks): the mark control is unavailable with a tooltip saying why, rows show ○ / ● from drafts
  only, and the header shows no progress. Comments keep working as today. A mark request that fails
  leaves the state as it was and shows the failure where the view shows other review errors.
- **Concurrent windows** (Principle II): the session service holds one set of marks per entry and
  source; the last change of a file's mark wins and is pushed to every attached window, so all
  converge.
- **Sending a review, Clear sent, Discard drafts**: none of them add or remove marks. A file whose
  drafts are sent or discarded and that has no mark goes from ● to ○.
- **Older review files**: a review file written by a build without marks loads with no marks and
  every comment intact. A file holding marks follows the existing versioning of the review file
  (see Assumptions).
- **Cross-platform** (Principle VI): paths are compared as the session service already stores them
  for comments (worktree-relative, `/`-separated), so a mark set on one platform matches the same
  file on another; line endings are part of the diff, as they are for the view.

## Requirements *(mandatory)*

### Functional Requirements

**Review state and marks**

- **FR-001**: Every file row of the review view MUST show the file's review state — Unseen (○),
  Has drafts (●) or Reviewed (✓) — as defined in Terms, Reviewed taking precedence over Has drafts.
  The row's existing draft count stays as it is.
- **FR-002**: The selected file's diff header MUST have a mark control: **Mark reviewed** when the
  file is not Reviewed, **Reviewed** shown pressed when it is. Pressing it marks or unmarks the file.
  It has a tooltip and is reachable by keyboard focus like the header's other controls.
- **FR-003**: A mark MUST record the file's change it was made on. A file MUST be Reviewed only
  while its current file's change is the same as the recorded one; otherwise it MUST be shown as
  Changed since reviewed: Unseen or Has drafts, with a *changed* dot on its row and a tooltip
  saying the file changed since it was marked.
- **FR-004**: The review state MUST be worked out again every time the view reads the list or the
  diff (including the automatic re-read of feature 482's file watching and #1269's re-read when the
  tab is shown again), without any action by the user.
- **FR-005**: Whether a file is Reviewed MUST NOT depend on the Committed / Uncommitted toggles; it
  is judged on the file's change with both kinds included.
- **FR-006**: Marks MUST be kept per entry and per source: the working tree's marks and each pull
  request's marks are separate, and a mark in one never shows in another.
- **FR-007**: Marking or unmarking MUST NOT change any comment, its status or the draft count, and
  sending, clearing or discarding comments MUST NOT change any mark.

**Progress**

- **FR-008**: The review view's header MUST show *n of m reviewed* and a progress bar filled to
  *n / m*, for the source shown, whenever the list has at least one file and marks are available.
  It MUST update as soon as a mark, a re-read or a toggle changes *n* or *m*.
- **FR-009**: With no files listed, or marks unavailable, the header MUST show neither the figure
  nor the bar, and the header's other content MUST NOT shift because of it.

**Storage and agreement**

- **FR-010**: The session service MUST keep each entry's marks next to its review comments: kept
  across app and service restarts, pushed to every window attached to the project after a window
  attaches and after every change, and removed with the worktree's comments when the worktree or
  the project is removed (feature 482's rules).
- **FR-011**: A mark change made in one window MUST be shown in every other window attached to the
  project within 2 seconds, and concurrent changes to the same file's mark MUST leave every window
  showing the same state.
- **FR-012**: A review file without marks (written before this feature) MUST load with no marks and
  with every comment as before.
- **FR-013**: When the session service cannot keep marks (unreachable, or too old to know them),
  the mark control MUST be unavailable with a tooltip saying why, the header MUST show no progress,
  and comments MUST keep working.
- **FR-014**: Deciding a file's review state, whether two file's changes are the same, and *n* and
  *m* MUST be pure logic in the render-free core, covered by unit tests (Principle I).

**Documentation and visuals**

- **FR-015**: `docs/user-guide/reviewing-changes.md` MUST describe the review states, the mark
  control, the progress figure, what returns a file to Unseen, the per-source marks and where marks
  are kept; the layout snapshot MUST include the states and the progress; the visual pass MUST
  cover the view at 1280×800 in both themes.

### Key Entities

- **Reviewed mark**: one file of one source of one entry; the file's path and an identity of the
  file's change it was made on (enough to tell whether the current change is the same, not the
  diff itself).
- **Entry marks**: all marks of one entry, grouped by source (working tree, or a pull request
  number); kept by the session service beside the entry's comments and removed with them.
- **Review state**: derived for each listed file from its marks, its Draft comments and its current
  file's change; never stored.
- **Progress**: derived *n of m* for the listed files of the source shown; never stored.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a review of 30 changed files, a user can tell how many are left to review from the
  header alone, without scrolling the list.
- **SC-002**: Marking a file updates its row and the header progress in the same window within
  0.5 seconds, and in another window on the same project within 2 seconds.
- **SC-003**: In 100% of cases where a marked file's change is altered, the file stops being counted
  as reviewed by the next re-read; in 100% of cases where it is unaltered (other files changing, a
  commit of the same change, a toggle), it stays counted.
- **SC-004**: Marks are still shown after an app restart in 100% of cases where the file's change
  is the same.
- **SC-005**: The view at 1280×800 in both themes shows the three row states, the changed dot, the
  mark control in both positions and the progress bar empty, partial and full without overlap or
  truncation of the header's other controls.

## Assumptions

- #1269 merges before this feature's code does; if it has not, the same controls go into today's
  Changes view unchanged, since marks belong to the entry, not to the view.
- "Per entry and head commit" in the redesign is read as "per entry, judged on the file's change":
  a new head commit resets only files whose change it alters, not every mark, because an agent that
  commits often would otherwise erase all progress.
- Reviewed takes precedence over Has drafts in a row's symbol: marking is an explicit statement,
  and the draft count stays visible next to it.
- The changed dot is the per-file part of redesign item 1; the header's "*n* files changed since you
  looked" and not reflowing the open diff are #1275.
- The review file's version goes up only while some entry has marks, so a file without marks stays
  readable by an older build, as feature 1149 did for pull request comments. An older build that
  meets a file with marks refuses it as an unknown version, the existing behaviour for newer files.
- A pull request's file's change is judged on its diff at its head commit as feature 1149 reads it.

## Out of Scope

- Keyboard shortcuts for marking and jumping to the next unreviewed file (#1274).
- Sorting or filtering by review state, and the file tree (#1273).
- The header-level "changed since you looked" summary and holding the open diff still (#1275).
- Bulk actions (mark all, clear all marks), and marking individual hunks or lines.
- Showing review progress outside the review view (sidebar rows, Overview cards, the Review tab's
  badge, which stays the draft count of #1269).
- Marks for the repository browser (**Files**).
