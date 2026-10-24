# Feature Specification: Review UI — changed-file tree, filter and sort

**Feature Branch**: `claude/project-thread-p2tb7i`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Review UI: file tree, filter and sort (issue #1273). Blocked by: #1272, #1150 (merged). Collapsible directories, name-first rows, +/- bar, filter box, sort by path / size / unreviewed first; reuse `tree_rows`. Part of the review UI redesign: `docs/development/review-redesign.md`. Mockup: Review tab redesign. Acceptance also: pure logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes) updated."

## Background

The Changes view lists the changed files of a worktree, or of one of its pull requests, as a flat
list: one row per file, the full path first, then the change kind, `+a −r` and the number of unsent
comments (`docs/user-guide/reviewing-changes.md`, *What the list shows*). After a large agent run the
directory prefix repeats on every row, the file name — what a reviewer scans for — is the least
prominent part, and there is no way to narrow the list or to see the biggest changes first
(`docs/development/review-redesign.md`, problem 3).

This feature is slice R3 of that redesign (proposal item 2). It turns the changed-file list into a
tree with collapsible folders and name-first rows, draws each file's change size as a small bar, and
adds a filter box and a sort control. The repository browser of feature 1150 (**Files**) already lays
out a tree; this feature shares that tree layout rather than adding a second one.

## Clarifications

### Session 2026-10-24

- Q: Under **Size** and **Unreviewed first**, does the list keep its folders or flatten into a list of
  files? → A: It keeps the tree; each sort orders the rows within each level (scenarios 4.2, 4.3,
  SC-004). _(agent-resolved: docs/development/review-redesign.md#Layout, item 2 "File tree, not flat
  list" names the sorts as options of the tree, and the mockup there shows folders only)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read the changed files as a tree (Priority: P1)

A reviewer opens the Changes view for an agent run that touched 30 files across several folders. The
list shows the folders, each with its changed files below it; each file row leads with the file's
name, and a folder whose only content is one folder shows as one row (`src/daemon/mcp`). The reviewer
collapses a folder they do not care about and its files disappear from the list.

**Why this priority**: The tree with name-first rows is the core of the slice; filter and sort refine
it. Delivered alone, it already removes the repeated prefixes and lets the reviewer hide parts of a
large change.

**Independent Test**: Open a change that touches files in nested folders; check folder rows, chained
single-child folders, name-first file rows, collapsing and expanding, and that selecting a file row
opens its diff.

**Acceptance Scenarios**:

1. **Given** a change touching `src/a.rs`, `src/ui/b.rs` and `README.md`, **When** the list is shown,
   **Then** it shows a folder row `src` with `ui` under it and `b.rs` under `ui`, `a.rs` under `src`,
   and `README.md` at the top level; within a level, folders come before files.
2. **Given** a change whose only changed files are under `crates/core/src/review/`, **When** the list
   is shown, **Then** the chain of single-child folders shows as one folder row labelled
   `crates/core/src/review`, not four nested rows.
3. **Given** an open folder, **When** the reviewer collapses it, **Then** the rows of everything below
   it are hidden and the folder row shows as collapsed; **When** they expand it again, **Then** the
   same rows return, with any sub-folder that was collapsed still collapsed.
4. **Given** a file row, **When** the reviewer selects it, **Then** that file's diff is shown and the
   row is marked selected, as selecting a row does today.
5. **Given** a file row, **Then** the row shows the file's name first and most prominent, then its
   change kind (Added, Deleted, Renamed, Mode; Modified needs no tag), Binary / Not text when that
   applies, its change-size bar, and the number of its unsent comments when there are any; the full
   path (and the old path of a rename) is available on hover.

---

### User Story 2 - See how big each change is at a glance (Priority: P1)

The reviewer scans the tree and sees, beside each file, a small bar whose green and red parts show
lines added and removed, so the large changes stand out without reading numbers. The exact counts
stay readable next to it.

**Why this priority**: Change size is what decides where a reviewer looks first; the bar replaces the
bare `+a −r` text on every row and is part of the same row layout as story 1.

**Independent Test**: Open a change with one large and several small files; check each bar's added
and removed parts against the file's counts and that the largest file has the longest bar.

**Acceptance Scenarios**:

1. **Given** a file with 40 lines added and 10 removed, **Then** its bar's added part is four times its
   removed part, and `+40 −10` is shown with it.
2. **Given** two text files whose total changed lines are 100 and 10, **Then** the first file's bar is
   longer than the second's, and no bar is longer than the row allows.
3. **Given** a binary or non-text file, **Then** its row shows no bar and no counts, and keeps its
   Binary or Not text tag.
4. **Given** a file whose only change is its permissions (Mode), **Then** its row shows no bar.
5. **Given** a folder row, **Then** it shows the summed `+a −r` of the files below it, so a collapsed
   folder still says how much changed inside it.

---

### User Story 3 - Narrow the tree with a filter (Priority: P2)

The reviewer types `tests` into the filter box above the tree. Only files whose path matches stay
listed, inside the folders that hold them, with the matched characters highlighted. Clearing the box
brings back the whole tree as it was.

**Why this priority**: Large runs make finding one file slow; the filter answers "where is the test
for this" without scrolling. Useful only once the tree exists.

**Independent Test**: With a change of 30 files, type a fragment matching three of them; check exactly
those three files and their folders are listed, then clear the filter and check the tree, including
which folders were collapsed, is restored.

**Acceptance Scenarios**:

1. **Given** a change with `src/daemon/tests.rs`, `src/lib.rs` and `docs/tests.md`, **When** the
   reviewer types `tests`, **Then** `tests.rs` and `tests.md` are listed under their folders and
   `lib.rs` is not.
2. **Given** a filter that matches files inside a collapsed folder, **When** it is typed, **Then**
   those files are shown (their folders open while the filter is active); **When** the filter is
   cleared, **Then** that folder is collapsed again.
3. **Given** a filter that matches no file, **Then** the list says that no changed file matches the
   filter and offers to clear it; the diff pane keeps showing what it showed.
4. **Given** the selected file does not match the filter, **When** the filter is typed, **Then** its
   diff stays shown and the file is selected again when the filter is cleared.
5. **Given** a filter, **Then** matching follows the same rules as **Open file by path** (the same
   characters match the same paths), and the matched characters in each row are highlighted.
6. **Given** a non-empty filter, **When** the reviewer presses `Esc` in the box or the box's clear
   control, **Then** the filter is emptied.

---

### User Story 4 - Sort the tree by path, size of change, or unreviewed first (Priority: P2)

The reviewer switches the sort from **Path** to **Size** and sees the most-changed files and folders at
the top of each level. Later they choose **Unreviewed first** to work through what is left.

**Why this priority**: Sorting by size answers "where is the risk"; unreviewed-first supports working
through a large run. Both refine a tree that already works.

**Independent Test**: With files of known sizes, switch between the sorts and check the order at each
level for each sort.

**Acceptance Scenarios**:

1. **Given** the view opens, **Then** the sort is **Path**: at each level folders, then files, each in
   the order the list uses today.
2. **Given** sort **Size**, **Then** at each level folders come before files, folders are ordered by the
   total changed lines (added + removed) of the files below them, largest first, files by their own
   added + removed, largest first, and ties keep path order. Every sort keeps the tree: it orders the
   rows within each level and never flattens the list.
3. **Given** sort **Unreviewed first**, **Then** at each level unreviewed files come before reviewed
   ones and folders holding any unreviewed file before folders holding none, ties in path order.
   [NEEDS CLARIFICATION: reviewed marks come from #1271, which is not built yet. Until it lands, is
   **Unreviewed first** hidden, shown disabled with a note, or offered with every file counted as
   unreviewed (so it orders like Path)?]
4. **Given** a sort other than Path and a filter, **Then** the filtered tree is ordered by that sort.
5. **Given** a chosen sort, **When** the change list is read again (the agent edits a file), **Then**
   the sort stays chosen and the new list is ordered by it.

---

### Edge Cases

- **No changed files**: the list shows the existing "why empty" sentence (**No changes against**
  `<branch>`, **No uncommitted changes**, both kinds hidden); the filter box and sort control are
  hidden, and no empty tree is drawn.
- **Reading or failure**: while the list is read the existing **Reading changes…** text shows; when
  the read fails the existing error shows. A filter, sort and collapsed folders chosen before stay
  chosen and apply when the list is back.
- **A pull request not readable or too large**: the existing pull-request messages show unchanged;
  the tree, filter and sort apply only when there is a list of files.
- **One file only**, or **all files at the top level**: no folder rows; file rows only.
- **Very deep paths**: indentation stops growing past a depth that would leave the name unreadable at
  the list's narrowest width; the name is never cut before the folder indentation is.
- **Long file names**: the name truncates with an ellipsis; the bar, counts and comment count keep
  their place; the full path is on hover.
- **Large changes** (5,000 files): only the rows on screen are drawn, as today, and collapsing,
  filtering and sorting stay responsive (SC-003).
- **A file with zero added and zero removed lines** (a pure rename, a Mode change): no bar; the
  change-kind tag says what changed.
- **Rename across folders**: the file sits under its new path's folder; the old path is on hover.
- **The list changes while shown** (the agent writes files; Principle II: sessions keep working while
  a reviewer reads): the tree is rebuilt from the new list, keeping the filter, the sort, the
  selection when the file is still listed, and every folder's collapsed state; a folder that is new
  appears open; a folder whose files are all gone disappears; the scroll position is kept when the
  rows above it did not change.
- **Two windows on the same entry**: each window has its own collapsed folders, filter and sort; one
  window's choices never change the other's. Unsent comment counts on rows stay shared, as today.
- **Collapsing the folder that holds the selected file**: the folder collapses, the file stays
  selected and its diff stays shown; expanding the folder shows its row selected again.
- **A chained folder splits or merges** (files appear or disappear so `a/b/c` becomes `a` and
  `a/b/c`, or the reverse): collapsed state belongs to each real folder path; a chained row shows
  collapsed when any folder in its chain is collapsed.
- **Selection inside a collapsed folder**: when a file is selected by anything other than its row (a
  reference followed from a diff, **Open diff** from a browsed file), its folders open so its row is
  visible.
- **Switching source** (working tree ↔ a pull request): the tree shows the new source's files; the
  filter and sort are kept; collapsed state is kept per folder path, so a folder of the same path
  stays collapsed.
- **Cross-platform (Principle VI)**: paths are shown with `/` on every platform; ordering by path and
  filter matching give the same result on macOS, Linux and Windows for the same list; hover, `Esc` and
  the clear control behave the same on each.
- **Repository browser (Files)**: its tree keeps working as it does today; this feature does not add
  the filter box, the bar or the sort to it.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The changed-file list MUST show the files as a tree: a folder row for each folder that
  holds a changed file, its contents indented below it, folders before files at each level.
- **FR-002**: A chain of folders where each holds exactly one folder and no file MUST show as one
  folder row labelled with the joined path (`a/b/c`).
- **FR-003**: Every folder row MUST collapse and expand on activation; collapsing hides all rows
  below it, and expanding restores them with each sub-folder's own collapsed state.
- **FR-004**: When the view opens, every folder MUST be open; a folder's collapsed state MUST be kept,
  by folder path, for as long as the view is kept (across list re-reads and source switches).
- **FR-005**: A file row MUST show the file's own name first and most prominent, then its change-kind
  tag (Added, Deleted, Renamed, Mode; none for Modified), a Binary / Not text tag when that applies,
  its change-size bar with `+a −r`, and its unsent-comment count when non-zero; hovering it MUST show
  the full path and, for a rename, the old path.
- **FR-006**: A text file's bar MUST show added and removed lines as two parts in proportion to each
  other, and its total length MUST grow with the file's added + removed lines relative to the other
  files listed, never exceeding the bar's maximum width; a file with no counted lines (binary, not
  text, Mode only, pure rename) MUST show no bar.
- **FR-007**: A folder row MUST show the summed `+a −r` of the files below it.
- **FR-008**: Selecting a file row MUST open that file's diff and mark the row selected, as the list
  does today; selecting a file by other means MUST open the folders above it.
- **FR-009**: Above the tree there MUST be a filter box; while it holds text, only files whose path
  matches it MUST be listed, with the folders that hold them shown open, using the same matching as
  **Open file by path**, with matched characters highlighted. The filter only decides which files
  are listed: it does not reorder them (the sort does) and does not add the typed text as a path.
- **FR-010**: Clearing the filter (emptying the box, its clear control or `Esc` in the box) MUST
  restore the tree with the folders' collapsed state as it was before filtering.
- **FR-011**: A filter that matches no file MUST show a sentence saying no changed file matches and a
  way to clear the filter.
- **FR-012**: A sort control MUST offer **Path** (the default), **Size** and **Unreviewed first**,
  ordering each level as acceptance scenarios 4.1–4.3 describe.
- **FR-013**: The filter text and the chosen sort MUST be kept for as long as the view is kept, across
  list re-reads and source switches, and MUST NOT be shared with other windows.
- **FR-014**: When the change list is read again, the tree MUST keep the selection (if the file is
  still listed), the filter, the sort and the folders' collapsed state.
- **FR-015**: The tree MUST draw only the rows on screen, as the flat list does today.
- **FR-016**: The empty, reading, failed and pull-request-unreadable states of the list MUST look and
  read as they do today; with no changed files the filter box and sort control MUST be hidden.
- **FR-017**: The tree's layout (folder grouping, chaining, ordering, filtering, sorting, totals, bar
  proportions) MUST be computed by render-free logic in the shared core, covered by unit tests, and
  the repository browser's tree (**Files**) MUST keep its current behaviour while sharing that layout.
