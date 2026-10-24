# Feature Specification: Layout redesign — user guide, layout snapshot and visual pass

**Feature Branch**: `claude/project-thread-bznqpy`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Issue #1128 — Layout: docs, layout snapshot and visual pass for the
redesign. Blocked by: #1124, #1125, #1126, #1127. Update user guide pages (worktrees-and-sessions,
terminal-panes, overview, project-selection), regenerate the layout snapshot fixture, and run the
visual pass at 1280x800 in both themes. Mockup: https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e"
Labels: enhancement, flow:feature.

## Context

The layout redesign (mockup *Micold Layout Redesign*, boards **Proposal: session view** and
**Proposal: Overview**, both 1280×800) is split into four features, one per window region:

| Issue | Region | State on 2026-10-24 | Its own spec |
|---|---|---|---|
| #1124 | Bottom status bar (connection, sandbox, CLI, plan usage, key hints) | merged | `specs/1124-bottom-status-bar` |
| #1126 | Worktree context header and session tab strip above the terminal | merged | `specs/1126-session-context-header` |
| #1125 | Sidebar grouped by state, Overview as its first row, chip bar, two-line rows | open, partly merged (groups, two-line rows) | `specs/1125-sidebar-state-groups` |
| #1127 | One 44 px title bar: breadcrumb `project / worktree`, jump-to field, plan usage, Settings, overflow menu | open, no spec yet | — |

Each of those features updates the user-guide sections, layout-snapshot states and visual pass of
**its own region**. None of them owns the window as a whole: the four pages a user reads to learn
the window were each edited region by region, by four features, and can still name an element
another feature removed (for example *Press **Overview** in the top bar*, *Open another project
(app bar)*, *Switching projects from the top bar*, *the filter button in the sidebar header*). No
covered layout state draws all four regions together as the mockup's boards do, and no visual pass
has compared the finished window with the boards.

This feature is the redesign's closing pass. It changes no behaviour. It makes the user guide
describe the window that shipped, as one window; records that window in the layout snapshot; and
checks it by eye against the mockup at 1280×800 in both themes.

**Staging.** The work is split by region so that the parts for merged features are done now and the
parts for open ones wait:

