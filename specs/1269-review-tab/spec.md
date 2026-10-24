# Feature Specification: Review as a per-worktree tab that keeps its state

**Feature Branch**: `claude/project-thread-6rh2yo`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1269: Review UI: Review as a per-worktree tab
that keeps its state. Blocked by: #1126 (worktree header and session tab strip), #1124, #1125, #1127
(app layout rebuild). The Changes view stops replacing the terminal. It becomes a **Review** tab in
the session tab strip, one per entry, with a draft-count badge. `State.open` becomes a map keyed by
entry; the header **Changes** button and **Review changes** open or focus the tab. Switching tab,
session, worktree or project keeps the view (file, scrolls, expansions, whole-file, composer, picks,
source). Dropped only by the tab's x or worktree removal. A hidden tab shows 'changed since you
looked' and re-reads when shown. Acceptance: today's `Msg::Closed | SessionSelected` no longer drops
the view; pane chords and focus unchanged. Part of the review UI redesign:
`docs/development/review-redesign.md`. Mockup: Review tab redesign canvas. Acceptance also: pure
logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes)
updated." Labels: enhancement, flow:feature. Slice R0 of `docs/development/review-redesign.md`;
builds on feature 482 (Changes view), 946 (review replies) and 1126 (context header and tab strip,
merged).

## Terms

- **Location**, **context header**, **tab strip**, **session tab**, **shell tab**, **Changes
  button**, **minimum window width**: as defined by feature 1126
  (`specs/1126-session-context-header/spec.md#Terms`). A location is the Default entry or one
  worktree of a project; an *entry* in the issue text is a location.
- **Changes view**: the review surface of feature 482 (`docs/user-guide/reviewing-changes.md`): file
  list, diff, comments and review panel for one location.
- **Review tab**: the strip tab that shows a location's Changes view. At most one per location per
  window.
- **Kept view**: everything the Changes view of one location holds on screen: the selected file,
  every scroll position, expanded and collapsed regions, the whole-file choices, the Committed /
  Uncommitted / Files choices, the source (working tree, a pull request, or a browsed file and where
  **Back** leads), the open comment composer and its text, the picked lines, and the review panel's
  state (shown or hidden, its scroll).
- **Shown tab**: the one strip tab of the shown location whose content fills the area under the
  strip: a session (its terminal panes) or the Review tab.
