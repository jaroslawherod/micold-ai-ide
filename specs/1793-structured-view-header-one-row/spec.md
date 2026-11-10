# Feature Specification: Structured view header in one row; status and actions move to the top bar

**Feature Branch**: `feat/1793_feat-structured-view-header-in-one-row-status-and`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "GitHub issue #1793: feat: structured view: header in one row, status and actions move to the top bar" (labels: enhancement, flow:feature)

## Clarifications

### Session 2026-11-10

- Q: Does the `+` menu's terminal entry open a new terminal each time or focus an existing one? → A: A new terminal tab each time, like the former add-terminal icon. _(agent-resolved: specs/1793-structured-view-header-one-row/spec.md#User Story 3 — "replaces the two former add icons")_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One-row header in the structured view (Priority: P1)

A user opens a worktree's structured view. Today four rows of header sit above the first event: a title row, a tab row, a context-meter and actions row, and (sometimes) a hidden-events row. The title repeats the breadcrumbs, and `running` repeats the status badge. The user wants the first event near the top, so the content area keeps a single 40 px row: the session tabs, one add button, and a `N hidden` chip when events are hidden.

**Why this priority**: It is the point of the feature: it frees vertical space for events.

**Independent Test**: Open a structured view with and without hidden events; count header rows above the first event and check that no title row is present.

**Acceptance Scenarios**:

1. **Given** a selected session, **When** the structured view is shown, **Then** exactly one header row (the tab row) precedes the event list and no title row, `Open in terminal` button, `Show raw events` control, second `+` icon or view toggles appear in the content area.
2. **Given** 3 events are hidden, **When** the view is shown, **Then** a `3 hidden` chip appears at the end of the tab row; **Given** no events are hidden, **Then** the chip is absent.
3. **Given** a session is working, **When** the tab row is shown, **Then** the session's tab shows a small activity dot and no `running` text appears.
4. **Given** the user activates the `N hidden` chip, **Then** the hidden events are shown (the former `Show` action).

---

### User Story 2 - Status, Changes and context meter in the top bar (Priority: P1)

With a worktree or session selected, the top bar shows, right of the search box and in this order: the context meter (compact ring with `8% · $1.28`), the `Idle`/`Working` status, the `Changes · no changes` button, and a worktree `⋮` menu, before the app's own settings and menu. Hovering the ring shows a tooltip with tokens used/total, cost and turns.

**Why this priority**: The removed header content must live somewhere; this keeps status always visible.

**Independent Test**: Select a worktree, check the top-bar items and their order; select Overview, check they are gone.

**Acceptance Scenarios**:

1. **Given** a worktree or session is selected, **When** the top bar is shown, **Then** the context meter, status, Changes button and worktree `⋮` appear in that order, before the app settings and app menu.
2. **Given** Overview is shown (nothing selected), **Then** none of those four items is shown.
3. **Given** a session with 84,671 of 1,000,000 tokens used, **When** the user hovers the ring, **Then** the tooltip shows tokens used/total, cost and turn count, and the ring shows `8% · $1.28`.
4. **Given** the user activates Changes, **Then** the same changes view opens as before.

---

### User Story 3 - Single `+` to add a session or a terminal (Priority: P1)

One `+` icon at the end of the tabs replaces the two former add icons and the `Open in terminal` button. Pressing it opens a small menu anchored to the icon offering another session in this worktree (with the same AI CLI and kind choice as today) or a terminal in this worktree.

**Why this priority**: Without it, removing the button and the second icon loses functions.

**Independent Test**: Press `+`, choose each entry, and verify the session or terminal opens as before.

**Acceptance Scenarios**:

1. **Given** a selected worktree, **When** the user presses `+`, **Then** a menu anchored to the icon lists "another session" and "terminal".
2. **Given** the menu is open, **When** the user chooses a session, **Then** the existing add-session choice (AI CLI and kind) is offered and the session is created as today.
3. **Given** the menu is open, **When** the user chooses terminal, **Then** a new terminal tab opens in the worktree each time, as the former add-terminal icon did.
4. **Given** the menu is open, **When** the user presses Escape or clicks elsewhere, **Then** it closes and nothing opens.

---

### User Story 4 - Worktree `⋮` menu holds raw events and view toggle (Priority: P2)

The worktree `⋮` menu next to the status holds the `Show raw events` toggle and the view toggle (segmented control), so worktree actions are not mixed into the global app menu.

**Why this priority**: Keeps these features reachable; lower than the layout itself because it is secondary actions.

**Independent Test**: Open the worktree `⋮`, flip each control, check the view reacts; open the app `⋮` and check it holds no worktree entries.

**Acceptance Scenarios**:

1. **Given** a selected session, **When** the user opens the worktree `⋮`, **Then** it shows the raw-events toggle and the view segmented control reflecting current state.
2. **Given** the user toggles raw events or switches view, **Then** the structured view changes accordingly and the menu reflects the new state.
3. **Given** the app `⋮` is opened on any screen, **Then** it holds no worktree-scoped entries.

---

### User Story 5 - Narrow windows degrade gracefully (Priority: P2)

At narrow widths the ring's text drops first, then the `Changes` label (the icon stays); the tabs shrink and scroll. No item overlaps or disappears.

