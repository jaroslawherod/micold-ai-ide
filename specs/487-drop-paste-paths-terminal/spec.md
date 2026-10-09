# Feature Specification: Drop Files or Paste Images into a Terminal to Insert Their Paths

**Feature Branch**: `487-drop-paste-paths-terminal`

**Created**: 2026-10-09

**Status**: Draft

**Input**: GitHub issue #487, "Drop files or paste images into a session terminal to insert their
paths": handing a file or screenshot to an AI session means typing or pasting its path by hand. The
user wants dropped files to insert their shell-quoted paths, a pasted clipboard image to be saved to
a temporary file whose path is inserted, and, for the sandboxed runtime, paths translated to what the
container sees, with files outside the registered projects refused with a clear message.

## Terms

- **Terminal**: an AI session's CLI or one of its regular terminals, as shown in the terminal area.
- **Insertion**: text placed at the terminal's input as if typed, **without** a trailing newline, so
  nothing runs until the user presses Enter.
- **Sandboxed runtime**: a session whose terminal runs inside a container, where host paths differ
  from the paths the process sees.
- **Registered project**: a project the app knows about; its directory and worktrees are the only
  host locations a sandboxed session can see.
- **Session temp directory**: a per-session location, owned by the app, for files it creates on the
  user's behalf (here, pasted images).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Drop files to insert their paths (Priority: P1)

A developer drags one or more files from the file manager onto a terminal and the paths appear at
the prompt, ready to be completed and sent.

**Why this priority**: It is the core of the issue and works for every terminal kind.

**Independent Test**: Drop two files, one with a space in its name, onto a regular terminal and
check the input line holds both quoted paths separated by a space and that nothing has executed.

**Acceptance Scenarios**:

1. **Given** a terminal at an empty prompt, **When** the user drops one file, **Then** its absolute
   path, quoted for the shell, appears at the input and no newline is sent.
2. **Given** a terminal, **When** the user drops three files at once, **Then** their quoted paths
   appear in drop order, separated by single spaces.
3. **Given** a file named `my file's (1).png`, **When** it is dropped, **Then** the inserted text,
   when run by the shell, names exactly that file.
4. **Given** text already typed at the prompt, **When** files are dropped, **Then** the paths are
   added after it and the existing text is untouched.
5. **Given** several panes (feature 484), **When** files are dropped on one pane, **Then** only that
   pane's terminal receives them, whether or not it was focused.

---

### User Story 2 - Paste a clipboard image into an AI session (Priority: P1)

A developer takes a screenshot, pastes it into an AI session, and the session gets a file path it
can read.

**Why this priority**: Screenshots are the commonest thing handed to an agent and there is no path to
copy.

**Independent Test**: Put an image on the clipboard, paste into an AI session, and check a file
with that image exists at the inserted path, inside the worktree or the session temp directory.

**Acceptance Scenarios**:

1. **Given** an image on the clipboard and an AI session focused, **When** the user pastes, **Then**
   the image is saved as a file and its quoted path is inserted, with no newline.
2. **Given** text on the clipboard, **When** the user pastes, **Then** behaviour is unchanged from
   today.
3. **Given** both text and an image on the clipboard, **When** the user pastes, **Then** the text is
   pasted as today and no file is created (text is always preferred, so a copied spreadsheet range or web image pastes its text).
4. **Given** an image on the clipboard that cannot be read or saved, **When** the user pastes,
   **Then** nothing is inserted and a message states the cause (FR-012).
5. **Given** two pastes of images, **When** both are saved, **Then** they have distinct paths and
   neither overwrites the other.

---

### User Story 3 - Sandboxed sessions get container paths, outside files refused (Priority: P2)

For a sandboxed session, the inserted path is the one the container sees. A file the container cannot
see is not inserted; the user is told why.

**Why this priority**: Without it, the feature inserts paths that do not exist for sandboxed agents.

**Independent Test**: In a sandboxed session, drop a file from a registered project and check the
container-side path is inserted; drop a file from the home directory outside any project and check
nothing is inserted and a message names the file and the reason.

**Acceptance Scenarios**:

1. **Given** a sandboxed session and a file inside a registered project, **When** it is dropped,
   **Then** the inserted path is the container-visible path of that file.
2. **Given** a sandboxed session and a file outside every registered project, **When** it is
   dropped, **Then** nothing is inserted for it and a message states the file is outside the
   registered projects and not visible to the sandbox.
3. **Given** a drop of three files, one outside the projects, **When** dropped in a sandboxed
   session, **Then** the two visible paths are inserted and the message names the refused one.
4. **Given** a pasted image in a sandboxed session, **When** it is saved, **Then** it is saved where
   the container can see it and the container-side path is inserted.
5. **Given** a sandboxed session whose worktree is not writable, **When** an image is pasted,
   **Then** the session temp directory is made visible to the container and its container-side path
   is inserted; if that is impossible, nothing is inserted and a message says why.

---

### User Story 4 - Temporary images are cleaned up (Priority: P2)

Pasted images do not accumulate on disk after the session they belonged to is gone.

**Why this priority**: Screenshots can be large and sensitive; leaving them is a leak and clutter.

**Independent Test**: Paste an image into a session, delete the session, and check the file is gone
and no file the user dropped is touched.

**Acceptance Scenarios**:

1. **Given** a session with pasted images, **When** the session is deleted, **Then** all its pasted
   images are removed.
2. **Given** a pasted image saved inside the worktree, **When** the user inspects version control
   status, **Then** the image does not show up as an untracked change.
3. **Given** a file the user dropped, **When** the session is deleted, **Then** that file is never
   deleted.
