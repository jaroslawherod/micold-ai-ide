# Feature Specification: Hide Agent Worktrees

**Feature Branch**: `fix/hide-agent-worktrees`

**Created**: 2026-07-23

**Status**: Closed (implemented and shipped; the manual quickstart pass ran 2026-08-21 — [evidence](./evidence/T034-manual-procedure.md). SC-001/002/003/005 confirmed; SC-004 is a before/after comparison and stays unmeasured, since no pre-feature build exists to compare against.)

**Input**: User description: "Hide Claude Code's internal subagent worktrees from the app's worktree UI. These are worktrees Claude Code creates for isolated subagents: directory `.claude/worktrees/agent-<hex-id>` with branch `worktree-agent-<hex-id>`. They are not user worktrees and should not appear in the worktree list / project switcher. Filter them out by matching the `worktree-agent-` branch prefix (and/or the `agent-<hex>` directory name under `.claude/worktrees/`), while making sure a user-created branch merely named e.g. `agent-foo` is not accidentally hidden."

## Context

The app stores every worktree it manages under the project's `.claude/worktrees/` directory. An
AI coding assistant working inside the same project uses that *same* directory for the throwaway,
isolated worktrees it creates for its own background sub-tasks. Those assistant-owned worktrees
have machine-generated names (a fixed prefix plus a long opaque hexadecimal identifier), are
created and destroyed without the user's involvement, and carry work the user never asked to see
as a session. Today they are indistinguishable from the user's own worktrees in the sidebar, so
the list fills up with entries the user did not create, cannot meaningfully name, and should not
be starting sessions in.

## Clarifications

### Session 2026-07-23

- Q: What actions are available on a revealed assistant-owned worktree row? → A: Full actions, identical to any worktree row — start session, rename, delete
- Q: What happens to the reveal control when the user switches projects mid-run? → A: Reset to off on every project switch, so each project starts hidden
- Q: How long must the machine-generated identifier be to count as assistant-owned? → A: At least 16 hexadecimal characters, and the whole remainder must be hexadecimal
- Q: What term should user-visible text use for these worktrees? → A: "Agent" — chip reads `agent`, control reads "Show agent worktrees"; spec prose keeps "assistant-owned" as internal wording

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The worktree list shows only my worktrees (Priority: P1)

A developer opens a project in which an AI assistant has previously run background sub-tasks,
leaving several assistant-owned worktrees on disk. The sidebar lists only the worktrees the
developer created themselves; the assistant-owned ones are simply absent, with no extra rows,
badges, or cleanup prompts to dismiss.

**Why this priority**: This is the whole point of the feature. Without it, the sidebar is
polluted with unrecognizable machine-named entries and the user cannot quickly find their own
work. Delivered alone, it already restores a usable worktree list.

**Independent Test**: Open a project containing a mix of user-created worktrees and
assistant-owned ones, and confirm the sidebar lists exactly the user-created set.

**Acceptance Scenarios**:

1. **Given** a project with 3 user-created worktrees and 3 assistant-owned worktrees on disk,
   **When** the user opens the project, **Then** the sidebar lists exactly the 3 user-created
   worktrees.
2. **Given** a project whose only worktrees are assistant-owned, **When** the user opens the
   project, **Then** the sidebar presents the project's empty "no worktrees yet" state rather
   than a list of machine-named entries.
3. **Given** an assistant creates a new assistant-owned worktree while the project is open,
   **When** the app next refreshes its view of the project's worktrees, **Then** no new row
   appears in the sidebar.
4. **Given** assistant-owned worktrees exist, **When** the user views the sidebar, **Then** the
   app has not deleted, pruned, renamed, or otherwise modified any of them or their branches.

---

### User Story 2 - My own worktrees are never hidden by mistake (Priority: P2)

A developer has created their own worktrees whose names happen to begin with the word "agent"
(for example a worktree for an "agent" feature). Those worktrees stay fully visible and usable —
the hiding rule only removes the assistant's machine-generated worktrees.

**Why this priority**: A false positive is strictly worse than the original problem: the user's
own work silently disappears with no explanation and no way to reach it. This guard must exist,
but it is only meaningful once P1 hides anything at all.

**Independent Test**: Create worktrees with names that share the assistant's prefix but not its
machine-generated identifier shape, and confirm every one of them remains listed.

