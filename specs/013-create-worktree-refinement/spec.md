# Feature Specification: Worktree Creation & Deletion Flow Refinement

**Feature Branch**: `013-create-worktree-refinement`

**Created**: 2026-07-21

**Status**: Draft

**Input**: User description: "change the overlay form to create new worktree instead of radio buttons create a select from list. Create material select item component which will fallow material design look. After clicking create show progress bar with information what is happening and what staging is happenging. The delete of worktree should also ask if the branch should be deleted"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Choose the worktree type from a list instead of a row of buttons (Priority: P1)

Today, when a user opens the "create worktree" form, the type of change (feat, fix, chore, docs, refactor, test, build, ci, perf, style) is chosen from a row of individually clickable buttons, one per type. A user opening the form wants to instead pick the type from a single, compact list control — matching the look and interaction pattern of other dropdown/list-style controls already used elsewhere in the app — so the form reads consistently and doesn't present ten separate buttons at once.

**Why this priority**: This is the form control the user interacts with first, on every single worktree creation, and is the change explicitly called out as the primary ask. It also introduces a reusable list-selection control other parts of the form (and future forms) can build on.

**Independent Test**: Can be fully tested by opening the create-worktree overlay, using the new list control to choose a type, and confirming the selected type drives the same derived name preview and creation behavior as before — without touching progress display or deletion behavior.

**Acceptance Scenarios**:

1. **Given** the create-worktree overlay is open, **When** the user opens the type control, **Then** all available types (feat, fix, chore, docs, refactor, test, build, ci, perf, style) are listed for selection, and none are pre-selected unless the user has already chosen one earlier in this session.
2. **Given** the type list is open, **When** the user picks a type, **Then** the list closes, the control displays the chosen type, and the directory/branch name preview updates to reflect it — the same outcome as clicking a type button today.
3. **Given** a type is already selected, **When** the user reopens the list, **Then** the previously selected type is visibly indicated as the current selection.
4. **Given** no type has been selected, **When** the user attempts to submit the form, **Then** creation is rejected with the same validation message shown today (unchanged behavior).

---

### User Story 2 - Decide whether the branch is also deleted when deleting a worktree (Priority: P2)

Today, confirming "delete worktree" always deletes the worktree's directory, its sessions, and its git branch together — there is no way to remove the worktree while keeping the branch. A user who wants to stop working in a worktree but keep the branch around (e.g., to pick it up again later, or because someone else wants to check it out) needs the confirmation step to let them choose.

**Why this priority**: Branch deletion is the one part of today's delete flow that destroys something the user may still want (a line of commits), so giving them a choice prevents unwanted, hard-to-recover data loss. It's independent of the type-selection and progress-bar work.

**Independent Test**: Can be fully tested by requesting deletion of a worktree, choosing to keep the branch in the confirmation step, confirming, and verifying the worktree directory and its sessions are gone while the git branch still exists — independent of anything else in this feature.

**Acceptance Scenarios**:

1. **Given** a worktree delete is requested, **When** the confirmation dialog appears, **Then** it presents an explicit choice for whether the associated git branch should also be deleted, in addition to the existing explanation of what gets removed.
2. **Given** the confirmation dialog is showing, **When** the user leaves the branch-deletion choice at its default and confirms, **Then** the branch is deleted along with the worktree directory and sessions — matching today's behavior for anyone who doesn't change the default.
3. **Given** the confirmation dialog is showing, **When** the user opts to keep the branch and confirms, **Then** the worktree directory and its sessions are removed but the git branch remains in the repository afterward.
4. **Given** the user opted to keep the branch, **When** deletion completes, **Then** the branch is not offered again from within this worktree's now-removed entry, but remains an ordinary branch usable elsewhere in the repository (e.g., a future worktree could be created from it).

---

### User Story 3 - See what's happening while a worktree is being created (Priority: P3)

Today, after clicking "Create," the user sees a static "Creating worktree…" message plus a scrolling text log. A user wants a clearer, more visual indication that creation is actively progressing and which specific step is currently happening (for example, creating the branch versus setting up submodules), especially for repositories with submodules where creation takes noticeably longer.

**Why this priority**: This improves confidence that the app hasn't frozen during a longer-running operation, but the operation already reports progress via the text log today, so this is a refinement of existing feedback rather than filling a gap.

**Independent Test**: Can be fully tested by creating a worktree (with and without submodules) and confirming a continuously visible progress indicator is shown alongside a current-stage description that changes as creation moves from one stage to the next, then disappears on completion or failure.

