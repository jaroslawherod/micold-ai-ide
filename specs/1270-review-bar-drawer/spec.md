# Feature Specification: Review bar and comments drawer

**Feature Branch**: `claude/project-thread-ttad83`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Review UI: review bar and comments drawer (#1270). Blocked by: #1269,
#1124. Fixed review bar above the status bar (counts per status, **Send review (n)**, overflow with
Clear sent / Discard drafts...). The right-hand panel becomes a drawer over the diff with **Drafts**
and **Replies** tabs; Esc closes; agent replies also shown on the comment cards. Part of the review
UI redesign: `docs/development/review-redesign.md`. Acceptance also: pure logic in micold-core with
unit tests; docs, layout snapshot and visual pass (1280x800, both themes) updated."

Design source: `docs/development/review-redesign.md`, slice 1 (items 4, 5, 7) and task row R1.

## Context and dependencies

Today the Changes view has three columns: the file list, the diff, and a review panel on the right
that holds **Clear sent**, **Discard drafts…**, **Send review (n)**, the status counts and one row
per comment. The header's **Review (n)** button closes and reopens the panel, and while the panel
is closed **Send review** moves into the header. This feature gives review state one fixed place
(a bar at the bottom of the Review tab) and turns the panel into a drawer that covers part of the
diff only when asked for.

- **#1124 (merged)**: the window has a bottom status bar in every body branch. The review bar sits
  directly above it.
- **#1269 (R0, not merged)**: each entry (worktree, or the project root's Default entry) gets its
  own **Review** tab whose view is kept while the user switches tab, session, worktree or project,
  and is dropped by the tab's × or by removing the worktree. This spec relies on that contract
  only: "the Review tab of an entry" and "the entry's review view state survives switching". It
  does not depend on how #1269 stores or keys that state.

## Clarifications

### Session 2026-10-24

- Q: How much of an agent reply does a comment card show — the first 4 lines or the first 4,000
  characters (today)? → A: At most the first 4 lines as drawn at the card's width, then **Show
  all**; the drawer's **Replies** row shows the whole reply (4,000-character cap and **Show all**
  as today) _(agent-resolved: docs/development/review-redesign.md#layout items 5 and 7)_
- Q: Does the drawer open by itself when replies arrive? → A: Yes, on its **Replies** tab at the
  first report of a send started in this window, once per send, only while this entry's Review
  tab is shown and the user has not closed the drawer since that send; an open drawer keeps its
  tab _(agent-resolved: docs/development/review-redesign.md#layout item 5, "Opens itself on a
  send that gets replies"; the limits keep FR-019's per-view state and Edge Cases "Two windows on
  one entry")_
- Q: Is the auto-open rule (FR-020) tested like the bar and drawer rules? → A: Yes, it joins
  FR-023's drawing-free rules _(agent-resolved: .specify/memory/constitution.md#i-test-first-development;
  spec FR-023)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Send a review from one fixed place (Priority: P1)

A user reviewing an agent's changes writes draft comments on the diff. At the bottom of the Review
tab, just above the window's status bar, a bar always shows how many comments are in each status
and a **Send review (n)** button. The button is always in the same place, whether the comments
drawer is open or not, so the user never has to look for it.

**Why this priority**: Sending the review is the most important action of the view and today it
moves between the panel and the header. A fixed bar is the core of the slice and is useful on its
own, even with the old panel still in place.

**Independent Test**: Open a Review tab with drafts, check the bar shows the counts and **Send
review (n)**; send; check the bar shows **Sending…**, then the result, and the counts move from
draft to sent. Open and close the drawer and check the button has not moved.

**Acceptance Scenarios**:

1. **Given** a Review tab whose shown diff (working tree or one pull request) has 3 drafts, 2 sent,
   1 applied and 1 answered comment, **When** the tab is shown, **Then** the bar reads
   "3 drafts · 2 sent · 1 applied · 1 answered" and offers **Send review (3)**, shown as the
   primary (filled) action.
2. **Given** the shown diff has no drafts, **When** the tab is shown, **Then** **Send review (0)**
   is shown in the same place, unavailable and not filled.
3. **Given** drafts exist, **When** the user presses **Send review (n)**, **Then** the button reads
   **Sending…** and is unavailable until the send ends, and the send follows today's rules (which
   session receives it, starting a session when none runs, which drafts are included).
