# UI Contract: Project Selection and Workspace Management

This is a desktop application; its external contract is the **user-facing interaction surface**,
not a network API. Each clause traces to a functional requirement (FR) in [spec.md](../spec.md).
The selector is an **in-app folder browser** (research R3), rendered as a modal overlay within
the single main window, consistent with feature 001's overlay pattern.

## C1. Shell active-project indicator & empty state

| Condition | Behavior | Traces to |
|-----------|----------|-----------|
| A project is active | The shell displays the active project's **display name**. | FR-014, FR-015 |
| No project has ever been opened | The shell shows an **empty state** inviting the user to open a project. | FR-016 |
| A project is activated / reopened | The indicator updates to the new active project's display name. | FR-013, FR-014 |

- At most one project is active at any time (FR-013).

## C2. Opening the selector

- The shell provides an affordance to **open the project selector** (from the empty state and/or
  a toolbar entry) (FR-001).
- Activating it opens the folder-browser overlay within the main window (FR-001, FR-002).

## C3. Folder browser

| Element | Behavior | Traces to |
|---------|----------|-----------|
| Current directory | The browser shows which directory is being browsed. | FR-002 |
| Subfolder list | Lists **folders only** in the current directory. | FR-002 |
| Git marker | Each folder that is a git repository shows a **git icon**; non-git folders show none. | FR-006 |
| Navigate into | Selecting a folder enters it. | FR-002 |
| Navigate up | An "up"/parent action moves to the parent; at a drive/`/` boundary it presents roots (Windows drive letters / `/`). | FR-002 (research R5) |
| Unreadable directory | Shows a handled inline error/empty listing; **never crashes**. | edge case, SC-009 |
| Choose | An explicit action opens the **current** folder as a project. | FR-003, FR-005 |

- **Any** folder is choosable regardless of git status (FR-003); the git icon is informational
  only.

## C4. Creating / activating a project from a chosen folder

| Situation | Result | Traces to |
|-----------|--------|-----------|
| Chosen folder is **not** already known | A project is created; default display name = folder name; git status recorded; it becomes active. | FR-004, FR-005, FR-007 |
| Chosen folder **is** already known | The **existing** entry is activated; **no duplicate** is created. | FR-012 |
| Any successful open/activate | Previous active project is replaced; shell indicator updates. | FR-013, FR-014 |

## C5. Known-projects list (reopen after restart)

| Element | Behavior | Traces to |
|---------|----------|-----------|
| Listed projects | After restart, previously opened projects appear with their **stored display name** and recorded git status. | FR-008, FR-009 |
| Reopen | Selecting a known project makes it active **without browsing the filesystem**. | FR-011 |
| Last active | The list records/can indicate which project was last active. | FR-010 |
| Available project | Reopen succeeds and it becomes active. | FR-011, FR-013 |
| Unavailable project | Marked **unavailable**; reopen is blocked and the user is informed; **no crash**, active unchanged. | FR-022, FR-023, SC-009 |

## C6. Renaming a project

| Trigger | Result | Traces to |
|---------|--------|-----------|
| Start rename | A rename dialog opens pre-filled with the current display name. | FR-017 |
| Confirm valid name (non-empty, not all-whitespace) | Display name updates everywhere it is shown; folder on disk **unchanged**. | FR-017, FR-018 |
| Confirm empty / whitespace-only name | **Rejected**; previous display name preserved; dialog indicates the problem. | FR-020, SC-008 |
| After restart | The renamed display name **persists**. | FR-019 |
| Two projects, same name | Allowed; they remain distinct by filesystem path. | FR-021 |
| Cancel | No change to the display name. | FR-017 |

## C7. Persistence & offline behavior

- The known-projects list (records + last-active pointer) is persisted to the **local
  filesystem** and reloaded on launch; the feature works fully **offline** (FR-008; Principle
  IV). On-disk format is defined in [storage-schema.md](./storage-schema.md).
- A missing or corrupt store degrades to an **empty** known-projects list; the app remains usable
  and shows the empty state (research R8; SC-009).
- A save failure is surfaced non-fatally; it never crashes the app.

## C8. Filesystem safety (read-only)

- The feature **never** creates, renames, moves, or deletes anything on the filesystem. Git
  detection and browsing are **read-only**; a "rename" affects only the application-stored
  display name (FR-018; spec Out of Scope).

## C9. Cross-platform parity

- Every clause above behaves **identically on Linux, macOS, and Windows** (FR-024, SC-010). The
  only permitted platform difference is the *roots* presentation (Windows drive letters vs `/`),
  which does not change any observable outcome of opening, listing, reopening, or renaming.

## Contract test checklist

Maps to acceptance scenarios; covered by render-free core tests (`Workspace`/`Selector`/rename
transitions), store roundtrip tests (`tempfile`), and the manual `quickstart.md` walkthrough:

- [ ] Empty state on first-ever launch invites opening a project (C1 / FR-016)
- [ ] Open selector → browse folders; folders-only listing (C2, C3)
- [ ] Git-repo folders show the git icon; non-git folders do not (C3 / FR-006)
- [ ] Choose a non-git folder → project created + active (C3, C4 / FR-003)
- [ ] Default display name = folder name (C4 / FR-004)
- [ ] Choose an already-known folder → no duplicate; existing activated (C4 / FR-012)
- [ ] Activating replaces previous active; shell shows active name (C1, C4 / FR-013, FR-014)
- [ ] Restart → known projects listed with stored names; reopen without browsing (C5 / FR-011)
- [ ] Last-active recorded (C5 / FR-010)
- [ ] Missing folder → marked unavailable; reopen blocked; no crash (C5 / FR-022, FR-023)
- [ ] Rename to valid name updates everywhere; disk unchanged (C6 / FR-017, FR-018)
- [ ] Rename persists across restart (C6 / FR-019)
- [ ] Rename to empty/whitespace rejected; previous name kept (C6 / FR-020)
- [ ] Two projects with same name remain distinct by path (C6 / FR-021)
- [ ] Missing/corrupt store → empty list, app usable (C7 / SC-009)
- [ ] No filesystem mutation anywhere (C8)
- [ ] All of the above verified on Linux, macOS, Windows (C9 / SC-010)