- **Stage 1 (now)**: the status bar (#1124) and the session area's header and tab strip (#1126).
- **Stage 2 (after #1125 and #1127 merge)**: the sidebar (#1125), the title bar (#1127), and
  everything that needs the whole window: the whole-window covered states, the fixture
  regeneration and the visual pass.

## Terms

- **Region**: one of the four parts of the window the redesign rebuilt: the **title bar**, the
  **sidebar**, the **session area** (context header, tab strip and panes; or the Overview, Changes
  or Settings in its place) and the **status bar**.
- **The four pages**: `docs/user-guide/worktrees-and-sessions.md`, `terminal-panes.md`,
  `overview.md` and `project-selection.md`.
- **Removed element**: a control or row the redesign took away or moved to another region: the
  two-row top/app bar, the **Overview** button in it, the project switcher as a button next to the
  menu button, **Open another project** in the app bar (if #1127 moves it), the connection banner,
  sandbox banner and sandbox bring-up row stacked under the app bar in the steady state, the bar
  under the terminal, the sidebar's filter button and tag-filter panel (replaced by the chip bar),
  the sidebar's type/issue chips on rows. The list is final only once #1125 and #1127 have merged;
  each stage reads it from what shipped, not from this list.
- **Owning section**: the one section of the user guide that describes a region's element in full.
  Every other mention links to it.
- **Mockup board**: *Proposal: session view* (a session shown) or *Proposal: Overview* (the Overview
  open).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The status bar and session area read the same on every page (Priority: P1)

A user who reads any of the four pages to learn how to see the connection state, the sandbox, plan
usage, the worktree they are in, or how to switch between a session and its shells, finds the
status bar and the header and tab strip above the terminal — never a banner stacked under the top
of the window in the steady state, nor a bar under the terminal. Each element is described once, in
its owning section, and other pages link to it.

**Why this priority**: two of the four regions have shipped; the pages a user reads today must
match them now, not after the other two features land.

**Independent Test**: on main after #1124 and #1126, read the four pages and the pages they link
to for the status bar, the context header and the tab strip; every mention agrees with the running
window and with the owning section, and none names a removed element of these two regions.

**Acceptance Scenarios**:

1. **Given** the four pages after stage 1, **When** a reader searches them for the connection
   state, sandbox state, the session's CLI or plan usage, **Then** every hit places it in the
   status bar at the bottom of the window, and each hit outside the owning section links to it.
2. **Given** the four pages after stage 1, **When** a reader looks for where a failure that needs
   action is shown (daemon disconnected, sandbox failed), **Then** the text says a banner or
   notification raises it, as #1124 shipped, and the steady state shows no row between the top of
   the window and the body.
3. **Given** the four pages after stage 1, **When** a reader looks for a session's state, its
   restart control, or how to move between the AI CLI and its shells, **Then** the text names the
   context header and the tab strip above the terminal, and no sentence speaks of a bar under the
   terminal.
4. **Given** a sentence that says only "the bar" (for example *press **restart** in the bar*),
   **When** stage 1 is done, **Then** it names which bar (the status bar, the tab strip, or the
   context header) so a reader cannot take it for the removed bar under the terminal.

---

### User Story 2 - The sidebar and title bar read the same on every page (Priority: P1)

A user who reads any of the four pages to open the Overview, switch project, jump to a location,
filter by tag or find a worktree by its state finds the shipped title bar (breadcrumb, jump-to
field, Settings, overflow menu) and the shipped sidebar (Overview row first, chip bar, state
groups, two-line rows), with each element described once and linked elsewhere.

**Why this priority**: same value as story 1 for the other two regions; it can only be done once
#1125 and #1127 have merged.

**Independent Test**: on main after #1125 and #1127, read the four pages; every mention of
opening the Overview, the project switcher, the quick switcher, tag filtering and the sidebar's
rows matches the running window, and none names a removed element.

**Acceptance Scenarios**:

1. **Given** #1125 and #1127 merged, **When** a reader looks up how to open the Overview in
   `overview.md`, **Then** it names the sidebar's Overview row and Ctrl+Shift+O (Cmd+Shift+O on
   macOS), and no page says to press an Overview button in the top bar.
2. **Given** #1127 merged, **When** a reader looks up how to switch projects in
   `project-selection.md`, **Then** the section describes the breadcrumb's project part and where
   its panel opens, and its heading and text no longer say "top bar" or "next to the menu button".
3. **Given** #1127 merged, **When** a reader looks up the quick switcher, **Then** the pages name
   the jump-to field in the title bar and Ctrl+Shift+P (Cmd+Shift+P on macOS) as two ways to open
   it.
4. **Given** #1125 merged, **When** a reader looks up tag filtering in
   `worktrees-and-sessions.md`, **Then** it describes the chip bar and no sentence speaks of a
   filter button or a tag-filter panel.
5. **Given** both merged, **When** a reader opens `worktrees-and-sessions.md`, **Then** an opening
   section names the four regions top to bottom and links each to its owning section.

---

### User Story 3 - The layout snapshot holds the whole redesigned window (Priority: P1)

A developer who changes any region and breaks the composed window — two regions overlapping, a
stacked row reappearing under the title bar, the status bar pushed off the window — is stopped by
the layout snapshot, because it records the window with all four regions drawn together, as the
mockup's two boards draw it.

**Why this priority**: every region's own states hold that region; only a whole-window state holds
the regions against each other. This is the issue's acceptance item "regenerate the layout snapshot
fixture".

**Independent Test**: after stage 2, the snapshot's covered states include one state per mockup
board; moving any region's edge by one pixel makes the snapshot fail naming that state; the
recorded fixture is the one a fresh regeneration produces on main.

**Acceptance Scenarios**:

1. **Given** stage 2 done, **When** the layout snapshot runs on main, **Then** it passes, and a
   regeneration from main produces a fixture byte-identical to the committed one.
2. **Given** the whole-window state for *Proposal: session view*, **When** its geometry is read,
   **Then** the title bar is 44 px tall at the top, the status bar 28 px tall at the bottom, the
   sidebar and the session area fill the height between them, and nothing lies between the title
   bar's bottom edge and the body's top edge.
3. **Given** the whole-window state for *Proposal: Overview*, **When** its geometry is read,
   **Then** the same holds with the Overview in the session area's place and the sidebar's Overview
   row selected.
4. **Given** either whole-window state, **When** the text-overflow and containment gates run,
   **Then** they pass for it, in both colour schemes.
5. **Given** `docs/development/layout-snapshot.md`, **When** a reader looks for what the redesign's
   states cover, **Then** one passage lists each region's states and the whole-window states, and
   says which issue added each.

---

### User Story 4 - A visual pass checks the finished window against the mockup (Priority: P2)

A maintainer closing the redesign sees, at 1280×800 in light and in dark, screenshots of the
running window beside each mockup board, with a recorded verdict per region: matches, differs on
purpose (with the decision that made it differ), or differs by defect (with what was done about it).

**Why this priority**: the geometry gates cannot see colour, weight, glyph collisions or contrast;
this is the only check of how the redesign looks as a whole. P2 because the regions' own visual
passes already checked each region alone.

**Independent Test**: open the recorded visual pass; for each board and each theme there is a
screenshot at 1280×800 and one verdict per region, and every *differs* verdict names its reason or
its follow-up.

**Acceptance Scenarios**:

1. **Given** stage 2 done, **When** the visual pass runs, **Then** it records four screenshots at
   1280×800 — each mockup board's state, in light and in dark — and the commit they were taken at.
2. **Given** a screenshot, **When** it is compared with its board, **Then** the record gives one
   verdict for each of the four regions and one for the window as a whole (alignment between
   regions, no stacked chrome row).
3. **Given** a region that differs from the board because a region's own feature decided so (for
   example a clarification answered in its spec), **When** the verdict is recorded, **Then** it
   names that decision and its spec.
4. **Given** a defect found — text overlapping, clipped or illegible, contrast under 4.5:1 for body
   text, a region missing in one theme — **When** the verdict is recorded, **Then** the defect is
   either fixed in this feature with the fix re-checked in a new screenshot, or recorded as a
   follow-up with the region's issue named. [NEEDS CLARIFICATION: when the visual pass finds a
   defect in a shipped region, does this feature fix it (code change, red test first) or only
   record it as a follow-up issue?]

