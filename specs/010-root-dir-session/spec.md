# Feature Specification: Start a Session in the Project Root Directory

**Feature Branch**: `[010-root-dir-session]`

**Created**: 2026-07-18

**Status**: Draft

**Input**: User description: "Add the ability to start a session directly in the project's root directory, without it being bound to a git worktree.

Today, every session is created inside a worktree: a session always has a working directory under `.claude/worktrees/<name>`, and the only way to open a session is to first create (or pick an existing) worktree. There is no way to open a session whose working directory is the root of the project itself.

We need this because some work is not tied to any particular branch or isolated worktree — e.g. quick one-off commands, inspecting/running the project as it currently sits on its checked-out branch, or work the user deliberately does not want isolated into a throwaway worktree. In these cases, forcing worktree creation first is unnecessary overhead and sometimes actively wrong (it creates a branch/worktree nobody wants).

Desired capability:
- From the sidebar, a user can start a new session whose working directory is the project's root directory (the same directory that was opened as the project), as an alternative to starting a session inside a worktree.
- This 'root' session behaves like any other session for the rest of its lifecycle (runs a shell/terminal in that directory, appears in the session list, can be closed/reopened, etc.) — the only difference is which directory it runs in and that it is not associated with any worktree.
- A project can have multiple root sessions open at once, the same way a worktree can have multiple sessions.
- The root directory should be visually distinguishable from worktrees in the sidebar (it is not itself a worktree and should not be confused with one), but should be just as easy to start a session from.
- Existing worktree-bound sessions and the existing 'add worktree' flow are unaffected — this is an additional way to start a session, not a replacement.

Out of scope: changing how worktrees themselves are created/removed, and changing git behavior — this is purely about giving sessions an additional valid 'location' (the project root) alongside worktrees."

## Clarifications

### Session 2026-07-18

- Q: The project constitution's Principle III currently states "every session MUST map to a git worktree," which this feature directly needs to break. How should the spec handle this conflict? → A: Note in the spec that shipping this feature requires amending constitution Principle III first (to explicitly carve out project-root sessions as a non-worktree exception), and treat that amendment as a prerequisite before `/speckit-plan`.
- Q: What should the project root's sidebar entry point be labeled/named, to distinguish it from named worktree entries? → A: "Default" — the project root entry is presented in the sidebar's worktree list labeled as "Default".
- Q: How should users see where each entry (worktree or Default) actually lives? → A: Every entry in the sidebar's worktree list, including the Default entry, shows a tooltip on hover with that entry's location expressed relative to the project.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a session in the project root without creating a worktree (Priority: P1)

A user has a project open and wants to run a quick command, inspect the project as it currently sits on its checked-out branch, or do work they don't want isolated in a throwaway worktree. Today they are forced to create a worktree first just to get a session. Instead, they start a session directly against the project root from the sidebar.

**Why this priority**: This is the entire point of the feature — without it, there is no new capability. It must work end-to-end to deliver any value.

**Independent Test**: Can be fully tested by opening a project, starting a session against the project root (without creating any worktree), and confirming the session's shell runs commands in the project's root directory rather than under a worktree path.

**Acceptance Scenarios**:

1. **Given** a project is open with no worktrees created yet, **When** the user starts a session in the project root, **Then** a session opens with its working directory set to the project root, and no worktree is created as a side effect.
2. **Given** a project already has one or more worktrees with their own sessions, **When** the user starts a session in the project root, **Then** the new session runs in the project root directory, independent of and unaffected by the existing worktree sessions.
3. **Given** a root session is running, **When** the user runs a command that reads or writes files, **Then** the command operates on the project's root directory contents (i.e., whatever is currently checked out there), not on any worktree's contents.

---

### User Story 2 - Distinguish root sessions from worktree sessions in the sidebar (Priority: P2)

A user viewing the sidebar needs to immediately tell which sessions are running in the project root versus which are running inside a specific worktree, so they don't confuse the two or accidentally treat the root as an isolated branch workspace. They also need to see exactly where any entry (the project root or a given worktree) lives without having to start a session first.

**Why this priority**: Without a clear visual distinction, users could mistake root sessions for worktree sessions (or vice versa), leading to confusion about what branch/state their commands are running against. This is important for usability but the feature is still usable without it if the entry points are otherwise clearly labeled.

**Independent Test**: Can be fully tested by opening the sidebar with both a root session and a worktree session present, and confirming a user can identify which is which without opening either session, and can see each entry's location by hovering over it.

**Acceptance Scenarios**:

1. **Given** the sidebar shows both worktrees and the project root as places to start sessions, **When** the user looks at the sidebar, **Then** the project root entry is labeled "Default" and is visually distinct from worktree entries, not labeled or styled as a worktree.
2. **Given** a root session and a worktree session are both open, **When** the user views the session list, **Then** each session's entry indicates whether it belongs to the "Default" project-root entry or to a specific named worktree.
3. **Given** the sidebar lists the "Default" entry alongside named worktree entries, **When** the user hovers over any entry, **Then** a tooltip appears showing that entry's location relative to the project (e.g., the project root itself for "Default", or the relative path to that worktree's directory).

---

### User Story 3 - Run multiple concurrent root sessions (Priority: P3)

A user wants more than one session running in the project root at the same time (e.g., one for running tests, one for editing/inspecting), the same way they can already run multiple sessions in a single worktree.

**Why this priority**: This is a natural extension of parity with worktree sessions and is expected by users once single root sessions work, but it is not required for the core capability to deliver value.

