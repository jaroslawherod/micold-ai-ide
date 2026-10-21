# Feature Specification: Overview cards, a pull request list with state, linked

**Feature Branch**: `claude/project-thread-7041o6`

**Created**: 2026-10-21

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1123: Overview cards: pull request list with
state, linked. Blocked by: #1121 (expanded view for the rest of the list). The app reads PR counts
only. Read, for each PR of a branch: number, URL, state (open, draft, merged, closed), checks
(passing, failing, running, none), review (approved, changes requested) and merge conflict. Reuse
the existing PR status reading and its rate-limit handling; no new polling. The card shows up to two
PR rows (number as a link, state pill, checks or review in words), the rest on expand. A press on
the number opens the PR and does not activate the card. A worktree with no PR shows no row.
Acceptance: core type and parser tests; unreadable states keep today's dash reasons; counts derive
from the list; docs and layout snapshot updated." Labels: enhancement, flow:feature. Builds on
features 040 (`specs/040-worktree-pr-ci-status`, the pull request status reading), 813
(`specs/813-worktree-overview`, the cards and their counts), 1121
(`specs/1121-overview-card-full-text`, the expanded card), 1122
(`specs/1122-overview-card-issue-link`, the link on a card) and 1003
(`specs/1003-review-worktree-prs`, the pull request list of Review changes, whose state and check
words this feature reuses), all shipped.

## Terms

- **Overview**, **card**, **location**, **card state**, **pull request counts**, **unknown mark**:
  as defined by feature 813 (`specs/813-worktree-overview/spec.md#Terms`, FR-004). The unknown mark
  is the dash (*—*) a card shows instead of its counts, with its reason as the tooltip: *pull request
  status is off*, *GitHub is not reachable*, *rate-limited until HH:MM*, *not read yet*.
- **Collapsed card**, **expanded card**, **body**: as defined by feature 1121
  (`specs/1121-overview-card-full-text/spec.md#Terms`).
- **Issue link**, **link** (a text on a card that opens an address in the browser and does not
  activate the card): as defined by feature 1122 (`specs/1122-overview-card-issue-link/spec.md`,
  FR-005 to FR-009).
- **Status reading**: the periodic reading of a project's branches' pull requests from GitHub that
  feature 040 defines (its interval, its rate-limit pause, its failure kinds FR-019 and FR-025); the
  sidebar's indicator and the card counts come from it today.
- **Branch's pull requests**: the pull requests of the project's GitHub repository whose head
  branch is the location's branch, pull requests from other repositories (forks) left out, as
  feature 040 FR-004 and feature 813's counts already leave them out.
- **Current pull request**: the one the sidebar's indicator shows for the branch (feature 040
  FR-004): the newest open or draft one, else the newest of any state.
- **Pull request row**: one line on a card for one of the branch's pull requests: its number as a
  link ("#1180"), a **state pill** (*Open*, *Draft*, *Merged*, *Closed*), and its **status words**.
- **Status words**: the checks or review state of an open or draft pull request in words (FR-004).
- **Pull request state** and **check status**: the words of feature 1003
  (`specs/1003-review-worktree-prs/spec.md`, FR-001, FR-003): a state is *open*, *draft*, *merged*
  or *closed*; a check status is *passing*, *failing*, *pending* or *none*, by the combined check
  result of feature 040 FR-008. Checks that have not all finished are *pending* here as in Review
  changes, never *running*, so the app uses one word for one state. The order of rows differs from
  1003's on purpose (FR-003: open and draft ones first, so the first row is the current pull
  request).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See a worktree's pull requests and their state on its card (Priority: P1)

A developer scans the Overview to see where each worktree's work stands on GitHub. Today a card
says "1 open · 2 merged" and nothing about which pull requests those are or whether they are green.
With this change the card lists the branch's pull requests, up to two of them, each with its
number, a state pill and, for an open one, whether its checks are passing, failing or pending, or
its review state, so the developer sees "#1180 Open checks failing" without opening anything.

**Why this priority**: It is the issue's core: the list and its state are what the card lacks.

**Independent Test**: With pull request status on and a reading answered by a scripted source in
which branch `feat-1123_x` has an open pull request #12 with failing checks and a merged one #9,
show the Overview and check that the card shows the rows "#12 Open checks failing" and "#9 Merged",
in that order, and the counts "1 open · 1 merged".

**Acceptance Scenarios**:

1. **Given** a branch with one open pull request #12 whose checks fail, **When** its card is shown,
   **Then** it shows one pull request row: "#12", the pill *Open*, and the words *checks failing*.
2. **Given** a branch with pull requests in each of the four states, **When** its card is shown
   collapsed, **Then** it shows two rows, the open or draft ones first and newest first within
   each group (FR-003), and the counts line reads as today from the same pull requests.
