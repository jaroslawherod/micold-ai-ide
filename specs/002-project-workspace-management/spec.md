# Feature Specification: Project Selection and Workspace Management

**Feature Branch**: `002-project-workspace-management`

**Created**: 2026-07-13

**Status**: Draft

**Input**: User description: "Project selection and workspace management. Micold AI IDE lets a user choose a project to work on and set that project as the current working space. Opening a project, known projects (local-first), active working space, renaming a project, notable situations to handle, scope boundaries, and cross-platform parity."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Open a folder and set it as the active working space (Priority: P1)

From the application shell, the user opens a project selector, browses the local filesystem, and picks a folder. Any folder is allowed — a git repository is not required. When the folder is chosen, a project is created whose default display name is the folder's name, and that project immediately becomes the active working space. The shell shows the active project's display name. When no project has ever been opened, the shell instead presents an empty state that invites the user to open a project.

**Why this priority**: This is the foundational slice of the feature. Without the ability to pick a folder and make it the active working space, none of the later capabilities (persistence, git indication, renaming) have anything to operate on. It is the smallest slice that delivers demonstrable value: a user can point the application at a folder and start working there.

**Independent Test**: Launch the application with no prior projects, confirm the empty state invites opening a project, open the selector, pick any folder (git or non-git), and verify a project is created with the folder's name as its display name and that the shell now indicates it as the active working space.

**Acceptance Scenarios**:

1. **Given** the application has never had a project opened, **When** the user views the shell, **Then** an empty state is shown that invites the user to open a project.
2. **Given** the shell is open, **When** the user opens the project selector, **Then** the user can browse the local filesystem and select a folder.
3. **Given** a folder that is not a git repository is highlighted in the selector, **When** the user chooses it, **Then** a project is created (git is not required) and it becomes the active working space.
4. **Given** a folder is chosen, **When** the project is created, **Then** its default display name equals the folder's name.
5. **Given** a project has been made active, **When** the user views the shell, **Then** the shell displays that project's display name as the active working space.

---

### User Story 2 - Reopen a known project after restarting the application (Priority: P2)

The application remembers the projects a user has opened by persisting a known-projects list on the local filesystem. On a later launch, the user reopens a project directly from that list without browsing the filesystem again. The list also records which project was last active. Opening a folder that is already known does not create a duplicate — it activates the existing entry.

**Why this priority**: Persistence turns one-off folder selection into durable workspace management and directly realizes the Local-First Storage principle. It builds on Story 1 (a project must be opened before it can be remembered) and delivers the day-to-day value of returning to prior work quickly.

**Independent Test**: Open one or more projects, fully restart the application, and confirm each previously opened project appears in the known-projects list with its stored display name and can be reopened and made active without browsing the filesystem; confirm the last-active project is identifiable; confirm re-opening an already-known folder activates the existing entry instead of adding a second one.

**Acceptance Scenarios**:

1. **Given** the user has opened a project, **When** the application is restarted, **Then** that project appears in the known-projects list with its stored display name and recorded git status.
2. **Given** a known project is listed, **When** the user reopens it from the list, **Then** it becomes the active working space without the user browsing the filesystem.
3. **Given** several projects have been opened over time, **When** the application is restarted, **Then** the list records and can indicate which project was last active.
4. **Given** a folder is already a known project, **When** the user opens that same folder again, **Then** no duplicate entry is created and the existing entry is activated.

---

### User Story 3 - Distinguish git repositories in the selector (Priority: P2)

While browsing the filesystem in the project selector, the user can tell at a glance which folders are git repositories, because those folders are visually marked with a git icon. Git-repository status is also recorded for each known project so it is available when the project is reopened.

**Why this priority**: Version-controlled folders are the common case for development work, and marking them helps users pick the right folder confidently. It is valuable but not required for the core open-and-activate flow, so it ranks below Stories 1 and 2.

**Independent Test**: Browse a directory that contains both a git-repository folder and a non-git folder, and verify only the git-repository folder shows the git icon; then choose it and verify its git status is recorded with the created project.

**Acceptance Scenarios**:

1. **Given** the selector is browsing a directory, **When** a folder in it is a git repository, **Then** that folder is marked with a git icon.
2. **Given** the selector is browsing a directory, **When** a folder in it is not a git repository, **Then** that folder is not marked with a git icon.
3. **Given** a folder's git status was determined when it was inspected, **When** a project is created from it, **Then** that git status is stored with the project record.

---

### User Story 4 - Rename a project's display name (Priority: P3)

