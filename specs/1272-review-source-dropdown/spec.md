# Feature Specification: Review UI — source dropdown and two-column layout

**Feature Branch**: `claude/project-thread-43e62l`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Review UI: source dropdown and two-column layout (issue #1272). Blocked by: #1270, #1150 (merged). One **Working tree / Pull requests / Files** control in the review header; the PR section and Committed/Uncommitted toggles leave the left column. Keep Whole file, Files, Back and Open diff from #1150. Part of the review UI redesign: `docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu (Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes) updated."

## Background

The Changes view (482, 775, 1003, 1150) has a header over three columns. The left column stacks
the **Committed**, **Uncommitted** and **Files** toggles, the **Pull requests** section (a
**Working tree** row and one row per pull request, up to 220 px tall) and the file list. The
middle column is the diff. The right column is the review panel. The header already carries the
base line as its subtitle. **Files** is not a source today: it is an on/off mode that stays on over
whichever source is shown (the working tree or a pull request) and lists that source's files.
Choosing what to read (working
tree, a pull request, or the repository's files) is spread over the left column and pushes the file
list down (`docs/development/review-redesign.md`, problems 1, 7 and 8).

This feature is slice R2 of that redesign (proposal item 3). Choosing the source becomes one
control in the header. The left column holds only the file list (or, with **Files**, the
repository tree), and the body becomes two columns: the file column and the diff.

**Dependencies.** #1150 (whole file, Files, Back, Open diff) is merged and this feature keeps its
behaviour. #1270 (R1: review bar and comments drawer) moves the review panel's buttons and counts
into a bottom bar and replaces the right column with a drawer. Until #1270 lands, the third column
is still there. This feature removes the PR section and the toggles from the left column and
leaves the right column to #1270. The two-column result (FR-008) is complete only once both have
merged. #1269 (R0: Review as a tab) may change where the header sits (a tab rather than a view
that replaces the terminal). The source control belongs to the review's own header wherever that
header is drawn.

## Clarifications

### Session 2026-10-24

- Q: How does the Files option combine with the shown source (Story 3 s2)? → A: As today (option b):
  Files is an on/off mode over the shown source. Picking **Files** turns it on (or off, when on)
  and closes the list. It stays on when the reviewer picks Working tree, a pull request or a chip;
  the tree and the browsed file are then read again at the new source and **Back** is forgotten,
  as in 1150. The closed label reads `Files · ` before the source's label (`Files · Working tree`,
  `Files · #<number> · <title>`); the kinds suffix is left out, since the tree lists every file.
  **Open diff** shows the diff in the same source and leaves Files on. _(agent-resolved:
  docs/user-guide/reviewing-changes.md#Browsing the repository;
  specs/1150-diff-full-file-view/spec.md#Edge Cases (switching PR or source while browsing);
  crates/micold-client/src/features/changes.rs `FilesPaneToggled`, `reread_after_source_change`;
  this spec's Story 3 "no new behaviour", FR-006, FR-007, SC-004)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pick the source from one header control (Priority: P1)

A reviewer opens the review of a worktree and sees one source control in the header, labelled
with what is shown now (**Working tree** by default). Opening it lists **Working tree**, the
worktree's pull requests and **Files**. Picking an option shows that source. The left column no
longer has a Pull requests section or source toggles.

**Why this priority**: This is the change the issue asks for. It gives the file list back the
height the PR section took, and makes the current source visible as a mode.

**Independent Test**: Open the review of a worktree with two pull requests. Check the left column
holds only the file list. Open the source control, pick the second pull request, and check the
control's label, the file list, the diff and the base line all show that pull request. Pick
**Working tree** and check the working-tree changes return.

**Acceptance Scenarios**:

1. **Given** the review of a worktree is open, **When** it is shown, **Then** the header has one
   source control labelled **Working tree**, and the left column has no Pull requests section and
   no Committed, Uncommitted or Files toggles.
2. **Given** the source control, **When** the reviewer opens it, **Then** it lists **Working
   tree**, then the worktree's pull requests newest first (each with its state mark, check mark,
   `#number` and title, as the PR section showed them), then **Files**. The current source (and
   **Files**, when on) is marked as chosen.