3. **Given** a branch whose reading found no pull request, **When** its card is shown, **Then** it
   shows no pull request row and no counts line (feature 813 as today).
4. **Given** pull request status off, **When** the card is shown, **Then** it shows the dash with
   the reason *pull request status is off* and no pull request row. **Given** a branch with no
   earlier successful reading, and GitHub unreachable, the reading rate-limited or the branch not
   read yet, **When** the card is shown, **Then** it shows the dash with today's reason as the
   tooltip and no pull request row.
5. **Given** a draft pull request, **When** its row is shown, **Then** the pill reads *Draft* and
   the row carries its status words as an open one does; a merged or closed pull request's row
   carries no status words.
6. **Given** the *Default* card, a detached HEAD or an orphan location (no branch), **When** the
   card is shown, **Then** it shows no pull request row, as it shows no counts today.
7. **Given** a reading that changes a pull request's state (its checks finish, it is merged),
   **When** the reading arrives, **Then** the row shows the new state without a restart and
   without the user doing anything.
8. **Given** a card that shows rows from a successful reading, **When** the reading is then paused
   by the rate limit, or a later reading fails for a passing reason (feature 040 FR-019), **Then**
   the card keeps the same rows and counts and shows no dash, as the counts are kept today.

---

### User Story 2 - See every pull request of a busy branch (Priority: P2)

A branch that was reopened, or whose first pull request was closed and redone, has three or more
pull requests. The collapsed card shows two; the developer expands the card (pointer rest or
keyboard focus, feature 1121) and sees all of them.

**Why this priority**: Most branches have one or two pull requests; the rest matter less often.

**Independent Test**: With a branch that has four pull requests, show its card collapsed and check
two rows plus a line saying "+2 more"; expand it and check four rows in the order of FR-003.

**Acceptance Scenarios**:

1. **Given** a branch with three or more pull requests, **When** its card is collapsed, **Then** it
   shows the first two rows and a line "+N more" naming how many are not shown.
2. **Given** that card, **When** it expands, **Then** it shows every row the reading holds for the
   branch, in the same order, and no "+N more" line; when the body scrolls (1121 FR-008) the rows
   scroll with it.
3. **Given** a branch with one or two pull requests, **When** its card is shown collapsed or
   expanded, **Then** it shows the same rows in both and no "+N more" line.

---

### User Story 3 - Open a pull request from its row (Priority: P1)

The developer sees "#12 Open checks failing" and wants the failing run. A click on "#12" opens the
pull request's page in the browser; the Overview stays as it was and the card is not activated.

**Why this priority**: The issue names it ("a press on the number opens the PR and does not
activate the card"); without it the row is information the user cannot act on.

**Independent Test**: Click "#12" on a collapsed card and on an expanded card, and check that the
browser is asked to open the pull request's address as the reading gave it, that the Overview stays
open, that no location is selected and that the card shows no ripple.

**Acceptance Scenarios**:

1. **Given** a pull request row, **When** the user clicks its number, **Then** the browser is asked
   to open that pull request's address, and the card is not activated (as feature 1122 FR-005
   defines for the issue link).
2. **Given** the pointer over a row's number, **When** it is shown, **Then** it reads as a link as
   the issue link does (feature 1122 Story 1 scenario 3).
3. **Given** a row, **When** the user clicks the row outside its number (the pill, the words),
   **Then** the card activates as a click anywhere else on the card does (feature 813 FR-013).
4. **Given** keyboard focus on a card, **When** the user presses Tab, **Then** focus moves through
   the card's links in the order they are drawn: the issue link, then each shown row's number;
   Enter or Space on a focused number opens its pull request and does not activate the card; the
   arrow keys, Escape, the Menu key and Shift+F10 on a focused number do what feature 1122 FR-008
   and FR-009 define on a focused issue link.
5. **Given** the browser cannot be opened, **When** the user activates a number, **Then** the user
   sees the notification feature 031 FR-015 defines, and nothing else changes.

---

### Edge Cases

- **Reading fails for a passing reason** (offline, timeout, unreadable answer, feature 040
  FR-019): the rows of the last successful reading stay, as the counts stay today; the stale form
  of feature 040 FR-019 is not drawn on the card, as it is not drawn on the counts today.
- **Reading finds pull requests cannot be read at all** (feature 040 FR-025): the rows and counts
  go and the card shows the dash with its reason, as today.
- **Rate limit reached**: no new reading is sent until the reset (feature 040 FR-024); the card
  keeps its rows if it had them, else shows the dash *rate-limited until HH:MM*, as today.
- **More pull requests than one reading lists** (FR-008): the rows show the newest the reading
  listed; the counts read *N+* for the state whose count may be higher, as *100+* does today.
