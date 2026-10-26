# Feature Specification: Compact session header

**Feature Branch**: `feat/1463_compact-session-header-title-chips-and-changes-on`

**Created**: 2026-10-26

**Status**: Draft

**Input**: GitHub issue #1463 "Compact session header: title, chips and Changes on one line; switch button in the tab strip". Builds on spec 1126 (worktree context header and session tabs).

## Terms

- **Row**: a full-width band above the terminal. Before this change the header was two rows (title line, chips line) and the strip one; now the header is one row.
- **Header**: the band above the tab strip showing the worktree title, its status chips (state, branch, pull request) and the Changes button.
- **Strip**: the tab strip below the header: session tabs, the add control, the status words, and the switch control (single pane only), and the fixed controls. Left to right in the cluster: status words, switch control with its caption, then the fixed controls with restart last.
- **Switch control**: the control that switches a terminal session between its terminal view and its structured view ("Open in structured view"), with the held-queue caption beside it.
- **Single pane**: the session area shows one session; **tiled panes** show several, each with its own per-pane control row.

## Clarifications

### Session 2026-10-26

- Q: Is changing the fixed `tab::SESSION_WIDTH` in scope (issue item 4, optional)? → A: No, deferred. It is a fixed, derived constant (1126 R8) that the strip's reveal and edge-fade arithmetic and the stability gates rely on; changing it would re-record tab snapshots unrelated to the header move. _(agent-resolved: crates/micold-material/src/tab.rs#SESSION_WIDTH; specs/1126-session-context-header R8)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See the header on one line (Priority: P1)

A user opens a worktree. The title, the chips and the Changes button sit on a single line, so the header takes about one line of height and the terminal gets the room.

**Why this priority**: The header currently uses two rows for little content; this is the core saving.

**Independent Test**: Render the header for a worktree with state, branch and pull request, and measure rows and height.

**Acceptance Scenarios**:

1. **Given** a worktree with a state, a branch and a pull request, **When** the header is shown, **Then** the title, then the chips, then the Changes button appear on one line, in that order.
2. **Given** the Changes button with "no changes", **When** the header is shown, **Then** the button is no taller than the chips and does not set the line height.
3. **Given** a window too narrow for everything, **When** the header is laid out, **Then** the title is shortened with an ellipsis first; the chips keep their capped width; the Changes label is cut with an ellipsis only when the title is at its minimum (amended: the label keeps its requested width only where the line has room).

---

### User Story 2 - Switch view from the strip, not from its own row (Priority: P1)

A user with a Claude terminal session in a single pane finds "Open in structured view" in the strip beside the status words, not in a separate row.

**Why this priority**: Removes a whole row (about 50 px) for the most common session kind.

**Independent Test**: Render a Claude terminal session in a single pane; count rows and locate the switch control.

**Acceptance Scenarios**:

1. **Given** a Claude terminal session in a single pane, **When** the area is shown, **Then** only two rows sit above the terminal (header, strip), the switch control being in the strip after the status words and before restart.
2. **Given** that strip, **When** its controls are listed, **Then** restart is still the last control of the cluster.
3. **Given** a session with a held queue, **When** the strip is shown, **Then** the held-queue caption appears beside the switch control.
4. **Given** the Review tab is shown, **When** the strip is shown, **Then** the switch control and its caption are hidden.
5. **Given** a session that has no switch control (not a terminal session), **When** the area is shown, **Then** it has two rows above the terminal.
6. **Given** tiled panes, **When** the area is shown, **Then** each pane keeps its own switch row exactly as before.

---

### User Story 3 - Tabs and controls stay steady (Priority: P2)

Existing behaviour of the strip (stable child list, fixed controls, scrolling) is unchanged by the move.

**Why this priority**: Guards the regressions 1126 already fixed.

**Independent Test**: The existing layout and stability gates pass; the layout snapshots are re-recorded for the new layout only.

**Acceptance Scenarios**:

1. **Given** any session state, **When** the strip is built, **Then** its child list does not vary between states (a press lands on its first try).
2. **Given** the smallest supported window (640x480 with the sidebar open), **When** the header and strip are shown, **Then** nothing overlaps and everything lies inside the session area.

## Edge Cases

