# Feature Specification: Review UI — Stale-Awareness and Quick Comment

**Feature Branch**: `claude/project-thread-1l3w9s`

**Created**: 2026-10-24

**Status**: Draft

**Input**: GitHub issue #1275 "Review UI: stale-awareness and quick comment". "Blocked by: #1271.
'n files changed since you looked' without reflowing the diff being read; drag over line numbers
opens the composer; optional suggestion chips (off by default). Part of the review UI redesign:
`docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu
(Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout
snapshot and visual pass (1280x800, both themes) updated."

## Context

This is slice R6 of the review UI redesign (`docs/development/review-redesign.md`, items 8 and 9 of
*Proposal*). It changes two things in the view where a worktree's changes are reviewed (today the
**Changes** view, later the **Review** tab):

1. **Today the diff moves under the reader.** When a file in the entry changes, the list and the
   diff on screen are read again within about two seconds with nothing to press
   (`docs/user-guide/reviewing-changes.md`, *Keeping up with changes*). While an agent is editing,
   the lines being read jump, scroll or vanish.
2. **Today a comment takes three steps.** Click a line number, **Shift**-click another, press
   **Add comment** (*Picking lines*, *Writing a comment*).

### What depends on which blocker

| Part | Needs | Can ship before its blocker |
|---|---|---|
| Stale tracking: which files differ between what is shown and what was last read, and the count (FR-001–FR-004) | a per-file change identity added to the list read (FR-001a); the comparison is then pure logic over two change lists | yes |
| Holding the shown snapshot and the *n files changed since you looked* notice in the view (FR-005–FR-012, FR-014) | the view that exists when it is built: today's Changes view, or the Review tab of #1269 | yes, in today's Changes view; it moves with the view when #1269 lands |
| Visible-tab only: while the Review tab is hidden, #1269 already marks it changed and re-reads when shown | #1269 | the hidden-tab part is #1269's, not this feature's |
| A held change to a file marked reviewed returns it to unseen with the changed dot (FR-013) | #1271 (reviewed marks) | no: this requirement is built with or after #1271 |
| Drag over line numbers picks and opens the comment box (FR-015–FR-021) | nothing beyond today's picking and comment box | yes |
| Suggestion chips and their setting (FR-022–FR-027) | nothing beyond today's comment box and Settings | yes |

The issue lists #1271 as its blocker because the redesign sequences R6 after R4; only FR-013 truly
needs it. The plan may ship the other parts first, in today's view.

## Clarifications

### Session 2026-10-24

- Q: While changes are held, is the whole view (file list, counts, every file) frozen, or only the
  open file's diff? → A: Whole-view hold (FR-014): the list, its counts and every file stay one
  snapshot until the notice is pressed; changed files carry a dot and count in *n*; selecting a
  dotted file shows its latest diff. _(A (recommended; provisional, awaiting user):
  `docs/development/review-redesign.md` item 8 counts 3 files with row dots while one diff is open)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read a diff without it moving while the agent works (Priority: P1)

A developer is reading a file's diff in the review view while the session's agent keeps editing
files in the same worktree. The diff on screen stays exactly as it is: no lines appear, vanish or
shift. Instead the header says *3 files changed since you looked*, and the rows of those files carry
a dot. When the developer is ready, they press the notice and the view shows the latest changes in
one step, keeping their selected file, picked lines and the comment they were writing.

**Why this priority**: The reported problem of the redesign: re-reading within two seconds moves
what the user is reading. It is the larger half of the issue and it needs no blocker.

**Independent Test**: Open the view on a worktree with changes, select a file, then edit that file
and two others from a terminal. Within about two seconds the header reads *3 files changed since you
looked*, the three rows carry a dot, and the diff on screen and its scroll position do not change.
Press the notice: the diff shows the edits, the dots and the notice go.

**Acceptance Scenarios**:

1. **Given** the view shows a file's diff, **When** that file and two others change on disk,
   **Then** within about two seconds of the burst settling the header shows *3 files changed since
   you looked*, those three file rows carry a dot, and the diff, its lines and its scroll position
   stay as they were.
2. **Given** the notice shows, **When** the user presses it, **Then** the view shows the latest
   list and diffs, the selected file stays selected while still listed, the dots and the notice
   disappear, and the count starts again from zero.
3. **Given** a held change, **When** the same file changes again, **Then** the count does not grow
   for it (each file counts once).
4. **Given** a held change to a file, **When** the file's content changes back to what it was when
   shown, **Then** that file loses its dot and leaves the count; at zero the notice disappears.
5. **Given** a new file appears in the entry or a listed file's changes are reverted, **When** the
   view notices it, **Then** it counts in *n files changed* like an edited file.
6. **Given** lines are picked and a comment is being written, **When** a change is held and later
   applied, **Then** the comment box keeps its text in both cases, and the picked lines are kept
   wherever they are still in the diff after the apply (today's rules for a refresh).
7. **Given** the user changes what the view shows themselves (switches the source, the committed or
   uncommitted toggle, the pull request, or the base), **When** that change is made, **Then** the view
   reads and shows the result at once, as today, and nothing remains held.
8. **Given** the view is showing a pull request, **When** the pull request list refreshes with a new
   head for it, **Then** the files whose diff differs are counted and held the same way as edits in
   the working tree.
9. **Given** changes are held, **When** the user selects a file that is not
   in the changed-since set, **Then** its diff is read and shown, as today; it matches the shown
   snapshot because the file has not changed since.
10. **Given** changes are held, **When** the user selects a file in the
   set that the latest read still lists, **Then** the view reads and shows that file's latest diff,
   its row takes the latest counts and loses its dot, it leaves the count, and every other held file
   stays held.
11. **Given** changes are held, **When** the user selects a file in the
   set that the latest read no longer lists (its changes were reverted or it was removed), **Then**
   the diff area says the file has no changes any more, the row keeps its dot and stays in the list
   until the user applies.
12. **Given** a file already shown as modified, **When** an already-changed line of it is edited
   again so its added and removed counts stay the same, **Then** it counts and carries a dot.

Scenarios 1 (the two other files), 5, 9, 10 and 11 follow from the whole-view hold (FR-014).

---

### User Story 2 - Comment on a range with one drag (Priority: P1)

A developer wants to comment on lines 12–18. They press on line 12's number, drag to line 18's
number and let go: the seven lines are picked and the comment box is already open under line 18,
ready for typing.

**Why this priority**: The most frequent review action, cut from three steps to one gesture. Needs
no blocker.

**Independent Test**: In a text diff, press on one added line's number, drag over five more numbers
on the same side, release. The six lines are picked and the comment box is open under the last one
with the focus in it; typing and **Ctrl+Enter** saves the draft.

**Acceptance Scenarios**:

1. **Given** a text diff, **When** the user presses on a line number and releases on another line
   number of the same side, **Then** every line between the two (inclusive, either direction) is
   picked and the comment box opens under the line released on, with the focus in it.
2. **Given** a drag in progress, **When** the pointer passes over numbers, **Then** the pick shown
   follows it line by line, before release.
3. **Given** a drag started on a new-side number, **When** the pointer passes over rows with only an
   old-side number, a hunk header, a collapsed gap or a comment card, **Then** those rows do not
   change the pick's side; the pick ends at the last line of the pressed side the pointer passed.
4. **Given** the user presses and releases on the same number without moving to another line,
   **When** they release, **Then** the behaviour is today's click: that line is picked and **Add
   comment** is offered; no comment box opens by itself.
5. **Given** a comment box for a new comment is open with text, **When** the user drags a new
   range, **Then** the box moves under the new range and keeps its text.
6. **Given** a drag in progress, **When** the user presses **Esc**, **Then** the drag is cancelled
   and the pick and comment box are as they were before the press.
7. **Given** a binary, non-text or not-shown large file, **When** the user drags in it, **Then**
   nothing is picked, as clicks pick nothing there today.

---

### User Story 3 - Fill common comment text with a chip (Priority: P3)

A developer who writes the same short comments often turns on **Suggestion chips** in Settings.
The comment box then shows chips (*Rename*, *Add test*, *Why?*) under it; pressing one puts its text
in the box, ready to finish or save.

**Why this priority**: Convenience, off by default; the issue calls it optional.

**Independent Test**: With the setting off (a fresh install) open a comment box: no chips. Turn the
setting on and save; open a comment box: three chips show. Press *Add test* in an empty box: the box
holds that chip's text and the focus stays in the box.

**Acceptance Scenarios**:

1. **Given** default settings, **When** a comment box opens, **Then** no chips show and the box looks
   and behaves as today.
2. **Given** the setting is on, **When** a comment box opens (new comment or **Edit**), **Then** the
   chips *Rename*, *Add test* and *Why?* show under the text box.
3. **Given** the setting is on and the box is empty, **When** the user presses a chip, **Then** the
   box holds the chip's text, **Save** becomes available, and the focus is in the box at the end of
   the text.
4. **Given** the box already has text, **When** the user presses a chip, **Then** the chip's text is
   added after the existing text on a new line; nothing typed is lost.
5. **Given** the setting is turned off, **When** a comment box is next shown, **Then** it shows no
   chips.

---

### Edge Cases

- **Nothing to read yet**: the view's list is empty (no changes) or failed to load. A change found
  then is applied at once: there is no shown diff to keep still.
- **Re-read fails**: a background read of the latest changes fails. The shown diff stays, no notice
  appears and no error is shown (as a failed refresh today); the next change tries again.
- **Open file removed by the agent**: it counts and carries its dot; the user keeps reading it until
  they apply. On apply it leaves the list as today, and an unsaved new comment on it is closed as
  today (*Writing a comment*).
- **Selected between reads**: a file changed after the last background read but before the user
  selects it shows the diff read on selection; its row takes that read's counts and identity, and
  the next background read compares against them, so it does not count for that change.
- **Large files**: a working-tree file over the size limits is identified by its size and
  modification time instead of its content (FR-001a); touching it without changing its bytes counts
  it once, which is accepted.
- **Comment written while stale**: saving a comment on the held diff quotes the lines shown. On
  apply, the comment follows its code by today's anchoring (stays, moves or is marked outdated).
- **Changes keep arriving**: during a long agent run the count keeps up within about two seconds of
  each settled burst; pressing applies the latest finished read. A read still running when the user
  presses is not waited for: if it finishes with further differences, the notice comes back.
- **Two windows, same entry** (Principle II): each window holds and applies on its own; what one
  window applies does not apply, clear or count in the other. Comments, statuses and reviewed marks
  stay shared as today.
- **Several sessions in one worktree** (Principle II): changes made by any session, or by the user
  outside the app, count the same; the view cannot tell them apart and does not try.
- **Different worktrees**: a change in one entry never counts in another entry's view.
- **Whole file and browsed files**: a whole-file view of a listed file is held with its diff. The
  repository browser (**Files**) keeps today's behaviour.
- **Drag leaves the diff**: released outside the gutter or outside the window, the pick ends at the
  last number passed and the comment box opens. Dragging does not scroll the diff.
- **Edit box open**: while an existing comment's text is being edited, a drag picks lines but does
  not replace the edit box or its text.
- **Read-only states**: where picking is not allowed today, dragging picks nothing.
- **Cross-platform** (Principle VI): the drag works with a mouse and with a touchpad press-and-drag
  on Linux, macOS and Windows; **Shift**-click is unchanged; the save shortcut stays **Ctrl+Enter**
  (**Cmd+Enter** on macOS). Holding and counting do not depend on how each platform reports file
  changes: a burst counts once it settles, as today.

## Requirements *(mandatory)*

### Functional Requirements

**Stale tracking (pure, ships first)**

- **FR-001**: Given the change list the view shows and the latest change list read, the system MUST
  report the set of files whose change differs between them, matching rows by path: a file listed in
  only one of them, or listed in both with a different kind, added or removed count, or change
  identity (FR-001a). A rename counts as one file under its new path: a latest row renamed from a
  path the shown list has does not count that old path separately.
- **FR-001a**: Each row of the change list MUST carry a change identity that differs whenever the
  file's content on either side of its diff differs, including an edit that keeps the counts the
  same. The identity is what the list read can give cheaply, with no per-file diff read: for a
  version in git (a commit or the index) its object id, which the list read's `git diff --raw`
  already reports; for a working-tree version, which `--raw` reports without an id, the file's
  content hash from one `git hash-object` call over those paths per read (untracked files too); for a
  file over the size limits, its size and modification time instead of its hash; for a pull request,
  a hash of the file's patch in the request's diff, which is already read whole.
- **FR-002**: The count shown MUST equal the size of that set; a file counts once however many
  times it changed.
- **FR-003**: A file whose latest kind, counts and change identity are again equal to what is shown
  MUST leave the set.
- **FR-004**: Applying MUST make the latest read the shown one and empty the set.

**Holding and the notice**

- **FR-005**: While the review view is shown and has a list on screen, a change found by the file
  watch (or a new pull request head for the shown pull request) MUST NOT change the shown diff or
  its scroll position, nor the shown list and its counts (FR-014), until the user applies it.
- **FR-006**: The view MUST still read the latest changes in the background, within about two
  seconds of a settled burst as today, and update the set and the count.
- **FR-007**: While the set is not empty the view's header MUST show *n file(s) changed since you
  looked* (singular for one), and that notice MUST apply the changes when pressed.
- **FR-008**: Each listed file in the set MUST carry a *changed since you looked* dot on its row.
  A file in the set that the shown list does not yet have MUST still count in *n*.
- **FR-009**: Changes the user makes to what the view shows (source, toggles, pull request, base)
  MUST take effect at once and leave nothing held. Selecting another file MUST NOT apply the other
  held changes. Selecting a file not in the set reads and shows its diff as today; selecting a file
  in the set that the latest read lists MUST read and show its latest diff, update its row and drop
  it from the set; selecting one the latest read no longer lists MUST say it has no changes any more
  and keep it in the set (US1 scenarios 9–11). For a pull request every file's diff is already in memory and selection shows the held snapshot's patch,
  except for a file in the set, which shows the latest patch and leaves the set.
- **FR-010**: Applying MUST keep the selected file while it is listed, and MUST keep picked lines
  and comment-box text by the rules a refresh follows today.
- **FR-011**: The comment-box text MUST be kept while changes are held.
- **FR-012**: When the view has no list on screen (empty, not yet read, or failed), a change MUST be
  applied at once.
- **FR-013** *(needs #1271)*: A held change to a file marked reviewed MUST show that file as unseen
  with the changed dot, and the *n of m reviewed* progress MUST drop by one for it.
- **FR-014**: The hold MUST cover the whole view: the file list, its counts and every file's diff
  stay one snapshot until the user applies (or selects a file in the set, FR-009); nothing on screen
  moves. The open diff alone being held, with the list updating live, does not meet this.

**Quick comment**

- **FR-015**: Pressing on a line number and releasing on another line number of the same side MUST
  pick every line between the two inclusive and open the comment box under the line released on,
  with the focus in it.
- **FR-016**: While dragging, the pick shown MUST follow the pointer line by line.
- **FR-017**: The pick's side MUST be the side pressed; rows without a number on that side MUST NOT
  change it, and the pick MUST end at the last line of that side the pointer passed.
- **FR-018**: A press and release on the same line MUST behave as today's click (pick only, **Add
  comment** offered); **Shift**-click MUST keep extending a pick as today.
- **FR-019**: A drag with a new comment's box already open MUST move the box to the new range and
  keep its text; a drag while an existing comment is being edited MUST NOT close or replace that box.
- **FR-020**: **Esc** during a drag MUST restore the pick and comment box from before the press.
- **FR-021**: Dragging MUST pick nothing where clicking picks nothing today.

**Suggestion chips**

- **FR-022**: Settings MUST offer a **Suggestion chips** switch, off by default, saved with the other
  settings and kept across restarts.
- **FR-023**: With the switch on, every comment box (new and **Edit**) MUST show the chips *Rename*,
  *Add test* and *Why?*; with it off, none.
- **FR-024**: Pressing a chip in an empty box MUST set the box to the chip's text; in a box with text
  it MUST add the chip's text on a new line after it. The focus MUST stay in the box at the end.
- **FR-025**: Chip text MUST count as typed text: **Save** becomes available, and the text is kept
  across holds and applies like typed text.
- **FR-026**: A change to the switch MUST show in comment boxes opened after the change is saved.
- **FR-027**: Chips MUST be reachable by keyboard focus order after the text box, and announce their
  label.

**Acceptance (from the issue)**

- **FR-028**: The decisions above (the stale set and count, a drag's resulting pick, chip
  insertion, the setting's default) MUST live in the render-free core with unit tests.
- **FR-029**: `docs/user-guide/reviewing-changes.md` (*Keeping up with changes*, *Picking lines*,
  *Writing a comment*) and `docs/user-guide/settings.md` MUST describe the new behaviour; the layout
  snapshot MUST include the notice, the row dot and the chips; a visual pass at 1280×800 in light and
  dark MUST cover them.

### Key Entities

- **Shown snapshot**: the change list the view is displaying, each row with its change identity,
  and the open file's diff. For a working tree only the open file's diff is in memory; another file's
  diff is read when it is selected (FR-009). For a pull request the whole request's diff is.
- **Change identity**: per row, what tells whether its content changed (FR-001a).
- **Latest read**: the most recent change list read in the background for the same entry and
  source.
- **Changed-since set**: the files that differ between the two; its size is *n*.
- **Drag pick**: the side and line pressed, and the last line of that side the pointer passed.
- **Suggestion chip**: a label and the text it inserts.
- **Suggestion chips setting**: on/off, default off, a per-user setting.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a run where 10 files are edited one after another while a file is open, the lines
  on screen and the first visible line do not move once until the user presses the notice.
- **SC-002**: The notice appears, with the right count, within 2 seconds of a burst of edits
  settling, in every scripted case of US1.
- **SC-003**: Commenting on a range takes one gesture plus typing, down from three actions (click,
  **Shift**-click, **Add comment**).
- **SC-004**: On a fresh install, the comment box shows no chips and is otherwise unchanged.
- **SC-005**: Comment-box text is kept in 100% of holds and applies; picked lines are kept in every
  case where their lines remain in the diff.
- **SC-006**: The user guide, layout snapshot and a 1280×800 visual pass in both themes show the
  notice, the dot and the chips.

## Assumptions

- Chip texts: *Rename* → "Rename this to ", *Add test* → "Add a test that covers this.", *Why?* →
  "Why is this needed?". The set is fixed and not user-editable in this feature.
- The notice sits in the view's header (today the Changes view header; the Review tab header once
  #1269 lands); the dot sits on the file row of whatever list the view has (flat today, tree after
  #1273).
- "Since you looked" is per window and in memory: it is not persisted and not shared across
  windows. It restarts when the view is reopened.
- The background read and its two-second budget reuse today's file watch and list read, which adds
  the change identity of FR-001a (one extra `git hash-object` call per read); no new watch and no
  per-file diff read.
- The setting's place in the Settings sections is the plan's choice.

## Out of Scope

- The hidden Review tab's *changed since you looked* mark and lazy re-read (#1269).
- Reviewed marks, their storage and the *n of m reviewed* progress themselves (#1271); this feature
  only adds FR-013's interaction with them.
- A key to apply the held changes, and the rest of the keyboard map (#1274).
- Auto-scrolling the diff while dragging past its edge.
- A control to show the latest of one file without selecting it (selecting a held file does, FR-009).
- User-defined or editable suggestion chips.
- The repository browser (**Files**) refresh behaviour.
- Final placement in the redesigned layout and the mockup sync (#1270, #1272, #1273, #1276).