The user renames a project. Renaming changes only the display name stored by the application; it never renames, moves, or otherwise modifies the folder on disk. The new display name persists in the known-projects list across restarts. Display names are not required to be unique — two projects may share a display name and remain distinct by their filesystem path. A rename to an empty or whitespace-only name is rejected.

**Why this priority**: Renaming is a convenience that improves organization once multiple projects accumulate. It depends on Stories 1 and 2 and is not required to derive core value, so it is the lowest priority.

**Independent Test**: Rename a known project, confirm the shell and list show the new display name, confirm the folder on disk is unchanged, restart the application and confirm the new name persists, and confirm that attempting to rename to an empty or whitespace-only name is rejected and leaves the previous name intact.

**Acceptance Scenarios**:

1. **Given** a project exists, **When** the user renames it to a valid non-empty name, **Then** the project's display name is updated everywhere it is shown.
2. **Given** a project has been renamed, **When** the application is restarted, **Then** the new display name persists in the known-projects list.
3. **Given** a project is renamed, **When** the rename completes, **Then** the folder on disk is not renamed, moved, or otherwise modified.
4. **Given** two projects at different paths, **When** the user renames one to match the other's display name, **Then** both are allowed and remain distinct by filesystem path.
5. **Given** a project exists, **When** the user attempts to rename it to an empty or whitespace-only name, **Then** the rename is rejected and the previous display name is unchanged.

---

### Edge Cases

- **Folder gone since it was added**: A known project's folder may have been deleted, moved, or renamed on disk. The list MUST degrade gracefully (no crash) and clearly mark that project as unavailable.
- **Reopening an unavailable project**: When the user attempts to reopen a project whose folder is no longer present, the application does not crash, does not activate a nonexistent working space, and communicates that the folder is unavailable.
- **Git status is point-in-time**: A folder's git-repository status is determined when the folder is inspected; a folder could gain or lose git status later on disk. The recorded status reflects the last time it was inspected.
- **Unreadable or permission-denied paths while browsing**: The selector handles paths it cannot read gracefully rather than crashing.
- **Re-opening the same folder**: Choosing a folder that is already known activates the existing entry and never creates a duplicate (identity is the filesystem path).
- **Whitespace-only or empty rename**: Rejected; the existing display name is preserved.
- **First-ever launch**: With no known projects, the shell shows the empty state inviting the user to open a project.

## Requirements *(mandatory)*

### Functional Requirements

#### Opening and selecting a project

- **FR-001**: The application shell MUST provide a way to open a project selector.
- **FR-002**: The project selector MUST let the user browse the local filesystem and select a folder.
- **FR-003**: ~~The system MUST allow any folder to be chosen as a project, whether or not it is a git repository.~~ (Superseded — spec/code alignment 2026-07-20: directly reversed by feature 005 FR-001a, which restricts projects to git repositories because every session maps to a git worktree.) The system MUST refuse to open a non-git directory as a project, per feature 005 FR-001a. The git-repository flag of FR-006/FR-007 therefore distinguishes *openable* folders in the browser rather than annotating projects that may be opened either way.
- **FR-004**: When a folder is chosen, the system MUST create a project whose default display name is the folder's name.
- **FR-005**: Choosing a folder (or reopening a known project) MUST make that project the active working space.

#### Git-repository indication

- **FR-006**: The selector MUST visually mark folders that are git repositories with a git icon, and MUST NOT mark folders that are not git repositories.
- **FR-007**: The system MUST detect a folder's git-repository status at the time the folder is inspected, and MUST record that status with the project.

#### Known projects and persistence (local-first)

- **FR-008**: The system MUST persist the known-projects list on the local filesystem so it survives application restarts, without requiring any network or cloud service.
- **FR-009**: Each known-project record MUST include the folder's filesystem path, its display name, and its git-repository status.
- **FR-010**: The persisted list MUST record which project was last active.
- **FR-011**: On launch, the system MUST let the user reopen a known project directly from the list without browsing the filesystem.
- **FR-012**: Opening a folder that is already a known project MUST activate the existing entry and MUST NOT create a duplicate; project identity is the filesystem path.

#### Active working space

- **FR-013**: Exactly one project MUST be the active working space at any time.
- **FR-014**: Selecting or reopening a project MUST make it active and replace any previously active project.
- **FR-015**: The application shell MUST indicate the active project by showing its display name.
- **FR-016**: When no project has ever been opened, the system MUST present an empty state that invites the user to open a project.

#### Renaming

