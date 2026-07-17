# Feature Specification: Copy Worktree Name to Clipboard

**Feature Branch**: `fix/copy-paste-for-all-inputs`

**Created**: 2026-07-17

**Status**: Implemented (retrofitted specification)

**Input**: User description: "Cross-application clipboard support for non-editable labels in the
sidebar. Users can right-click a worktree row and choose \"Copy name\" to copy its displayed name
to the system clipboard, so it can be pasted into any other application (browser, chat, terminal,
etc.). This closes the gap where text inputs (rename dialogs, worktree form, settings) already
support native OS copy/paste via Ctrl+C/V/X/A, and the embedded terminal already supports
Ctrl+Shift+C/V, but read-only labels like worktree names had no way to be copied at all since the
UI framework's plain text labels are not selectable."

> **Note on process**: this specification was written after the feature was implemented and
> merged on the branch above, to bring the change under this repository's spec-first process
> retroactively. The description, requirements, and acceptance scenarios below describe the
> shipped behavior; they were not used to drive the original implementation.

## Clarifications

### Session 2026-07-17

- Q: Should "Copy name" be exposed anywhere beyond the worktree right-click menu (e.g. session
  rows, known-projects list) in this change? → A: No — scope this change to the worktree context
  menu only (the concrete case the user reported). Extending the same pattern to session titles
  and project names is a natural follow-up but out of scope here (see Assumptions).
- Q: Which text is copied — the worktree's raw directory/branch name or its friendly displayed
  name (the one shown on the sidebar row, honoring any rename override)? → A: The friendly
  displayed name, i.e. exactly the text the user sees on the row and would have tried to
  select — including a custom rename if one is set.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Copy a worktree's name for use elsewhere (Priority: P1)

A developer wants to reference a worktree's name outside the app — for example, pasting it into
a chat message to a teammate, a commit message, or a search box in another tool. Today, sidebar
labels are plain, unselectable text: there is no way to drag-select or copy them, unlike a normal
text field. Right-clicking the worktree row now offers a **Copy name** action that copies exactly
the name shown on the row to the system clipboard, ready to paste into any other application.

**Why this priority**: This is the entire scope of the change and the concrete gap that was
reported — without it, worktree names are trapped inside the app with no way out except manually
retyping them.

**Independent Test**: Right-click a worktree row (with and without a custom rename applied),
choose "Copy name", then paste into another application's text field. Verify the pasted text
exactly matches the row's displayed name.

**Acceptance Scenarios**:

1. **Given** a worktree row showing the derived friendly name "Login page", **When** the user
   right-clicks it and chooses **Copy name**, **Then** the system clipboard contains exactly
   "Login page", pasteable into any other application.
2. **Given** a worktree that has been renamed (a custom display-name override is set), **When**
   the user copies its name, **Then** the clipboard contains the custom name, not the name
   derived from the branch/directory.
3. **Given** the worktree context menu is open, **When** the user chooses **Copy name**, **Then**
   the menu closes immediately afterward, matching the behavior of its other actions (Rename,
   Delete).
4. **Given** the worktree context menu is open, **When** the user looks at its entries, **Then**
   **Copy name** carries its own distinct icon, just like Rename and Delete do.

---

### Edge Cases

- What happens when the worktree's name is very long? The full displayed text is copied
  verbatim; the clipboard has no length limit imposed by this feature.
- What happens if the system clipboard is unavailable or the write fails? The action is
  best-effort; there is no in-app error surfaced, consistent with the existing terminal
  copy/paste actions in this application.
- What happens if the user copies a name, then immediately copies a different worktree's name?
  The clipboard holds only the most recent copy, same as any standard copy action.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The worktree right-click context menu MUST offer a **Copy name** action, alongside
  its existing Rename and Delete actions.
- **FR-002**: Choosing **Copy name** MUST place the worktree's current displayed name — the
  custom rename override if one is set, otherwise the name derived from its branch/directory —
  onto the system clipboard, exactly as shown on the sidebar row.
- **FR-003**: The copied text MUST be available to any other application on the system via a
  standard paste action, matching normal operating-system clipboard behavior.
- **FR-004**: Choosing **Copy name** MUST close the worktree context menu, consistent with its
  other actions.
- **FR-005**: The Copy name action MUST be visually distinguished by its own icon, consistent
  with every other context-menu action in the application.

### Key Entities

- **Worktree displayed name**: the human-friendly label shown on a worktree's sidebar row — a
  custom rename if the user has set one, otherwise a name derived from its branch/directory. This
  is the exact text copied by this feature.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can copy any worktree's displayed name and paste it into another
  application in two actions (right-click, then choose Copy name) with no retyping.
- **SC-002**: The pasted text matches the sidebar's displayed name character-for-character, 100%
  of the time, including when a custom rename is set.
- **SC-003**: The copy action is discoverable without documentation — it appears in the same
  context menu developers already use for Rename and Delete.

## Assumptions

- Text already entered into editable fields (rename dialogs, the worktree-creation form, the
  settings form) is out of scope: those are standard editable text inputs and already support
  native OS copy/paste/cut/select-all (Ctrl+C/V/X/A) without any change, because the underlying
  UI toolkit provides this for editable text fields for free.
- The embedded terminal's own copy/paste (Ctrl+Shift+C/V, and its existing right-click Copy/Paste
  menu) is a separate, already-existing feature and is unaffected by this change.
- Only the worktree context menu is in scope for this change. Other read-only labels in the
  application (session titles, known-project names in the project switcher) have the same
  underlying limitation and would benefit from the same pattern, but extending it to them is
  deferred to a follow-up rather than bundled here, keeping this change scoped to the reported
  case.
- Clipboard-write failures (e.g. no clipboard service available on an unusual Linux setup) are
  treated as best-effort with no in-app error message, matching the existing terminal
  copy/paste actions.
