# Feature Specification: Structured View — Drop Files and "Add attachment" Button

**Feature Branch**: `feat/1464_structured-view-drag-and-drop-files-into-the`

**Created**: 2026-10-27

**Status**: Draft

**Input**: GitHub issue #1464 "Structured view: drag-and-drop files into the composer and an 'Add attachment' button". Builds on `specs/941-structured-prompt-input` (FR-026 to FR-029), which defines what a drop on a structured pane does but leaves the composer with no way to add a file without dragging.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Drop files anywhere on the structured view (Priority: P1)

A user drags one or more files from the file manager onto a structured session's view, over the transcript or over the composer. Images become attachment chips in that session's composer; other files and folders are inserted into the draft as quoted paths at the caret. Nothing is typed into a terminal.

**Why this priority**: It is the primary, most natural way to give the assistant a file, and it must work reliably on the real desktop before anything else adds value.

**Independent Test**: Drop a PNG and a text file on the transcript area of a structured session; the composer shows one chip and the draft holds the quoted path; sending delivers the image with the message.

**Acceptance Scenarios**:

1. **Given** a structured session that accepts input, **When** the user drops an image onto the transcript, **Then** an attachment chip with its name appears in that session's composer and the image is sent with the next message.
2. **Given** the same session, **When** the user drops a non-image file onto the composer, **Then** its quoted path is inserted at the caret and no chip is added.
3. **Given** several files dropped at once, **When** the drop completes, **Then** all are attached or inserted, in the order they were dropped.
4. **Given** two panes (one structured, one terminal), **When** files are dropped on the structured pane, **Then** only that session's composer changes and no terminal receives input.
5. **Given** a dropped image that is unreadable, too large, or beyond the per-message image limit, **When** the drop completes, **Then** it is not attached and a notification names the file and the reason.

---

### User Story 2 - Drop-target cue while dragging (Priority: P2)

While files are dragged over a structured view, the view shows a visible cue that it will accept them, and the cue disappears when the drag leaves or the files are dropped.

**Why this priority**: It tells the user the drop will land in the composer rather than a terminal, but the drop works without it.

**Independent Test**: Drag files over the structured view and see the cue; move out and see it vanish.

**Acceptance Scenarios**:

1. **Given** files are dragged over a structured view that accepts input, **When** the pointer enters it, **Then** a drop-target cue is visible.
2. **Given** the cue is visible, **When** the drag leaves the view or the files are dropped, **Then** the cue disappears.
3. **Given** a view that cannot accept input, **When** files are dragged over it, **Then** no accepting cue is shown.

---

### User Story 3 - "Add attachment" button with file picker (Priority: P1)

A user clicks an "Add attachment" icon button next to send in the composer row, chooses one or more files in the system file picker, and gets exactly the result a drop of the same files would give.

**Why this priority**: It is the only way to attach files for users who cannot or do not want to drag, and for platforms where drops do not arrive.

**Independent Test**: Click the button, pick an image and a text file; a chip and a quoted path appear exactly as for a drop.

**Acceptance Scenarios**:

1. **Given** a structured session that accepts input, **When** the user clicks the button, **Then** the system file picker opens and allows selecting several files, without freezing the interface.
2. **Given** the picker is open, **When** the user confirms a selection, **Then** the files are admitted exactly as a drop of those files, in the order selected, with the same chips, quoted paths and rejection notifications.
3. **Given** the picker is open, **When** the user cancels, **Then** nothing changes and no notification appears.
4. **Given** the button has a tooltip, **When** the user hovers or focuses it, **Then** the tooltip names the action and its shortcut.
5. **Given** a session that cannot accept input, **When** the composer is shown, **Then** the button is disabled or hidden and cannot open the picker.

---

### User Story 4 - Keyboard access to the button (Priority: P2)

A keyboard user reaches the button with the focus order and triggers it with a shortcut that is listed in the shortcut sheet.

**Why this priority**: Required for accessibility and consistency with the other composer actions.

**Independent Test**: Tab to the button and press Enter or Space, or press the shortcut, and the picker opens; the shortcut sheet lists it.

**Acceptance Scenarios**:

1. **Given** the composer is shown, **When** the user moves focus with the keyboard, **Then** the button is reachable and visibly focused, and activating it opens the picker.
2. **Given** the composer has focus, **When** the user presses the shortcut, **Then** the picker opens.
3. **Given** the shortcut sheet is open, **Then** it lists the shortcut with the action's name.

### Edge Cases

