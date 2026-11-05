# Feature Specification: Structured view composer layout and large editor

**Feature Branch**: `feat/1703_structured-view-composer-layout-large-editor`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1703: Structured view: composer layout —
large-editor shortcut, activity icon beside input, settings toggle for model/effort/permission
row, unified look." Labels: flow:feature.

## Terms

- **Composer**: the input area at the bottom of the structured view, where the user writes a
  message to the agent.
- **Session options row**: the row above the composer's text area that holds the model, effort and
  permission selectors.
- **Selector**: one of the three controls in the session options row (model, effort, permission).
- **Activity indicator**: the small animated icon that shows whether the session is ready, busy or
  waiting.
- **Large editor**: a modal editor with a large multi-line text area, opened from the composer for
  writing long text.
- **Draft**: the text currently in the composer's text area.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Write long text in a large editor (Priority: P1)

A user composing a long multi-line message presses a keyboard shortcut (Ctrl+Shift+M, Cmd+Shift+M
on macOS) while the composer text area has focus. A modal large editor opens holding the draft. The
user writes, then confirms; the text returns to the composer. Cancelling leaves the draft as it was.

**Why this priority**: The only new capability; the rest rearranges existing controls.

**Independent Test**: Type a draft, press the shortcut, edit in the large editor, confirm, and
check the composer holds the edited text; repeat with cancel and check it holds the original.

**Acceptance Scenarios**:

1. **Given** the composer text area has focus, **When** the user presses the shortcut, **Then** the
   large editor opens containing the current draft with the cursor at its end.
2. **Given** the large editor is open, **When** the user activates Done or presses Ctrl/Cmd+Enter,
   **Then** it closes and the composer draft equals the edited text.
3. **Given** the large editor is open, **When** the user presses Esc, **Then** it closes and the
   composer draft is unchanged.
4. **Given** a draft of more than one line, or the text area is focused or hovered, **When** the
   composer is shown, **Then** an "Open large editor (Ctrl+Shift+M)" icon button at the text
   area's top-right opens the same editor; for a single-line, unfocused, unhovered draft it is
   hidden.
5. **Given** two sessions, one with the large editor open, **When** Done is activated, **Then**
   the text lands only in the draft of the session that opened it.
6. **Given** the shortcut list in the user guide and the in-app shortcut help, **When** the user
   looks for the large-editor shortcut, **Then** it is listed with both the Ctrl and Cmd forms.

---

### User Story 2 - Activity indicator beside the input (Priority: P1)

The activity indicator sits in the input row, right of the text area and left of the attach, Stop
and Send buttons, where the permission selector used to be. It shows only the icon; the status words
(Ready, Busy, Waiting) are available as its tooltip and accessible label.

**Why this priority**: Core layout change; frees the status row.

**Independent Test**: Render the composer in each activity state; the indicator is in the input
row, the permission selector is not, and the tooltip/label names the state.

**Acceptance Scenarios**:

1. **Given** any session state, **When** the composer is shown, **Then** the activity indicator is
   in the input row between the text area and the attach button, and no permission selector is
   there.
2. **Given** the session is Ready, Busy or Waiting, **When** the user hovers the indicator or a
   screen reader reads it, **Then** it reports that state's word.
3. **Given** the status row no longer carries the words, **When** the composer is collapsed,
   **Then** no status text is drawn beside the indicator.

---

### User Story 3 - Session options row behind a settings icon (Priority: P1)

A settings icon at the input row's leading edge (labelled "Session options") expands and collapses
the session options row, which holds the model, effort and permission selectors. The row is hidden
by default.

**Why this priority**: Reclaims vertical space and removes rarely changed controls from the
default view.

**Independent Test**: Start with the composer collapsed (row absent); activate the icon (row with
three selectors appears, icon shows tonal state); activate again (row disappears).

**Acceptance Scenarios**:

1. **Given** a fresh window, **When** the composer is shown, **Then** the session options row is
   hidden and the settings icon is in its collapsed look.
2. **Given** the row is hidden, **When** the user clicks the icon, or focuses it and presses Space
   or Enter, **Then** the row appears holding model, effort and permission selectors in that order,
   and the icon takes a tonal (selected) look.