- **Draft**: a review comment written but not yet sent (feature 482's *pending* comment).
- **Changed mark**: the sign on a Review tab that is not the shown tab saying the location's files
  changed since the developer last saw its view ("Changed since you looked").

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Review beside the sessions, without losing my place (Priority: P1)

A developer reviewing a worktree's changes switches to the agent's session to check what it said,
then back to the review, and finds the review exactly where they left it: same file, same scroll,
the half-written comment still in its box.

**Why this priority**: today any switch closes the Changes view and forgets everything but the
comments and the chosen pull request (`docs/development/review-redesign.md#reported-the-view-is-modal-and-forgets-everything`).
This is the reported defect the issue exists to fix.

**Independent Test**: open the review of a worktree, select a file, scroll the diff, expand a
region, type into a comment composer, pick lines; press the session tab, then the Review tab; check
each kept-view part is as left.

**Acceptance Scenarios**:

1. **Given** a shown location with a session, **When** the developer opens its review (Changes
   button or **Review changes**), **Then** a **Review** tab appears in that location's tab strip
   after its session and shell tabs and is the shown tab; the context header and the strip stay
   above it.
2. **Given** the Review tab is shown, **When** the developer presses a session tab or a shell tab of
   the same location, **Then** that session's panes are shown and the Review tab stays in the strip.
3. **Given** a review left on a session tab, **When** the developer presses the Review tab, **Then**
   every kept-view part is as it was when it was last shown.
4. **Given** a review of location A is open, **When** the developer selects a session of location B
   (sidebar, session chord, quick switcher), **Then** B is shown and A's kept view survives; **When**
   they come back to A, **Then** A's strip still holds its Review tab and pressing it shows A's kept
   view unchanged.
5. **Given** reviews open in project P, **When** the developer switches to project Q and back,
   **Then** P's Review tabs and kept views are all still there.
6. **Given** reviews of two locations are open, **When** the developer works in either, **Then** each
   Review tab shows only its own location's files, comments and kept view; nothing of one shows in
   the other.

---

### User Story 2 - Open or focus the one Review tab of a location (Priority: P1)

The developer reaches a location's review from the header's **Changes** button or the sidebar's
**Review changes**; either brings up the same tab rather than starting over.

**Why this priority**: the entry points must lead to the kept view, or story 1's state is
unreachable.

**Independent Test**: open a review, leave it on a session tab, press **Changes** and check the same
kept view comes back; choose **Review changes** on the same location and check no second Review tab
appears.

**Acceptance Scenarios**:

1. **Given** a location with no Review tab, **When** the developer presses the Changes button or
   chooses **Review changes**, **Then** a Review tab is created for it and shown, reading its
   changes as the Changes view does today.
2. **Given** a location whose Review tab exists but is not shown, **When** the developer presses the
   Changes button or chooses **Review changes**, **Then** the existing tab is shown with its kept
   view; no second tab is created.
3. **Given** the Review tab is the shown tab, **When** the developer looks at the context header,
   **Then** the Changes button shows itself selected; with a session tab shown it does not.
4. **Given** the Review tab is the shown tab, **When** the developer presses the Changes button,
   **Then** [NEEDS CLARIFICATION: does a second press return to the location's session tab with the
   view kept (today's toggle, minus the forgetting), or do nothing because the tab is already
   focused?].
5. **Given** the developer chooses **Review changes** on a location other than the shown one,
   **When** it is chosen, **Then** [NEEDS CLARIFICATION: is that location made the shown location
   (its last-shown, else main, session selected) with its Review tab shown — and for a worktree with
   no session, is the review shown under that location's header with a strip holding only the
   Review tab — or is the review shown without changing the shown location?].

---

### User Story 3 - Close the review only when I mean to (Priority: P1)

The review is dropped by the tab's × or when its worktree goes away, and by nothing else.

**Why this priority**: without an explicit close, kept views would pile up; with any other close,
story 1 breaks.

**Independent Test**: open a review, switch sessions, worktrees and projects, open and close the
Overview and Settings; check the tab and kept view remain. Press × and check the tab is gone and a
later open starts fresh. Remove the worktree and check its tab is gone.

**Acceptance Scenarios**:

1. **Given** a Review tab, **When** the developer presses its ×, **Then** the tab leaves the strip,
   its kept view is forgotten, and the location's session is shown (the one shown before, else
   main). Comments and the remembered pull request are kept, as **Close** keeps them today.
2. **Given** a Review tab closed with ×, **When** the developer opens the location's review again,
   **Then** it starts as a first open does today (no kept view).
3. **Given** a Review tab, **When** its worktree is removed (from this window, another window or
   outside the app), **Then** the tab and its kept view are dropped, as its comments are today.
4. **Given** a Review tab, **When** the developer selects a session, switches location or project,
   opens and closes the Overview or Settings, or closes a session of the location that is not the
   last one, **Then** the tab and its kept view remain.
5. **Given** the Changes view's own **Close** control today, **When** this feature ships, **Then**
   the tab's × takes its place and does what it did, except that switching never closes the view.

---

### User Story 4 - See how many drafts wait, and whether the code moved (Priority: P2)

The Review tab carries the number of drafts, and while the developer is on a session tab it says
when the location's files changed since they last looked; showing it brings the view up to date.

**Why this priority**: it makes a kept view trustworthy (it does not silently show old code) and
reminds the developer that drafts wait to be sent; the kept view is usable without it.

**Independent Test**: write two drafts, switch to the session tab, have the agent edit a file;
check the tab reads "Review" with a badge of 2 and a changed mark; press it and check the view shows
the edited file's new diff and the mark is gone.

**Acceptance Scenarios**:

1. **Given** a location with drafts, **When** its Review tab is drawn, **Then** it carries a badge
   with the number of drafts; with none, it carries no badge (never "0").
2. **Given** drafts are added, deleted or sent (in this window or another), **When** the tab is next
   drawn, **Then** the badge shows the new number.
3. **Given** the Review tab is not the shown tab, **When** the location's files change, **Then** the
   tab shows the changed mark, with the tooltip "Changed since you looked", and the view is not read
   while hidden.
4. **Given** a Review tab with the changed mark, **When** it is shown, **Then** it reads the
   location's changes again, the mark goes, and the kept view's parts are kept as an automatic
   re-read keeps them today (selected file, scrolls, composer text, picked lines).
5. **Given** a Review tab whose location was not the shown location while its files may have
   changed, **When** it is shown again, **Then** it reads the changes again before or as it is shown,
   so it never presents an older read as current.
6. **Given** the Review tab is the shown tab, **When** files change, **Then** it updates as the
   Changes view does today (no changed mark).

---

### User Story 5 - Keyboard and focus work as before (Priority: P1)

A developer who drives panes from the keyboard notices no difference.

**Why this priority**: the issue's acceptance requires pane chords and focus unchanged.

**Independent Test**: run every chord of `docs/user-guide/terminal-panes.md#keyboard-shortcuts`
with a session tab shown, then with the Review tab shown; check each acts as it did with the
terminal shown, and as it did with the Changes view open, respectively.

**Acceptance Scenarios**:

1. **Given** a session tab is shown, **When** the developer uses any pane chord, **Then** it acts
   exactly as before this feature, whether or not the location has a Review tab.
2. **Given** the Review tab is shown, **When** the developer uses any chord, **Then** it acts as it
   does today while the Changes view is open, except that a chord that selects a session shows that
   session and keeps the Review tab instead of closing the view.
3. **Given** the Review tab is shown and its composer has focus, **When** the developer types,
   **Then** keys go to the composer, as today.
4. **Given** the strip, **When** the developer presses the Review tab, its ×, or a session tab,
   **Then** the press acts on its first try, and a press in flight is not lost or given to a
   neighbour when the strip's contents change (a badge or mark appears, a tab opens or closes).

### Edge Cases

- **Empty**: a location with no changes — the Review tab opens and shows the Changes view's "no
  changes" state; the badge is absent. No location shown (no project, or no session and no review) —
  no strip and no Review tab are drawn, as today.
- **Failure**: the read on showing a tab (first open or re-read after a change) fails — the view shows
  its own error as today, the tab stays, and the kept parts not tied to the failed read are kept.
  Reading the draft count is never a separate read: it comes from the comments the window already
  holds; while those are unknown, no badge is shown.
- **A read in flight when the tab is hidden**: its result is applied to the kept view or discarded,
  never to another location's view; a hidden view starts no new read until shown.
- **A send in progress when the developer switches away**: the send carries on, its outcome reaches
  the kept view, and the badge reflects the drafts left.
- **Several sessions in one location (Principle II)**: they share the location's one Review tab;
  switching between them keeps it; a session's stopped state never shows on the Review tab.
- **Several windows on one project (Principle II)**: each window has its own Review tabs and kept
  views; opening or closing a Review tab in one does not open or close it in another; the draft badge
  agrees in every window because comments are shared, as today.
- **Location's last session closed**: the Review tab and kept view stay until × or worktree removal
  (clarification 2 decides where a sessionless location's review is drawn).
- **Project removed from the window**: its Review tabs and kept views go with it.
- **Overview or Settings open**: they take the area as today; the Review tabs and kept views return
  unchanged when they close.
- **Many tabs**: the Review tab is a strip member like the others: when the strip overflows it
  scrolls with them and is scrolled into view when shown; at the minimum window width its label,
  badge, mark and × stay whole and pressable and overlap nothing.
- **Pull request source**: a kept view showing a pull request keeps showing it; if that pull request
  can no longer be read, it behaves as the Changes view does today when a remembered pull request is
  gone.
- **Restart**: Review tabs and kept views are not restored after the app restarts (comments and the
  diff layout are, as today).
- **Platforms (Principle VI)**: the × and tooltip behave the same on Linux, macOS and Windows;
  tooltips that name a shortcut say Ctrl on Linux and Windows and Cmd on macOS; the no-overlap rule
  holds with each platform's fonts.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Opening a location's review MUST show it as a **Review** tab in that location's tab
  strip, after its session and shell tabs and before **New session**, with the context header and
  strip above it; the review MUST NOT replace the header or strip.
- **FR-002**: A window MUST hold at most one Review tab per location; opening the review of a
  location that has one MUST show that tab, not create another.
- **FR-003**: The Changes button and **Review changes** MUST create the location's Review tab when it
  has none and show it when it has one (for a location other than the shown one, see User Story 2
  scenario 5, open for clarification).
- **FR-004**: Pressing a session or shell tab MUST show that tab's content and leave the Review tab
  in the strip with its kept view.
- **FR-005**: Selecting a session (sidebar, session tab, chord, quick switcher), switching location
  or project, and opening or closing the Overview or Settings MUST keep every Review tab and its kept
  view.
- **FR-006**: Showing a Review tab again MUST present every part of its kept view as it was when it
  was last shown, except where a re-read (FR-011) changes the list or diff, which keeps those parts
  as an automatic re-read keeps them today.
- **FR-007**: A Review tab's × MUST remove the tab, forget its kept view, and show the location's
  session that was shown before, else its main session; comments and the remembered pull request
  MUST be kept. The view's own **Close** control is replaced by the ×.
- **FR-008**: Removing a worktree (by any means) MUST remove its Review tab and kept view in every
  window.
- **FR-009**: The Review tab MUST show the location's draft count as a badge when it is one or more,
  and no badge at zero; the count MUST follow drafts added, deleted or sent in any window.
- **FR-010**: While a Review tab is not the shown tab, the app MUST NOT read its location's changes
  for it; when the location's files change while the location is shown, the tab MUST show the
  changed mark with the tooltip "Changed since you looked".
- **FR-011**: Showing a Review tab that has the changed mark, or whose location was not shown since
  its last read, MUST read its changes again and clear the mark.
- **FR-012**: The Changes button MUST show itself selected exactly while the shown location's Review
  tab is the shown tab (what a press does then: User Story 2 scenario 4, open for clarification).
- **FR-013**: Every pane chord and focus rule of `docs/user-guide/terminal-panes.md` MUST behave as
  before this feature with a session tab shown, and as it does today with the Changes view open while
  the Review tab is shown, except that selecting a session keeps the Review tab (FR-005).
- **FR-014**: Whether a session is "in view" for attention and notifications MUST be decided as today:
  a shown Review tab counts as the Changes view open does today, a shown session tab as the terminal
  shown does.
- **FR-015**: Presses on the Review tab, its × and its neighbours MUST act on their first try and not
  be lost or given to a neighbour when the strip's contents change; at the minimum window width the
  Review tab with badge and mark MUST overlap nothing.
- **FR-016**: The rules that decide which tab is shown, opening or showing a Review tab, dropping it,
  and setting and clearing the changed mark MUST be checkable by automated tests that need no window
  or renderer.
- **FR-017**: The user guide (`reviewing-changes`, `terminal-panes`, `worktrees-and-sessions` where it
  describes the strip), the layout snapshot's covered states, and a visual pass at 1280×800 in both
  themes MUST show the Review tab (shown, hidden with badge and changed mark).

### Key Entities

- **Review tab**: one per (project, location) per window while open; carries the location, its kept
  view, whether it has the changed mark, and the time or read it was last shown with.
- **Kept view**: the parts listed under Terms; owned by its Review tab; forgotten by × or worktree
  removal.
- **Shown tab per location**: which strip tab a location shows — one of its sessions or its Review
  tab.
- **Draft count**: derived from the location's comments the window already holds (pending ones);
  never stored on its own.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For each of the 4 switch kinds (tab, session, location, project) and each kept-view
  part listed under Terms, switching away and back leaves the part as it was in 100% of test cases.
- **SC-002**: Across any sequence of switches, Overview and Settings opens, and session closes, the
  number of Review tabs drops only on × or worktree removal (0 other drops in tests).
- **SC-003**: No read of a location's changes is started for a Review tab while it is not shown (0
  reads counted in tests while hidden); showing a tab with the changed mark starts exactly one read.
- **SC-004**: Every pane chord in `docs/user-guide/terminal-panes.md#keyboard-shortcuts` (100%)
  behaves as specified by FR-013 in tests.
- **SC-005**: Opening a location's review from either entry point never yields a second Review tab
  for that location (0 duplicates across repeated opens in tests).
- **SC-006**: At 1280×800 in both themes and at the minimum window width, the strip with a Review tab
  (badge and changed mark) shows no overlap or clipping of the tab's label, badge, mark or ×.

## Out of scope

- Splitting the Review tab beside a session (drag to the right half): a follow-up named in
  `review-redesign.md` section 0.
- Restoring Review tabs or kept views after a restart.
- The review bar and comments drawer (#1270), source dropdown and two-column layout (#1272), file
  tree, filter and sort (#1273), reviewed marks and progress (#1271), the keyboard map (#1274),
  "n files changed since you looked" without reflowing the shown diff, and quick comment (#1275), the
  full docs and mockup sync of the redesign (#1276).
- The sidebar row's "Review · n drafts" line (#1125 territory) and any change to the sidebar menu
  beyond **Review changes** opening or showing the tab.
- Changes to the title bar (#1127) or status bar (#1124).
- Any change to comment storage, anchoring, the review message, or diff reading and limits.

## Assumptions

- Feature 1126 (merged) provides the context header, the tab strip, its scrolling and the Changes
  button; this feature adds one member kind to that strip and changes what the button's press does.
- Kept views live in the window's memory only, as the remembered pull request does today (feature
  040 FR-032 keeps nothing about pull requests on disk); the issue asks for nothing to persist.
- The changed mark for the shown location comes from the file watch feature 1126 already keeps on the
  shown location for the header's counts; no new watch of locations that are not shown is added, so
  a kept view of another location is simply re-read when next shown (FR-011).
- The look follows the *Review tab redesign* mockup: label "Review", a filled badge with the draft
  count, a small dot for the changed mark on a hidden tab, and a ×; the Changes button selected while
  the Review tab is shown.
- #1125 (sidebar) and #1127 (title bar) are not merged; this feature does not depend on or change
  their work.