- **Checks unknown to this version** (a new GitHub check state): read as pending, never as passing
  (the combined check result of feature 040 FR-008).
- **Checks hidden from the sign-in** (GitHub refuses the check rollup): the row has no checks
  words, as the sidebar today shows no check state.
- **A pull request from a fork** with the same head branch name: not listed, not counted.
- **Same pull request on two worktrees** (two worktrees cannot share a branch, so only after a
  branch is renamed): each card lists what the reading found for its own branch.
- **Narrow card (280 px)**: the number, the pill and the status words are never clipped; the words
  end in an ellipsis before the number or the pill give way (feature 1121 FR-007).
- **Card in full screen** (feature 970): rows and links behave the same.
- **Multi-session (Principle II)**: the rows are per branch, whatever sessions the location has;
  opening a pull request touches no session.
- **Two windows on the same project**: each window shows what its own reading found, as the
  sidebar does today (feature 040); no new sharing between windows.
- **Platforms (Principle VI)**: the browser is asked through each platform's opener (feature 031);
  rows read and behave the same on Linux, macOS and Windows, at display scale 1.0 and 2.0, in both
  themes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The status reading MUST read, for every pull request of each branch it covers: its
  number, its address, its state (open, draft, merged, closed), the combined result of its checks
  (passing, failing, pending, none), its review decision (approved, changes requested, review
  required, none) and whether GitHub reports a merge conflict. It MUST do so within the readings
  feature 040 already sends: no new reading, timer or request kind, the same requests per hour
  (feature 040 SC-006), and the same rate-limit pause (feature 040 FR-024).
- **FR-002**: A card with a branch whose last successful reading lists at least one pull request
  MUST show one pull request row per listed pull request, up to two on the collapsed card and all
  of them on the expanded card. A card whose reading lists none, or that has no branch, MUST show
  no row.
- **FR-003**: The rows MUST be ordered open and draft pull requests first, then merged and closed
  ones, each group newest first (by creation time, then the higher number). The first row is
  therefore the current pull request.
- **FR-004**: A row MUST show the number as "#N", a state pill reading *Open*, *Draft*, *Merged* or
  *Closed*, and, for an open or draft pull request, status words by
  [NEEDS CLARIFICATION: when a pull request has more than one of a merge conflict, failing,
  pending or passing checks, requested changes or an approval, does the row name one of them (by a
  fixed precedence) or each of them? The check words are feature 1003's (*checks passing*, *checks
  failing*, *checks pending*), not the issue's *running*: keep them, or state a different word on
  purpose?]. A merged or closed pull request's row MUST show no status
  words. Each state is told apart by its word, never by colour alone (feature 813 FR-005).
- **FR-005**: The collapsed card MUST end its rows with a line "+N more" when the reading lists
  more pull requests than it shows; the expanded card shows no such line.
- **FR-006**: The card's pull request counts MUST be derived from the listed pull requests: the
  open count is the listed open and draft ones, the merged count the listed merged ones, and a
  count MUST read *N+* when the list may hold fewer than the branch has (FR-008). The counts line
  keeps its text and its rules of feature 813 (zero counts left out, no line when both are 0).
- **FR-007**: When pull request status is off, or the branch has no successful reading to show
  (the reading never succeeded for it, or found pull requests cannot be read at all, feature 040
  FR-025), the card MUST show the dash with today's reasons (*pull request status is off*, *GitHub
  is not reachable*, *rate-limited until HH:MM*, *not read yet*) exactly as feature 813 defines
  them, and no row. A card that shows rows from a successful reading MUST keep them, and its
  counts, through a rate-limit pause and through a reading that fails for a passing reason (feature
  040 FR-019, FR-024), as it keeps its counts today.
