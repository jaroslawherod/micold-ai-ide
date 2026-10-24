# Feature Specification: Keyboard Map for Reviewing Changes

**Feature Branch**: `claude/project-thread-m7gq7k`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Issue #1274: Review UI: keyboard map for reviewing. Blocked by: #1273,
#1271, #1127. j/k file, n/p hunk, ]/[ comment, v reviewed, g next unreviewed, c comment,
Ctrl+Shift+Enter send, ? shortcut sheet; the jump-to field finds files of the open review and
comments. Part of the review UI redesign: `docs/development/review-redesign.md`. Mockup:
https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu (Review tab redesign). Acceptance also: pure logic
in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes)
updated."

This is slice R5 of `docs/development/review-redesign.md` (Proposal item 6, *Next/prev everywhere*).

## Clarifications

### Session 2026-10-24

- Q: When the comment composer has unsaved text, does the send chord save it as a draft and send it,
  or send only the saved drafts? → A: Only the saved drafts are sent; the composer stays open with
  its text and the keyboard (User Story 4 scenario 4, FR-010). _(agent-resolved:
  spec.md#FR-010 "exactly the result of pressing Send review (n)";
  crates/micold-client/src/features/changes.rs `Msg::SendPressed` counts saved pending comments only
  and leaves the composer untouched; spec.md#FR-009 and docs/development/review-redesign.md#proposal
  item 8: composer text is kept)_

## Dependencies

This feature adds keys and jump-to entries on top of work that is not merged yet. It uses what those
issues deliver and does not define it:

- **#1269 (R0) Review tab**: the per-entry **Review** tab that keeps its view while hidden. "The
  Review tab is shown" below means that tab is the displayed body of the window.
- **#1270 (R1) review bar and comments drawer**: the bar's **Send review (n)** and overflow menu, and
  the drawer with its **Drafts** / **Replies** tabs.
- **#1273 (R3) file tree, filter, sort**: the tree's order of files under the current sort and
  filter, and collapsible directories. File order below is that order.
- **#1271 (R4) reviewed marks**: each file's reviewed state (unseen, has drafts, reviewed), the
  **Mark reviewed** control, and storage that every window agrees on. `v` and `g` act on those marks.
- **#1127 title bar**: the title bar's jump-to field, which opens the existing quick-switcher of
  `specs/968-quick-switcher/spec.md` (Ctrl/Cmd+Shift+P). This feature only adds entries to it.

Where one of these is not merged when a story is built, that story waits for it; no story builds a
stand-in for another issue's work.

## Terms

- **Review keys**: the single keys of FR-001 plus the send chord of FR-010.
- **Review has the keyboard**: the Review tab is shown, and no text field (comment composer, tree
  filter, drawer edit field, jump-to field, any other input), dialog, menu, dropdown or the
  quick-switcher holds the keyboard, Settings is closed and the Overview is closed.