3. **Given** the list is open, **When** the reviewer picks a pull request, **Then** the list closes,
   the control reads `#<number> · <title>`, and the file list, the diff, the PR header above the
   diff and the base line show that pull request, as clicking its row did before (1003).
4. **Given** a pull request is shown, **When** the reviewer picks **Working tree**, **Then** the
   working-tree changes return with the selection they had, as clicking the Working tree row did
   before.
5. **Given** a pull request is shown, **When** the reviewer uses the PR header's previous/next
   buttons, **Then** the shown pull request and the control's label change together.
6. **Given** the list is open, **When** the reviewer presses `Esc` or clicks outside it, **Then**
   it closes and the source does not change.

---

### User Story 2 - Choose committed and uncommitted changes inside the Working tree option (Priority: P1)

The Committed and Uncommitted toggles move from the left column into the **Working tree** option
of the source control, as two chips. They keep their meaning and their availability.

**Why this priority**: Without it, moving the toggles out of the left column would remove the
ability to narrow the working-tree list.

**Independent Test**: Open the source control on a worktree, turn **Committed** off in the
Working tree option, and check that only uncommitted files are listed and the control's label says
so. On the Default entry, check that **Committed** is unavailable and the note says why.

**Acceptance Scenarios**:

1. **Given** the source control is open, **When** the reviewer looks at the **Working tree**
   option, **Then** it shows a **Committed** chip and an **Uncommitted** chip with their current
   on/off state (both on when the review opens).
2. **Given** both chips are on, **When** the reviewer turns **Committed** off, **Then** the file
   list shows only uncommitted changes, and the list stays open so the other chip can be changed
   too.
3. **Given** a pull request is shown, **When** the reviewer changes a chip, **Then** the working
   tree is shown with the chip's new state (the chips belong to the Working tree option). This is
   new: today the toggles are hidden while a pull request is shown.
4. **Given** only one kind is on, **When** the control is closed, **Then** its label says which
   kind is shown (**Working tree · committed** or **Working tree · uncommitted**). With both on it
   reads **Working tree**. With both off it reads **Working tree · nothing shown**, and the list's
   empty state says both kinds are hidden, as today.
5. **Given** the Default entry (project root), **When** the control is open, **Then** the
   **Committed** chip is unavailable and the note that only uncommitted changes are listed is shown
   in the option, as it was beside the toggle.

---

### User Story 3 - Browse the repository's files from the source control (Priority: P2)

**Files** is the third option of the source control. Picking it puts the repository tree and the
**Open file by path** picker in the left column, as the **Files** toggle did (1150). Whole file,
Back and Open diff work as before. Files stays what it is today: an on/off mode over the shown
source, which picking a source does not turn off.

**Why this priority**: Repository browsing exists. Its toggle leaves the left column with the
others, so it needs a new place, but no new behaviour.

**Independent Test**: Pick **Files**, open a file from the tree, press **Open diff** (the file is
changed) and **Back**, toggle **Whole file** on a diff. Check each behaves as in 1150 and the
control's label tracks the mode.

**Acceptance Scenarios**:

1. **Given** the working tree is shown, **When** the reviewer picks **Files**, **Then** the left
   column shows the repository tree and the path picker for the working tree's files, the list
   closes, and the control reads `Files · Working tree`.
2. **Given** a pull request is shown, **When** the reviewer picks **Files**, **Then** the tree
   lists the files of that pull request's head, as **Files** did with a pull request shown (1150),
   and the control reads `Files · #<number> · <title>`.
3. **Given** a browsed file, **When** the reviewer presses **Back** or **Open diff**, **Then** they
   behave as in 1150. **Open diff** shows the file's diff in the shown source; the source and
   Files stay as they were.
4. **Given** **Files** is on, **When** the reviewer picks another source (**Working tree**, a pull
   request, or a Working-tree chip while a pull request is shown), **Then** Files stays on, the tree and the browsed file are read again at the
   new source (a path it lacks says so), and **Back** forgets where it led, as in 1150. The label
   changes to `Files · ` and the new source.