**Acceptance Scenarios**:

1. **Given** the user has filled in a valid form, **When** they click "Create," **Then** a continuously visible progress indicator appears immediately, replacing the static "Creating worktree…" text.
2. **Given** creation is in progress, **When** the operation moves from one stage to the next (e.g., from creating the branch/worktree to setting up submodules), **Then** the displayed stage description updates to name the current stage in plain language.
3. **Given** the target repository has no submodules, **When** creation runs, **Then** no submodule-setup stage is shown as pending or active — only the stages that actually apply are represented.
4. **Given** creation fails partway through (e.g., during submodule setup), **When** the failure occurs, **Then** the progress indicator stops advancing, the stage at which it failed is identifiable, and the existing failure message is shown instead of the indicator continuing to imply progress.
5. **Given** creation completes successfully, **When** the last stage finishes, **Then** the progress indicator and stage description are cleared and the overlay closes, same as today's successful-creation behavior.

---

### Edge Cases

- What happens if the user opens the type list, then closes it without picking anything (e.g. by clicking the control again)? The list closes and the previous selection (if any) is unchanged.
- What happens if the user picks a branch-deletion choice, then cancels the delete confirmation instead of confirming? Nothing is removed — the choice is discarded along with the rest of the cancelled confirmation, matching today's "cancel removes nothing" behavior.
- What happens if the user opts to delete the branch, but git refuses to delete it (for example, because it has commits not present anywhere else)? The system must surface this as a distinct, specific failure rather than silently keeping the branch or silently forcing deletion — the worktree directory and session removal should still be treated as having succeeded even if the branch step fails, consistent with today's "already-removed target counts as success" handling of other cleanup steps.
- What happens if the worktree being deleted has no meaningful branch to offer a choice about (edge/unexpected repository state)? The branch-deletion choice is only presented when the worktree has an associated branch to act on; the rest of the deletion proceeds unchanged if not.
- What happens if creation fails before any stage-specific work begins (during the existing pre-flight duplicate checks)? The progress indicator should reflect that failure occurred at the earliest stage rather than appearing to have made partial progress.
- What happens to the "Create"/"Cancel" buttons while the progress indicator is showing? Unchanged from today — creation continues to run without the user needing to keep the button available for further action.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The create-worktree form MUST present the worktree-type choice (feat, fix, chore, docs, refactor, test, build, ci, perf, style) as a single list/select control that opens to show all types and closes on selection, replacing today's row of individually clickable type buttons.
- **FR-002**: The list/select control MUST visually and behaviorally match the app's existing Material Design language (consistent look, motion, and interaction pattern with the app's other dropdown/overlay-style controls), as a reusable control rather than a one-off widget specific to this form.
- **FR-003**: The list/select control MUST clearly display the currently selected type when closed, and MUST indicate which type is selected when reopened.
- **FR-004**: Selecting a type from the list MUST update the derived directory/branch name preview exactly as selecting a type button does today.
- **FR-005**: The form MUST continue to require that a type be selected before creation can be submitted, and MUST show the existing validation message when it is not — this feature does not change validation behavior, only the control's presentation.
- **FR-006**: Upon the user clicking "Create" with a valid form, the overlay MUST show a continuously visible progress indicator for the duration of the creation operation, in place of today's static "Creating worktree…" text.
- **FR-007**: The progress indicator MUST be accompanied by a plain-language description of the current stage being performed (for example: checking for naming conflicts, creating the branch and worktree, setting up submodules), and MUST update this description as the operation moves between stages.
- **FR-008**: The progress display MUST only represent stages that actually apply to the current creation (for example, no submodule-setup stage is shown when the target repository has no submodules).
- **FR-008a**: FR-006's "for the duration of the creation operation" MUST hold for a stage that lasts minutes as well as one that lasts a moment — including across the session-service boundary the operation now runs behind. Reporting only stage *transitions* is permitted as a display rule, but MUST NOT become the only traffic that keeps that boundary's connection alive: a stage that legitimately produces nothing for longer than the service's liveness deadline MUST still leave the indicator on screen and the connection healthy (see `010-daemon-session-persistence` FR-026a).
  **Bugfix**: 2026-08-06 — BUG-009 added this requirement; a submodule-setup stage emitted one frame and then nothing, the client's liveness deadline reaped the connection at 9 s, and the progress display was replaced mid-creation by a disconnect banner — the "is it hung?" state FR-006 exists to prevent. See `010-daemon-session-persistence/bugs/BUG-009.md`.
