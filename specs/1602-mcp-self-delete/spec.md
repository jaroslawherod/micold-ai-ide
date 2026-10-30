# Feature Specification: MCP: a session ends itself and removes its own worktree

**Feature Branch**: `feat/1602_mcp-let-a-session-stop-itself-and-delete-its-own`

**Created**: 2026-10-29

**Status**: Closed 2026-10-30 — shipped in PRs #1611, #1613, #1619, #1621, and the close PR

**Input**: GitHub issue #1602 — "MCP: let a session stop itself and delete its own worktree (after confirmation)". Labels: enhancement, flow:feature.

## Clarifications

### Session 2026-10-29

- Q: Is story 4 (`stop_session`/`delete_session` on self) in scope? → A: Yes, same confirmation and deferred execution. _(decided by user)_
- Q: How is a failed deferred stop or removal reported? → A: App notice naming W and the reason, plus a daemon log entry; the session stays stopped. _(decided by user)_

## Background

The daemon's MCP server (feature 034-daemon-mcp-server) refuses by policy (FR-015) any request by which a session stops itself, deletes itself, or deletes the worktree it runs in: `stop_session` and `delete_session` on the caller's own id, and `delete_worktree` on the caller's own worktree. An agent that finishes its work (for example a speckit-autopilot run whose pull requests are merged) can therefore only tell the user to close the session and delete the worktree by hand, or to ask another session to do it. The refusal exists because an operation that kills the caller while it waits for the answer would lose the answer. It is not a statement that a session may never end itself.

Today the confirmation prompt for `delete_worktree` names the caller, the operation and the worktree, but does not show whether the worktree holds uncommitted changes or unpushed commits, and the tool does not check either.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A finished session removes its own worktree (Priority: P1)

An agent running in worktree W calls `delete_worktree` for W with `stop_sessions: true`. The user confirms in an app window. The agent receives a success result, and afterwards the app stops the agent's session and removes W and its branch.

**Why this priority**: This is the request. Without it the agent cannot finish its own cleanup.

**Independent Test**: Start a session in a scratch worktree with a clean tree, call the tool from that session, accept the prompt, and check that the result arrived and that the session and worktree are gone afterwards.

**Acceptance Scenarios**:

1. **Given** a session S in worktree W, **When** S calls `delete_worktree(worktree: W, stop_sessions: true)`, **Then** a confirmation prompt appears in every connected window naming S, the operation and W, and nothing changes until the user answers.
2. **Given** the prompt from scenario 1, **When** the user accepts, **Then** the tool returns a success result to S first, and only after that result is delivered the app stops S (and any other session in W) and removes W and its branch.
3. **Given** the same call with `delete_branch: false`, **When** the user accepts, **Then** W is removed and its branch is kept.
4. **Given** the prompt from scenario 1, **When** the user declines, **Then** S receives a refusal ("declined by the user") and no session, worktree or branch changes.
5. **Given** the prompt from scenario 1, **When** nobody answers within 60 seconds or no window is attached, **Then** S receives "needs confirmation" and nothing changes.
6. **Given** a session S in W, **When** S calls `delete_worktree` for W with `stop_sessions` false or absent and W has live sessions (S itself is one), **Then** the call is refused with the live-sessions conflict, as for any worktree, and no prompt appears.

---

### User Story 2 - The prompt shows what would be lost (Priority: P1)

Before the user confirms a `delete_worktree`, the prompt lists the worktree's uncommitted changes and its unpushed commits, so an agent cannot discard work the user has not seen.

**Why this priority**: Self-removal makes silent data loss easy, since the agent decides when to ask. The issue's acceptance criteria require it.

**Independent Test**: Make W dirty (an edited tracked file, an untracked file) and give its branch a commit that is not on its upstream, call the tool, and read the prompt.

**Acceptance Scenarios**:

1. **Given** W holds uncommitted changes, **When** the prompt for deleting W appears, **Then** it lists the changed files.
2. **Given** W's branch has commits not pushed to its upstream (or has no upstream and holds commits not on any remote branch), **When** the prompt appears, **Then** it lists those commits.
3. **Given** W is clean and fully pushed, **When** the prompt appears, **Then** it shows no such list.
4. **Given** the prompt lists changes or commits, **When** the user accepts, **Then** the deletion proceeds as in story 1; declining leaves everything unchanged.
5. **Given** the target is a worktree other than the caller's own, **When** `delete_worktree` is called, **Then** the same lists appear in its prompt.

---

### User Story 3 - The Default session and other guards stay (Priority: P1)

The existing refusals that protect the project root and other users of the worktree keep working.

**Why this priority**: Opening the self-delete path must not weaken Principle III.

**Independent Test**: Call `delete_worktree` from the Default session, and from a session in worktree A targeting an unknown worktree.

**Acceptance Scenarios**:

1. **Given** the caller is a Default session (runs in the project root), **When** it calls `delete_worktree` for any worktree, including with `worktree: "default"`, **Then** the call is refused by policy (034 FR-015a) with no prompt.
2. **Given** the caller's own worktree is named, **When** the worktree no longer exists or is outside the caller's project, **Then** the call fails "not found" with no prompt.

---

### User Story 4 - A session stops or deletes only itself (Priority: P3)

`stop_session` and `delete_session` accept no id, or the caller's own id, and act on the calling session with the same confirmation and deferred execution.

**Why this priority**: Optional in the issue, in scope by decision; the `delete_worktree` path covers the main need.

**Independent Test**: From session S, call `stop_session` without an id, accept, and check S's result arrived before S stopped.

**Acceptance Scenarios**:

1. **Given** session S, **When** S calls `stop_session` with no id or with its own id, **Then** the prompt names S as both caller and target, and on accept S gets the success result before it is stopped.
2. **Given** the same for `delete_session`, **When** the user accepts, **Then** S gets the result first, then S is stopped and removed from the catalog.
3. **Given** the user declines or no answer arrives, **Then** S keeps running and receives the refusal or "needs confirmation".

---

### Edge Cases

- **Answer lost on stop**: the result must be delivered before the caller stops. The caller receives the success result before it stops; if its connection is already closed, the confirmed stop and removal still happen.
- **Deferred step fails** (git error removing W, the branch is not deletable, a leftover file): the agent is gone and cannot be told. The daemon logs the failure and the app shows a notice naming W and the reason; the session stays stopped.
- **Other sessions in W**: with `stop_sessions: true` they stop as for any worktree delete; the prompt names that sessions will stop.
- **Concurrency (Principle II)**: another session in the project deletes or renames W while the prompt is pending: the request ends "not found" or is withdrawn as FR-014 already specifies, and the deferred step never runs. Two sessions in W both asking: the first accepted deletion wins; the other gets "not found" or its caller stops with the first.
- **Caller closed while prompt pending**: the prompt is withdrawn (existing behavior) and nothing runs.
- **Dirty or unpushed state changes between the prompt and the deferred step**: the lists reflect the state when the prompt was shown; the user's acceptance applies to that state.
- **Git unavailable or W is not a repository worktree**: the prompt shows that the state could not be read and the user may still accept; the deletion then fails or succeeds as the sidebar's delete does.
- **Windows and macOS (Principle VI)**: removing a worktree whose directory the stopped session's shell still holds open must work on all three platforms, and the worktree directory is gone on all three platforms even when the stopped session's shell had it open.
- **Branch is checked out elsewhere or protected**: `branch_delete_failed` is reported as for any delete (feature 013-create-worktree-refinement FR-015); the worktree removal still counts as done.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `delete_worktree` MUST accept the caller's own worktree. The policy refusal for it (034 FR-015, "the calling session runs in this worktree") MUST be replaced by the same user confirmation every other `delete_worktree` gets (034 FR-014).
- **FR-002**: For the caller's own worktree the tool MUST require `stop_sessions: true`; otherwise it fails with the live-sessions conflict (the caller is a live session of that worktree).
- **FR-003**: After the user accepts a delete of the caller's own worktree, the tool MUST return its success result to the caller first, and the app MUST stop the caller's session (and the other live sessions of W) and remove W only after that result has been delivered.
- **FR-004**: The deferred removal MUST delete W's branch unless `delete_branch` is `false`, with the same outcome rules as a sidebar delete.
- **FR-005**: Nothing about the worktree, branch or any session MUST change before the user accepts; a decline, timeout, missing window, or withdrawn prompt leaves everything unchanged and returns the category 034 FR-014 defines.
- **FR-006**: The confirmation prompt of every `delete_worktree` MUST list the worktree's uncommitted changes (changed and untracked files) and its unpushed commits, and MUST show neither list when there are none.
- **FR-007**: A Default session MUST keep being refused `delete_worktree` and `rename_worktree` by policy (034 FR-015a); `worktree: "default"` MUST stay invalid input.
- **FR-008**: The result of an accepted self-delete MUST say that the stop and removal are scheduled, not that they are done, and MUST report the worktree directory name and whether the branch will be deleted.
- **FR-009**: A failure of the deferred stop or removal MUST be recorded in the daemon log and shown to the user in the app. The agent is not expected to receive it. The app notice names the worktree and the reason; the session stays stopped.
- **FR-010**: Every accepted or refused self-delete MUST write the audit line 034 FR-018 requires for mutating tools.
- **FR-011**: `stop_session` and `delete_session` MUST accept an omitted id or the caller's own id, MUST ask the user to confirm, and MUST follow FR-003 (result first, then the action). `interrupt_session`, `send_session_input` and `read_session_output` on the caller itself MUST stay refused as invalid input.
- **FR-012**: The tool descriptions in `tools/list` and the user documentation of the MCP tools MUST describe the new behavior and drop the statements that a session cannot stop or delete itself or its own worktree.
- **FR-017**: Feature 034's FR-014 (destructive operations), FR-015 (self-targeted refusals) and its tool table MUST be amended alongside FR-012 to match this feature.
- **FR-016**: A Default session MAY stop or delete itself under story 4 (Principle III forbids only modifying or removing worktrees); FR-007 is unchanged.
- **FR-013**: The tools' behavior for a target other than the caller (another session, another worktree) MUST be unchanged except for FR-006.