- **FR-008**: A reading MUST list at most
  [NEEDS CLARIFICATION: how many pull requests per branch: the newest 10 (no more GitHub cost than
  today; counts read *10+* beyond), the newest 100 (counts read *100+* as today; each reading costs
  GitHub several times today's points), or another number?]
  pull requests per branch, the newest first, always including the current pull request.
- **FR-009**: A primary click on a row's number, and Enter or Space while it has keyboard focus,
  MUST ask the browser to open that pull request's address as the reading gave it, and MUST NOT
  activate the card; every pointer and keyboard rule feature 1122 FR-005 to FR-009 sets for the
  issue link (no ripple from a press on it, press and release both on it, no focus move on a pointer
  press, no expansion from a rest on a collapsed card's link, Tab and Shift+Tab stops, arrows
  between cards only, Escape, the Menu key, scrolling a focused link into view) MUST hold for a row's
  number in the same way.
- **FR-010**: The Tab order within a card MUST be the card, its issue link when it has one, then
  each row's number in row order, then the next card; Shift+Tab is its reverse. Only numbers drawn
  on the card are stops. A focused card is expanded (feature 1121 FR-002), so every row's number is
  a stop; after Escape collapses a focused card (feature 1121 FR-005), the numbers of the rows the
  collapsed card does not draw are not stops until the card expands again, and moving focus
  between the card and its own links does not expand it (feature 1122 FR-008).
- **FR-011**: A press anywhere on a row outside its number MUST behave as a press on the card
  outside its links (feature 1122 FR-006).
- **FR-012**: When the browser cannot be opened, the user MUST see the notification of feature 031
  FR-015, and nothing else MUST change.
- **FR-013**: The sidebar's pull request indicator, its tooltip line and the card states and
  reasons that depend on the current pull request (feature 813: *checks failing*, *changes
  requested*, *merge conflict*, *merged*) MUST show exactly what they show today for the same
  GitHub data.
- **FR-014**: Pull request addresses and titles MUST stay in memory only and MUST NOT be written to
  a stored file or a log (feature 040 FR-032); showing rows MUST make no network request, and
  activating a number none from the app other than asking the operating system to open the address.
- **FR-015**: Rows, pills and links MUST be built from the shared UI components with their colours
  from the theme's roles (Principle VIII), MUST meet the contrast of feature 1120 FR-003 in every
  card state and both themes, and MUST NOT be clipped at any card width (FR-004's words end in an
  ellipsis first); the component showcase MUST show a card with pull request rows, collapsed and
  expanded.
- **FR-016**: Reading a pull request's fields from GitHub's answer, deriving the rows, their order,
  their status words, the "+N more" line and the counts from the list, and the unknown reasons MUST
  live in render-free code and be unit-tested without the GUI (Principle I), including parser tests
  over recorded answers.
- **FR-017**: The user guide's Overview section MUST describe the pull request rows (what a row
  shows, the order, "+N more", opening a pull request by pointer and keyboard), and the recorded
  layout snapshot MUST include a card with pull request rows (Principle VII).

### Key Entities

- **Branch pull request list**: per branch, per reading: the listed pull requests in FR-003's order,
  and whether the list may be short of the branch's total. Held in memory only, replaced by each
  successful reading, cleared when feature 040 clears its statuses.
- **Listed pull request**: number, address, title (read but not shown here), state, checks, review,
  conflict, creation time. Never stored, never logged.
- **Pull request row**: derived per render from a listed pull request: number text, pill word,
  status words, address.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a branch with up to two pull requests, a user reads every pull request's number,
  state and check or review state from the collapsed card without opening anything.
- **SC-002**: A user reaches any listed pull request's page in one click from the Overview, or,
  from its focused expanded card, in N+1 key presses for the Nth row (Tab N times, then Enter), N+2
  when the card has an issue link.
- **SC-003**: Across clicks and key presses on pull request numbers, collapsed and expanded, no
  card is activated by a press on a number, and every press elsewhere on a card behaves as before.
- **SC-004**: The status reading sends no more requests per hour than before this change (at most
  30 per hour for one open project with up to 50 worktrees, feature 040 SC-006).
- **SC-005**: For every combination of state, checks, review and conflict, the card's counts equal
  the counts of the listed pull requests, and the four unknown reasons read word for word as
  before.
- **SC-006**: At card widths of 280, 380 and 480 px, every shown row's number and pill are drawn
  whole, in both themes.
- **SC-007**: The user guide's Overview section has an entry for the pull request rows and the
  recorded layout snapshot includes a card with them (FR-017).

## Out of scope

- Pull request titles, authors, base branches or check names on the card.
- Rows anywhere but the Overview's cards (the sidebar keeps its one indicator).
- A new or faster reading, a reading on demand from the Overview, or a refresh button.
- Pull requests from forks, and GitHub Enterprise hosts (feature 040 leaves them out).
- A card menu entry per pull request, or acting on a pull request (merge, re-run checks) from the
  card.
- Changing the card state rules of feature 813, which keep reading the current pull request.

## Assumptions

- The issue's "running" checks is feature 040's and feature 1003's *pending* (no check failed and
  at least one has not finished), and the rows use 1003's word (Terms).
- A review decision of *review required* has no status words of its own; *none* and *review
  required* read the same on a row (the issue names only approved and changes requested).
- A merged or closed pull request's checks and review are not shown: its pill says what matters.
- The row's number links to the address GitHub gave in the reading, so a renamed or transferred
  repository still opens the right page.
- The counts were read from separate up-to-100 pages per state; deriving them from the list
  replaces those pages, so the reading keeps its request count (FR-001, FR-006).
- The status words and the pill use the words above; their colours follow the card-state colour
  roles of feature 813 (failing as attention, merged as done), always beside their words.
- Keyboard and pointer rules for the number reuse feature 1122's link rules unchanged; this spec
  adds stops to the Tab order and nothing else.