5. **Given** **Files** is on, **When** the reviewer picks **Files** again, **Then** it turns off,
   the list closes, and the left column shows the shown source's changed files.
6. **Given** any diff, **When** the reviewer toggles **Whole file**, **Then** it works as in 1150.

---

### User Story 4 - The diff gets the width (Priority: P2)

With the source control in the header and the review panel replaced by #1270's drawer, the body is
two columns: a narrow file column and the diff, which takes the rest of the width.

**Why this priority**: This is the layout win the redesign is after (problem 1). It is complete
only once #1270 has removed the right column.

**Independent Test**: At 1280×800, open the review of a worktree with a long changed path. Measure
the diff column against the window and check the file column has the fixed width.

**Acceptance Scenarios**:

1. **Given** #1270 has merged and the review is open at 1280×800, **When** it is shown, **Then** the
   body has exactly two columns, the file column and the diff, and the diff is at least 70% of the
   body's width.
2. **Given** #1270 has merged and the window is resized, **When** it gets wider or narrower,
   **Then** the file column keeps its width and the diff takes the change.
3. **Given** the base line (`vs <base>@<commit>`, or `Pull request #N: base ... head`), **When**
   the reviewer hovers or focuses the source control, **Then** its tooltip shows the base line; the
   header's subtitle shows it as today.

### Edge Cases

- **PR status setting off, or the Default entry**: the source control lists only **Working tree**
  and **Files**, with no pull request rows and no Pull requests heading.
- **Pull requests still loading**: the list shows **Reading pull requests…** in place of the rows.
  The working tree can be picked meanwhile and is never held back by the read.
- **No pull requests**: the list shows **No pull requests from this worktree yet**.
- **GitHub unreachable and no list read** (no `gh`, signed out, offline): the list shows **Pull
  requests can't be shown for this worktree** and raises no error.
- **List older than the stale threshold**: the list shows the *possibly stale* note under its
  heading, as the section did.
- **Many pull requests**: the list scrolls inside itself. It is never taller than the space below
  the header and never pushes the body down.
- **Shown pull request disappears after a refresh**: the view returns to the working tree (1003),
  and the control's label changes with it.
- **Long PR title**: the closed control truncates the title with an ellipsis and keeps `#<number>`
  visible. The full title is in the open list and the tooltip.
- **Picking while a diff is loading**: the last pick wins, as with the rows today (1003).
- **Multi-session (Principle II)**: agent sessions editing the worktree, or another window
  reviewing the same entry, do not change this window's chosen source. Each entry remembers its own
  source while the window is open, as today. A pull request list refreshed by another window's read
  updates this window's list without closing it while it is open.
- **Narrow window (900 px)**: the header keeps the source control and **Close** visible. The
  subtitle truncates before the control does.
- **Keyboard**: the control opens with Enter or Space when focused. Arrow keys move through the
  options and Enter picks one. `Esc` closes it.
- **Cross-platform (Principle VI)**: the control, its list and its tooltip behave the same on
  Linux, macOS and Windows. The list opens over the body, below the control, on all three.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The review header MUST have one source control whose closed label names the shown
  source and whether Files is on: **Working tree** (with the shown kinds per Story 2 s4),
  `#<number> · <title>` for a pull request, and, with Files on, `Files · ` before the source's
  label (`Files · Working tree`, `Files · #<number> · <title>`, with no kinds suffix).
- **FR-002**: Opening the control MUST list, in this order, **Working tree**, the worktree's pull
  requests newest first (state mark, check mark, `#number`, title), and **Files**, with the shown
  source (and Files, when on) marked. Picking Working tree or a pull request MUST show that source
  and close the list, leaving Files on or off as it was. Picking Files MUST turn Files on (or off,
  when on) over the shown source and close the list. Changing a Working-tree chip MUST keep the
  list open.
- **FR-003**: The left column MUST NOT contain the Pull requests section, or the Committed,
  Uncommitted or Files toggles. It MUST contain only the changed-file list of the shown source,
  its empty or loading state, or (with **Files**) the repository tree and path picker.