- Very long title, branch or pull request title: the title is cut first; chips stay capped.
- Four-digit line counts in the Changes label (for example `Changes · 128 files · +2048 −1024`) at 640x480 with the sidebar at 300 px: the one case where the label is cut with an ellipsis, its full text in the tooltip, after the title is already at its minimum; the 1126 bound (H10) is restated for the one-line header in the H9/H10 update. At 1280x800 counts up to five digits of files and six of lines are whole.
- Narrow strip at 640x480 with a long held-queue caption: the caption is cut with an ellipsis (full text in a tooltip); the switch control and restart are never clipped; the session tabs scroll as in 1126.
- Empty or untitled worktree: the header line still shows the chips and the Changes button; the title area stays empty.
- No branch or no pull request: those chips are absent; the line still holds title and Changes button.
- Review tab shown and then closed: the switch control disappears and returns; restart and the other fixed controls keep their position from the right edge, only the space the switch control took opens or closes.
- Several windows or sessions: each session area lays out independently; no shared state is added.
- Light and dark themes, and each platform's window chrome: same layout.
- A session that is stopped or restarting: the switch control follows the same enabled rules as before; restart remains last.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The header MUST show the title, the chips and the Changes button on one line, in that order, with the title taking the remaining width.
- **FR-002**: When width is short, the title MUST shorten before the chips or the Changes button lose any width; the chips stay capped at their maximum width. At the smallest window (640 px, sidebar 300 px) the line cannot hold the chips and a three-digit Changes label: once the title is at its minimum the Changes label is cut with an ellipsis (tooltip whole). The full width of the label is guaranteed only where the line has room.
- **FR-003**: The Changes button MUST keep its look and glyph but be compact, so it does not set the header line height; the button MUST be no taller than the title line and the whole header about 40 px or less at default scale.
- **FR-004**: In a single pane, the switch control and held-queue caption MUST appear in the strip after the status words and before restart; no separate switch row MAY be shown.
- **FR-005**: Restart MUST remain the last control of the strip's cluster.
- **FR-006**: The switch control and caption MUST be hidden while the Review tab is shown.
- **FR-007**: In tiled panes, each pane MUST keep its own switch row unchanged.
- **FR-008**: No strip control moves or is missed between session states (idle, running, stopped, held queue, Review shown) beyond the switch control's own space; a press lands on its first try.
- **FR-009**: A Claude terminal session in a single pane MUST need two rows above the terminal (about 100 px instead of about 190 px); any other session two rows instead of three.
- **FR-010**: The layout snapshots, the header layout contract (H9) of spec 1126 and the user-guide header description MUST describe the new layout.
- **FR-011**: The session tab width (`tab::SESSION_WIDTH`) MUST stay as it is; reconsidering it is deferred (see Clarifications).
- **FR-012**: In a narrow strip the held-queue caption MUST be cut with an ellipsis (full text in a tooltip) first, then the switch control's label (full text in a tooltip), before restart, New session or the split buttons are clipped; the tab region may shrink to nothing; session tabs scroll as in 1126.

### Key Entities

- **Header**, **Strip**, **Switch control**: as in Terms. No new data.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A Claude terminal session in a single pane shows 2 rows above the terminal, about 100 px, down from 4 rows and about 190 px.
- **SC-002**: Every other session shows 2 rows, down from 3.
- **SC-003**: At 640x480 with the sidebar at 300 px and at 1280x800, light and dark, no header or strip element overlaps another or leaves the session area, checked by the visual pass.
- **SC-004**: Tiled panes render identically to before (each pane's row size and position relative to the pane-area origin unchanged; absolute fixtures move with the shorter header and are re-recorded).
- **SC-005**: Every control reachable before the change is still reachable, and a press on any acts on the first try.
- **SC-006**: At 640x480 with a long held-queue caption, the switch control and restart are fully visible.

## Out of scope

- New widgets or new controls; only existing ones are moved and resized.
- Changing what the chips, Changes button or switch control do.
- Tiled-pane layout.
- The session tab width (FR-011): deferred to a separate issue.

## Assumptions

- The pre-existing failure of the 24-character-name test in the changes view is unrelated and fails on main; it is not fixed here.
- Behaviour already specified in 1126 (fixed controls, scroll, tooltips, ellipsis rules) holds unless this spec says otherwise.