**Why this priority**: Without it the enlarged top bar breaks small windows.

**Independent Test**: Resize through wide and narrow widths and check order of collapse.

**Acceptance Scenarios**:

1. **Given** the window narrows until the top bar is full, **Then** the ring text (`8% · $1.28`) hides first while the ring stays.
2. **Given** it narrows further, **Then** the `Changes` label hides and its icon stays.
3. **Given** many tabs, **Then** tabs shrink and then scroll horizontally; the `+` and chip stay reachable.

---

### Edge Cases

- No worktree or session selected (Overview): top-bar worktree items and worktree `⋮` hidden; app `⋮` and settings unchanged.
- Selection changes while the `+` menu or worktree `⋮` is open: the menu closes.
- Context data unavailable (new session, no usage yet): the ring shows an empty state and the tooltip says so; no cost is invented.
- Zero hidden events: the chip is absent; many hidden events (for example 1,000+) must still fit the chip.
- Several sessions in the same worktree (Principle II): the top-bar status, meter and tab dots follow the selected session; other tabs show their own activity dot, and one session's state never leaks into another's.
- Session fails or exits: status badge and tab dot show the non-working state; terminal tabs are unaffected.
- Cross-platform (Principle VI): the same layout and collapse order on Linux, macOS and Windows; the title bar controls of each platform stay clear of the new items.
- Keyboard-only and screen reader use: `+`, the worktree `⋮`, the ring and the chip are focusable and labelled; the ring's tooltip content is available as an accessible label.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The structured view's content-area header MUST be a single 40 px row containing the session tabs, one `+` control and, only when events are hidden, a `N hidden` chip at the end.
- **FR-002**: The title row MUST be removed; the breadcrumbs MUST remain the only place the title is shown.
- **FR-003**: The status badge (`Idle`/`Working`), the `Changes` button, the context meter with cost and the worktree `⋮` menu MUST appear in the top bar only when a worktree or session is selected, and MUST be hidden on Overview.
- **FR-004**: The top bar order MUST be: context meter, status, Changes, worktree `⋮`, then the app's settings and menu.
- **FR-005**: The context meter MUST show a compact ring with `<percent> · <cost>` text; a tooltip MUST show tokens used/total, cost and turns.
- **FR-006**: The `running` text MUST be removed; a working session's tab MUST show a small activity dot instead.
- **FR-007**: Exactly one `+` control MUST exist; activating it MUST open a small menu anchored to it offering another session in the worktree (with today's AI CLI and kind choice) or a terminal in the worktree.
- **FR-008**: `Open in terminal` MUST no longer exist as a separate button; its function MUST be reachable from the `+` menu.
- **FR-009**: The worktree `⋮` menu MUST contain the `Show raw events` toggle and the view toggle (segmented control). The app `⋮` MUST NOT gain worktree entries.
- **FR-010**: The `N hidden` chip MUST give access to the former hidden-events `Show` action.
- **FR-011**: On narrow widths the ring text MUST hide first, then the `Changes` label (icon stays); tabs MUST shrink then scroll. When the ring text hides, the tooltip and accessible label MUST still carry the cost.
- **FR-012**: Nothing reachable before MUST be lost: view toggles, raw events, hidden-events toggle, open in terminal, add session.
- **FR-013**: All new controls MUST be keyboard reachable and have accessible names; the menus MUST close on Escape and on outside click.
- **FR-014**: The layout MUST be verified by the visual pass (quickstart Part B) at wide and narrow widths.

### Key Entities

- **Selection**: the worktree or session currently selected; decides whether the worktree items are shown.
- **Session status**: Idle or Working per session; drives the badge and tab dot.
- **Context usage**: tokens used, token limit, cost and turn count of a session; drives ring, text and tooltip.
- **Hidden events**: count of events filtered out of the structured view; drives the chip.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Header rows above the first event drop from up to 4 to exactly 1 (40 px) in the structured view.
- **SC-002**: Every action reachable before (view toggles, raw events, hidden-events toggle, open in terminal, add session) is reachable after, in at most two clicks from the top bar or tab row.
- **SC-003**: Status, Changes and context meter are visible without scrolling or opening a menu whenever a worktree or session is selected, and absent on Overview.
- **SC-004**: At a wide and a narrow window width, no top-bar or tab-row item overlaps another, and collapse follows the specified order.
- **SC-005**: A user can open a terminal or a new session from the single `+` in one menu selection.

## Assumptions

- The tab row is 40 px high and the breadcrumbs already show the worktree/session title.
- The add-session flow (AI CLI and kind choice) is unchanged; only its entry point moves into the `+` menu.
- Related issues #1764 (settings icon position) and #1792 (Material 3 select and settings animation) are separate; this feature only places items before the app's settings and menu.
- The existing reusable menu, chip and segmented-control components are reused, and the docs are updated with the new layout.
- Out of scope: changing what the status, Changes, meter or raw events show; new view kinds; Overview layout; mobile layouts.
- The `N hidden` chip uses the wording `N hidden`; the ring tooltip's exact format follows the existing meter text.
- Every working session's tab shows its own activity dot, not only the selected one.
- The `+` menu's terminal entry opens a new terminal tab in the worktree each time (it never focuses an existing one), like the former add-terminal icon it replaces.
