# Feature Specification: Review tab fits the smallest window

**Feature Branch**: `fix/1474_review-tab-parts-are-squeezed-out-of-their-boxes`

**Created**: 2026-11-01

**Status**: Draft

**Input**: User description: "Issue #1474: Review tab: parts are squeezed out of their boxes at the smallest window (640x480). Parts 1 and 2 (progress slot, sort chips) shipped in #1549 and #1592. Remaining: the diff header row, the diff placeholder, the comments drawer. User decision: auto-collapse the sidebar at the smallest window so the tree and diff panes get the width." Labels: bug, flow:feature (switched from flow:bug). Builds on feature 1276 (review tab, `gates/review_tab_fits.rs`) and feature 970 (`specs/970-overview-fullscreen`, sidebar retracted in full screen).

## Terms

- **Smallest window**: the window at `MIN_WINDOW_SIZE`, 640x480 logical pixels.
- **Compact width**: a window narrower than 720 logical pixels (the **compact threshold**). Only 640 to 719 are reachable, since the window cannot be made smaller than 640.
- **Sidebar state**: expanded (at its width) or collapsed to its thin strip, as the user set it with the hide and show buttons (persisted today).
- **Auto-collapse**: the sidebar shown as its collapsed strip because the window is at compact width, not because the user hid it.
- **Review tab**: the Changes tab of a session area: file tree, diff pane (header row, diff, placeholder) and comments drawer.
- **Squeezed**: a layout node laid out outside the parent that owns it, painting over its neighbours (the `escapes` check of `gates/containment.rs`).

## Clarifications

### Session 2026-11-01

- Q: How is the smallest window to give the tree and diff panes room? -> A: Auto-collapse the sidebar at compact width (user answer, authoritative).
- Compact threshold, restore on resize, manual toggle and full screen: decided in this spec, see FR-001 to FR-006.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The review tab fits at 640x480 (Priority: P1)

A reviewer working in a small window opens the Changes tab. The tree, the diff header, the diff placeholder and the comments drawer each stay inside the box that owns them; nothing paints over a neighbour.

**Why this priority**: it is the defect the issue reports.

**Independent Test**: lay out every review state at 640x480 with the compact-width sidebar and assert no node escapes its parent, apart from declared scroll overhangs.

**Acceptance Scenarios**:

1. **Given** the smallest window and a review with changed files, **When** the Changes tab is open, **Then** the diff header's controls (view mode chips, previous and next change, mark control) all lie inside the header row, which lies inside the diff pane.
2. **Given** a diff still loading, **When** the placeholder shows, **Then** it ends at or above the bottom of its box.
3. **Given** the comments drawer open (drafts, replies or empty), **When** the window is 640x480, **Then** the header's close control, the tab labels and each row's action buttons lie inside their rows and the drawer.
4. **Given** the tree with its filter field, **When** the window is 640x480, **Then** the filter's placeholder text and every tree row lie inside the tree pane (no row overflows its parent).
5. **Given** the same states at 1280x800, **When** laid out, **Then** nothing changed from today.

---

### User Story 2 - The sidebar gives way at the smallest window (Priority: P1)

At compact width the sidebar shows as its collapsed strip so the main area has the width. Widening the window brings the sidebar back as the user had it.

**Why this priority**: it is the user's chosen way to give the panes width.

**Independent Test**: reducer-level tests on resize and toggle; layout test that the main area is wider at 640 than 1280 minus the sidebar.

**Acceptance Scenarios**:

1. **Given** the sidebar expanded at 1000px, **When** the window is resized to 640px, **Then** the sidebar shows as the collapsed strip and the main area uses the freed width.
2. **Given** that auto-collapse, **When** the window is resized back to 720px or wider, **Then** the sidebar is expanded again at its previous width.
3. **Given** the user had hidden the sidebar, **When** the window is resized to 640px and back, **Then** the sidebar is still hidden at both.
4. **Given** the app starts at 640px, **When** it shows, **Then** the sidebar is the collapsed strip, and the persisted sidebar state is unchanged.

---

### User Story 3 - The user can still open the sidebar in a small window (Priority: P2)

At compact width the user presses **show** on the strip, or the sidebar shortcut/toggle, and the sidebar expands, at the cost of main-area width. The choice holds until the window leaves compact width.

**Why this priority**: the sidebar must never be unreachable.

**Independent Test**: reducer test: toggle at compact width, then resize within compact width; then resize out and back in.

**Acceptance Scenarios**:

1. **Given** auto-collapse at 640px, **When** the user presses **show**, **Then** the sidebar expands and stays expanded through resizes that remain below the threshold.
2. **Given** the user expanded it at 640px, **When** the window grows to 720px or wider, **Then** the sidebar shows as the persisted state (not as the override); **and When** it shrinks below 720px again, **Then** it auto-collapses again (a fresh crossing).
3. **Given** auto-collapse at 640px, **When** the user presses hide (after showing), **Then** the sidebar is the strip, and at 720px or wider it shows as the persisted state.

---

### User Story 4 - Full screen and the sidebar agree (Priority: P3)

Overview full screen retracts the whole drawer as before; leaving it at compact width shows the strip.

**Independent Test**: reducer/view test combining `full_screen` with compact width.

**Acceptance Scenarios**:

1. **Given** the Overview in full screen at 640px, **When** it closes, **Then** the sidebar returns as the collapsed strip (auto-collapse still applies).
2. **Given** the Overview in full screen at 1000px and the sidebar expanded, **When** it closes, **Then** the sidebar returns expanded, as feature 970 promises.

### Edge Cases

- Window size not yet reported (0, 0): not compact; no auto-collapse (existing meaning of "not reported").
- Resize events at compact width that stay below the threshold never change the sidebar.
- Several windows or sessions: the sidebar state is per window; auto-collapse reads that window's size only. Two windows at different widths collapse independently.
- A resize and a toggle in the same update: applied in arrival order; the toggle acts on the effective state as it is after the resize.
- A manual toggle while a collapse/expand animation runs reverses it as today; reduced motion switches in one step.
- Platform differences: logical pixels on every platform (HiDPI scale does not change the threshold); a window manager that ignores the minimum size and goes below 640 is also compact.
- Empty review (no changed files), failed diff load: same fit rules apply to those states.
- The drag handle on a strip does not exist; a persisted wide sidebar width is kept untouched.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The window is compact when its reported width is below 720 logical pixels. An unreported size (0, 0) is not compact.
- **FR-002**: Entering compact width (a resize from not compact to compact, or the first report being compact) MUST show the sidebar as its collapsed strip, unless it already was one, and MUST NOT change the persisted sidebar state or width.
- **FR-003**: Leaving compact width (a resize to 720 or wider) MUST show the sidebar as the persisted state and clear any compact override (an expansion the user made at compact width ends here).
- **FR-004**: Show and hide at compact width MUST work. A show at compact width expands the sidebar until the window next leaves compact width; a hide returns to the strip (a no-op when it already is the strip); the toggle shortcut flips the effective state. Neither writes the persisted state; a toggle at 720px or wider flips the persisted state, which is what the user sees there, as today.
- **FR-005**: Overview full screen MUST keep retracting the whole drawer; closing it MUST show the sidebar as FR-002 and FR-003 say for the current width.
- **FR-006**: `docs/user-guide/overview.md` and `docs/user-guide/worktrees-and-sessions.md` MUST state the behaviour: the promise "the sidebar always returns exactly as you left it" gains the exception for windows narrower than 720px.
- **FR-007**: At the smallest window with every review state of feature 1276, the diff header row's controls MUST lie inside the row and the row inside the diff pane.
- **FR-008**: At the smallest window the diff loading placeholder MUST lie inside its box.
- **FR-009**: At the smallest window the comments drawer's header, tabs and rows (drafts, replies, empty) MUST lie inside their parents, with controls reachable (never clipped to nothing).
- **FR-010**: At the smallest window the tree's filter placeholder and every tree row MUST lie inside the tree pane.
- **FR-011**: The smallest-window run of the layout gates MUST be compact (sidebar strip, main area 608px wide) and the 1280 run MUST NOT be, so the two runs differ.
- **FR-012**: Each fixed defect MUST delete its known-defect entry in `crates/micold-client/tests/gates/review_tab_fits.rs`; an entry that no longer fires fails the gate (existing behaviour). Scroll-overhang entries stay.
- **FR-013**: Layouts at 1280x800 MUST be unchanged.

### Key Entities

- **Compact flag**: derived from the window's reported width; not stored.
- **Compact override**: transient, per window: whether the user expanded the sidebar at compact width; cleared when the window leaves compact width.
- **Effective sidebar state**: persisted `hidden`, combined with the compact flag and the override (collapsed when compact and not overridden, else the persisted state). Every reader uses it: the drawer's open flag, the strip or width, the main area's freed width. Full screen (`retracted`) applies on top and is independent of it; closing full screen reveals the effective state.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the sidebar strip at 640x480, every review state escapes its parent by no more than the declared scroll overhangs, in light and dark: zero `#1474` entries remain in `review_tab_fits.rs`, except the stale-notice source-control entry if it does not fall out of the wider main area; that one is re-filed under its own issue.
- **SC-002**: Every resize across 720 px in either direction restores the user's sidebar state in the same update as the resize (the visual transition follows the existing sidebar animation, one step under reduced motion), in 100% of tested sequences.
- **SC-003**: The persisted sidebar state is byte-identical before and after any resize sequence that makes no manual toggle at 720px or wider.
- **SC-004**: At 640px the tree and diff panes together get at least 608px of width (640 minus the 32px strip), against about 334px with the default sidebar.
- **SC-005**: The 1280x800 layout snapshot of every review state is unchanged.

## Assumptions

- 720 is the threshold: the window cannot go below 640, and at the default 300px sidebar a 720px window leaves 420px for tree plus diff, too narrow for both, so the strip is chosen below 720. Wider windows are not changed here.
- The sidebar's own contents are not redesigned; only its shown state changes.

## Out of Scope

- Redesigning the review tab for widths other than the smallest window.
- Changing the minimum window size.
- Fixes to the already-merged progress slot and sort chips (#1549, #1592), and the scroll overhang entries.
- The stale-notice source control entry (`0/0/0/1/1/2/0/0/2/0/0/0`, filed under #1474 in the gate): fixed here only if it falls out of the wider main area; otherwise left filed.
