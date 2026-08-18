# Feature Specification: Worktree Sidebar Refinement

**Feature Branch**: `008-worktree-sidebar-refinement`

**Created**: 2026-07-17

**Status**: Draft

**Input**: User description: "Refinement of worktree sidebar — minimal left/right padding, remove the git icon next to worktrees, show a worktree name with type/Jira tags below it (color-coded, later filterable), right-click actions to delete (worktree + all sessions + branch) and rename (displayed name only), and a smaller sidebar font at 80% of current size."

## Clarifications

### Session 2026-07-17

- Q: When a worktree has no custom rename, what friendly name shows on line 1? → A: The descriptive remainder only — type prefix and Jira key removed, dashes turned into spaces, sentence case (e.g. `feat/abc-123-login-page` → "Login page"); the type and ticket appear only as tags.
- Q: How should a worktree whose name does not match the convention be tagged/filtered? → A: Show no tag on the row, but offer an "untyped" bucket in the filter so these worktrees are still findable.
- Q: When multiple tag filters are active at once, what shows? → A: Match ANY — a worktree is shown if it matches any active filter (OR).
- Q: When Delete targets a worktree that has a running session (live terminal process), what happens? → A: After confirmation, terminate the running session processes, then remove the directory, sessions, and branch.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Recognize a worktree at a glance (Priority: P1)

A developer scanning the sidebar wants to know, without reading the full technical
directory name, what each worktree is for and what kind of work it holds. Each worktree
is shown as a clean, human-friendly name on the first line, with a row of small
color-coded tags directly beneath it: a **type tag** (feat, fix, chore, docs, refactor,
test, build, ci, perf, style) and, when the worktree's name embeds a Jira/issue key, an
**issue tag** (e.g. `ABC-123`). Colors are distinct and stable per type, so the developer
learns the color language once and then identifies worktree types by color alone.

**Why this priority**: This is the core informational upgrade and the foundation the
filtering story builds on. Delivered alone, it already makes the sidebar meaningfully
more useful.

**Independent Test**: Create worktrees whose branches follow the convention (with and
without a Jira key) and one that does not. Verify each row shows the friendly name plus
the correct color-coded type tag, that the issue tag appears only when a key is present,
and that the non-conforming worktree shows no misleading type tag — all without changing
any branch or directory name on disk.

**Acceptance Scenarios**:

1. **Given** a worktree whose branch is `feat/abc-123-login-page` and whose directory is
   `feat-abc-123_login-page`, **When** it is shown in the sidebar, **Then** the row displays
   the friendly name "Login page" plus a `feat` type tag and an `ABC-123` issue tag, each
   color-coded, while the branch and directory on disk are unchanged.
1a. **Given** a worktree whose directory is `feat-reporting-2` — no ticket boundary — **When**
   it is shown, **Then** the row displays "Reporting 2" with a `feat` tag and **no** issue tag
   (BUG-003).
2. **Given** a worktree whose branch is `fix/crash-on-open` (no Jira key), **When** it is
   shown, **Then** the row displays a friendly name plus a `fix` type tag and no issue tag.
3. **Given** two worktrees of different types, **When** they are shown together, **Then**
   their type tags use visibly different colors, and the same type always uses the same
   color.
4. **Given** a worktree whose name does not match the convention, **When** it is shown,
   **Then** no type tag is shown that would misrepresent its type.

---

### User Story 2 - Delete a worktree and everything it owns (Priority: P2)

A developer finished with a line of work wants to remove it completely from one place.
Right-clicking a worktree opens a context menu with a **Delete** action. Choosing it
shows a confirmation that clearly states the worktree's working directory, all of its
sessions, and its git branch will be removed. On confirmation, all three are removed and
the worktree disappears from the sidebar. Cancelling changes nothing.

**Why this priority**: Cleanup is a frequent, currently-missing action; consolidating
directory, sessions, and branch removal into one guarded action is high value. It is
destructive, so the confirmation gate is essential.