- Files dropped on a structured view whose session is ended or cannot accept input: they are not attached and the user is told why (no silent loss).
- A picker selection or drop mixing images and other files: images become chips and the rest become quoted paths, in selection order.
- The picker is requested while one is already open: a second picker does not open.
- The session is closed or becomes unable to accept input while the picker is open: a late selection is dropped with a notification, not applied to another session.
- The same file chosen twice or dropped twice: handled as by the existing drop rules, identically on both routes.
- An image removed between selection and reading: reported as unreadable, as for a drop; a non-image path is inserted without being read.
- Cross-platform: the shortcut uses each platform's usual modifier; Windows paths with backslashes, drive letters and spaces are quoted as in spec 941; picker look and behaviour differ per platform but results do not.
- Empty selection, zero-file drop or a text drop: nothing changes (FR-016).
- Drops of paths containing spaces, quotes or non-ASCII characters are quoted as in spec 941.
- The window has several structured sessions: a drop or selection goes only to the session whose view received it or whose button was used.

## Requirements *(mandatory)*

### Functional Requirements

**Drop**

- **FR-001**: Files dropped anywhere on a structured session's view (transcript or composer) MUST be added to that session's composer and MUST NOT be sent to any terminal.
- **FR-002**: Multiple files dropped at once MUST all be processed, in the order they were dropped.
- **FR-003**: The behaviour of spec 941 FR-026 to FR-029 MUST be kept: images become attachment chips, other files and folders become quoted path references at the caret, and rejected images (unreadable, too large, over the count limit) produce a notification naming the file and the reason.
- **FR-004**: Drops on a structured view MUST be received, and the picker and its shortcut MUST work, on Linux (Wayland and X11), macOS and Windows. If drops are not received today on any of them, that is a defect fixed as part of this feature.
- **FR-005**: While files are dragged over a structured view that accepts input, the view MUST show a drop-target cue, which MUST disappear when the drag leaves or the drop completes.

**Add attachment button**

- **FR-006**: The composer row MUST contain an "Add attachment" icon button next to the send control, with a tooltip.
- **FR-007**: Activating the button MUST open the system file picker, allowing multiple selection, without blocking the interface.
- **FR-008**: Files chosen in the picker MUST go through the same admission path as a drop, so a given set of files gives identical chips, path references and notifications by either route.
- **FR-009**: Cancelling the picker MUST leave the composer unchanged and show no notification.
- **FR-010**: The button MUST be reachable and operable by keyboard, with a visible focus state, and MUST have a shortcut listed in the shortcut sheet.
- **FR-011**: The button MUST be disabled or hidden, and a drop MUST NOT be admitted, when the session cannot accept input; a refused drop MUST tell the user why.
- **FR-012**: A picker result MUST be applied only to the session that requested it; if that session can no longer accept input, the result MUST NOT be applied and the user MUST be told.
- **FR-015**: While a picker is open for a session, activating the button again MUST NOT open a second picker.
- **FR-016**: A drop or picker confirmation carrying no files, or carrying a non-file payload such as text, MUST leave the composer unchanged and show no notification.

**Verification**

- **FR-013**: Automated tests MUST verify that picker-selected files are admitted as specified and that drop and picker give identical results for the same files.
- **FR-014**: The showcase or visual pass MUST cover the button (normal, hover, focus, disabled) and the drop cue.

### Key Entities

- **Attachment chip**: A removable image attachment in the composer showing the file name and a preview.
- **Path reference**: A quoted file or folder path inserted into the draft text at the caret.
- **Admission result**: The outcome of offering a list of files to the composer: chips added, paths inserted, files rejected with reasons. Identical for drop and picker.
- **Drop-target cue**: The visual state of a structured view while files hover over it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For any fixed set of files, a drop and a picker selection produce the same chips, inserted paths and rejection messages in 100% of the tested cases.
- **SC-002**: A user can attach a file with the button in at most 3 interactions (click, choose, confirm).
- **SC-003**: Dropping files on any part of a structured view reaches the composer in 100% of tested cases on Linux (Wayland and X11), macOS and Windows, with no input reaching a terminal.
- **SC-004**: While the picker is open the interface still responds to input within 100 ms.
- **SC-005**: The drop cue is visible while files hover and is gone by the next rendered frame after the drag leaves or drops.
- **SC-006**: Every rejected image on either route is reported with its name and the reason.

## Assumptions

- The attachment rules of spec 941 (accepted image types, size and count limits, quoting) are unchanged and are reused as is.
- The cue is shown only where the windowing system reports a file hover; where it does not, the drop still works without the cue [NEEDS CLARIFICATION: is a missing cue on a platform that does not report hover acceptable, or must hover be emulated?].
- A disabled button is preferred over a hidden one, so the layout does not shift; the choice may be revisited in planning.
- The shortcut key combination is chosen in planning to avoid clashing with existing composer and shell shortcuts.
- The picker starts in the platform's default location and has no file type filter, since any file may be referenced by path.
- Scope is the structured view only.

## Out of Scope

- Terminal-pane drop behaviour (feature 487) and changes to the image size, type or count limits.
- Attaching files by pasting from the clipboard, and non-image attachment chips.
- Previewing or editing attached files beyond the existing chip.