4. **Given** a send is delivered, **When** it ends, **Then** today's snackbar reads "Sent a review
   of *n* comments to *session*" (or "Started a session and sent a review of *n* comments") and the
   bar's counts move the sent comments from draft to sent.
5. **Given** a send fails, **When** it ends, **Then** the bar shows "Couldn't send the review:
   *why*", the comments stay drafts, and the message stays until the next send starts or the user
   dismisses it.
6. **Given** the comments drawer is closed, **When** the user opens it, **Then** **Send review
   (n)** and the counts stay at exactly the same position and size.

---

### User Story 2 - Tidy comments from the bar's overflow (Priority: P1)

After a round of feedback, the user clears the sent comments or discards drafts from an overflow
menu on the bar, instead of buttons in a side panel.

**Why this priority**: Without it the old panel's buttons have no home once the panel becomes a
drawer; Story 1 and Story 2 together replace the panel's header.

**Independent Test**: With sent comments and drafts present, open the bar's overflow, use **Clear
sent**, then **Discard drafts…** and confirm; check the comments go and the items become
unavailable when nothing is left for them.

**Acceptance Scenarios**:

1. **Given** the shown diff has sent comments, **When** the user opens the bar's overflow menu and
   chooses **Clear sent**, **Then** every sent comment of what is shown is removed and the drafts
   stay.
2. **Given** the shown diff has drafts and no send is under way, **When** the user chooses
   **Discard drafts…**, **Then** the existing confirmation "Discard *n* draft comments? This cannot
   be undone." opens; **Discard** removes the drafts of what is shown, **Cancel** keeps them.
3. **Given** the shown diff has no sent comments, **When** the overflow menu is open, **Then**
   **Clear sent** is listed but unavailable; likewise **Discard drafts…** with no drafts, or while
   a send of the entry is under way.
4. **Given** the overflow menu is open, **When** the user presses Esc or clicks outside it,
   **Then** the menu closes and nothing else changes.
5. **Given** a pull request's diff is shown while the working tree and another pull request also
   hold sent comments and drafts, **When** the user uses **Clear sent** or **Discard drafts…**,
   **Then** only the shown pull request's comments are removed (the confirmation's *n* counts only
   those), as the per-pull-request rule does today.

---

### User Story 3 - Read drafts and replies in a drawer over the diff (Priority: P2)

The user opens a comments drawer from the bar. It slides over the right-hand part of the diff
without making the diff narrower, and has two tabs: **Drafts** (the comments not sent yet, with
Edit and Delete) and **Replies** (sent comments and the agent's Applied / Answered outcomes, with
the reply text in full and **Mark applied** / **Answer…** on comments still waiting). Esc closes
it. The diff keeps its full width while the drawer is closed.

**Why this priority**: It replaces the always-open third column, which is what gives the diff its
width back, but the bar (Stories 1 and 2) delivers the fixed send action without it.

**Independent Test**: With drafts and replied comments present, open the drawer from the bar,
switch between the tabs, jump to a comment from a row, edit a draft, mark a sent comment applied,
and close the drawer with Esc; check the diff's width never changed.

**Acceptance Scenarios**:

1. **Given** a newly created Review tab, **When** it is first shown, **Then** the drawer is closed
   and the diff takes the whole width beside the file list.
2. **Given** the drawer is closed, **When** the user presses **Comments** on the bar, **Then** the
   drawer opens over the right side of the diff, between the review header and the bar, and the
   diff's lines do not reflow or move.
3. **Given** the drawer is open, **When** the user presses Esc (with no dialog or menu above the
   drawer), presses the drawer's ×, or presses **Comments** again, **Then** the drawer closes.
4. **Given** the drawer is open on **Drafts**, **When** it is shown, **Then** it lists every draft
   of the entry, grouped as today (working tree first, then each pull request by number) and
   ordered by file, then line; each row has **Edit** and **Delete** unless a send holds it.
5. **Given** the drawer is open on **Replies**, **When** it is shown, **Then** it lists every sent,
   applied and answered comment of the entry in the same grouping and order; each row shows the
   status and, when the agent or the user has replied, the whole reply wrapped (a reply longer than
   4,000 characters shows its start and **Show all**, as today); a Sent row offers **Mark applied**
   and **Answer…**.
6. **Given** either tab, **When** the user presses a row, **Then** the diff selects that comment's
   file and scrolls to it with the card outlined, or the drawer says why it cannot (file hidden by
   a toggle, or not in the diff), exactly as the panel does today.
