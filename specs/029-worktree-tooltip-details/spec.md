# Feature Specification: Worktree tooltip shows the full name and its details

**Feature Branch**: `feat/tooltip-of-worktree-should-show-full-name`

**Created**: 2026-09-03

**Status**: Closed 2026-09-13 — implemented and shipped in PR #281 (merged 2026-09-12); all 30 tasks
in [tasks.md](./tasks.md) are done. The quickstart §B pass ran 2026-09-12 and B1–B6 all pass; the
results table and screenshots are in [quickstart.md](./quickstart.md#the-pass--2026-09-12) and
[evidence/](./evidence/).

**Input**: User description: "tooltip of worktree should beside location should full worktree name and details"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read a name the row had to shorten (Priority: P1)

A worktree row in the sidebar shows the worktree's name on one line, ending in an ellipsis whenever
the name is longer than the space the row's controls leave for it. A user who cannot tell two rows
apart — "Tooltip of worktree should…" and "Tooltip of worktree shown…" — hovers the row and reads
the whole name in the tooltip, instead of widening the window or opening the rename dialog to find
out which one it is.

**Why this priority**: This is the reported problem. The row already tells the user where the
worktree lives; what it cannot always tell them is *which* worktree it is. Shipping only this story
already fixes the case that prompted the request.

**Independent Test**: Give a project a worktree whose name is far longer than the sidebar is wide,
hover its row, and confirm the tooltip carries the name in full while the row itself still shows one
ellipsized line.

**Acceptance Scenarios**:

1. **Given** a worktree whose name is too long for the row, **When** the user hovers the row,
   **Then** the tooltip shows the complete name, unshortened and without an ellipsis.
2. **Given** a worktree whose name fits the row without shortening, **When** the user hovers the row,
   **Then** the tooltip still shows the name — the tooltip's content does not depend on how wide the
   window happens to be.
3. **Given** a worktree the user has renamed, **When** the user hovers the row, **Then** the name in
   the tooltip is the user's chosen name, matching the row.

---

### User Story 2 - See what the row's shortened label leaves out (Priority: P2)

A worktree's row label is a prettified reading of its folder name: the leading type token and any
ticket reference are stripped out and the rest is sentence-cased, so the row never states the
folder or the git branch the worktree is bound to. A user about to start a session on a worktree
hovers the row and confirms, before pressing anything, which branch that session will run against.

**Why this priority**: Valuable, and the natural companion to the full name, but a user can reach
the same facts today by other means (the terminal, the rename dialog). Story 1 has no such fallback.

**Independent Test**: Hover a worktree whose folder name carries a type token and a ticket
reference, and confirm the tooltip names both the folder and the bound branch that the row's
prettified label does not show.

**Acceptance Scenarios**:

1. **Given** a worktree bound to a git branch, **When** the user hovers the row, **Then** the tooltip
   states the branch name in full.
2. **Given** a worktree whose folder name differs from its displayed name, **When** the user hovers
   the row, **Then** the tooltip states the folder name as it is on disk.
3. **Given** a worktree with no bound branch, **When** the user hovers the row, **Then** the tooltip
   omits the branch line rather than showing an empty or placeholder value.

---

### User Story 3 - Understand a row that is flagged (Priority: P3)

A worktree may be flagged in the list: missing or invalid on disk (shown by an error tint and a
status chip), or living outside the directory this app creates worktrees in (shown by an "outside
this app" chip). A user hovers such a row and the tooltip says the same thing in words, next to the
location that explains it.

**Why this priority**: These rows are the ones a user is most likely to hover for an explanation,
but the chips already carry the fact; the tooltip is reinforcement rather than the only channel.

**Independent Test**: Hover a missing worktree and an included (outside-the-app) worktree and
confirm each tooltip states its condition alongside the location.

**Acceptance Scenarios**:

1. **Given** a worktree whose directory is missing or invalid, **When** the user hovers the row,
   **Then** the tooltip states that condition in words.
2. **Given** a worktree that lives outside the directory this app creates worktrees in, **When** the
   user hovers the row, **Then** the tooltip states so and shows its absolute location.
3. **Given** a healthy worktree in the usual place, **When** the user hovers the row, **Then** the
   tooltip carries no status wording — a normal row reads as normal.

---

### Edge Cases

- **A name that is already fully visible**: the tooltip repeats it rather than suppressing it, so the
  tooltip's shape does not change with window width.
- **A name longer than the tooltip can reasonably be**: the tooltip stays readable and bounded — it
  does not stretch past the window or cover the sidebar it describes.
- **No bound branch** (orphan or detached worktree): the branch line is omitted entirely.
- **Folder name equals the displayed name**: the tooltip does not print the same string twice.
- **A worktree outside the project root**: the location shown is the absolute path, as it is today —
  a path relative to a root the worktree is not under would be misleading.
- **The "Default" entry**: it is the project root, not a worktree; it keeps its existing fixed
  location wording and gains nothing from this feature.
- **Session rows nested under a worktree**: out of scope; this feature changes worktree rows only.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Hovering a worktree row MUST show a tooltip that includes the worktree's full displayed
  name, regardless of whether the row itself had to shorten that name.
- **FR-002**: The name in the tooltip MUST be the same name the row shows — including a user-assigned
  rename — so the tooltip can never disagree with the row it describes.
- **FR-003**: The tooltip MUST continue to show the worktree's location, with the same wording and
  the same relative/absolute choice it uses today.
- **FR-004**: The tooltip MUST state the worktree's bound git branch when it has one, and MUST omit
  that line when it does not.
- **FR-005**: The tooltip MUST state the worktree's folder name on disk when that name differs from
  the displayed name, and MUST NOT repeat it when the two are the same.
- **FR-006**: The tooltip MUST state, in words, when a worktree is missing or otherwise invalid on
  disk, and MUST say nothing about status for a healthy worktree.
- **FR-007**: The tooltip MUST state when a worktree lives outside the directory this app creates its
  worktrees in.
- **FR-008**: Each fact in the tooltip MUST be presented on its own labelled line so a user can find
  one without reading the rest.
- **FR-009**: The tooltip MUST remain bounded in width — long names, paths, and branches wrap or are
  otherwise contained rather than pushing the tooltip past the window edge.
- **FR-010**: The tooltip MUST appear on hover and dismiss on unhover exactly as the current location
  tooltip does; no new gesture, delay, or click is introduced.
- **FR-011**: The "Default" entry's tooltip MUST keep its current fixed wording — this feature does
  not change it.
- **FR-012**: The tooltip's content MUST be derived without reading the disk while the user hovers, so
  hovering a row never blocks or lags the list.

### Key Entities

- **Worktree row**: one entry in the sidebar's worktree list. Carries a displayed name (derived from
  the folder name, or overridden by a user rename), a folder name, an optional bound branch, a
  location, a health status, and whether it was included from outside the app's own worktree
  directory.
- **Row tooltip**: the hover surface attached to a worktree row. Today it holds the location alone;
  this feature makes it hold the identifying facts about that worktree.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every worktree in the list, a user can read its complete name without resizing the
  window, scrolling horizontally, or opening any dialog — in one hover.
- **SC-002**: A user can name the git branch a worktree is bound to within 3 seconds of pointing at
  its row, without leaving the sidebar.
- **SC-003**: Two worktrees whose displayed names share a long common prefix can be told apart from
  their tooltips alone, in 100% of cases.
- **SC-004**: Every fact the row conveys through colour or a chip alone — missing, invalid, outside
  this app — is also available as words on hover, so nothing about a row is colour-only.
- **SC-005**: The tooltip never extends beyond the application window, at the narrowest window size
  the app supports, for a name of at least 120 characters.

## Assumptions

- "Details" means the facts that identify a worktree and explain its row: displayed name, folder
  name, bound branch, location, health status, and outside-this-app inclusion. Session counts,
  timestamps, and git ahead/behind state are **not** included — the first is already visible when the
  row is expanded, and the last two are not held anywhere the list can read cheaply.
- The tooltip stays a single hover surface on the worktree row, in the same position it uses today;
  no popover, no click target, no second tooltip on the name itself.
- Facts are rendered as short labelled lines (e.g. `Branch: …`) rather than a single run-on string,
  which is what makes FR-008 checkable.
- The location wording keeps its existing behaviour (project-relative under the app's worktree root,
  absolute otherwise) rather than being redesigned here.
- The existing behaviour where a long name is ellipsized *in the row* is correct and stays — this
  feature adds a way to read the whole name, it does not stop the row shortening it.
- Everything the tooltip shows is already held in the app's in-memory worktree list, so no new data
  source, git call, or filesystem read is needed.
