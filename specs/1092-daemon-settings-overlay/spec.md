# Feature Specification: Daemon settings in an overlay with their own Save

**Feature Branch**: `feat/1092_daemon-settings-should-open-in-an-overlay-with-its`

**Created**: 2026-10-19

**Status**: Closed 2026-10-21 — shipped in PRs #1160, #1170, #1174, #1182, plus the close PR

**Input**: User description: "Issue #1092, Daemon settings should open in an overlay with its own Save, not rely on the Settings save button. Daemon settings are edited inline in the Settings page and only take effect after the Settings save button on the right is clicked; it is easy to edit a daemon and leave without saving, and the button is far from the daemon being edited. Expected: adding, editing and viewing details of a daemon open in an overlay (dialog) instead of inline; the overlay has its own Save and Cancel, and saving applies that daemon's settings immediately; Cancel or dismiss discards the edits; daemon settings no longer depend on the page-level Settings save button. Related: #890, #1091 (Settings accordions), #868."

## Context: what exists today

- Settings > Daemons lists each daemon as a row with actions (Edit, Details / Hide details, Remove,
  and per-kind ones). Add and Edit open the **daemon form** *inline* in the page, below or among the
  rows. The form already ends in its own **Save daemon** and **Cancel** buttons, and saving it is
  already separate from the page save.
- What the issue reports is therefore the inline placement: an open inline form can be left half
  edited and forgotten while the user's attention is on the page-level Save at the right, and
  nothing marks the form as modal work. This feature moves the form into a modal overlay; it does
  not invent a second storage path.
- **Details** is a per-row toggle that expands, inline, a read-only block: the runtime-missing
  caution, the daemon's connection errors, and its log tail. It is live: the state, errors and log
  change while it is open.
- Saving an edit to a running daemon's runtime settings, removing a daemon, and the Kubernetes drift
  and stop prompts each go through a counted confirmation before they act.

## Terms

- **Daemon form**: the fields that add or edit one daemon, in the titled sections feature 890
  delivered.
- **Daemon overlay**: the modal dialog this feature opens to hold the daemon form, or the details.
- **Page save**: the Settings page's own Save button, for the page-level settings.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Edit a daemon in a dialog (Priority: P1)

A user opens a daemon's Edit action. A modal dialog opens over the page with that daemon's form
(stored values filled in), a Save daemon button and a Cancel button. The user changes a field and
presses Save daemon: the settings are validated and applied (after any confirmation the daemon needs,
see US5), the dialog closes and the list shows the new values, with no use of the page save.

**Why this priority**: it is the heart of the issue: editing is a bounded, modal task with its
confirming button next to the fields.

**Independent Test**: open Edit on a daemon, change its name, press Save daemon; the list shows the
new name and the stored settings hold it after a restart; the page save was never touched.

**Acceptance Scenarios**:

1. **Given** Settings > Daemons lists a daemon, **When** the user opens Edit, **Then** a modal dialog
   opens with the daemon's form populated from its stored values, and no daemon form is shown inline
   in the page.
2. **Given** the dialog is open with a valid change and the daemon needs no confirmation, **When** the
   user presses Save daemon, **Then** the change is stored, the daemon's connection is updated as the
   inline form did, the dialog closes and the list shows the new value.
3. **Given** the dialog is open with an invalid or empty required field, **When** the user presses
   Save daemon, **Then** nothing is stored, the dialog stays open and the error is shown against the
   field.
4. **Given** a daemon saved from the dialog, **When** the user leaves Settings without pressing the
   page save, **Then** the daemon's change is still in effect.

---

### User Story 2 - Cancel or dismiss discards the edits (Priority: P1)

The user changes fields in the dialog, then presses Cancel, presses Escape, or clicks outside it. The
dialog closes and nothing is stored; reopening shows the stored values.

**Why this priority**: without it the dialog would trap the user; it is the other half of the issue's
contract.

**Independent Test**: edit a field, then dismiss by each of the three ways in turn: the list and the
reopened form show the old value every time.

**Acceptance Scenarios**:

1. **Given** a dialog with an edited field, **When** the user presses Cancel, **Then** the dialog
   closes and the stored settings are unchanged.