3. **Given** the row is shown, **When** the user activates the icon, **Then** the row hides.
4. **Given** the user expanded or collapsed the row, **When** the app is restarted, **Then** the
   same state is restored, per window.
5. **Given** the permission mode is Bypass, **When** the row is collapsed or expanded, **Then**
   the settings icon carries a warning marker.
6. **Given** the row is collapsed and the app raises a refusal notice about the model, the effort
   or the permission mode, **When** the notice is raised, **Then** the row expands automatically.
7. **Given** the row is collapsed, **When** the session is overdue ("Stop session" is offered) or
   a permission refusal note exists, **Then** each stays visible outside the row.

---

### User Story 4 - One consistent selector style (Priority: P2)

The model, effort and permission selectors look and behave the same: same height, shape, border,
typography, inline "Label: Value" text with a trailing chevron, and the same hover, focus, pressed,
open and disabled states. A disabled selector explains why in its tooltip. The model selector opens
the same floating list as the others, with a type-to-filter field as the list's first row.

**Why this priority**: Visual polish on top of the layout change.

**Independent Test**: Render the three selectors in each state and compare their geometry and
state treatments; they match, differing only in label and value.

**Acceptance Scenarios**:

1. **Given** the row is shown, **When** the three selectors are measured, **Then** they have equal
   height (32 dp), corner radius (8 dp), border (1 dp outline variant) and body typography, and a
   minimum width of 140 dp with the value truncated by an ellipsis.
2. **Given** a selector, **When** hovered, focused, pressed, open or disabled, **Then** it shows the
   hover (+8%), focus ring (3 dp secondary), pressed (+10% with ripple), open (2 dp primary border,
   rotated chevron) and disabled (38% content, 12% container) treatments identically across all
   three.
3. **Given** the model selector, **When** opened, **Then** the floating list shows a filter field
   as its first row and typing narrows the models.
4. **Given** a disabled selector, **When** hovered, **Then** its tooltip gives the reason.

### Edge Cases

- Shortcut pressed while the text area is not focused: nothing happens.
- Shortcut pressed with an empty draft: the large editor opens empty.
- Shortcut pressed while a large editor is already open: no second editor opens.
- Draft contains attachments: attachments stay on the composer, untouched by the large editor.
- Session ends or the composer becomes disabled while the large editor is open: the editor stays
  open; Done still writes the edited text to that session's draft (kept, not sent), and Esc leaves
  the draft unchanged. No typed text is lost.
- Very long drafts: the large editor scrolls; confirming returns the whole text.
- Narrow window: the selectors in the options row wrap or shrink to the 140 dp minimum without
  overlapping.
- A selector is disabled while the row is collapsed: the reason is reachable after expanding.
- Several sessions or windows open: each window keeps its own row state; the large editor is modal
  to its window, so the session cannot be switched while it is open, and it is bound to the session
  that opened it (FR-019).
- Linux/Windows versus macOS: the shortcut and its tooltip use Ctrl on the former and Cmd on the
  latter; a terminal pane never receives the chord from the composer.
- Persisted state unreadable or missing: the row falls back to collapsed.
- Another view's shortcut also uses Ctrl+Shift+M: none exists (checked against the keymap and the
  user guide's shortcut tables).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The composer MUST open the large editor on Ctrl+Shift+M (Cmd+Shift+M on macOS) while
  its text area has focus.
- **FR-002**: The large editor MUST be a modal with a large multi-line text area initialised with
  the draft; Done and Ctrl/Cmd+Enter MUST return the edited text to the draft; Esc MUST discard
  changes.
- **FR-003**: The large editor MUST handle text only; attachments are neither shown nor changed.
- **FR-004**: The composer MUST show an "Open large editor" icon button at the text area's
  top-right while the draft has more than one line or the text area is focused or hovered, with the
  tooltip "Open large editor (Ctrl+Shift+M)" (Cmd on macOS).
- **FR-005**: The shortcut MUST be listed in the in-app shortcut help and in
  `docs/user-guide/structured-input.md` and the shortcuts table of
  `docs/user-guide/terminal-panes.md`.
