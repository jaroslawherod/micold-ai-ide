# Feature Specification: Forget a Project

**Feature Branch**: `feat/forget-the-project`

**Created**: 2026-07-23

**Status**: Draft

**Input**: User description: "the user should be able to \"forget\" the project and remove it from project list"

## Clarifications

### Session 2026-07-23

- Q: When the user forgets a project that has running sessions, what should happen to those sessions? → A: Stop the project's running sessions (end their processes), then remove the entry; worktrees and files are never deleted.
- Q: When forgetting a project that has running sessions, should the confirmation prompt explicitly warn that those sessions will be stopped? → A: Yes — the confirmation states how many running sessions will be stopped, in addition to noting nothing on disk is deleted.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Forget a known project and remove it from the list (Priority: P1)

From the known-projects list, the user chooses to "Forget" a project. Because forgetting
permanently discards the metadata the application remembers about that project (its custom
display name, any per-worktree display-name overrides, and the record of its sessions), the
application asks the user to confirm first, making clear that nothing on disk is deleted —
only the application's remembered entry. On confirmation, the project disappears from the
known-projects list and the removal is persisted immediately, so the project does not
reappear after a restart. The folder itself, its files, and any git worktrees remain
untouched on disk.

**Why this priority**: This is the core of the feature and the smallest slice that delivers
value on its own: a user who has accumulated projects they no longer care about can clean up
the list. Every other scenario (forgetting the active project, forgetting an unavailable one)
is a variation on this same flow. Without it, nothing else in the feature exists.

**Independent Test**: Open two or more projects so they appear in the known-projects list,
invoke "Forget" on one, confirm the prompt, and verify that entry is removed from the list
while the others remain; verify the folder and its contents still exist on disk; restart the
application and verify the forgotten project does not return to the list.

**Acceptance Scenarios**:

1. **Given** a project appears in the known-projects list, **When** the user invokes "Forget"
   on it, **Then** the application asks the user to confirm and states that only the
   remembered entry is removed and nothing on disk is deleted.
2. **Given** the confirmation prompt is shown, **When** the user confirms, **Then** the
   project is removed from the known-projects list and the remaining projects are unchanged.
3. **Given** the confirmation prompt is shown, **When** the user cancels, **Then** nothing
   changes and the project remains in the list.
4. **Given** a project was forgotten, **When** the application is restarted, **Then** the
   forgotten project does not appear in the known-projects list.
5. **Given** a project was forgotten, **When** the user inspects the filesystem, **Then** the
   folder, its files, and any git worktrees are unchanged.

---

### User Story 2 - Forget the currently active project (Priority: P2)

The user forgets the project that is currently the active working space. Because a forgotten
project can no longer be the active working space, after forgetting it there is no active
working space: the application returns to a no-active-project state, still showing any
remaining known projects (or the first-run empty state if none remain). To avoid leaving
live processes running for a project the application no longer tracks, any of that project's
running sessions are stopped as part of forgetting; their git worktrees and files are not
deleted from disk.

**Why this priority**: Forgetting the active project is a natural and expected action, but it
introduces additional consequences (clearing the active working space, stopping running
sessions) beyond the basic list-pruning of Story 1, so it builds on top of Story 1 rather
than being the minimal slice.

**Independent Test**: Make a project active and start a session in it, invoke "Forget" on the
active project, confirm, and verify the project is removed, no project is active afterward,
the remaining projects are still listed (or the empty state is shown if none remain), the
previously running sessions are stopped, and the worktrees/files remain on disk.

**Acceptance Scenarios**:

1. **Given** a project is the active working space, **When** the user forgets it and confirms,
   **Then** the project is removed and there is no active working space afterward.
2. **Given** the active project was the only known project, **When** the user forgets it,
   **Then** the application shows the first-run empty state inviting the user to open a
   project.
3. **Given** the active project has one or more running sessions, **When** the user invokes
   "Forget", **Then** the confirmation prompt states how many running sessions will be stopped;
   **and When** the user confirms, **Then** those sessions are stopped and no session keeps
   running for the forgotten project.
4. **Given** the active project had running sessions in git worktrees, **When** it is
   forgotten, **Then** the worktrees and their files remain on disk (forgetting never deletes
   them).

---

### User Story 3 - Forget an unavailable project (Priority: P3)

A known project whose folder was deleted, moved, or renamed on disk is marked unavailable in
the list. Forgetting is available for unavailable projects too — in fact it is the primary way
to remove a stale entry that can no longer be opened. The same confirmation applies, and the
entry is removed and persisted just as for an available project.

**Why this priority**: Cleaning up unavailable entries is a real and common need, but it is a
straightforward reuse of the Story 1 flow applied to an entry that already cannot be opened,
so it is the lowest-priority slice.