2. **Given** the same state, **When** the user presses Escape or clicks outside the dialog, **Then**
   the result is the same as Cancel.
3. **Given** a dismissed dialog, **When** it is reopened, **Then** it shows the stored values, not the
   discarded edits.

---

### User Story 3 - Add a daemon from a dialog (Priority: P2)

The user chooses to add a daemon. The same dialog opens with a blank form (kind selectable). Save
daemon validates and stores the new daemon at once; Cancel or dismiss adds nothing.

**Why this priority**: same mechanism as edit, on the creation path.

**Independent Test**: add a daemon through the dialog and save: it is listed and stored without the
page save; repeat and cancel: no new entry.

**Acceptance Scenarios**:

1. **Given** Settings > Daemons, **When** the user chooses to add a daemon, **Then** the dialog opens
   with a blank form.
2. **Given** a valid new-daemon form, **When** the user presses Save daemon, **Then** the daemon is
   stored and listed and the dialog closes.
3. **Given** a filled new-daemon form, **When** the user dismisses it, **Then** no daemon is added.

---

### User Story 4 - Details in the same kind of dialog (Priority: P3)

The user opens a daemon's Details. A modal dialog shows the same read-only content as the inline
block does today (runtime-missing caution, connection errors, log tail) with a Close button; it
replaces the per-row Details / Hide details toggle. Its content keeps updating while it is open.

**Why this priority**: completes "add / edit / details"; read-only and rarely used.

**Independent Test**: open Details on a daemon with a log, see the errors and log; let a new log line
arrive and see it appear; close it: nothing stored, no inline block on the page.

**Acceptance Scenarios**:

1. **Given** a daemon row, **When** the user opens Details, **Then** a modal dialog shows the
   caution, errors (or "No connection errors") and log (or "Nothing logged yet"), and the row has no
   inline details block or Hide details toggle.
2. **Given** the Details dialog is open, **When** the daemon logs a line or its errors change,
   **Then** the dialog shows it without being reopened.
3. **Given** the Details dialog, **When** the user presses Close, Escape or clicks outside, **Then**
   it closes and nothing is stored.

---

### User Story 5 - Confirmations still apply (Priority: P2)