**Independent Test**: Right-click a worktree that has one or more sessions, choose Delete,
confirm, and verify the worktree, its sessions, and its branch are gone and the sidebar
updates. Repeat and cancel at the confirmation step; verify nothing is removed.

**Acceptance Scenarios**:

1. **Given** a worktree with two open sessions, **When** the user right-clicks it and
   chooses Delete, **Then** a confirmation appears naming the directory, its sessions, and
   the branch that will be removed.
2. **Given** the confirmation dialog, **When** the user confirms, **Then** the worktree
   directory, all of its sessions, and its git branch are removed and the worktree no
   longer appears in the sidebar.
3. **Given** a worktree with a running session (live terminal process), **When** the user
   confirms Delete, **Then** the running session processes are terminated first, then the
   directory, sessions, and branch are removed.
4. **Given** the confirmation dialog, **When** the user cancels, **Then** no worktree,
   session, or branch is removed.
5. **Given** the deleted worktree currently held the active session, **When** deletion
   completes, **Then** the application settles on a consistent state with no session from
   the removed worktree still active.
6. **Given** a deletion that fully completes, **When** the sidebar updates, **Then** no error,
   warning, or leftover-path notice is shown — the absence of the working directory after
   removal is the success case, not a failure. *(Added by BUG-001.)*
7. **Given** a worktree whose directory contains files the app has no permission to remove
   (for example build output a container wrote as another user), **When** the user confirms
   Delete, **Then** the worktree is released from git, its sessions are archived, and its row
   leaves the sidebar — and a non-fatal notice names the surviving paths and their owner. The
   worktree MUST NOT reappear in the sidebar as an unregistered orphan on the next refresh.
   *(Added by BUG-002.)*

---

### User Story 3 - Rename a worktree's displayed name (Priority: P3)

A developer wants a friendlier or clearer label for a worktree without touching the git
branch or the folder on disk. Right-clicking a worktree offers a **Rename** action that
edits only the name shown in the sidebar. The custom name is remembered across
application restarts. The type and issue tags continue to derive from the underlying
branch, unaffected by the rename.

**Why this priority**: A pure convenience/clarity improvement with no destructive risk;
valuable but lower stakes than deletion.

**Independent Test**: Rename a worktree, restart the application, and verify the custom
name persists while the on-disk directory, the git branch, and the derived tags are
unchanged.

**Acceptance Scenarios**:

1. **Given** a worktree, **When** the user right-clicks it and chooses Rename and enters a
   new name, **Then** the sidebar shows the new name and the on-disk directory and git
   branch are unchanged.
2. **Given** a renamed worktree, **When** the application is restarted, **Then** the custom
   name is still shown.
3. **Given** a renamed worktree, **When** its row is shown, **Then** the type and issue
   tags are still derived from the branch and are unchanged by the rename.
4. **Given** a rename in progress, **When** the user cancels or provides an empty name,
   **Then** the previously shown name is kept.

---

### User Story 4 - Filter the worktree list by tag (Priority: P4)

A developer with many worktrees wants to narrow the list to just the relevant ones. From
the sidebar they can activate one or more tag filters (by type, and by "has a Jira issue")
and the list collapses to only matching worktrees. Clearing the filter restores the full
list.

**Why this priority**: Findability scales with the number of worktrees and depends on the
tags from User Story 1 already existing, so it follows them.

**Independent Test**: With worktrees of several types, activate a type filter and verify
only matching worktrees remain; activate a second filter and verify the combined result;
clear filters and verify the full list returns.

**Acceptance Scenarios**:

1. **Given** worktrees of several types, **When** the user activates the `fix` filter,
   **Then** only `fix` worktrees are listed.
2. **Given** an active filter, **When** the user clears it, **Then** every worktree is
   listed again.
3. **Given** an active filter that matches no worktree, **When** the list is shown, **Then**
   an empty-state message is shown and the filter can be cleared in one action.
4. **Given** an active filter, **When** a worktree is renamed or deleted, **Then** the
   filtered list stays consistent with the active filter.