**Independent Test**: Create a known project, remove its folder on disk so it is marked
unavailable, invoke "Forget" on the unavailable entry, confirm, and verify it is removed from
the list and does not reappear after a restart.

**Acceptance Scenarios**:

1. **Given** a known project is marked unavailable, **When** the user invokes "Forget" on it,
   **Then** the confirmation prompt is shown and, on confirmation, the entry is removed.
2. **Given** an unavailable project was forgotten, **When** the application is restarted,
   **Then** it does not reappear in the list.

---

### Edge Cases

- **Forgetting the only project**: After forgetting the last remaining known project, the
  application shows the first-run empty state that invites the user to open a project.
- **Forgetting the active project**: The active working space is cleared; there is no active
  project until the user opens or reopens one.
- **Forgetting a project with running sessions**: The project's running sessions are stopped
  as part of forgetting so no process keeps running for an untracked project; worktrees and
  files are not deleted.
- **Re-opening a forgotten folder**: Opening the same folder again after forgetting it creates
  a fresh entry with the default display name; the previously stored custom name, per-worktree
  name overrides, and session records are gone (forgetting discarded them).
- **Cancelling the confirmation**: No change occurs; the project stays in the list with all its
  metadata intact.
- **Forgetting an unavailable project**: Allowed; the entry is removed even though its folder
  is no longer on disk.

## Requirements *(mandatory)*

### Functional Requirements

#### Forgetting a project

- **FR-001**: The known-projects list MUST offer a "Forget" action for each listed project.
- **FR-002**: Invoking "Forget" MUST prompt the user to confirm before any change is made,
  and the prompt MUST make clear that only the application's remembered entry is removed and
  that nothing on disk (the folder, its files, or any git worktrees) is deleted.
- **FR-002a**: When the project being forgotten has running sessions, the confirmation prompt
  MUST additionally state how many of its running sessions will be stopped. When the project
  has no running sessions, the prompt MUST NOT show a session-stop warning.
- **FR-003**: On confirmation, the system MUST remove the project from the known-projects list.
- **FR-004**: On cancellation, the system MUST make no change: the project remains in the list
  with all of its stored metadata intact.
- **FR-005**: Forgetting a project MUST also discard all application-stored metadata associated
  with that project's path — its custom display name, any per-worktree display-name overrides,
  and its persisted session records — both in memory and on disk (the application's per-project
  stored state for that path MUST be removed, not merely dropped from memory), so that nothing
  the application remembered about the project survives the removal.
- **FR-006**: Forgetting a project MUST NOT rename, move, delete, or otherwise modify the
  folder, its files, or any git worktrees on disk.

#### Persistence

- **FR-007**: The removal MUST be persisted to the local known-projects storage immediately, so
  a forgotten project does not reappear after an application restart.

#### Active working space and sessions

- **FR-008**: If the forgotten project is the active working space, the system MUST clear the
  active working space so that no project is active afterward.
- **FR-009**: When no known projects remain after forgetting, the system MUST present the
  first-run empty state that invites the user to open a project.
- **FR-010**: If the forgotten project has running sessions, the system MUST stop those sessions
  as part of forgetting, so no session keeps running for a project the application no longer
  tracks. Stopping a session MUST NOT delete its git worktree or files on disk.

#### Availability

- **FR-011**: The "Forget" action MUST be available for projects marked unavailable (folder
  deleted, moved, or renamed on disk), and forgetting such a project MUST behave the same as
  forgetting an available one.

#### Re-opening

- **FR-012**: Opening a previously forgotten folder again MUST create a fresh known-project
  entry with the default display name (the folder's name), retaining none of the application's
  previously-stored metadata for that folder — no custom name, no per-worktree overrides, and
  no persisted session metadata (labels, modes, archived flags). The re-opened entry starts from
  a clean slate.
  - **Note (interaction with on-disk conversations)**: Forgetting does not delete the worktrees'
    on-disk `claude` conversations (FR-006). On re-open, the application MAY rediscover those
    on-disk conversations through its normal session-reconciliation — exactly as it would when
    opening any folder that already contains conversations on disk. That rediscovery reflects the
    current on-disk reality, not retention of the forgotten entry's remembered state, and is
    therefore consistent with (not a violation of) this requirement.

#### Cross-platform parity

- **FR-013**: The forget action, its confirmation, persistence of the removal, and the
  associated active-working-space and session behavior MUST behave equivalently on Linux,
  macOS, and Windows.

### Key Entities *(include if data involved)*

- **Known-Projects List**: The locally persisted collection of project records plus the
  last-active pointer. Forgetting removes exactly one record (identified by its filesystem
  path) and, if that record was the active one, clears the active pointer.
- **Project Metadata**: The application-managed data tied to a project's path — its display
  name, per-worktree display-name overrides, and persisted session records. The application
  stores each project's session/override state in its own per-project state file (separate from
  the known-projects catalog). Forgetting a project discards all of this metadata for that path:
  the catalog entry is removed, and the project's own state file is deleted from the
  application's data directory. (This is the application's private data store — distinct from the
  project's folder on disk, which is never touched; see FR-006.)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can remove an unwanted project from the known-projects list in a single
  confirm-and-forget flow, without browsing the filesystem or reading documentation.