Saving an edit to a running daemon's runtime settings still asks the counted confirmation before it
acts. The confirmation is shown inside the dialog (replacing its buttons' row, or as a panel in it),
not as a second modal. Answering no returns to the form with the edits kept; answering yes stores the
change and closes the dialog. Escape while the confirmation is shown answers no.

**Why this priority**: the existing safety behaviour must not be lost by the move.

**Independent Test**: edit a running daemon's runtime setting, press Save daemon: the confirmation
appears in the dialog; No keeps the edits, Yes applies and closes.

**Acceptance Scenarios**:

1. **Given** an edit to a running daemon's runtime settings, **When** the user presses Save daemon,
   **Then** the confirmation is shown, nothing is stored yet and the dialog stays open.
2. **Given** the confirmation, **When** the user answers no (or presses Escape), **Then** the form is
   shown again with the edits intact and nothing is stored.
3. **Given** the confirmation, **When** the user answers yes, **Then** the change is stored and the
   dialog closes.

---

### Edge Cases

- Save with a validation error: the dialog stays open with the error shown; other daemons and the
  page-level settings are untouched.
- Page save independence (regression guard): saving from the dialog never enables, requires or
  performs the page save, and a page save never stores daemon edits; the page's own pending edits
  survive a daemon save or cancel unchanged.
- The stored registry changes while the dialog is open (another window of the same project saves or
  removes this daemon; Principle II): the dialog keeps the user's draft untouched. On Save, if the
  daemon no longer exists, the user is told and nothing is stored; if it exists, the last save wins,
  as it does today.
- The daemon's connection state changes while an edit dialog is open: the form is unaffected; the
  running / not-running decision for the confirmation is taken at Save.
- Removal stays a row action with its present confirmation; it is not part of the dialog.
- A keyboard-only user: focus moves into the dialog on open, Tab cycles inside it, Escape dismisses
  it, and focus returns to the control that opened it on close.
- A narrow or short window: the dialog fits the window and its form scrolls inside it; Save daemon and
  Cancel stay visible and operable.
- Behaviour is the same on Linux, macOS and Windows; no platform-specific shortcut (Principle VI).
- While the dialog is open the page behind takes no input, so the user cannot open a second dialog.
- A system-raised prompt (Kubernetes drift or stop) that arrives while the overlay is open waits,
  queued, until the overlay closes; it never discards or alters the draft.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Adding a daemon and editing a daemon MUST open the daemon form in a modal overlay over
  the Settings page; the form MUST NOT appear inline in the page.
- **FR-002**: The overlay MUST offer Save daemon and Cancel buttons, reachable by pointer and keyboard.
- **FR-003**: Save daemon MUST validate with the rules the inline form used; on success, after any
  confirmation the change needs (FR-008), it MUST store that daemon's settings at once, close the
  overlay and apply the change to the daemon's connection as before.
- **FR-004**: On a validation error Save daemon MUST store nothing, keep the overlay open and show the
  error against the field.
- **FR-005**: Cancel, Escape and an outside click MUST close the overlay and discard every edit;
  reopening MUST show the stored values.
- **FR-006**: The overlay MUST be modal: the page behind takes no input while it is open.
- **FR-007** (regression guard): Daemon settings MUST remain independent of the page save: it MUST NOT
  store, validate or count daemon edits, and saving a daemon MUST NOT mark the page as having unsaved
  changes or alter its pending edits.
- **FR-008**: Where a save needs the counted confirmation (a running daemon's runtime settings), the
  confirmation MUST be shown inside the overlay; answering no (or Escape) MUST return to the form with
  the edits kept, answering yes MUST store and close.
- **FR-009**: The overlay MUST keep keyboard focus inside it while open and return focus to the
  opening control on close.
- **FR-010**: The overlay's form MUST keep the titled sections, fields and per-kind variants
  (container, Kubernetes, SSH, host) of today's form; this feature changes where the form is shown,
  not what it contains.
- **FR-011**: The overlay MUST fit the window and scroll its form when taller, keeping Save daemon and
  Cancel visible.
- **FR-012**: Opening Details MUST show a modal overlay with the runtime-missing caution, the
  connection errors and the log tail that the inline block shows today, and a Close button; it MUST
  replace the Details / Hide details toggle and keep updating while open; closing it stores nothing.
- **FR-013**: Removal, the row status display and the other row actions MUST keep working as today;
  removal keeps its own confirmation and is not a dialog action.
- **FR-014**: If the stored daemon registry changes while the overlay is open, the overlay's draft MUST
  NOT be replaced or discarded by that change.

### Key Entities

- **Daemon configuration**: one daemon's stored settings, stored by the overlay's Save daemon.
- **Daemon overlay draft**: the unsaved edits held while the overlay is open; discarded on Cancel or
  dismiss, replaced by the stored configuration on a successful save.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can change one daemon field and have it in effect with two actions (open Edit,
  press Save daemon) and no use of the page save, for a daemon needing no confirmation.
- **SC-002**: In 100% of cancel paths (Cancel, Escape, outside click) no daemon setting changes.
- **SC-003**: After leaving Settings without pressing the page save, 100% of daemons saved from the
  dialog keep their values after a restart.
- **SC-004**: The page save's enabled state depends on page-level edits alone: a daemon edit in
  progress or just saved never changes it.
- **SC-005**: At the smallest supported window size, Save daemon and Cancel remain visible and
  operable.
- **SC-006**: With the overlay open, 0 daemon form fields or Details blocks are shown inline.

## Assumptions

- The daemon form's content, validation rules, section layout (feature 890), the counted
  confirmations' wording and counts, and the removal flow are unchanged.
- The app's existing dialogs (overlay registry) give a scrim, Escape routed to the topmost surface and
  modality. Outside-click dismissal and focus return are required by FR-005 and FR-009 for this
  overlay; if the existing dialog pattern lacks them, building them is part of this feature, and a
  large scrolling form (unlike the small confirm dialogs) is accommodated by FR-011.
- The Settings page keeps its own save for the page-level settings (GitHub tokens and others); out of
  scope.
- Dismissing with unsaved edits asks no confirmation: the issue says Cancel or dismiss discards.
- Out of scope: the Settings accordions of #1091, and moving any non-daemon setting into a dialog.