---

### User Story 5 - A compact, space-efficient sidebar (Priority: P5)

A developer working on a narrow sidebar wants to see as much of each worktree name as
possible. The sidebar uses minimal left and right inner padding, removes the git-status
icon that used to sit next to each worktree, and renders sidebar text at 80% of its
previous size — reclaiming horizontal space for names and tags.

**Why this priority**: Pure density/polish; independent of the other stories and lowest
risk, so it can land last without blocking them.

**Independent Test**: Compare the sidebar before and after: confirm the git icon is gone,
inner left/right padding is reduced to a minimal value, and sidebar text is visibly
smaller (80%), while remaining legible in light and dark themes.

**Acceptance Scenarios**:

1. **Given** the refined sidebar, **When** it is shown, **Then** no git-status icon appears
   next to any worktree.
2. **Given** the refined sidebar, **When** it is shown, **Then** the left and right inner
   padding is reduced to a minimal value that keeps content legible.
3. **Given** the refined sidebar, **When** text is shown, **Then** it renders at 80% of the
   previous size, everywhere in the sidebar (names, tags, sessions), and nowhere else in
   the app.
4. **Given** a worktree that is missing or invalid on disk, **When** it is shown without
   the git icon, **Then** its problem state is still distinguishable through another
   lightweight cue.

---

### Edge Cases

- **Non-conforming names**: A worktree whose branch/directory does not follow the
  convention (e.g. `main`, `my-experiment`) shows a friendly name and no type tag; it is
  matched by the "untyped" filter, not by any type filter.
- **Long names**: A worktree name longer than the row truncates with an ellipsis while its
  tags remain visible.
- **Duplicate display names**: Renaming two worktrees to the same displayed name is allowed
  because the underlying identity stays distinct; the app never confuses them.
- **Branch deletion refused**: If removing the branch would otherwise be blocked (e.g.
  unmerged work), the explicit confirmation has already authorized full removal, so the
  branch is removed; any failure to complete removal surfaces a clear error and leaves the
  system in a consistent state (no partial phantom worktree).
- **Delete while filtered**: Deleting the last worktree matching an active filter shows the
  filtered empty state, not a broken list.
- **Rename then restart before persistence**: The custom name is durable; an abrupt restart
  never leaves a half-applied rename.
- **Context menu scope**: The right-click menu targets worktree rows; session rows retain
  their existing close action and are not deleted/renamed by this menu.
- **Missing/invalid worktree deletion**: A worktree that is already missing or invalid on
  disk can still be removed (cleaned up) through Delete.
- **Directory already gone at cleanup time**: Removing the git worktree also removes its
  working directory, so the follow-up directory cleanup normally finds nothing left. This is
  the ordinary success path and MUST be silent; only a directory that genuinely survives
  removal is worth reporting (see FR-023a).
- **Worktree directory holds files the app cannot remove**: build output written by a container
  as another user (root, typically) cannot be unlinked by the app at any privilege it holds.
  `git worktree remove --force` still succeeds and deregisters the worktree, so the delete has
  genuinely happened; only the directory residue remains. This is partial success, not failure —
  the sessions are still archived and the row still leaves the sidebar, and the surviving paths
  are reported so the user can clear them (see FR-023c, FR-023d).

## Requirements *(mandatory)*

### Functional Requirements

**Worktree identity & tags**

- **FR-001**: The sidebar MUST display each worktree as a friendly name on the first line
  with a row of tags directly beneath it.
- **FR-002**: The system MUST derive a **type tag** from the worktree's existing conventional
  name for each of the supported types: feat, fix, chore, docs, refactor, test, build, ci,
  perf, style.
- **FR-003**: The system MUST derive an **issue tag** (Jira-style key such as `ABC-123`) when
  the worktree's name embeds one, and MUST omit it otherwise.
- **FR-004**: Tags MUST be typed/structured values (not free text) so that display and
  filtering behave consistently.