- **Selected file**: the file whose diff the Review tab shows.
- **File order**: the order of files in the file tree (#1273) under its current sort and filter,
  read top to bottom with every directory taken as expanded.
- **Current hunk**: the hunk the last `n`/`p` press landed on in the selected file, or, before any
  press, none.
- **Comment order**: the comments of the shown source (working tree or a pull request), ordered by
  their file in file order, then by their line in the diff as shown.
- **Shortcut sheet**: the panel that lists the review keys (FR-012).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Move through files and hunks without the mouse (Priority: P1)

A developer reviewing a 30-file agent run wants to step from file to file and from change to change
with one key each, so reading a large run does not need a click per file and a scroll per hunk.

**Why this priority**: moving is the step every other key builds on, and on its own it removes most
of the pointer work of reading a review (problem 6 of the redesign).

**Independent Test**: open the Review tab of an entry with three changed files, two hunks each, and
press `j`, `j`, `k`, `n`, `n`, `n`, `p` with no pointer input; the selected file and the hunk in view
follow each press.

**Acceptance Scenarios**:

1. **Given** the Review tab is shown with `a.rs`, `b.rs`, `c.rs` in file order and `a.rs`
   selected, **When** the user presses `j`, **Then** `b.rs` is selected, its diff is shown from the
   top and its row is in view in the tree.
2. **Given** `c.rs`, the last file, is selected, **When** the user presses `j`, **Then** `a.rs`, the
   first file, is selected (the order wraps); `k` on the first file selects the last.
3. **Given** `b.rs` is selected with no current hunk, **When** the user presses `n`, **Then** the
   diff scrolls so that `b.rs`'s first hunk header is at the top of the diff and that hunk is
   marked as current.
4. **Given** the current hunk is the last hunk of `b.rs`, **When** the user presses `n`, **Then**
   `c.rs` is selected and its first hunk is current; `p` on the first hunk of a file selects the
   previous file and makes its last hunk current.
5. **Given** the tree filter shows only `b.rs` and `c.rs`, **When** the user presses `j` and `k`,
   **Then** only those two files are visited.
6. **Given** the comment composer has the keyboard, **When** the user types `j`, **Then** the
   letter goes into the comment and the selection does not move.

---

### User Story 2 - Mark progress and jump to what is left (Priority: P1)

The developer marks each file reviewed as they finish it and jumps straight to the next file not yet
reviewed, so a long run is worked through to the end without hunting for what remains.

**Why this priority**: this is the keyboard half of the reviewed marks (#1271), the feature the
redesign names as the one that makes large runs tractable.

**Independent Test**: with five files of which two are marked reviewed, press `v` on the selected
file and `g` three times; the mark toggles and `g` visits the unreviewed files in file order.

**Acceptance Scenarios**:

1. **Given** the selected file is not marked reviewed, **When** the user presses `v`, **Then** the
   file is marked reviewed exactly as the **Mark reviewed** control marks it, its row shows the
   reviewed state and the header's *n of m reviewed* count goes up by one.
2. **Given** the selected file is marked reviewed, **When** the user presses `v`, **Then** the mark
   is removed and the count goes down by one.
3. **Given** files 1–5 in file order, files 2 and 4 not reviewed and file 1 selected, **When** the
   user presses `g`, **Then** file 2 is selected; a second `g` selects file 4, a third selects file
   2 again.
4. **Given** every file is marked reviewed, **When** the user presses `g`, **Then** the selection does
   not move and a short note reads "All files reviewed".
5. **Given** another window of the same project marks a file reviewed, **When** the user presses `g`
   here, **Then** that file is skipped.

---

### User Story 3 - Step through comments and write one (Priority: P2)

The developer steps through the comments already written (their own drafts and the sent ones with
the agent's replies) and opens the composer from the keyboard on the lines they are reading.

**Why this priority**: comments are the review's output; reaching each one and adding one without the
mouse completes the keyboard flow, but reading and marking (P1) come first.

**Independent Test**: with three comments in two files, press `]` three times and `[` once, then
`c`; each press shows the next or previous comment card and `c` opens the composer.

**Acceptance Scenarios**:

1. **Given** comments on `a.rs` line 3, `a.rs` line 20 and `c.rs` line 7, and `a.rs` selected,
   **When** the user presses `]` three times, **Then** the diff shows each card in turn, selecting
   `c.rs` for the third; a fourth `]` wraps to `a.rs` line 3.
2. **Given** a comment card is shown by `]`, **When** the user presses `[`, **Then** the previous
   comment in comment order is shown.
3. **Given** a comment whose line is no longer in the diff (outdated, shown in the drawer only),
   **When** `]` reaches it, **Then** its file is selected and the comment drawer opens on it.
4. **Given** lines are picked in the gutter, **When** the user presses `c`, **Then** the composer
   opens on the picked lines with the keyboard in it, as **Add comment** does.
5. **Given** no lines are picked and a current hunk exists, **When** the user presses `c`, **Then**
   the current hunk's changed lines are picked and the composer opens on them.
6. **Given** no lines are picked and there is no current hunk, **When** the user presses `c`,
   **Then** nothing opens and a short note reads "Pick lines or move to a hunk (n) to comment".

---

### User Story 4 - Send the review from the keyboard (Priority: P2)

Having written their comments, the developer sends the review with one chord, without reaching for
the bar's button.

**Why this priority**: it closes the loop of a keyboard-only review; the button already does the
same thing, so it is a convenience over P1.

**Independent Test**: with two draft comments and a running session in the entry, press
Ctrl+Shift+Enter (Cmd+Shift+Enter on macOS); the review is sent exactly as **Send review (2)** sends
it.

**Acceptance Scenarios**:

1. **Given** the Review tab is shown with 2 drafts, **When** the user presses the send chord,
   **Then** the review is sent with the same message, target session, statuses and result note as
   pressing **Send review (2)**.
2. **Given** there are no drafts, **When** the user presses the send chord, **Then** nothing is sent
   and nothing changes.
3. **Given** a send of this entry is under way in any window, **When** the user presses the send
   chord, **Then** nothing is sent, as **Sending…** is unavailable.
4. **Given** the comment composer has the keyboard with text in it, **When** the user presses the
   send chord, **Then** only the saved drafts are sent, exactly as **Send review (n)** sends them;
   the composer stays open with its text and keeps the keyboard, and its text is not saved or sent.
   With no saved drafts, nothing is sent (scenario 2).

---

### User Story 5 - Find the keys (Priority: P2)

A developer new to the review keys presses `?` and sees every key the Review tab answers to, so the
map can be learnt without the user guide.

**Why this priority**: single-letter keys are invisible without a list; the sheet makes P1–P2
discoverable.

**Independent Test**: press `?` in the Review tab; the sheet lists every review key with what it
does, in the current platform's form; Escape closes it.

**Acceptance Scenarios**:

1. **Given** the review has the keyboard, **When** the user presses `?`, **Then** the shortcut sheet
   opens over the Review tab and lists every review key of FR-001 and FR-010 and the existing
   **Ctrl+Enter** (Cmd+Enter) save-comment chord, each with what it does.
2. **Given** the sheet is open, **When** the user presses Escape, `?` or clicks outside it, **Then**
   it closes and the keyboard returns to the Review tab.
3. **Given** the sheet is open, **When** the user presses `j`, **Then** the selection does not move.
4. **Given** the user prefers the mouse, **When** they open the review bar's overflow menu, **Then**
   a **Keyboard shortcuts** item opens the same sheet.
5. **Given** the pointer rests on the next/previous hunk control or **Mark reviewed**, **When** its
   tooltip shows, **Then** it names the key (`n` / `p`, `v`).

---

### User Story 6 - Jump to a file or comment by name (Priority: P3)

From anywhere in the window, the developer opens the jump-to field, types part of a file's name or
part of a comment's text, and lands on it in the Review tab.

**Why this priority**: with a large run, typing a name beats stepping with `j`; it extends an
existing control rather than adding a flow, and depends on #1127 landing first.

**Independent Test**: with a Review tab open on an entry with 40 files and 3 comments, open the
jump-to field, type `tests`, choose `daemon/tests.rs`; the Review tab is shown with that file
selected. Then type a word from a comment and choose it; its card is shown.

**Acceptance Scenarios**:

1. **Given** the current entry has a Review tab (shown or not), **When** the user opens the jump-to
   field with an empty query, **Then** it lists that review's files (in file order) and comments (in
   comment order) after the active project's locations and sessions and before the other projects.
2. **Given** the query `tests`, **When** the list updates, **Then** files whose path matches are
   listed, ranked by the quick-switcher's existing rules applied to the file name first and then to
   the path.
3. **Given** a file entry is chosen, **When** the quick-switcher closes, **Then** the Review tab is
   shown with that file selected and its row in view.
4. **Given** a comment entry is chosen, **When** the quick-switcher closes, **Then** the Review tab
   is shown with the comment's file selected and its card in view (or the drawer open on it when it
   is not in the diff).
5. **Given** the current entry has no Review tab, **When** the jump-to field opens, **Then** it lists
   no file or comment entries.

### Edge Cases

- **Empty review**: no changed files: `j`, `k`, `n`, `p`, `g`, `v` and `c` do nothing; `]` and `[`
  still visit comments of files no longer changed when the drawer lists them; `?` works.
- **Nothing selected**: `j` and `n` start at the first file (its first hunk), `k` and `p` at the last
  file (its last hunk); `v` and `c` do nothing.
- **No comments**: `]` and `[` do nothing.
- **File without hunks** (binary, too large and not shown, pure rename, no-text change): `n`/`p` pass
  over it to the adjacent file that has hunks; `j`/`k` still stop on it.
- **Diff still loading or failed to load**: `n`/`p` and `c` do nothing for that file until it is
  shown; `j`/`k`/`g` still move.
- **Collapsed directory**: `j`/`k`/`g` reaching a file inside a collapsed directory expand that
  directory so the selected row is visible.
- **Changes while reviewing (Principle II)**: the agent, another session or another window changes
  the files, marks or comments: each press uses the order as it is at that press. A selected file
  that left the list counts as no selection (`j` goes to the first file, `k` to the last). A mark
  set in another window is respected by the next `g` and shown by the count. Keys never change
  another entry's review.
- **Several windows**: each window's keys act only on its own shown Review tab; a mark set by `v`
  reaches every window through #1271's storage; a send started from the chord follows the existing
  "Sending from two windows" rule (one delivery, the second refused).
- **Several entries**: keys act on the shown Review tab's entry only; a hidden Review tab of another
  entry never reacts.
- **Key held down**: auto-repeat of `j`, `k`, `n`, `p`, `]`, `[` steps once per repeat; `v`, `g`,
  `c`, `?` and the send chord act once per physical press.
- **Modifiers**: a single key with Ctrl, Cmd or Alt held is not a review key (so Ctrl+C, Cmd+V and
  the app chords keep their meaning); Shift is allowed only where it is part of the character (`?`)
  or of the send chord.
- **Keyboard layouts and input methods**: the single keys are recognised by the character the
  active layout produces (so `?`, `[` and `]` work wherever the layout types them, AltGr included);
  the send chord is recognised by the Enter key with the modifiers. A key the input method is
  composing (dead key, IME pre-edit) is not a review key.
- **Focus on a button**: with keyboard focus on a control of the Review tab (Tab traversal), the
  review keys still act; Enter and Space stay the control's.
- **Cross-platform (Principle VI)**: the send chord is Ctrl+Shift+Enter on Linux and Windows and
  Cmd+Shift+Enter on macOS; the sheet, tooltips and docs show the platform's form. Single keys are
  the same on all three platforms.
- **Terminal programs**: the review keys are heard only while the review has the keyboard, which a
  focused terminal never is, so no key reaches or is taken from a terminal.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: While the review has the keyboard, the Review tab MUST act on these keys: `j` next
  file, `k` previous file, `n` next hunk, `p` previous hunk, `]` next comment, `[` previous comment,
  `v` toggle the selected file's reviewed mark, `g` next unreviewed file, `c` comment, `?` open the
  shortcut sheet. [NEEDS CLARIFICATION: are the single-letter keys on by default, or behind a
  Settings switch (redesign open question 5)? If behind a switch, which default?]
- **FR-002**: When the review does not have the keyboard, none of the single keys of FR-001 MUST
  change the Review tab (the send chord's one exception, the composer, is FR-010's); the key MUST reach the field, dialog or surface that has the keyboard as it does today.
- **FR-003**: `j` and `k` MUST select the next or previous file in file order, wrapping at both ends,
  show its diff from the top and bring its row into view, expanding a collapsed directory that holds
  it. With no selected file (or one no longer listed), `j` MUST select the first file and `k` the
  last.
- **FR-004**: `n` and `p` MUST make the next or previous hunk current and scroll the diff so its hunk
  header is at the top of the diff. Past the last hunk of the selected file, `n` MUST select the
  next file in file order that has hunks and make its first hunk current; before the first hunk, `p`
  MUST select the previous such file and make its last hunk current; both wrap at the ends of the
  file order. With a selected file and no current hunk, `n` MUST make its first hunk current and
  `p` its last. Selecting another file by any means MUST clear the current hunk. Moving MUST NOT change picked lines, comment text, whole-file or unified/side-by-side
  choices.
- **FR-005**: The current hunk MUST be visibly marked in the diff, in both themes.
- **FR-006**: `]` and `[` MUST show the next or previous comment in comment order, wrapping at both
  ends: select its file when it is another file, and scroll its card into view; a comment that is
  not in the diff MUST open the comments drawer on it instead.
- **FR-007**: `v` MUST toggle the selected file's reviewed mark with exactly the result of the
  **Mark reviewed** control of #1271 (stored, counted and shown in every window the same way).
- **FR-008**: `g` MUST select the first file after the selected one in file order, wrapping, that is
  not marked reviewed. With every file reviewed it MUST leave the selection and show "All files
  reviewed".
- **FR-009**: `c` MUST open the comment composer, with the keyboard in it, on the picked lines; with
  no picked lines, on the current hunk's changed lines (new-version lines, or the base version's
  when the hunk only removes); with neither, it MUST open nothing and show "Pick lines or move to a
  hunk (n) to comment". It MUST NOT discard text of a composer already open elsewhere in the view;
  that composer is focused instead.