**Independent Test**: Can be fully tested by starting two root sessions for the same project and confirming both remain open and independently usable at the same time.

**Acceptance Scenarios**:

1. **Given** a root session is already open for a project, **When** the user starts another session in the project root, **Then** a second, independent root session opens alongside the first without closing or interfering with it.

---

### Edge Cases

- What happens when the user tries to start a root session before any project is open? The action should not be available / should be a no-op, consistent with how worktree-session actions behave when no project is open.
- What happens when the project is closed and reopened (or the app restarts)? Root sessions should be restored the same way existing worktree sessions are restored, so users don't lose in-progress root session work.
- What happens when the currently checked-out branch in the project root changes (e.g., via a root session running `git checkout`) while other root or worktree sessions are open? All root sessions share the single project-root checkout, so a branch change in one root session's shell affects the working directory contents seen by all other root sessions for that project — this is expected behavior of working directly in the root, not a defect, and should not be silently prevented.
- What happens when a root session is closed? It should be removed from the session list the same way a worktree session is, without affecting the project root directory itself or any worktrees.
- What happens if the project root directory becomes unavailable (e.g., deleted or unmounted) while a root session is running? The session should surface a failure/disconnected state consistent with how a worktree session behaves if its directory disappears.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST allow a user to start a new session whose working directory is the project's root directory, directly from the sidebar, without first creating a worktree.
- **FR-002**: Starting a session in the project root MUST NOT create, modify, or remove any git worktree.
- **FR-003**: A root session MUST run in the exact directory the project was opened from, reflecting whatever branch/state is currently checked out there.
- **FR-004**: The system MUST allow a project to have multiple concurrent root sessions open at the same time, independent of one another.
- **FR-005**: A root session MUST support the same session lifecycle actions available to worktree sessions (open, run commands, close, reopen/restore) except that it has no associated worktree.
- **FR-006**: The sidebar MUST present a clearly distinct entry point for starting a session in the project root, separate from and not styled as a worktree entry, labeled "Default".
- **FR-007**: The system MUST visually indicate, wherever sessions are listed **in the sidebar**, whether a given session belongs to the "Default" project-root entry or to a specific worktree. (Aggregate, project-level indicators that do not list individual sessions — e.g. a running-session count badge — are out of scope for this per-session distinction.)
- **FR-008**: Existing worktree creation, worktree removal, and worktree-bound session flows MUST continue to work unchanged after this feature is added.
- **FR-009**: Root sessions MUST persist across application restarts in the same manner as existing worktree-bound sessions.
- **FR-010**: The sidebar MUST show, on hover, a tooltip for every entry in the worktree/session location list — including the "Default" entry — displaying that entry's location expressed relative to the project (e.g., the project root itself for "Default", or the relative directory path for a worktree).
- **FR-011**: The "Default" entry MUST NOT be affected by the sidebar's tag-filter panel (feature 009) — it MUST remain visible regardless of which tag filters are active, since type/issue/status tags are derived from worktree branch naming and do not apply to the project root.

### Key Entities

- **Default Session** (formerly described as "Root Session" during drafting): A session whose working directory is the project's own root directory rather than a worktree directory. Belongs to a project, not to any worktree. Otherwise shares the same lifecycle, listing, and interaction behavior as a worktree-bound session. Presented in the sidebar under the label "Default" to distinguish it from named worktree entries.
- **Project**: The existing top-level entity representing an opened codebase; already has a known root directory. This feature adds the project root — presented as the "Default" entry — as a valid "location" that sessions can run in, alongside its worktrees.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can go from "project open" to "a session running in the project root" in a single action from the sidebar, with no intermediate worktree-creation step.
- **SC-002**: 100% of commands run in a root session operate against the project's actual root directory contents, verified by file operations in the session matching the root directory rather than any worktree path.
- **SC-003**: Users can correctly identify, without opening a session, whether it is a root session or a worktree session, in a sidebar containing both types.
- **SC-004**: After this feature ships, all existing worktree-creation and worktree-session workflows continue to pass their existing acceptance checks with zero regressions.
- **SC-005**: A project supports at least two simultaneous root sessions with no observed interference between them (e.g., closing one does not affect the other).
- **SC-006**: Users can determine the on-disk location of any sidebar entry — the "Default" entry or any worktree — relative to the project, without leaving the sidebar or starting a session.

## Assumptions

- Root sessions are scoped per project, the same way worktree sessions already are — closing or switching the active project hides that project's root sessions the same way it hides its worktree sessions.
- Root sessions persist and restore across application restarts using the same persistence mechanism already used for worktree-bound sessions, since users would otherwise lose root session work every time the app restarts.
- The project root directory is guaranteed to already exist on disk whenever a project is open (this is an existing precondition of opening a project), so no additional directory-creation step is needed to support root sessions.
- Root sessions run with the same permissions and command execution model as worktree sessions; no new security or sandboxing model is introduced by this feature.
- Because a root session shares the project's single checkout (unlike an isolated worktree), users are expected to understand that actions affecting the working tree (e.g., changing branches) are visible to all root sessions of that project — this is treated as inherent to working in the root, not something the feature needs to prevent.
- **Dependency**: This feature directly conflicts with the current wording of constitution Principle III ("Native Worktree Integration"), which states every session MUST map to a git worktree. Before this feature proceeds to `/speckit-plan`, Principle III MUST be amended (via `/speckit-constitution`) to explicitly carve out project-root sessions as a sanctioned non-worktree exception. This spec is written assuming that amendment will be made; it is a blocking prerequisite, not an implementation detail.