**Acceptance Scenarios**:

1. **Given** a user-created worktree whose name begins with "agent" but continues with ordinary
   words (e.g. `agent-foo`), **When** the user views the sidebar, **Then** that worktree is
   listed as normal.
2. **Given** a user-created worktree named after a feature that includes the word "agent"
   somewhere in the middle, **When** the user views the sidebar, **Then** that worktree is
   listed as normal.
3. **Given** a user-created worktree whose name is the reserved prefix followed by fewer than 16
   hexadecimal characters, or by 16-or-more characters of which any is not hexadecimal, **When**
   the user views the sidebar, **Then** that worktree is listed as normal.
4. **Given** a worktree that lives outside the project's managed worktrees directory but whose
   name matches the assistant's naming convention, **When** the app builds its worktree list,
   **Then** the app's existing rules for out-of-scope worktrees apply unchanged (the hiding rule
   introduces no new behavior there).

---

### User Story 3 - Everything derived from the list stays consistent (Priority: P3)

Wherever the app counts, filters, or acts on worktrees, hidden assistant-owned worktrees are
absent there too — filter results, the "nothing matched" state, and the set of worktrees a
session can be started in, renamed, or deleted.

**Why this priority**: Without this, hiding is only skin-deep: a filter chip could report a count
that includes invisible entries, or an assistant-owned worktree could still be reachable as a
session target — reintroducing the confusion P1 removed, in a more surprising form.

**Independent Test**: With assistant-owned worktrees present, exercise the sidebar's filtering
and session-start flows and confirm no count, result set, or target list includes them.

**Acceptance Scenarios**:

1. **Given** assistant-owned worktrees exist, **When** the user applies a sidebar filter,
   **Then** counts and results reflect only user-owned worktrees.
2. **Given** assistant-owned worktrees exist and are hidden, **When** the user starts a new
   session, **Then** no assistant-owned worktree is offered as a location.
3. **Given** an assistant-owned worktree's directory exists on disk but git does not know it as a
   worktree (an orphan left behind), **When** the app builds its worktree list, **Then** that
   orphan is hidden as well rather than surfacing as a broken entry.
4. **Given** a session the app previously recorded against a worktree that is now classified as
   assistant-owned, **When** the app loads the project, **Then** the worktree stays hidden and
   the session is handled exactly as the app already handles a session whose worktree is no
   longer available — with no dedicated error path for this case.

---

### User Story 4 - Reveal them on demand (Priority: P4)

A developer suspects the assistant left worktrees behind and wants to look. They open the
sidebar's filter panel, switch on "show agent worktrees", and the assistant-owned entries join
the list — visibly marked as assistant-owned so they are never mistaken for the developer's own.
Switching it back off, or restarting the app, returns the list to showing only their worktrees.

**Why this priority**: Pure escape hatch. The feature is valuable without it, but without any way
to see them the user has no in-app path to notice or clean up leftovers.

**Independent Test**: With assistant-owned worktrees present, toggle the reveal control on and off
and confirm the list gains and loses exactly those entries.

**Acceptance Scenarios**:

1. **Given** assistant-owned worktrees exist and the reveal toggle is off, **When** the user
   switches it on, **Then** those worktrees appear in the list, visually marked as
   assistant-owned.
2. **Given** the reveal toggle is on, **When** the user switches it off, **Then** the
   assistant-owned entries disappear again and the user's own worktrees are unaffected.
3. **Given** the user left the reveal toggle on, **When** the app is restarted, **Then** the
   toggle is off again and assistant-owned worktrees are hidden.
4. **Given** the reveal toggle is on in one project, **When** the user switches to another
   project, **Then** the toggle is off there and that project's assistant-owned worktrees are
   hidden; switching back does not restore it either.
5. **Given** no assistant-owned worktrees exist in the project, **When** the user switches the
   reveal toggle on, **Then** the list is unchanged and no error or empty-state flicker occurs.

---

### Edge Cases

- **Orphaned assistant directory**: a directory matching the assistant's naming convention exists
  under the managed worktrees directory but git no longer registers it as a worktree. It is
  hidden rather than shown as a broken/invalid entry (FR-007).
- **Registered but missing**: git still registers an assistant-owned worktree whose directory has
  been removed. It is hidden rather than shown as a missing entry.