7. **Given** the drawer's tabs, **When** they are shown, **Then** each label carries its count
   ("Drafts 3", "Replies 4"), and a tab with no rows says so ("No drafts", "No replies yet").
8. **Given** the drawer is open on a tab with a scroll position, **When** the user switches to
   another tab, session, worktree or project and back to this entry's Review tab, **Then** the
   drawer is in the same open/closed state, on the same tab and at the same scroll position.

---

### User Story 4 - See the agent's outcome on the comment card (Priority: P2)

A sent comment's card in the diff shows its status and the agent's reply under the comment, so the
user reads the outcome where the code is; the drawer is an index, not the only place replies are
visible.

**Why this priority**: Most of it exists already (feature 946 draws the reply under the card); this
story keeps it intact through the redesign and aligns its length rule with the drawer.

**Independent Test**: Have the agent report Applied with a reply and Answered with a long reply;
check both cards show their status and reply, and a long reply offers **Show all**.

**Acceptance Scenarios**:

1. **Given** a comment the agent reported as Applied or Answered, **When** its card is shown in the
   diff, **Then** the card shows its status and the reply under the comment's text, with the drawer
   open or closed.
2. **Given** a reply longer than the card shows, **When** the card is shown, **Then** it shows the
   reply's start and **Show all**, which opens the whole reply in the existing dialog. The card
   shows at most the first 4 lines of the reply as drawn at the card's width (a long line that
   wraps counts as the lines it takes); the drawer's **Replies** row shows the whole reply
   (FR-016).
3. **Given** a reply arrives from the session, **When** it reaches the window, **Then** the card,
   the drawer's **Replies** row and the bar's counts all show it within two seconds.

---

### Edge Cases

- **No comments at all**: the bar reads "No review comments yet", **Send review (0)** is
  unavailable, both overflow items are unavailable, and the drawer's tabs say "No drafts" and
  "No replies yet"; the **Drafts** tab adds today's hint on how to start a comment.
- **A status with zero comments** is left out of the bar's counts ("3 drafts · 1 applied"), so the
  counts stay short.
- **Pull request shown**: the bar's counts, **Send review (n)**, **Clear sent** and **Discard
  drafts…** apply to the shown pull request's comments only, as today; **Send review** is
  unavailable while that pull request is not in the list. The drawer still lists every comment of
  the entry, grouped by scope.
- **Files browser shown**: while the repository browser lists files instead of the changes, the
  bar keeps the scope of the source it was opened from (the working tree, or the pull request
  shown), and the drawer stays available.
- **Send under way in another window** (Principle II): this window's bar reads **Sending…** too,
  **Discard drafts…** is unavailable and no drawer row offers Edit or Delete until it ends; a
  second send is refused as today.
- **Comments changed in another window**: a report, **Clear sent**, **Discard drafts…**, a new
  draft or an edit made in another window (or by the session) updates this window's counts, tabs,
  rows and cards within two seconds. A row the drawer had focused that disappears loses its focus
  without an error; an open **Show all** dialog of a removed comment closes, as today.
- **Two windows on one entry**: each window has its own drawer open/closed state, tab and scroll;
  only comments and send state are shared.
- **Esc order**: Esc first closes the topmost thing above the drawer (the overflow menu, the
  **Discard drafts…** confirmation, the **Show all** dialog, a list opened inside a field); only
  with nothing above it does Esc close the drawer. With the drawer closed, Esc does what it does
  today.
- **Unsaved text in the drawer**: an **Answer…** note or a draft being edited in the drawer is kept
  when the drawer closes and is there again when it reopens on the same entry; it is lost only when
  the Review tab is closed.
- **Narrow window**: the drawer is 360 px wide but never wider than half of the diff area. The bar
  keeps **Send review (n)**, **Comments** and the overflow visible at every supported window width;
  when space runs out the counts shorten first (to the draft count alone, then hidden).
- **Drawer opens over the line being read**: the drawer covers the diff's right side and never
  hides the comment a row jump scrolls to; the jump scrolls so the target line starts left of the
  drawer's edge, or the line is visible under the drawer's left part when the diff is narrower
  than the drawer allows.