- **FR-005**: Each supported type MUST have its own distinct, stable color; the same type
  MUST always render in the same color, and the issue tag MUST have a consistent style
  distinct from the type tags.
- **FR-006**: Tag colors and all sidebar text MUST remain legible and meet the project's
  accessibility contrast standard (WCAG AA) in both light and dark themes.
- **FR-007**: The system MUST NOT change the git branch or on-disk directory naming
  convention; tags and the friendly name are presentation only, derived from the existing
  branch/directory.
- **FR-008**: A worktree whose name does not follow the convention MUST NOT display any type
  tag on its row; it remains reachable through the filter's "untyped" bucket (see FR-024).

**Compact layout**

- **FR-009**: The sidebar MUST use minimal left and right inner padding while keeping content
  legible.
- **FR-010**: The system MUST remove the git-status icon previously shown next to each
  worktree.
- **FR-011**: The system MUST still convey a worktree's missing/invalid state through a
  lightweight cue that does not reintroduce the removed git icon.
- **FR-012**: Sidebar text (names, tags, session labels) MUST render at 80% of its previous
  size, and this reduction MUST apply only within the sidebar, not elsewhere in the app.

**Context menu — Rename**

- **FR-013**: Right-clicking a worktree MUST open a context menu offering Rename and Delete.
- **FR-014**: Rename MUST change only the displayed name of the worktree; it MUST NOT rename
  the on-disk folder or the git branch.
- **FR-015**: A custom displayed name MUST persist across application restarts.
- **FR-016**: Type and issue tags MUST continue to derive from the underlying branch and MUST
  be unaffected by a rename.
- **FR-017**: When no custom name is set, the system MUST derive the friendly name from the
  worktree's descriptive name portion only — removing the conventional type prefix and the
  ticket, replacing separators with spaces, and applying sentence case (e.g.
  `feat-abc-123_login-page` → "Login page"). The removed type and ticket appear only as tags.
- **FR-017a** (bugfix BUG-003): The descriptive portion and the ticket MUST be separated by an
  explicit boundary in the directory name (`_`), and a directory name without one MUST be read
  as having no ticket. Nothing may infer a ticket from the *shape* of a name segment: a bare
  ticket (`feat-abc-123`) and a descriptive name with a disambiguator (`feat-reporting-2`) are
  the same pattern, so any shape rule reads one of them wrong — and the rule that existed read
  `feat-reporting-2` as issue `REPORTING-2`, which emptied the descriptive portion and made the
  label fall back to "Feat reporting 2", type prefix and all, violating this requirement.
- **FR-017b** (bugfix BUG-003): A ticket that is only digits MUST be preserved and displayed as
  an issue number (`#123`), so a GitHub/GitLab reference is as usable as a Jira-style key. The
  previous shape rule required a leading letter, so `#123` was accepted by the form, slugified to
  `123`, matched nothing, and was discarded — while its digits remained in the friendly name.

**Context menu — Delete**

- **FR-018**: Delete MUST require an explicit confirmation before anything is removed.
- **FR-019**: The confirmation MUST clearly state that the worktree's working directory, all
  of its sessions, and its git branch will be removed.
- **FR-020**: On confirmation, the system MUST first terminate any running session processes
  belonging to the worktree, then remove the worktree's working directory, all of its
  sessions, and its git branch, and MUST remove the worktree from the sidebar.
- **FR-021**: On cancellation, the system MUST remove nothing.
- **FR-022**: If the deleted worktree held the active session, the system MUST settle into a
  consistent state with no session from the removed worktree still active.
- **FR-023**: ~~If removal cannot fully complete, the system MUST surface a clear error and
  leave the system in a consistent state (no partially-removed worktree lingering).~~
  *(Superseded by BUG-001: the one-directional phrasing never required the success path to be
  silent, so an unconditional error on a fully-successful delete did not violate the letter of
  this requirement.)* Removal reporting MUST be exact in **both** directions: if removal
  cannot fully complete, the system MUST surface a clear error and leave the system in a
  consistent state (~~no partially-removed worktree lingering~~ *(further superseded by
  BUG-002: when the obstruction is a file the app has no permission to unlink, a partially-
  removed worktree lingering is the only reachable outcome — no retry or rollback available to
  the app can reach the state this clause demanded)*); and if removal **does** fully complete,
  the system MUST surface **no** error, warning, or leftover-path notice.