- **FR-010**: **Ctrl+Shift+Enter** on Linux and Windows and **Cmd+Shift+Enter** on macOS MUST send
  the review with exactly the result of pressing **Send review (n)**, and MUST do nothing when that
  button is unavailable (no drafts, or a send under way in any window). The chord MUST work while
  the review has the keyboard and while the comment composer has it (see User Story 4 scenario 4).
- **FR-011**: The send chord MUST NOT equal a reserved system shortcut on macOS, Windows or GNOME (the
  lists of `specs/803-keyboard-navigation/spec.md#FR-006`) or an existing app chord.
- **FR-012**: `?` and the review bar overflow's **Keyboard shortcuts** item MUST open the shortcut
  sheet: a panel over the Review tab listing every review key, the save-comment chord, and what
  each does, with keys in the current platform's form. Escape, `?` or a click outside MUST close
  it, returning the keyboard to the Review tab. While it is open, no other review key MUST act.
- **FR-013**: The controls that a key duplicates (the diff header's previous/next hunk controls,
  **Mark reviewed**, **Send review (n)**) MUST name their key in their tooltip, in the platform's
  form.
- **FR-014**: A press MUST act on the Review tab of the shown entry only, using the files, marks and
  comments as they are at that press; it MUST NOT start, stop or close a session (other than what
  **Send review** already does) or change another entry's review.