- **Name/branch mismatch**: a worktree's directory matches the assistant convention but its branch
  does not (or vice versa). The app treats it as assistant-owned if *either* identifier carries
  the convention, since both are generated by the same mechanism (FR-005).
- **Detached worktree**: an assistant-owned worktree not currently on a branch is still hidden on
  the strength of its directory name alone.
- **All worktrees hidden**: a project whose entire worktree set is assistant-owned presents the
  normal empty state, not an error or a blank region.
- **Deliberate collision**: a user creates a worktree that exactly imitates the assistant's naming
  convention, machine-shaped identifier and all. It is hidden — accepted, because the convention
  is treated as reserved. Revealing it (FR-010) is the way back to it.
- **Session bound to a hidden worktree**: a session the app recorded earlier points at a worktree
  now classified as assistant-owned. The worktree stays hidden; the session falls into the app's
  existing "worktree unavailable" handling (FR-011).
- **Revealed then removed**: the reveal control is on and the assistant deletes one of its
  worktrees. On the next refresh the entry disappears like any removed worktree — no stale row and
  no error.
- **Reveal with nothing to reveal**: the project has no assistant-owned worktrees. The control is
  still present and switching it on changes nothing (FR-010c).
- **Reveal combined with tag filters**: tag filters are active when the reveal control is switched
  on. Revealed entries are subject to the same filters, and the filters themselves are untouched
  (FR-010d).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST classify every discovered worktree as either user-owned or
  assistant-owned before presenting it.
- **FR-002**: Assistant-owned worktrees MUST NOT appear in the sidebar's worktree list while the
  reveal control (FR-010) is off.
- **FR-003**: While hidden, assistant-owned worktrees MUST be excluded from every quantity and
  state derived from the worktree list, including filter results, worktree counts, and the
  decision to show an empty state.
- **FR-004**: While hidden, assistant-owned worktrees MUST NOT be offered as a target for any user
  action the app provides over worktrees — starting a session, renaming, or deleting.
- **FR-005**: A worktree MUST be classified as assistant-owned only when it is located directly
  under the project's managed worktrees directory AND its directory name or its bound branch name
  follows the assistant's reserved naming convention: the reserved prefix followed by a
  machine-generated identifier of **at least 16 characters, every one of them hexadecimal**.
- **FR-006**: A worktree MUST remain visible whenever the text following the reserved prefix falls
  short of that rule — because it is shorter than 16 characters, or because any part of it is not
  a hexadecimal character. A name that merely starts with or contains the reserved prefix as
  ordinary words is therefore never hidden.
- **FR-007**: The hiding rule MUST apply regardless of a worktree's health state, so that
  assistant-owned worktrees that are registered-but-missing or present-but-unregistered are hidden
  too rather than surfacing as broken entries.
- **FR-008**: Hiding MUST be presentation-only: the app MUST NOT delete, prune, rename, check out,
  or otherwise modify an assistant-owned worktree, its directory, or its branch as a consequence
  of hiding it.
- **FR-009**: Classification MUST be re-evaluated whenever the app refreshes its view of a
  project's worktrees, so assistant-owned worktrees that appear or disappear during a session
  never become visible.
- **FR-010**: The sidebar's filter panel MUST offer a reveal control, labelled
  **"Show agent worktrees"**, that includes assistant-owned worktrees in the list while it is on.
- **FR-010a**: The reveal control MUST be off — assistant-owned worktrees hidden — every time the
  app starts, regardless of how it was left in a previous run.
- **FR-010e**: The reveal control MUST reset to off whenever the user switches to a different
  project, so every project is entered with assistant-owned worktrees hidden. It is never carried
  from one project to another.
- **FR-010b**: While the reveal control is on, assistant-owned worktrees MUST be visually
  distinguishable from user-created ones — each carrying a badge reading **`agent`** — so a
  revealed entry can never be mistaken for the user's own work.
- **FR-010c**: The reveal control MUST be discoverable in the filter panel even when the project
  currently contains no assistant-owned worktrees, and switching it on in that case MUST leave the
  list unchanged.
- **FR-010d**: Toggling the reveal control MUST NOT change, clear, or otherwise alter any active
  tag filters, and tag filters MUST continue to apply to revealed entries the same way they apply
  to user-created ones.