- **FR-009**: If creation fails at any stage, the progress indicator MUST stop advancing and the failed stage MUST be identifiable from the display, followed by the existing failure/error message — the indicator must never continue to suggest progress after a failure.
- **FR-010**: On successful creation, the progress indicator and any stage description MUST be cleared and the overlay MUST close, consistent with today's successful-creation behavior.
- **FR-011**: The worktree-delete confirmation MUST let the user explicitly choose whether the worktree's associated git branch is also deleted, in addition to the existing explanation of what is being removed.
- **FR-012**: The delete confirmation's default branch-deletion choice MUST be "delete the branch," preserving today's behavior for a user who confirms without changing it.
- **FR-013**: If the user opts to keep the branch, confirming deletion MUST remove the worktree directory and its sessions while leaving the git branch untouched and intact in the repository.
- **FR-014**: If the user opts to delete the branch and confirms, the system MUST behave as it does today: the worktree directory, its sessions, and the git branch are all removed.
- **FR-015**: If the user opts to delete the branch but the branch cannot be deleted (for example, it holds commits unreachable from elsewhere), the system MUST report this as a specific, distinguishable failure rather than silently discarding the choice or silently force-deleting the branch; the worktree directory and session removal are still treated as successful independent of this failure.
- **FR-016**: Cancelling the delete confirmation MUST discard the branch-deletion choice along with the rest of the confirmation, removing nothing — unchanged from today's cancel behavior.

### Key Entities

- **Worktree Type Selection**: The Conventional-Commit-style category (feat, fix, chore, docs, refactor, test, build, ci, perf, style) chosen for a new worktree; unchanged in meaning from today, now presented and chosen through a list/select control instead of a row of buttons.
- **Worktree Creation Progress**: An ordered sequence of named stages a creation operation moves through (naming/duplicate checks, branch and worktree creation, conditional submodule setup), together with a current-stage indicator and a success/failure outcome, shown to the user for the duration of creation.
- **Worktree Deletion Choice**: The user's explicit, per-deletion decision — made at confirmation time — on whether the worktree's associated git branch is deleted alongside the worktree directory and its sessions, or preserved.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can select a worktree type in a single open-then-pick interaction with the new list control, with the currently selected type always visible without needing to scan multiple buttons at once.
- **SC-002**: 100% of worktree-creation attempts that take longer than a couple of seconds show a specific, current-stage description at some point beyond the initial state — never only a generic "creating…" message for the entire duration.
- **SC-003**: 100% of worktree deletions require the user to see and confirm (or change) an explicit branch-deletion choice before anything is removed; zero git branches are deleted as an unannounced side effect of removing a worktree.
- **SC-004**: Users who choose to keep a branch when deleting its worktree can confirm afterward that the branch still exists in the repository, while the worktree directory and its sessions are gone.
- **SC-005**: Creation failures at any stage are reported with the specific stage identifiable, eliminating cases where the user only sees a generic failure with no sense of how far creation got.

## Assumptions

- The set of worktree-type values (feat, fix, chore, docs, refactor, test, build, ci, perf, style) is unchanged by this feature — only how the user picks among them changes.
- The progress indicator communicates stage-by-stage advancement rather than a numeric percentage, since at least one stage (submodule setup) has unpredictable duration that can't be reliably expressed as a percentage of total time.
- The delete confirmation's branch-deletion choice defaults to "delete the branch," matching today's unconditional behavior, so a user who doesn't engage with the new choice sees no behavior change.
- When the user opts to keep the branch during deletion, no automatic follow-up action (renaming, tagging, or reusing the branch) occurs — it simply remains an ordinary local branch in the repository.
- This feature does not change the underlying git operations used for creation (worktree add, conditional submodule update) or deletion (worktree remove, prune, conditional branch delete) — it changes how their progress and choices are presented to the user.
- This feature does not add, remove, or reorder any creation or deletion stages beyond what already exists; it makes existing stages visible where they weren't before.


---

**Bugfix**: 2026-08-06 — BUG-009 Added FR-008a: reporting only stage transitions is a display rule and
must not double as the only traffic keeping the session-service connection alive, since a stage can
legitimately be silent for longer than that connection's liveness deadline. **No task reopened** —
this feature's stage model is intact; the defect is in how the service thins it on the wire. The fix
is `010-daemon-session-persistence` Phase 22 (T120, T123). See
`010-daemon-session-persistence/bugs/BUG-009.md`.