- **FR-006**: The activity indicator MUST be icon-only (24 dp), placed in the input row between the
  text area and the attach/Stop/Send buttons.
- **FR-007**: The activity state words MUST be the indicator's tooltip and accessible label, and
  MUST no longer be drawn as visible text in the status row.
- **FR-008**: The permission selector MUST NOT appear in the input row.
- **FR-009**: A settings icon button (40 dp, labelled "Session options") MUST sit at the input
  row's leading edge, be a keyboard tab stop, and toggle on click, Space and Enter; it MUST look
  tonal while the row is expanded.
- **FR-010**: The session options row MUST hold the model, effort and permission selectors, in that
  order, and MUST be hidden by default.
- **FR-011**: The expanded/collapsed state MUST be kept per window and persisted across restarts;
  the default is collapsed.
- **FR-012**: The row MUST expand automatically when a refusal notice about the model, the effort
  or the permission mode is raised.
- **FR-013**: "Stop session" (when offered) and the permission refusal note MUST stay visible while
  the row is collapsed.
- **FR-014**: The settings icon MUST show a warning marker while the permission mode is Bypass.
- **FR-015**: The three selectors MUST share one trigger style: 32 dp height, 8 dp radius, padding
  of the design system's XS (vertical) and SM (horizontal) spacing steps, 1 dp outline-variant border, body typography,
  inline "Label: Value" text, a trailing chevron, minimum width 140 dp, value ellipsised.
- **FR-016**: The three selectors MUST share the same state treatments for hover, focus, pressed
  (with ripple), open and disabled as listed in User Story 4; a disabled selector MUST give its
  reason in a tooltip.
- **FR-017**: The model selector MUST open the same floating list as the other selectors, with a
  type-to-filter field as its first row; the former borderless text-button and filled permission
  variants MUST no longer be used by the composer.
- **FR-018**: Existing behaviour of the selectors (choices offered, what they change, when they
  are disabled) MUST be unchanged.
- **FR-019**: The large editor MUST be modal to its window and bound to the session whose composer
  opened it; Done MUST write only to that session's draft, also if the session has ended meanwhile.

### Key Entities

- **Composer layout state**: whether the session options row is expanded; per window, persisted.
- **Draft**: the composer text, shared between the composer and the large editor while it is open.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can move a long draft into the large editor and back using only the keyboard
  in two key chords (open, confirm).
- **SC-002**: In the default (collapsed) state the composer shows a single control row (the input row)
  above the text area, where today it shows two (status row plus input row); the three selectors are
  reachable in one action.
- **SC-003**: The activity state is conveyed in 100% of states (ready, busy, waiting) by icon plus
  tooltip and accessible label, with no visible status text.
- **SC-004**: The three selectors have identical height, radius, border and typography, and
  identical state treatments in all five states.
- **SC-005**: After restart, the row is in the state the user last left it in, for every window.
- **SC-006**: The shortcut appears in every shortcut list the user guide and in-app help maintain.

## Decisions

Open questions of the design brief, resolved by the owner's suggested defaults.

| # | Question | Decision |
|---|---|---|
| D1 | Shortcut | Ctrl+Shift+M (Cmd+Shift+M on macOS); no existing binding uses it. |
| D2 | "Stop session" and permission refusal note while collapsed | Stay visible; the row also auto-expands on a refusal that needs a selector. |
| D3 | Model typeahead | In-list: the filter is the floating list's first row. |
| D4 | Persist expanded state | Yes, per window, default collapsed. |
| D5 | Bypass-mode warning | Marker on the settings icon while collapsed or expanded. |
| D6 | Large editor attachments | Text only. |

## Assumptions

- The component library already offers an outlined menu-select trigger, an icon button, a busy
  indicator and a modal surface; this feature reuses them.
- The existing settings icon glyph is available or can be added to the icon set.
- Window-level UI state already has a persistence mechanism that this feature extends.
- The visual numbers (dp, percentages) come from the owner's design brief and the existing
  Material state-layer conventions.
- Out of scope: changing what the selectors choose, the terminal-pane composer, attachments in the
  large editor, and rebinding the shortcut.