- **FR-018**: `docs/user-guide/reviewing-changes.md` (*What the list shows*), the layout snapshot and
  the visual pass at 1280×800 in light and dark themes MUST be updated to show the tree, the bar, the
  filter box and the sort control.

### Key Entities

- **Changed file**: an entry of the change list — path, old path for a rename, change kind, text or
  not, lines added and removed, unsent-comment count, and (once #1271 lands) reviewed or not.
- **Tree row**: one visible row — a folder (joined label, depth, open or collapsed, summed counts) or a
  file (name, depth, the changed file it stands for, matched-character spans when filtered).
- **List view choices**: per open view, not shared — filter text, sort (Path, Size, Unreviewed first),
  and the set of collapsed folder paths.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a fixture change of 30 files across 5 folders where `daemon/tests.rs` is the only
  path containing `tests.r`, typing `tests.r` into the filter leaves exactly that file listed, and it
  opens with one click and no scrolling.
- **SC-002**: For a change of 30 files, the list shows every file's name without its folder prefix,
  and the number of characters of repeated directory prefix drawn on file rows is zero.
- **SC-003**: With 5,000 changed files, collapsing a folder, typing a filter character or changing the
  sort updates the list within 100 ms in a release build on a developer laptop (measured by the core
  layout benchmark or a timed unit test), and scrolling stays smooth.
- **SC-004**: With sort **Size**, the file with the most changed lines is the first file row of its
  level in 100% of test cases, and the folder holding the most changed lines is the first folder row
  at the top level.
- **SC-005**: After a list re-read while the reviewer has a filter, a sort and collapsed folders, all
  three are as they were in 100% of test cases.
- **SC-006**: The visual pass at 1280×800 shows the tree, bars, filter box and sort control legible
  in both light and dark themes, with no clipped name at the default list width for names up to 24
  characters.

## Assumptions

- **Depends on #1272 (not yet merged).** #1272 owns the two-column layout, the source dropdown, and
  moving the Committed / Uncommitted / Files toggles and the Pull requests section out of the left
  column. This spec covers only what the left column's file list shows and how it is narrowed and
  ordered. The left column's width, resizing and collapsing, and where the filter box sits relative to
  the header chrome, follow #1272's layout; if #1272 lands differently the filter box and sort control
  sit directly above the tree in whatever left column it leaves.
- **Reviewed marks come from #1271.** This feature does not add reviewed marks, the `○ ● ✓` glyphs on
  rows, or the *n of m reviewed* count; it only orders by the reviewed state when it exists (see the
  open question in scenario 4.3).
- **Reuses #1150's tree** (`tree_rows`, folder open state) rather than adding a second tree layout;
  the **Files** repository browser keeps its look and behaviour.
- **Filter matching** is the matcher **Open file by path** already uses, applied to the full path of
  each changed file.
- **Collapsed folders, filter and sort are view state**: kept while the view is kept (with #1269 that
  is across tab and session switches), not persisted across an app restart, not shared between
  windows. No daemon or protocol change.
- **Bar scale** is relative to the largest change in the current list, so bars stay comparable within
  one review; the exact maximum width and colours follow the design tokens and the mockup.
- **Out of scope**: keyboard navigation of the tree (`j`/`k`, #1274); reviewed marks and progress
  (#1271); stale-awareness dots (#1275); the source dropdown and two-column layout (#1272); the
  review bar and comments drawer (#1270); changing the **Files** repository browser; persisting tree
  choices across restarts.