### Key Entities

- **Self-targeted request**: a `delete_worktree`, `stop_session` or `delete_session` whose target is the calling session or the worktree it runs in.
- **Deferred action**: the stop and removal the app performs after a self-targeted request was confirmed and its result delivered.
- **Loss summary**: the uncommitted files and unpushed commits of a worktree, shown in its delete prompt.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of accepted self-deletes of a clean worktree, the caller receives the success result, and within 10 seconds of that result the session is stopped and neither the worktree directory nor its branch exists (unless `delete_branch: false`).
- **SC-002**: In 100% of declined, timed-out or withdrawn self-deletes, no session, worktree or branch differs from its state before the call.
- **SC-003**: For a worktree with N uncommitted files and M unpushed commits, the prompt shows all N and all M (or states a count and shows a bounded prefix when N or M exceeds 50) in 100% of tested cases.
- **SC-004**: A Default session's `delete_worktree` is refused in 100% of cases, with no prompt shown.
- **SC-005**: All acceptance scenarios of feature 034-daemon-mcp-server not about self-targeted requests pass unchanged.
- **SC-006**: The same scenarios pass on Linux, macOS and Windows.

## Assumptions

- The app's existing confirmation channel (034 FR-014, 60-second timeout, all windows) is reused; no new dialog type is added beyond the loss summary.
- Uncommitted and unpushed state is read with git at the moment the prompt is built. A worktree whose branch has no upstream counts as having unpushed the commits not reachable from any remote-tracking branch.
- The caller's session is identified as today by the binding its MCP connection carries.
- A bounded list (first 50 entries plus a count) is acceptable for very large changes.

## Out of Scope

- Letting an agent delete a worktree it does not run in, or another project's worktrees, beyond what 034 already allows.
- Letting a Default session delete or rename worktrees (Principle III).
- Interrupting, reading or typing into itself.
- Any retry or rollback of a failed deferred removal, and any message back to the dead agent.
- Changing the sidebar's own delete dialog.
- Auto-confirming self-deletes by a Settings option.