- **FR-015**: A single key MUST NOT act when Ctrl, Cmd or Alt is held, or while an input method is
  composing. Single keys MUST be recognised by the character produced; the send chord by the Enter
  key with its modifiers. Holding `j`, `k`, `n`, `p`, `]` or `[` MUST step once per auto-repeat;
  `v`, `g`, `c`, `?` and the send chord MUST act once per press.
- **FR-016**: When the current entry has a Review tab, the jump-to field (quick-switcher) MUST also
  list that review's files, in file order, and the comments of its shown source, in comment order,
  after the active project's locations and sessions and before the other projects. A file entry
  MUST show the file name, its directory and its reviewed state; a comment entry its status, file
  and line, and the first line of its text.
- **FR-017**: A query MUST match a file entry on its path and a comment entry on its text and file
  path, with the quick-switcher's matching and ranking rules (968 FR-005), a file name match ranking
  before a directory-only match.
- **FR-018**: Choosing a file entry MUST show the Review tab with that file selected and its row in
  view; choosing a comment entry MUST do what reaching it with `]` does (FR-006). Neither MUST
  change the current location, session or project, nor start or stop a session.
- **FR-019**: With no Review tab for the current entry, the jump-to field MUST list no file or
  comment entries; its other entries and behaviour MUST be unchanged.