- **Worktree removed while its Review tab is open**: the tab, its bar and its drawer go away with
  it (#1269), and the comments are deleted as today.
- **Platforms** (Principle VI): the bar, the overflow menu, the drawer and Esc behave the same on
  macOS, Linux and Windows; the overflow menu is the app's own menu, not a native one, so it looks
  the same everywhere.
- **Auto-opening the drawer**: when the agent's first report (Applied or Answered) of a comment
  sent by a send started in this window arrives, the drawer opens itself on its **Replies** tab,
  once per send, and only if this entry's Review tab is shown at that moment and the user has not
  closed the drawer since that send started. A drawer that is already open keeps its tab. In any
  other case (another window's send, the Review tab not shown, a later report of the same send)
  the drawer stays as it is.

## Requirements *(mandatory)*

### Functional Requirements

#### Review bar

- **FR-001**: While an entry's Review tab is shown, the window MUST show a review bar directly
  above the status bar, one row high, spanning the Review tab's width; it MUST NOT show while
  another tab (a session) is shown.
- **FR-002**: The bar MUST show how many comments of the shown diff (the working tree, or the shown
  pull request) are Draft, Sent, Applied and Answered, leaving out statuses with none, and
  "No review comments yet" when there are none.
- **FR-003**: The bar MUST hold **Send review (n)**, *n* being the drafts of the shown diff, at a
  position that does not change when the drawer opens or closes, when counts change, or when an
  error is shown. It is the primary (filled) action when *n* > 0 and unavailable otherwise.
- **FR-004**: **Send review (n)** MUST keep today's sending rules: what is sent, to which session,
  starting a session when none runs, refusing a second send of the entry from any window, and
  leaving comments drafts when delivery fails.
- **FR-005**: While a send of the entry is under way in any window, the bar's button MUST read
  **Sending…** and be unavailable.
- **FR-006**: When a send started from this Review tab fails, the bar MUST show "Couldn't send the
  review: *why*" beside the button until the next send starts or the user dismisses it. A delivered
  send keeps today's success snackbar and wording.
- **FR-007**: The bar MUST hold an overflow menu with **Clear sent** and **Discard drafts…**,
  each available under today's conditions (Clear sent: the shown diff has sent comments; Discard
  drafts…: it has drafts and no send of the entry is under way) and acting on the shown diff's
  comments only. **Discard drafts…** MUST open the existing confirmation.
- **FR-008**: The bar MUST hold a **Comments** toggle that opens and closes the drawer and shows
  which state the drawer is in.
- **FR-009**: The Changes header MUST no longer hold **Review (n)** or **Send review**; the review
  panel's header buttons and counts MUST be gone (the bar holds them).

#### Comments drawer

- **FR-010**: The right-hand review column MUST be replaced by a drawer that, when open, covers the
  right side of the diff area between the review header and the bar, 360 px wide but at most half
  the diff area's width. Opening or closing it MUST NOT change the diff's width or move its lines.
- **FR-011**: The drawer MUST be closed when an entry's Review tab is created.
- **FR-012**: The drawer MUST close on Esc when nothing is open above it, on its × and on the bar's
  **Comments** toggle. Esc MUST first close any menu, dialog or field list open above the drawer.
- **FR-013**: The drawer MUST have two tabs, **Drafts** and **Replies**, each labelled with its row
  count. **Drafts** lists the entry's draft comments; **Replies** lists its Sent, Applied and
  Answered comments. Every comment of the entry is in exactly one tab.
- **FR-014**: Both tabs MUST group and order rows as the panel does today (working tree first, then
  pull requests by number, a pull request not in the list flagged; within a group by file, then
  line, then side, then creation), and mark a row whose file is not in the current diff.
- **FR-015**: A **Drafts** row MUST show the file, the lines, the comment's text and **Edit** and
  **Delete**; Edit and Delete MUST be withheld while a send holds the draft.
- **FR-016**: A **Replies** row MUST show the file, the lines, the start of the comment, its status
  and its whole reply wrapped (a reply over 4,000 characters shows its start and **Show all**). A
  Sent row MUST offer **Mark applied** and **Answer…**, with today's rules for the note.
- **FR-017**: Pressing a row in either tab MUST select the comment's file, scroll the diff to the
  comment with its card outlined, and keep the target line visible beside the drawer; when the jump
  is refused the drawer MUST say why (hidden by a toggle, or not in the diff) and the diff stays.
- **FR-018**: A tab with no rows MUST say so: "No drafts" (with today's hint on writing a comment
  when the entry has no comments at all) and "No replies yet".
- **FR-019**: The drawer's open/closed state, selected tab, scroll position and any unsaved text in
  it MUST belong to the entry's review view, so they survive switching away and back as #1269
  keeps that view, and are dropped when the Review tab is closed.
- **FR-020**: The drawer MUST open itself on its **Replies** tab when the first report (Applied or
  Answered) of a send started in this window arrives, at most once per send, only while this
  entry's Review tab is shown and only if the user has not closed the drawer since that send
  started; an already open drawer keeps its tab. Otherwise it opens only when the user asks.

#### Comment cards

- **FR-021**: A comment card in the diff MUST show its status and, once replied to, the reply under
  the comment's text, with the drawer open or closed. The card MUST show at most the reply's first
  4 lines as drawn at the card's width; a longer reply ends there and offers **Show all**, which
  opens the existing whole-reply dialog.

#### Consistency and quality

- **FR-022**: Counts, tab counts, rows and cards MUST reflect changes made in any window or by the
  session (reports, sends, clears, discards, new or edited drafts) within two seconds.
- **FR-023**: The rules that decide the bar's content (counts, button label and availability,
  overflow item availability, message) and the drawer's content (which comment is in which tab,
  row order, tab counts, empty texts, and whether an arriving report opens the drawer) MUST be
  verifiable by automated tests that run without drawing the window.