- **FR-004**: The **Working tree** option MUST hold **Committed** and **Uncommitted** chips with
  today's meaning, defaults and availability: both on when the review opens, **Committed**
  unavailable on the Default entry with its note. Changing a chip while a pull request is shown
  MUST show the working tree with the new state (a deliberate change: today the toggles are hidden
  then).
- **FR-005**: Every pull-request list state the section had (loading, unavailable, empty, list,
  possibly stale, absent when the setting is off or for the Default entry) MUST appear in the open
  control with the same wording. No state may raise an error.
- **FR-006**: Choosing a source through the control MUST behave exactly as choosing the same
  source did before (1003, 1150). This covers the diff read, kept selections, the PR header with
  previous/next, last pick wins, falling back to the working tree when a PR disappears, and the
  per-entry memory of the choice.
- **FR-007**: **Whole file**, the **Files** option, **Back** and **Open diff** MUST keep
  their 1150 behaviour, including Files staying on across a source change with the tree and browsed
  file read again at the new source. **Open diff** from a browsed file MUST show the diff in the
  shown source and leave the source and Files unchanged.
- **FR-008**: Once #1270 has removed the review panel's column, the review body MUST be two
  columns: a file column of fixed width (about 260 px) and the diff filling the rest. Until then
  the columns keep today's proportions. At 1280×800
  the diff MUST take at least 70% of the body's width.
- **FR-009**: The source control's tooltip MUST show the base line. The header subtitle that shows
  it today stays.
- **FR-010**: The control's options, their order, their availability, the closed label (including
  truncation of a long title) and which option is marked MUST be decided by render-free logic that
  unit tests cover without drawing the interface.
- **FR-011**: The control MUST be operable with the keyboard (open, move, pick, close with `Esc`),
  and closing it without a pick MUST NOT change the source.
- **FR-012**: The user guide's review page, the layout snapshot and the visual pass (1280×800,
  light and dark) MUST show the new header and left column.

### Key Entities

- **Source option**: one row of the control. It is Working tree (with its two kind chips), one pull
  request (number, title, state, checks), or Files. Each has whether it is available and whether it
  is the shown source.
- **Source control label**: the closed control's text, derived from the shown source and the
  working-tree kinds, prefixed `Files · ` while Files is on.
- **Working-tree kinds**: the Committed and Uncommitted on/off pair, per open review, as today.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Switching between the working tree and any listed pull request takes at most 2
  actions (open the control, pick), against a scroll-and-click in the section before.
- **SC-002**: At 1280×800, once #1270 has merged, the diff column is at least 70% of the body's
  width (it was 3/7, about 43%, before).
- **SC-003**: The file list starts directly below the header in 100% of source states: no section
  or toggle row sits above it.
- **SC-004**: Every behaviour of 1003 and 1150 listed in FR-006 and FR-007 passes as before. No
  regression is accepted.
- **SC-005**: Every option-list state named in FR-005 has a unit test that runs without the
  interface.

## Assumptions

- The control is the component library's existing select/menu pattern (Principle VIII). If no
  existing component can hold chips inside an option, the library gains that variant. The view does
  not build a one-off.
- The left column's fixed width is about 260 px, from the redesign's layout. Resizing and
  collapsing the column belong to R3 (#1273), not here.
- Until #1270 merges, the right-hand review panel and its toolbar toggle stay where they are. This
  feature does not move the review buttons or counts.
- If #1269 (Review tab) lands first, the header is the Review tab's header. Nothing in this
  feature depends on whether the review is a tab or a view that replaces the terminal.
- The Committed/Uncommitted state and the chosen source keep their current lifetime (per entry,
  while the window is open). #1269 may extend it, and this feature does not change it.
- The mockup artifact was not consulted. The redesign document is the reference.

## Out of scope

- The bottom review bar and the comments drawer (#1270).
- The file tree's collapsing, name-first rows, change bars, filter and sort (#1273).
- Reviewed marks and progress such as *12/30 reviewed* in the header (#1271).
- Keyboard shortcuts beyond operating the control itself (#1274).
- Any change to how pull requests are listed, read or refreshed, to the diff rendering, or to
  comments.