- **FR-020**: The user guide MUST describe the review keys, the send chord, the shortcut sheet and the
  new jump-to entries in `docs/user-guide/reviewing-changes.md`, and list the send chord with both
  platform forms in every place it lists the app's chords (today
  `docs/user-guide/terminal-panes.md#keyboard-shortcuts` and
  `docs/user-guide/worktrees-and-sessions.md#interacting-with-the-terminal`) (Principle VII).
- **FR-021**: The layout snapshot MUST cover the Review tab with a current hunk marked and with the
  shortcut sheet open, and a visual pass at 1280×800 in the light and dark themes MUST show both.

### Key Entities

- **Review key**: a key or chord, the action it triggers, the platform form shown for it, and
  whether auto-repeat steps.
- **Navigation position**: the selected file, the current hunk and the last comment reached, all
  kept in the Review tab's view (so kept while the tab is hidden, per #1269).
- **Jump-to review entry**: a file (path, reviewed state) or a comment (status, file, line, text),
  belonging to the current entry's review.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can open every file of a 30-file review, mark each reviewed and send the review
  with no pointer input, in at most 2 key presses per file plus the send chord.
- **SC-002**: In 100% of presses typed into a text field (composer, tree filter, jump-to field), no
  review action happens and the character reaches the field.
- **SC-003**: Each review key shows its result (selection, scroll, mark, composer, sheet) within
  100 ms in the app's test harness on a 200-file review.
