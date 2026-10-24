# Feature Specification: Review UI — docs, layout snapshot, visual pass and mockup sync

**Feature Branch**: `claude/project-thread-kxels1`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Issue #1276 — Review UI: docs, layout snapshot, visual pass and mockup
sync. Blocked by: #1269, #1270, #1272, #1273, #1271, #1274, #1275, #1128. Update
reviewing-changes, terminal-panes, worktrees-and-sessions, the layout snapshot, run the visual
pass, and bring the mockup canvas to the as-built state. Take #1260 (bundled monospace font) first
so the diff polish is judged on the final font. Part of the review UI redesign:
`docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu
(Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout
snapshot and visual pass (1280x800, both themes) updated." Owner's comment on the issue: "Also sync
the Overview board and OverviewCard component of the mockup with the as-built Overview cards (see
the Overview section of docs/development/review-redesign.md)."

## Context

This is slice R7, the closing slice of the review UI redesign (`docs/development/review-redesign.md`,
epic #1277). It builds nothing new in the application. It makes the user guide, the layout
snapshot, the recorded visual pass and the design mockup describe **the application as built** once
every earlier slice has merged:

| Slice | Issue | What it delivers (as the issue and the redesign doc describe it) |
|---|---|---|
| R0 | #1269 | **Review** is a tab in the session tab strip, one per worktree, with a draft-count badge (*Review · 3*); the header **Changes** button and the sidebar's **Review changes** open or focus it; switching tab, session, worktree or project keeps the view; only the tab's × or removing the worktree drops it; a hidden tab shows *changed since you looked* and re-reads when shown. |
| R1 | #1270 | A fixed review bar above the status bar (counts per status, **Send review (n)**, overflow with **Clear sent** / **Discard drafts…**); the right-hand panel becomes a drawer over the diff with **Drafts** and **Replies** tabs, closed by `Esc`; agent replies also shown on the comment cards. |
| R2 | #1272 | One **Working tree / Pull requests / Files** source control in the review header; the PR section and the Committed / Uncommitted toggles leave the left column; **Whole file**, **Files**, **Back** and **Open diff** kept. Two columns: file list and diff. |
| R3 | #1273 | File tree with collapsible directories, name-first rows, a `+a −r` bar, a filter box, sort by path / size of change / unreviewed first. |
| R4 | #1271 | Per-file reviewed state (unseen / has drafts / reviewed), *n of m reviewed* with a progress bar, a file whose diff changes after the mark returns to unseen; stored by the session service so every window agrees. |
| R5 | #1274 | Keyboard map: `j`/`k` file, `n`/`p` hunk, `]`/`[` comment, `v` reviewed, `g` next unreviewed, `c` comment, `Ctrl+Shift+Enter` send, `?` shortcut sheet; the jump-to field finds files and comments of the open review. |
| R6 | #1275 | *n files changed since you looked* without reflowing the diff being read; drag over line numbers opens the composer; optional suggestion chips, off by default. |
| — | #1128 | The app-layout redesign's own docs, layout snapshot and visual pass (title bar, grouped sidebar, worktree header and tab strip, status bar). |
| — | #1260 | Bundled monospace face (merged): diffs are judged in the final font. |

Where a slice lands differently from this table, the as-built application is the source of truth
for every deliverable below, and the difference is written down (FR-013).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The user guide describes the review tab as it is (Priority: P1)

A user who opens **Reviewing a worktree's changes** in the user guide finds the review as the
application now shows it: a **Review** tab beside the sessions, the source control in its header,
the file tree with filter, sort and reviewed marks, the review bar and the comments drawer, the
keyboard map and the stale-awareness behaviour. Nothing in the guide tells them to look for the
three-column view, the left-column Pull requests section, the Committed / Uncommitted toggles in the
list, the right-hand review panel, or a view that replaces the terminal and closes when they pick a
session.

**Why this priority**: The constitution (Principle VII) makes the user guide a deliverable; after
seven slices of layout change the guide is the part a user reads, and a guide describing the old
modal view is wrong on almost every page of the review chapter.

**Independent Test**: Read `reviewing-changes.md`, `terminal-panes.md` and
`worktrees-and-sessions.md` beside the running application (or the visual-pass screenshots) and
check every statement about where a review control is and what it does; the docs CI checks pass.

**Acceptance Scenarios**:

1. **Given** the guide's review chapter, **When** a reader follows "Opening the Changes view", **Then**
   it says the header **Changes** button and the sidebar's **Review changes** open or focus the
   worktree's **Review** tab, that the tab keeps its state across tab, session, worktree and project
   switches, and that its × closes it.
2. **Given** the guide, **When** a reader looks for how to send a review, **Then** it points at the
   review bar's **Send review (n)** above the status bar and the overflow's **Clear sent** and
   **Discard drafts…**, and describes the **Drafts** / **Replies** drawer and `Esc`.
3. **Given** the guide, **When** a reader looks for pull requests, committed / uncommitted changes or
   repository browsing, **Then** they are described through the header's source control, with
   **Whole file**, **Files**, **Back** and **Open diff** still documented.
4. **Given** the guide, **When** a reader looks for reviewed marks, sorting, filtering, the keyboard
   map, *changed since you looked* and quick comment, **Then** each has a section that names the
   control or key and the visible result, and the shortcut list in the guide equals the keys the
   application binds.
5. **Given** `terminal-panes.md`, **When** a reader reads the tab strip section, **Then** the
   **Review** tab is listed as a strip member with its badge, and the statement that pane chords and
   focus are unchanged while it is shown is there.
6. **Given** `worktrees-and-sessions.md`, **When** a reader reads the worktree row menu and the
   sidebar row, **Then** **Review changes** opens or focuses the tab, and the *Review · n drafts* line
   is described if the build shows it.

---

### User Story 2 - The layout snapshot pins the finished review layout (Priority: P1)

A developer changing any part of the review tab later gets a failing layout check that names the
element that moved, because the committed geometry fixture records every review state the redesign
introduced, recorded once against the final layout and the bundled monospace face.

**Why this priority**: Each slice re-records the fixture for its own states; only after the last one
merges is there one fixture that reflects the finished layout with no stale or missing review
states. Without it a regression in the review tab waits for someone to notice it on screen, the
failure the layout snapshot exists to prevent.

**Independent Test**: Run the layout snapshot test binary on a clean checkout: it passes, its
coverage list holds each review state of FR-006, and a deliberate one-pixel change to the review
bar's padding fails it naming the bar.

**Acceptance Scenarios**:

1. **Given** all blockers merged, **When** the fixture is re-recorded, **Then** the diff to the
   fixture holds only review-tab and Overview-card records (or a stated reason for any other line),
   and the coverage-narrowing check passes.
2. **Given** the re-recorded fixture, **When** the test suite runs on Linux, macOS and Windows CI,
   **Then** it passes on all three without per-host text-width differences.
3. **Given** `docs/development/layout-snapshot.md`, **When** a developer reads the states list,
   **Then** each review state the fixture covers is named there with what it anchors, and the old
   `session-area-changes-open` description (header over the open Changes view, no strip) is
   replaced by what the build records.

---

### User Story 3 - A recorded visual pass shows the review tab looks right (Priority: P2)

A reviewer of the closing PR can open screenshots of the finished review tab at 1280×800 in the light
and the dark theme and see, for each check, a verdict: nothing overlaps, no text is clipped, the
diff uses the bundled monospace face, the review bar sits above the status bar, the drawer overlays
the diff rather than narrowing it, badges and reviewed marks read in both themes.

**Why this priority**: The geometry gates are blind to colour, weight, glyph collisions and what is
drawn on top of what; the visual pass is the only check of how the finished review looks.

**Independent Test**: The feature's `visual-pass/` directory holds a light and a dark screenshot per
check with a written verdict per check; any FAIL has a follow-up issue or a fix in this feature.

**Acceptance Scenarios**:

1. **Given** the application with a worktree holding changes, drafts, sent comments with replies
   and reviewed files, **When** the visual pass runs, **Then** each check of FR-009 has a light and a
   dark screenshot at 1280×800 and a PASS or FAIL verdict.
2. **Given** a FAIL, **When** the pass is recorded, **Then** the defect is fixed in this feature when
   it is a small styling or doc defect, or filed as an issue linked from the results, and the spec's
   status names it.

---

### User Story 4 - The mockup canvas matches the build (Priority: P2)

A designer or the owner opening the *Review tab redesign* canvas sees boards that show what the
application now draws: the Review tab, review bar, drawer, source control, file tree, reviewed marks,
keyboard sheet, stale banner, and the **Overview** board with its `OverviewCard` component matching
the as-built Overview cards. Future design work starts from a true picture.

**Why this priority**: The redesign doc asks for the canvas to be kept in step; the owner asked
explicitly for the Overview board and card. It is less urgent than the guide because users do not
read it.

**Independent Test**: Compare each board of the canvas with the matching visual-pass screenshot;
every difference is either removed or labelled on the board as a follow-up not built.

**Acceptance Scenarios**:

1. **Given** the canvas and the visual-pass screenshots, **When** a board is compared with its
   screenshot, **Then** labels, order of controls, presence of controls, counts format and state
   colours agree.
2. **Given** the canvas's `OverviewCard` and the as-built card, **When** they are compared, **Then**
   the tiers (state word and type, title, short branch and counts, activity / progress / PR chip),
   the stripe-on-neutral treatment, the empty-row removal and the title wrapping agree with the
   build, and the cards-per-row note matches the build at the board's width.
3. **Given** a mockup element the build does not have (for example the split option of
   section 0), **When** the canvas is synced, **Then** it is handled as FR-012 decides.

---

### User Story 5 - The redesign doc records what was built (Priority: P3)

A developer reading `docs/development/review-redesign.md` later sees that it is no longer a proposal:
its status says built, each slice row names the PR that shipped it, its open questions are answered
with what the build chose, and the follow-ups not built (split view, anything cut) are listed.

**Why this priority**: Useful for the next change to the review UI, but no user reads it.

**Independent Test**: Read the doc: status, slice rows, open questions and follow-ups each agree with
the merged PRs.

**Acceptance Scenarios**:

1. **Given** the doc, **When** a reader reads its status line and the Tasks table, **Then** the status
   is "built" with the epic, and each R0–R7 row names its merged PR.
2. **Given** the doc's open questions 0–5, **When** a reader reads them, **Then** each carries the
   answer the build took.

---

### Edge Cases

- **A blocker lands differently from its issue** (a key bound differently, a control renamed, a part
  cut): the docs, snapshot, visual pass and mockup follow the build, and the difference is listed in
  the redesign doc (FR-013); the spec's acceptance does not hold the build to the issue text.
- **A blocker is still unmerged when this feature is planned**: the docs and mockup work for merged
  slices can proceed; the fixture re-record (FR-005) and the visual pass (FR-009) run only after
  every blocker has merged, so they are recorded once against the final layout.
- **Empty review**: the guide and the visual pass cover a worktree with no changes (empty tree, bar
  with zero counts and a disabled **Send review**), and a review with no drafts and no replies (drawer
  tabs empty).
- **Failure states**: the guide documents and the visual pass shows the send error and *Sending…* in
  the review bar, and a PR source whose diff cannot be read.
- **Multi-session (Principle II)**: two worktrees each with an open Review tab keep separate state; two
  windows on one worktree agree on reviewed marks and drafts; sending while a second window has the
  tab open. The guide states each of these, and the visual pass includes the two-worktree switch.
- **Cross-platform (Principle VI)**: the fixture passes on Linux, macOS and Windows CI; the visual
  pass is recorded on Linux and the guide names any platform difference in keys (`Ctrl` vs `Cmd`)
  that the build has; macOS and Windows screens are checked as the project's quickstart §C does for
  other features (or recorded as not run, with the reason).
- **Long names**: a long worktree name, a deep path in the tree and a long PR title in the source
  control do not overlap their neighbours (snapshot overflow gate and visual pass).
- **Mockup not editable**: if the canvas cannot be written from the session that does the sync, the
  sync is handed to the owner with the list of differences, and the spec status says so.

## Requirements *(mandatory)*

### Functional Requirements

**User guide**

- **FR-001**: `docs/user-guide/reviewing-changes.md` MUST describe the review as built by R0–R6: the
  Review tab and how it opens, keeps and drops its state; the source control; the two-column layout
  with the file tree, filter, sort and reviewed marks with *n of m reviewed*; the review bar with
  counts, **Send review (n)**, overflow, *Sending…* and the send error; the comments drawer with
  **Drafts** / **Replies** and replies on cards; the keyboard map and shortcut sheet; *changed since
  you looked*; drag-to-comment and the suggestion-chips setting. Each section names the control or
  key and its visible result.
- **FR-002**: The guide MUST contain no statement that holds only for the pre-redesign view (three
  columns, left-column Pull requests section and Committed / Uncommitted / Files toggles, right-hand
  review panel, **Close** button, the view replacing the terminal or closing on session selection,
  re-reading the open diff within 2 s).
- **FR-003**: `docs/user-guide/terminal-panes.md` MUST list the **Review** tab as a member of the tab
  strip with its badge and *changed since you looked* mark, and state how pane chords and focus
  behave while it is shown. `docs/user-guide/worktrees-and-sessions.md` MUST describe **Review
  changes** in the row menu, the header **Changes** button, and any review line the sidebar row shows.
- **FR-004**: The keyboard shortcuts listed in the user guide MUST equal the review keys the
  application binds, checked by an automated test so a later key change that skips the guide fails
  CI. [NEEDS CLARIFICATION: is this check the "pure logic in micold-core with unit tests" the
  acceptance asks for, or does this docs-only slice satisfy that clause by stating it adds no logic?]

**Layout snapshot**

- **FR-005**: After every blocker has merged, the geometry fixture MUST be re-recorded once, against
  the bundled monospace face, and the re-record's diff MUST be explained line group by line group in
  the PR (review states, Overview cards, anything else with its reason).
- **FR-006**: The fixture's covered states MUST include at least: the Review tab shown beside session
  tabs with a badge; the hidden tab with *changed since you looked*; the review bar with drafts, sent
  and replies counts; the drawer open on **Drafts** and on **Replies**; the source control open (as an
  overlay state); the file tree filtered and sorted; reviewed marks with the progress figure; the
  shortcut sheet open; the *n files changed since you looked* notice; the composer opened by a line
  drag; an empty review. Each state MUST be anchored on the elements it introduces, and states made
  obsolete by the redesign (for example `session-area-changes-open` as a view with no strip) MUST be
  removed or redefined, not left recording a layout the app no longer draws.
- **FR-007**: The existing gates (text overflow, containment, coverage narrowing, session area fits,
  status bar fits) MUST pass for every review state at the smallest window and at 1280×800.
- **FR-008**: `docs/development/layout-snapshot.md` MUST name each review state and what it anchors,
  in the same form as the other features' paragraphs there.

**Visual pass**

- **FR-009**: A visual pass MUST be recorded at 1280×800 in the light and the dark theme, after every
  blocker has merged, with one verdict per check and the screenshots in the feature's
  `visual-pass/` directory. Checks: Review tab and badge in the strip; header with source control and
  *n of m reviewed*; file tree with reviewed / drafts / unseen marks, filter and `+a −r` bars; diff in
  the bundled monospace face with an inline draft card and a sent card showing its reply; review bar
  above the status bar in idle, *Sending…* and error states; drawer over the diff on both tabs;
  shortcut sheet; *changed since you looked* on the tab and the notice in the header; drag-to-comment
  composer; empty review; Overview board with as-built cards. [NEEDS CLARIFICATION: 1280×800 only, as
  the issue's acceptance says, or also 900 and 1920 px wide as the redesign doc's "Scope and files"
  lists?]
- **FR-010**: Each FAIL in the pass MUST be either fixed in this feature (styling or doc defects that
  do not change behaviour) or filed as an issue linked from the pass results; no FAIL is left
  unrecorded.

**Mockup**

- **FR-011**: The *Review tab redesign* canvas MUST be brought to the as-built state: every review
  board and shared component (`TitleBar`, `Sidebar`, `StatusBar`, tab strip, review bar, drawer,
  source control, file tree row, comment card, shortcut sheet) shows the labels, controls, order,
  count formats and state treatments the build draws, using the canvas's shared `tokens.css`. The
  **Overview** board and `OverviewCard` component MUST match the as-built Overview cards (tiers, stripe
  on a neutral card, empty rows removed, title wrapping, cards per row at the board's width).
- **FR-012**: Mockup elements the build does not have MUST be [NEEDS CLARIFICATION: removed from the
  boards, or kept and labelled "follow-up, not built" (for example the split option of section 0)?].

**Redesign record**

- **FR-013**: `docs/development/review-redesign.md` MUST change from proposal to a record: status
  built, each slice row with its merged PR, each open question answered with what the build chose,
  every difference between the build and the proposal listed, and the follow-ups not built listed.
- **FR-014**: The docs CI checks (links and docs build) MUST pass with the updated pages, and links
  from other user-guide pages to renamed or removed sections of `reviewing-changes.md` MUST be updated.

### Key Entities

- **Review state under test**: a named application state the layout fixture records (tab, bar,
  drawer, source overlay, tree, sheet, notice, composer, empty), with the elements it anchors.
- **Visual-pass check**: one look at one thing, with a light and a dark screenshot, a verdict and,
  on FAIL, a fix or an issue link.
- **Mockup board**: a canvas page showing one screen state, built from shared components and tokens;
  in sync when every difference to its screenshot is removed or labelled per FR-012.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 0 statements in the three user-guide pages describe the pre-redesign review view
  (checked against the FR-002 list), and every review control and key the build has appears in the
  guide at least once.
- **SC-002**: The layout snapshot passes on all three CI platforms with the re-recorded fixture, and
  covers 100% of the states listed in FR-006.
- **SC-003**: Every visual-pass check has 2 screenshots (light, dark) and a verdict; 0 FAILs are left
  without a fix or a linked issue.
- **SC-004**: Comparing each mockup board with its screenshot finds 0 unlabelled differences in
  labels, controls, order or count formats, including the Overview board and card.
- **SC-005**: A reader can find how to open, send and keyboard-drive a review from the guide's table
  of contents in under a minute (each is a heading).

## Assumptions

- Every blocker (#1269–#1275, #1128) lands substantially as its issue and the redesign doc describe;
  where it does not, the build wins (Context, FR-013).
- Each blocker already updates the guide and the fixture for its own slice; this feature reconciles
  them into one consistent chapter and one fixture recorded against the final layout, and fills the
  pages and states no single slice owned (`terminal-panes.md`, `worktrees-and-sessions.md`, the
  redesign record, cross-slice states).
- #1260 (bundled monospace face) has merged, so the fixture and the pass measure the shipped face.
- `docs/user-guide/overview.md` and `project-selection.md` belong to #1128 and are out of scope here,
  except that the Overview cards are checked in the visual pass and the mockup.
- The visual pass is recorded with the project's `visual-pass` procedure on Linux; macOS and Windows
  are checked or recorded as not run, as for earlier features.
- The mockup is a claude.ai artifact owned by the user; the session doing the sync can publish to it
  or hands the change list to the owner.

## Out of Scope

- Any change to review behaviour, layout or keys (that is R0–R6); a defect found here that needs a
  behaviour change is filed, not fixed.
- The split view (session beside the review) and anything else the redesign doc lists as follow-up.
- The app-layout redesign's own pages and states (#1128).
- The showcase gallery's review section beyond what the blockers delivered.