- **FR-023a**: A cleanup step that finds its target already absent MUST be treated as success,
  not as a failure. Specifically, removing the working directory after git has already removed
  it is the expected outcome, not an error condition.
- **FR-023b**: A genuine failure of any removal step MUST NOT be silently swallowed — it MUST
  reach the user via FR-023's error path.
- **FR-023c**: Removal has **three** outcomes, not two (added by BUG-002). Releasing the
  worktree from git and removing its directory are separately-failing steps, so:
  1. **Success** — git released the worktree and its directory is gone. Reported silently
     (FR-023).
  2. **Partial success** — git released the worktree, but part of its directory could not be
     removed. This MUST NOT be reported as a failed delete: the worktree is gone as far as git
     is concerned, so the system MUST still archive the worktree's sessions and MUST still
     remove the row from the sidebar, exactly as in case 1. The surviving paths MUST be
     reported to the user as a distinct, non-fatal notice.
  3. **Failure** — git did not release the worktree; nothing was removed. The system MUST
     surface a clear error and MUST leave the worktree's sessions untouched (neither stopped
     nor archived), so a later retry can still recover them.
- **FR-023d**: A partial-success or failure notice MUST identify *what* blocked the removal —
  the specific surviving paths, and (where the platform exposes it) the owning user of each.
  A notice carrying only an error code is not a "clear error" under FR-023: the ordinary cause
  is a file owned by another user, which the user can only resolve once they know which path
  and which owner (added by BUG-002).

**Filtering**

- **FR-024**: Users MUST be able to activate one or more tag filters from the sidebar: by
  type, by "has a Jira issue", and by an "untyped" bucket that matches worktrees whose names
  do not follow the convention.
- **FR-025**: When filters are active, the sidebar MUST show only matching worktrees; when
  multiple filters are active, a worktree matches if it satisfies ANY active filter
  (logical OR).
- **FR-026**: Users MUST be able to clear all active filters in a single action to restore the
  full list.
- **FR-027**: When an active filter matches no worktree, the system MUST show an empty-state
  message and keep a one-action way to clear the filter.
- **FR-028**: An active filter MUST remain consistent when worktrees are added, renamed, or
  deleted.

### Key Entities

- **Worktree (existing)**: A line of work bound to a git worktree. Identity is its on-disk
  directory; carries its branch, on-disk status (valid / missing / invalid), and its
  sessions. Gains a presentation layer: a friendly display name and derived tags.
- **Tag**: A typed label attached to a worktree for display and filtering. Has a category
  (conventional type, or issue key) and a value; type tags carry a stable per-type color.
- **Display-name override**: A persisted association from a worktree's identity to a
  user-chosen displayed name. Absent for worktrees never renamed.
- **Filter selection**: The set of tag filters currently active in the sidebar.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can identify a worktree's type by its tag color alone (without reading
  the name) in under 2 seconds.
- **SC-002**: For worktrees whose names follow the convention, 100% display the correct type
  tag, and the issue tag appears for exactly those that embed a Jira-style key.
- **SC-003**: A worktree that does not follow the convention shows no misleading type tag in
  100% of cases.
- **SC-004**: Deleting a worktree removes its directory, all of its sessions, and its branch,
  and 0% of deletions occur without an explicit confirmation step.
- **SC-004a**: 0% of fully-successful deletions produce an error, warning, or leftover-path
  notice, and 100% of deletions that genuinely leave something behind produce one.
  *(Added by BUG-001.)*
- **SC-004b**: 100% of deletions in which git released the worktree archive the worktree's
  sessions and drop its sidebar row, whether or not the directory was fully removed; 0% of them
  leave the worktree to reappear as an unregistered orphan. *(Added by BUG-002.)*