- **SC-002**: 100% of forget actions require an explicit confirmation before the project is
  removed; cancelling leaves the list and all metadata unchanged.
- **SC-003**: After forgetting a project and restarting the application, the forgotten project
  reappears 0% of the time.
- **SC-004**: Forgetting a project results in zero modifications to the folder, its files, and
  its git worktrees on disk (verifiable by comparing the on-disk state before and after).
- **SC-005**: When the active project is forgotten, there is no active working space afterward,
  and any sessions that were running for it are stopped (0 sessions remain running for the
  forgotten project).
- **SC-005a**: When a project with running sessions is forgotten, the confirmation prompt shows
  the exact number of sessions that will be stopped, matching the number actually stopped on
  confirmation (0 mismatches).
- **SC-006**: Forgetting the last remaining project shows the first-run empty state 100% of the
  time.
- **SC-007**: Re-opening a previously forgotten folder produces a fresh entry with the default
  display name and none of the prior metadata, every time.
- **SC-008**: Every acceptance scenario in this specification passes identically on Linux,
  macOS, and Windows.

## Assumptions

- **Confirmation required**: Because forgetting permanently discards the application's stored
  metadata for a project, a confirmation step is required (consistent with the existing
  destructive-confirm pattern used for removing worktrees). This is a deliberate default rather
  than a stated requirement in the original request.
- **Forgetting is non-destructive to the project's folder**: "Forget" only prunes the
  application's own remembered entry and metadata (the catalog entry and the project's private
  per-project state file in the app data directory). It never deletes or modifies the project's
  folder, its files, or its git worktrees — deleting worktrees remains a separate, explicitly
  destructive action. Removing the app's private per-project state file is part of discarding
  metadata (FR-005), not a modification of the project on disk (FR-006).
- **Storage layout (aligned with the current codebase, 2026-07-23)**: The application persists
  the project catalog and each project's session/override state in separate files (per-project
  state files). Forgetting removes the catalog entry *and* deletes the project's own state file,
  so the application retains no remembered session metadata for that path (supports FR-005/FR-012).
  This deletes the app's *stored metadata* only; it does not delete the worktrees' on-disk
  conversations (FR-006), which a later re-open may legitimately rediscover (see FR-012 note).
- **No per-session archiving needed on forget**: When a *worktree* is deleted while its project
  stays in the catalog, that project's sessions are individually archived so reconciliation cannot
  rebuild them. Forget removes the *entire project* from the catalog and deletes its state file, so
  its sessions are never reconciliation targets within the session — no per-session `archive` step
  is required (a simplification the whole-project removal makes safe).
- **Stopping vs. deleting sessions**: Confirmed by clarification (2026-07-23) — running sessions
  of a forgotten project are stopped (their processes end) to avoid orphaned processes, but their
  underlying worktrees and files are left on disk; the user can reopen the folder and rediscover
  them. The confirmation prompt names how many sessions will be stopped (FR-002a).
- **Identity is the filesystem path**: A project is forgotten by its path identity; re-opening
  the same folder later yields a brand-new entry, because the prior entry and its metadata were
  discarded.
- **Single running instance**: A single running application instance manages the known-projects
  list; coordinating a forget across multiple simultaneously running instances is not addressed
  (consistent with the existing workspace-management assumptions).

## Out of Scope

The following are explicitly **not** part of this feature:

- Deleting or modifying anything on the filesystem (folders, files, or git worktrees). Removing
  a worktree remains its own separate, explicitly destructive action.
- Bulk-forgetting multiple projects at once, or an "undo" / restore of a forgotten project
  (a forgotten folder can be re-opened, but its prior metadata is not recoverable).
- Archiving or hiding projects without discarding their metadata (forget is a removal, not a
  hide).
- Automatically forgetting unavailable projects; removal is always user-initiated.

## Related

- Feature `002-project-workspace-management` explicitly listed "Removing entries from the
  known-projects list" as out of scope; this feature addresses exactly that gap.
- Renumbered from `013` to `014` on 2026-07-23 after rebasing onto `main`, which had merged
  `013-create-worktree-refinement` under the same number.
- Depends on the per-project storage split and session-archiving/reconciliation behavior
  introduced by `main`'s `fix/state-lost` work; forgetting deletes the project's per-project
  state file so those mechanisms cannot resurrect a forgotten project's sessions.