- **FR-024**: The user guide's review chapter, the layout snapshot and the component showcase MUST
  describe the bar and the drawer, and a visual pass at 1280×800 in the light and the dark theme
  MUST show them without clipping or overlap.

### Key Entities

- **Review bar**: what the bar shows for the shown diff: per-status counts, the send button's
  label and availability, the overflow items' availability, the **Comments** toggle state and the
  last failed send's error.
- **Comments drawer state**: part of an entry's review view: open or closed, the selected tab
  (Drafts or Replies), each tab's scroll position, the focused row, and unsaved edit or answer
  text.
- **Drawer tab**: Drafts or Replies; derived from the entry's comments by status. A comment moves
  from Drafts to Replies when it is sent, and leaves both when it is cleared or discarded.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the rendered window, **Send review (n)** has the same position and size with the
  drawer closed and open, with 0 and with 99 drafts, and with and without a send error (0 px
  difference).
- **SC-002**: Opening and closing the drawer changes the diff area's width and its first visible
  line by 0 px and 0 lines.
- **SC-003**: From the drawer open with nothing above it, one Esc press closes it; with a dialog or
  menu above it, the first Esc closes only that.
- **SC-004**: At a 1280×800 window with the drawer closed, the diff area is at least as wide as
  today's diff area plus today's review column, measured on the same entry.
- **SC-005**: A report, send, clear or discard made in another window shows in this window's bar,
  drawer and cards within 2 seconds.
- **SC-006**: Every comment of an entry appears in exactly one drawer tab, verified by automated
  tests over all four statuses and both scopes.
- **SC-007**: The visual pass at 1280×800 in both themes shows the bar, the overflow menu and the
  drawer (both tabs) with no clipped or overlapping text.

## Out of scope

- The source dropdown and the two-column layout (R2, #1272); the PR list and toggles stay where
  they are in this slice.
- The file tree, filter and sort (R3, #1273); reviewed marks and progress (R4, #1271).
- The keyboard map, including a send shortcut and next/previous comment keys (R5, #1274).
- Stale-awareness and quick comment (R6, #1275); split view of session and review.
- Any change to comment anchoring, statuses, the review message, `report_review_comment`, storage
  or the daemon protocol.
- The Review tab itself, its badge and keeping views across switching (#1269).

## Assumptions

- #1269 lands first, or together: this feature attaches the bar and drawer to an entry's Review
  tab. If it is built before #1269 merges, the same behaviour applies to the single open Changes
  view, and FR-019's "survive switching" applies once #1269 keeps the view.
- The status bar from #1124 is present in the Review tab's body; the bar sits above it and does not
  replace any of its content.
- The 44 px bar height and 360 px drawer width come from the redesign proposal; the layout snapshot
  records the final values.
- Bar and drawer reuse the app's existing component library (buttons, menu, tabs, surfaces) before
  any new component is added (Principle VIII).
- Send outcomes are shown today as snackbars. The success snackbar stays; the failure also stays
  in the bar (FR-006) so it is not missed. Wording is unchanged.