### Edge Cases

- **No project open** (empty input): the empty screen shows the title bar and the status bar only.
  The pages that describe the empty state (`project-selection.md#opening-a-project`,
  `worktrees-and-sessions.md#opening-a-project-git-repositories-only`) describe what that screen
  shows after the redesign and where the open-project actions now are.
- **Project with no worktrees** (empty input): the sidebar shows the Overview row, the Default
  entry and no group; the pages say so where they describe the groups.
- **Failure**: daemon disconnected or sandbox failed. The pages say where the failure appears (the
  banner or notification #1124 kept) and what the status bar shows meanwhile; the visual pass does
  not need a failure screenshot, since #1124's pass recorded those (`specs/1124-bottom-status-bar/visual`).
- **A blocker ships unlike the mockup**: #1125 or #1127 may resolve an open question differently
  from the mockup (the mockup's own note asks whether groups stay collapsible and whether Changes
  opens as an inspector). The pages describe what shipped; the visual pass records the difference
  as on purpose, naming the decision.
- **A blocker reverts or never merges**: stage 2 does not start until both #1125 and #1127 have
  merged; if one is closed unmerged, stage 2's parts for that region are dropped and the ledger
  says so.
- **Multi-session (Principle II)**: several sessions run in several worktrees, in different card
  states. The pages say that a worktree's row moves between sidebar groups as its state changes,
  that the context header and the status bar follow the session shown, and that the Overview's
  badge counts every worktree, not only the one shown. The whole-window state for *Proposal:
  session view* holds more than one worktree, in more than one group, and a session tab strip with
  more than one tab.
- **Cross-platform (Principle VI)**: every shortcut the pages name gives both forms (Ctrl on Linux
  and Windows, Cmd on macOS). Where the title bar differs by platform (for example macOS window
  controls, if #1127 draws them), the pages say so. The visual pass runs on Linux only; the record
  says macOS and Windows were not checked by eye.
- **Smallest window**: the pages make no claim about 1280×800 that is false at the smallest
  window; the regions' own gates hold the smallest window, so the visual pass stays at 1280×800 as
  the issue asks.

## Requirements *(mandatory)*

### Functional Requirements

**Stage 1 — status bar and session area (now)**

- **FR-001**: No sentence in the four pages MAY place the connection state, the sandbox state, the
  session's CLI or plan usage anywhere but the status bar, nor describe a row stacked under the
  top of the window in the steady state.
- **FR-002**: No sentence in the four pages MAY describe a bar under the terminal; a session's
  state, restart control and instances are placed in the context header or the tab strip as #1126
  shipped them.
- **FR-003**: Every mention of "the bar" in the four pages MUST name which bar it is.
- **FR-004**: The status bar, the context header and the tab strip each MUST have exactly one
  owning section in the user guide; every other mention in the four pages MUST link to it rather
  than describe the element again.

**Stage 2 — sidebar and title bar (after #1125 and #1127 merge)**

- **FR-005**: No sentence in the four pages MAY name a removed element of the sidebar or the title
  bar (the Overview button, the top/app bar, the switcher button next to the menu button, the
  filter button or tag-filter panel, the type/issue chips on rows), as the removed list stands once
  both have merged.
- **FR-006**: `overview.md` MUST name the sidebar's Overview row and the Overview shortcut as the
  ways to open and close the Overview, and say which regions stay while it is open and while it is
  full screen.
- **FR-007**: `project-selection.md` MUST describe switching projects from the title bar's
  breadcrumb, including where its panel opens, under a heading that names the breadcrumb.
- **FR-008**: The four pages MUST name the jump-to field and the quick-switcher shortcut wherever
  they describe opening the quick switcher.
- **FR-009**: `worktrees-and-sessions.md` MUST open with a short section that names the four
  regions top to bottom (title bar, sidebar beside the session area, status bar) and links each to
  its owning section.
- **FR-010**: The sidebar's elements (Overview row, chip bar, groups, two-line rows) and the title
  bar's elements (breadcrumb, jump-to field, plan usage, Settings, overflow menu) each MUST have one
  owning section, linked from every other mention in the four pages.
- **FR-011**: User-guide pages outside the four that name a removed element MUST be
  [NEEDS CLARIFICATION: in scope? `appearance-theming.md`, `icons.md`, `settings.md` and
  `sandboxed-daemon.md` still say "top app bar", "Top bar, **Overview** button" or "banner under
  the app bar" — are those swept here, or left to #1127 and a follow-up?].
- **FR-012**: The layout snapshot MUST cover one whole-window state per mockup board, drawn at
  1280×800: *Proposal: session view* (a worktree's session shown, several worktrees in more than
  one sidebar group, more than one session tab, the status bar steady) and *Proposal: Overview*
  (the Overview open, its sidebar row selected). [NEEDS CLARIFICATION: add these two whole-window
  covered states, or only regenerate the fixture over the states the four features added?]
- **FR-013**: The committed layout snapshot fixture MUST equal a fresh regeneration on main after
  the last of #1124–#1127 has merged, and the text-overflow and containment gates MUST pass for
  every covered state in both colour schemes.
- **FR-014**: `docs/development/layout-snapshot.md` MUST describe the redesign's covered states in
  one passage: each region's states with the issue that added them, and the whole-window states.
- **FR-015**: A visual pass MUST record, at 1280×800, a screenshot of each mockup board's state in
  light and in dark, the commit they were taken at, and per screenshot one verdict for each region
  and one for the whole window: *matches*, *differs on purpose* (naming the decision and its spec)
  or *differs by defect* (with what was done about it).
- **FR-016**: The visual pass MUST check body text contrast at or above 4.5:1 in every region in
  both themes, and record any text under it as a defect.

**Both stages**

- **FR-017**: The pages MUST describe what shipped; where it differs from the mockup, the text
  follows the running window.
- **FR-018**: Every shortcut the four pages name MUST give the Linux/Windows and the macOS form.
- **FR-019**: Every link added or changed in the four pages and in `layout-snapshot.md` MUST
  resolve, including its `#anchor`.
- **FR-020**: This feature MUST NOT change the application's behaviour or layout, except to fix a
  defect the visual pass finds, if the clarification in User Story 4, scenario 4 allows it.

### Key Entities

- **Owning section**: one user-guide section per region element; the target every other mention
  links to.
- **Whole-window covered state**: a named layout-snapshot state that draws all four regions at
  1280×800, one per mockup board.
- **Visual-pass record**: per board and theme, a screenshot, the commit, and one verdict per region
  plus one for the whole window.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A search of the four pages for each removed element's name returns 0 hits describing
  it as present (stage 1 for the status bar and session area's removed elements, stage 2 for all).
- **SC-002**: 100% of the four pages' mentions of a region element either are its owning section or
  link to it.
- **SC-003**: 100% of the links in the four pages and in `layout-snapshot.md` resolve, anchors
  included.
- **SC-004**: In each whole-window state, the gap between the title bar's bottom edge and the
  body's top edge is 0 px, the title bar is 44 px and the status bar 28 px tall.
- **SC-005**: Moving any region's outer edge by 1 px in either whole-window state fails the layout
  snapshot, naming the state.
- **SC-006**: The visual pass holds 4 screenshots (2 boards × 2 themes) and 20 verdicts (5 per
  screenshot), each with a reason when it is not *matches*.
- **SC-007**: A new reader, given only the four pages, can name where to open the Overview, switch
  project, open the quick switcher, see the connection state and switch to a shell, in one look
  each: every one of those five answers is in the first paragraph of its owning section.

## Assumptions

- The mockup is the canvas *Micold Layout Redesign*, boards *Proposal: session view* and *Proposal:
  Overview*, 1280×800 each; the board *Current (schematic)* is the old layout and is not compared.
- Each of #1124–#1127 already updated its own region's sections, states and visual pass; this
  feature sweeps the whole pages and the whole window, and does not redo a region's own pass.
- The visual pass is driven by the `visual-pass` skill or, as #1126 did in the cloud container, by
  drawing the application's own view through its software renderer from fixture states; either is
  acceptable if it shows the real window's drawing.
- No screenshots are added to the user guide; the pages are text, as they are today. Screenshots
  live with the visual-pass record under this feature's directory.
- Stage 2 begins only after both #1125 and #1127 have merged on main; stage 1 does not wait.
- `docs/development/review-redesign.md` lists #1128 among the redesign's tasks; it needs no change
  beyond what its own feature does.

## Out of scope

- Any behaviour or layout change to a region, except a defect fix the clarification in User Story 4, scenario 4 may allow.
- The review redesign (`docs/development/review-redesign.md`, #1269 onward).
- The mockup's open questions (collapsible groups, Changes as a right inspector): decided by the
  regions' own features, recorded here only as *differs on purpose* if they differ.
- Visual passes at sizes other than 1280×800, and on macOS or Windows.
- Overview card restyling (the mockup's item 5, the state stripe): feature 1120's, not this
  redesign's four issues.