4. **Given** pasted-image files whose session no longer exists (e.g. after a crash), **When** the app
   next starts, **Then** those files are removed.

---

### Edge Cases

- Drop of a directory: its path is inserted like a file's.
- Drop of zero usable paths (e.g. a drop carrying no files): nothing is inserted, no error.
- Path containing a newline, tab, quote, `$`, backtick, backslash, non-ASCII or leading `-`: quoted
  so the shell reads it as one literal argument.
- Dropped file that no longer exists by the time it is processed: still inserted for a regular
  session (the user may be dropping a path to create), refused with a message for sandboxed only if
  outside the projects.
- Clipboard image too large, unreadable, or disk full / directory not writable: nothing is inserted
  and a message explains the failure; the session is not disturbed.
- Terminal whose process has exited, or a pane showing the empty-pane state: drop and paste do
  nothing and say why.
- Two sessions pasting at the same time: each writes to its own session's location; no collision.
- Paste into a regular (non-AI) terminal: unchanged; only AI sessions convert a pasted image to a path (the issue scopes the feature to AI sessions).
- Quoting differs by shell and platform (POSIX shells, Windows shells): the inserted text must be
  correct for the shell the terminal runs (Principle VI).
- Session without a worktree (the project-root Default session): images go to the session temp
  directory only; the same sandbox and cleanup rules apply.
- Symlink dropped into a sandboxed session: judged by where it resolves; one pointing outside the
  registered projects is refused.
- Non-UTF-8 file names and Windows drive-letter or UNC paths: quoted correctly in the shell's own syntax when
  representable; a path the shell cannot represent without loss is refused with a message naming it;
  never translated between path styles and never inserted garbled.
- Worktree is on a read-only or network location: falls back to the session temp directory.

## Clarifications

### Session 2026-10-09

- Q: Text and image both on the clipboard? → A: Text always wins, no file created. _(agent-resolved: spec.md#User Story 2, matches today's paste behaviour)_
- Q: Do regular terminals convert a pasted image? → A: No, AI sessions only. _(agent-resolved: issue #487 scope)_
- Q: Non-UTF-8 / Windows paths: translated or refused? → A: Quoted natively when representable, else refused with a message; no translation. _(agent-resolved: spec.md#FR-002, never garbled)_

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Dropping one or more files or directories onto a terminal MUST insert their absolute
  paths at that terminal's input, in drop order, separated by single spaces.
- **FR-002**: Every inserted path MUST be quoted so the terminal's shell reads each as exactly one
  literal argument, for any characters in the name, on every supported platform and for each shell the app offers for terminals (POSIX shells such as sh, bash, zsh and fish; PowerShell and cmd on Windows).
- **FR-003**: Insertion MUST NOT send a newline or any submit key; the terminal runs nothing until
  the user presses Enter.
- **FR-004**: Insertion MUST preserve any text already at the input and place the paths after it.
- **FR-005**: A drop MUST be delivered to the terminal it landed on, not to the focused pane.
- **FR-006**: Pasting while the clipboard holds an image and no text, into an AI session, MUST save
  the image to a new file and insert that file's quoted path.
- **FR-007**: A pasted image MUST be saved inside the session's worktree, or its session temp
  directory when the session has no worktree or the worktree is not writable, under a name that cannot collide with another paste.
- **FR-008**: Pasted-image files saved inside a worktree MUST NOT appear as untracked changes in
  that worktree's version control status.
- **FR-009**: Pasting text MUST behave exactly as before this feature.
- **FR-010**: In a sandboxed session, every inserted path MUST be translated to the path the
  container sees; pasted images MUST be saved at a location the container can see.
- **FR-011**: In a sandboxed session, a dropped path outside every registered project MUST be
  refused: not inserted, and the user shown a message naming the file and the reason. Other paths in
  the same drop are still inserted.
- **FR-012**: Failures (unreadable clipboard image, write failure, exited process) MUST insert
  nothing and show a message stating the cause.
- **FR-013**: When a session is deleted, its pasted-image files MUST be removed. Files the user
  dropped MUST never be deleted.
- **FR-014**: Pasted-image files MUST also be removed if the app finds them orphaned after a crash
  (session no longer exists) at next start.

### Key Entities

- **Inserted path**: a host or container path for a file, plus its shell-quoted text form.
- **Pasted image**: a file created from clipboard image data, owned by exactly one session, removed
  with it.
- **Refusal message**: user-visible text naming a rejected path and why.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user hands a file to a session in one gesture (drop) with zero typed characters; the
  path is at the prompt within 1 second of release.
- **SC-002**: A user hands a screenshot to an AI session with one paste and no file-manager step.
- **SC-003**: For each supported shell, in a test set of at least 20 awkward names (spaces, quotes, `$`, backticks, newline,
  non-ASCII, leading dash), 100% round-trip to the exact file when run by the shell.
- **SC-004**: In every drop and paste test, 0 bytes of newline or submit input reach the terminal.
- **SC-005**: After deleting a session, 0 of its pasted-image files remain on disk and 0 dropped
  files are removed.
- **SC-006**: 100% of sandboxed drops of files outside the registered projects are refused with a
  message; 0 unreachable host paths are inserted.

## Assumptions

- Drops come from the operating system's file manager; dragging text or in-app items is out of scope.
- The session temp directory lives under the app's own data location and is per session.
- Drop and paste work in any pane of a split layout (feature 484).
- Images saved into a worktree are excluded from version control status without altering the project's tracked files.
- Out of scope: pasting non-image files from the clipboard, drag-out of paths, image preview or
  thumbnails, size limits beyond reporting write failure.