- **SC-004**: The shortcut sheet lists 100% of the keys of FR-001 and FR-010, and the user guide
  lists the same set.
- **SC-005**: From the jump-to field, any file of a 200-file review is chosen with at most 4 typed
  characters of a distinguishing part of its name, at most 2 Up/Down presses and Enter.
- **SC-006**: Sending with the chord and with **Send review (n)** produce the same message and
  statuses in 100% of compared sends.

## Assumptions

- The key set and the meaning of each key are those of the issue and of
  `docs/development/review-redesign.md` item 6; they are fixed, as other app keys are
  (user-configurable bindings stay out of scope, as in #803 and #968).
- `g` is a single key for next unreviewed (the issue's wording). The redesign table's `g f` for the
  jump-to field is not adopted: the jump-to field keeps its own chord (Ctrl/Cmd+Shift+P) and a `g`
  prefix would delay every `g` press.
- `j`/`k` and `]`/`[` wrap at the ends, as the app's other next/previous keys do (#803's step rule);
  `n`/`p` continue into the adjacent file, as reading a review is one stream of hunks.
- File order is the tree's visible order under its filter; a filtered-out file is not visited.
- Comment order covers drafts and sent comments of the shown source alike; replies are reached
  through their comment.
- The quick-switcher's existing behaviour (open/close, highlight, Escape, focus return) is unchanged
  and applies to the new entries.
- Acceptance from the issue: the decision of which file, hunk or comment a press lands on, which
  key maps to which action, and which review entries the jump-to field lists and in what order is
  render-free logic in `micold-core` with unit tests (Principle I); the user guide, layout snapshot
  and a visual pass at 1280×800 in both themes are updated (Principle VII).

## Out of Scope

- User-configurable key bindings, and a vim-style `g`-prefix chord set.
- A key for every control of the Review tab (source dropdown, sort, drawer tabs, whole file,
  unified/side-by-side, Discard, Clear sent): only the keys listed in FR-001 and FR-010.
- An app-wide shortcut sheet for keys outside the Review tab.
- Searching inside diff text from the jump-to field; it matches file paths and comment text only.
- Stale-awareness and quick-comment drag (R6, #1275), and the split view of review beside a session.
- The reviewed marks themselves, the file tree, the drawer and the jump-to field (#1271, #1273,
  #1270, #1127): this feature only adds keys and entries to them.