- **SC-004c**: 100% of leftover-path notices name at least one specific surviving path, and
  name the owning user wherever the platform exposes it; 0% report an error code alone.
  *(Added by BUG-002.)*
- **SC-005**: Renaming a worktree changes only its displayed name in 100% of cases — the
  on-disk directory, the git branch, and the derived tags are unchanged — and the new name
  survives an application restart.
- **SC-006**: Users can narrow the list to a chosen tag and restore the full list, each in a
  single action.
- **SC-007**: All type tag colors and the reduced-size sidebar text meet WCAG AA contrast in
  both light and dark themes.
- **SC-008**: The sidebar no longer shows a git icon next to worktrees, and reclaims
  horizontal space through minimal left/right padding and 80% text size, so a worktree name
  displays more of its characters before truncation than before at the same sidebar width.

## Assumptions

- **Tag source**: Tags are derived on the fly from the existing branch/directory naming
  convention; no new per-worktree metadata is persisted except the display-name override.
- **Default friendly name**: When no rename override exists, the friendly name is the
  descriptive remainder in sentence case (type prefix and Jira key removed, separators turned
  into spaces) — e.g. "Login page" (see FR-017).
- **At most one issue key**: A worktree name embeds at most one Jira-style key, consistent
  with the naming convention.
- **Branch removal is authoritative**: Because Delete's confirmation explicitly authorizes
  removing the branch, branch removal proceeds even if the branch has unmerged work; the
  destructive nature is communicated in the confirmation.
- **Filter semantics**: Multiple active filters combine with logical OR — a worktree is shown
  if it matches any active filter (see FR-025).
- **Missing/invalid cue**: The lightweight cue replacing the git icon is a subtle text/color
  treatment on the row, decided in planning.
- **Context menu scope**: Rename and Delete apply to worktree rows only; session rows keep
  their existing behavior.
- **Persistence reuses local-first storage**: The display-name override is stored using the
  application's existing local, offline storage; nothing is sent off-device.
- **80% coverage**: The 80% reduction applies uniformly to all text scales used within the
  sidebar.
- **Git removes the working directory**: `git worktree remove` deletes the worktree's working
  directory itself. Any follow-up directory cleanup is therefore best-effort belt-and-braces
  for the case where something survives; finding the directory already absent is success.
- **The app does not own every file in a worktree**: a worktree is a working directory for real
  builds, and tooling run inside it — a container writing through a bind mount, most commonly —
  can leave files owned by another user. The app therefore cannot assume it is able to remove
  everything under a worktree it created, at any privilege it holds. *(Added by BUG-002.)*

**Bugfix**: 2026-07-20 — BUG-001 Every worktree delete reported a folder-removal error despite
fully succeeding, because the post-removal directory cleanup treated "already gone" as a
failure. FR-023 amended to require exact reporting in both directions (success MUST be silent),
FR-023a added (an absent cleanup target is success), FR-023b added (a genuine failure MUST NOT
be swallowed), US2 acceptance scenario 6 added, SC-004a added, plus an edge case and an
assumption recording that `git worktree remove` deletes the directory itself.

**Bugfix**: 2026-08-04 — BUG-002 A delete blocked by foreign-owned files reported a bare errno,
left its sessions un-archived, and let the worktree return to the sidebar as an unregistered
orphan. The spec modelled removal as one atomic act with two outcomes; releasing the worktree
from git and removing its directory fail independently, and FR-023's "no partially-removed
worktree lingering" is unreachable when the app cannot unlink the blocking files at all.
FR-023's consistent-state clause further superseded, FR-023c added (three outcomes, with partial
success still archiving sessions and dropping the row), FR-023d added (a notice MUST name the
surviving paths and their owner), US2 acceptance scenario 7 added, SC-004b/SC-004c added, plus
an edge case and an assumption recording that the app does not own every file in a worktree.