- **FR-017**: The user MUST be able to rename a project's display name.
- **FR-018**: Renaming MUST change only the display name stored by the application, and MUST NOT rename, move, or otherwise modify the folder on disk.
- **FR-019**: A renamed display name MUST persist in the known-projects list across restarts.
- **FR-020**: The system MUST reject a rename to an empty or whitespace-only name and MUST leave the previous display name unchanged.
- **FR-021**: Display names MUST NOT be required to be unique; projects MUST remain distinct by filesystem path.

#### Graceful degradation and availability

- **FR-022**: If a known project's folder has been deleted, moved, or renamed on disk, the system MUST NOT crash and MUST clearly mark that project as unavailable in the list.
- **FR-023**: The system MUST NOT activate a project whose folder is unavailable, and MUST communicate the unavailability to the user rather than failing silently or crashing.

#### Cross-platform parity

- **FR-024**: Filesystem browsing, git-repository detection, the persisted known-projects list, and renaming MUST behave equivalently on Linux, macOS, and Windows.

### Key Entities *(include if feature involves data)*

- **Project**: A folder that has been chosen as a workspace. Attributes: filesystem path (the stable identity of the project), display name (application-managed label, defaults to the folder's name, may be renamed, need not be unique), git-repository status (whether the folder was a git repository when last inspected), and availability status (whether the folder currently exists on disk). A project never owns or modifies its folder's on-disk name.
- **Known-Projects List**: The locally persisted collection of Project records that survives restarts. In addition to the records, it tracks which project was last active. It contains at most one entry per filesystem path.
- **Active Working Space**: The single project that is currently active. At any moment it references exactly one Project (or none, before any project has been opened). Selecting or reopening a project replaces the current active working space.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A new user, with no prior projects, can go from the empty state to a chosen folder being the active working space in a single browse-and-pick flow, without reading documentation.
- **SC-002**: 100% of chosen folders become the active working space regardless of git status (no folder is rejected for lacking git).
- **SC-003**: After restarting the application, 100% of previously opened projects reappear in the known-projects list with their stored display names, and the last-active project is identifiable.
- **SC-004**: Reopening a known project makes it active without any filesystem browsing.
- **SC-005**: Opening an already-known folder produces zero duplicate entries (exactly one entry per filesystem path at all times).
- **SC-006**: In the selector, every git-repository folder shows the git indicator and every non-git folder does not (0 false indicators, 0 missed indicators for the folders shown).
- **SC-007**: Renaming updates the display name everywhere it is shown and results in zero modifications to the folder on disk.
- **SC-008**: 100% of attempts to rename a project to an empty or whitespace-only name are rejected with the previous name preserved.
- **SC-009**: When a known project's folder is missing, the application remains usable with zero crashes and the project is clearly marked unavailable.
- **SC-010**: Every acceptance scenario in this specification passes identically on Linux, macOS, and Windows.

## Assumptions

- **Storage location**: The known-projects list is stored in a conventional per-user application data location on the local filesystem; the exact location is an implementation detail chosen during planning.
- **Selector mechanism**: "Browse the local filesystem and pick a folder" is satisfied by an in-application folder browser and/or the platform's native folder-picker; either is acceptable as long as behavior is equivalent across platforms.
- **Git detection scope**: A folder is considered a git repository based on inspecting the folder itself (e.g., the presence of standard git repository markers); detection does not require running a networked git operation and works offline.
- **Availability check timing**: A project's availability is determined when the known-projects list is presented and/or when the user attempts to reopen it; the specification does not require continuous background monitoring of the filesystem.
- **Concurrent instances**: A single running application instance manages the known-projects list; coordinating the list across multiple simultaneously running instances of the application is not addressed by this feature.
- **Display-name characters**: Any non-empty, non-whitespace-only string is an acceptable display name; no further character restrictions are imposed beyond that.

## Out of Scope

The following are explicitly **not** part of this feature:

- Creating or managing git worktrees or sessions (Constitution Principles II and III) — deferred to a later feature.
- Opening, reading, editing, or displaying the contents of files within a project.
- Initializing git in a non-git folder.
- Renaming, moving, or deleting anything on the filesystem (all rename behavior affects only the application-stored display name).
- Removing entries from the known-projects list (managing/pruning the list is not addressed here).

**Alignment**: 2026-07-20 — Spec/code alignment audit. FR-003 amended: "any folder, git or not" was directly reversed by feature 005 FR-001a (git repositories only), because every session maps to a git worktree. The code has enforced the git-only gate since feature 005; this spec had not been updated. No behaviour change. Separately, a real defect was found on this path and is tracked for fix, not spec'd away: the refusal message is written to a state field whose only render site is the add-worktree modal, so a non-git folder is refused silently (see feature 005 FR-001a, which requires informing the user).