- **FR-011**: When a session recorded by the app is bound to a worktree that is now classified as
  assistant-owned, the worktree MUST stay hidden and the session MUST be handled by the app's
  existing behavior for a session whose worktree is unavailable — no dedicated handling is added
  for this case.
- **FR-012**: The user-facing documentation MUST state that assistant-owned worktrees exist, that
  the app hides them by default, how to reveal them, and that their lifecycle belongs to the
  assistant rather than the app.
- **FR-013**: While revealed, an assistant-owned worktree MUST offer exactly the same row actions
  as a user-created one — starting a session, renaming, and deleting — with no action disabled,
  hidden, or given an extra confirmation step because the worktree is assistant-owned. Deletion
  remains behind the app's existing delete confirmation, which is the only guard.

### Key Entities

- **Worktree entry**: an isolated working copy the app presents to the user, identified by its
  directory name under the project's managed worktrees directory and its bound branch, and carrying
  a health state.
- **Ownership classification**: a derived property of a worktree entry — user-owned or
  assistant-owned — determined solely from the worktree's location and its naming, with no extra
  stored state or user input.
- **Reserved naming convention**: the pattern that marks a worktree as assistant-owned — a fixed
  reserved prefix on the directory name or branch name, followed by a machine-generated identifier
  of at least 16 characters, all hexadecimal.
- **Reveal control**: an on/off sidebar filter-panel option that decides whether assistant-owned
  worktrees are included in the list. Off at every app start and again on every project switch, so
  it applies only to the project it was switched on for. Independent of the tag filters it sits
  alongside.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a project containing any number of assistant-owned worktrees, zero of them appear
  anywhere in the app's worktree surfaces in a default run — that is, without the user switching
  the reveal control on.
- **SC-002**: 100% of user-created worktrees remain visible — across a naming corpus that
  deliberately includes names sharing the reserved prefix, the hiding rule produces zero false
  positives.
- **SC-003**: A user scanning the worktree list can identify their own worktrees without needing
  to skip over any entry they did not create.
- **SC-003a**: A user who wants to inspect assistant-owned worktrees can reveal them from the
  sidebar in a single action, without leaving the app or editing any configuration.
- **SC-004**: Opening a project and rendering its worktree list is no slower than before the
  feature, with no user-perceptible delay introduced by the classification step.
- **SC-005**: After the app has been running with assistant-owned worktrees present, the on-disk
  state of those worktrees and their branches is byte-for-byte unchanged by the app.

### Terminology

This spec says **assistant-owned** in its prose to stay vendor-neutral about which tool created a
worktree. All user-visible text — the reveal control, the row badge, and the user-guide section
required by FR-012 — uses **agent** instead, matching the `agent-` / `worktree-agent-` names the
user can already see on disk. The two terms refer to exactly the same thing; no user-facing string
should say "assistant".

## Assumptions

- The assistant's worktree naming convention (reserved prefix plus a long machine-generated
  hexadecimal identifier, under the project's managed worktrees directory) is stable enough to
  detect by name. Name-based detection is the only signal available, because the assistant leaves
  no other marker behind.
- The detection rule is built in and fixed; it is not a user-configurable pattern. No requirement
  for user-defined hide rules is in scope.
- The assistant owns the full lifecycle of its worktrees — creating and cleaning them up. The app
  takes no responsibility for removing leftovers, so no cleanup or "prune abandoned assistant
  worktrees" capability is in scope.
- Hiding applies to the app's own worktree surfaces only. It changes nothing about the underlying
  repository, and worktrees remain fully visible to git and to any terminal the user opens.
- No migration or persisted state is needed: classification is recomputed from names on every
  refresh, so the feature takes effect immediately and can be reverted by removing the rule.
- The reveal control is transient — never persisted. Its scope is the current project in the
  current run: the safe default (hidden) is restored both on every launch and on every project
  switch. Persisting it, or remembering it per project, are deliberate non-goals.
- The accepted consequence of FR-013 is that a worktree an assistant is actively using can be
  deleted from the app, exactly as it could from a terminal. The user opted in by revealing the
  row, and the existing delete confirmation is judged sufficient protection.
- Hiding is not itself a cleanup mechanism, and no notification, badge, or prompt about leftover
  assistant worktrees is in scope.
