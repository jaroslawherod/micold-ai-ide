# Feature Specification: Switchable Regular Terminal Mode

**Feature Branch**: `feat/switch-to-regular-terminal` (spec `010-regular-terminal-mode`)

**Created**: 2026-07-18

**Status**: Closed

**Input**: User description: "Let the user switch a session's embedded terminal between running the `claude` CLI (the current default) and a regular shell, without losing the terminal pane's real-terminal behavior (colors, live keystroke input, scrollback, focus gating) established in feature 006. The user needs to occasionally run ordinary shell commands (git, package managers, ad-hoc scripts) in the same worktree the session is scoped to, without leaving the app or losing their place in the `claude` conversation. Add a toggle/action (e.g. in the terminal's toolbar dropdown, alongside the existing Settings item) that switches the active pane between 'AI CLI' mode and 'regular terminal' mode for the current session. Switching to regular terminal mode should start (or resume) a plain shell process in the session's worktree directory. Switching back to AI CLI mode should resume the existing `claude` session (same session id, `--resume`) rather than starting a new conversation — the `claude` conversation must survive round-trips through regular-terminal mode. Decide and specify: whether the `claude` process is suspended/killed while in regular-terminal mode or kept running in the background; what happens to a regular shell process when switching away (killed vs. kept alive and resumable); whether each session remembers its own last-used mode across restarts of the app; and how the current mode is indicated visually so the user always knows which process their keystrokes are going to."

## Clarifications

### Session 2026-07-18

- Q: Where should the AI CLI / Regular Terminal mode toggle live, and how should it be presented? → A: In the terminal's existing bottom status bar (the same bar that already shows the session name and lifecycle status), as a single icon button. The button's icon (and label/tooltip) changes to reflect the currently active mode, so the button itself doubles as the mode indicator rather than needing a separate indicator alongside it.
- Q: The AI CLI process already has crash-loop protection (auto-restart on unexpected exit, up to 3 attempts, then a Failed state). Should the new shell process get the same automatic-restart behavior, or is restart always manual for the shell? → A: Manual restart only. A shell exiting — whether the user typed `exit` or it crashed — always shows a not-running state with a manual restart affordance; there is no automatic retry loop for the shell process, unlike the AI CLI process.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Drop into a regular shell without leaving the session (Priority: P1)

A developer is mid-conversation with `claude` in a session and needs to run an ordinary shell command — check `git status`, run a package manager, inspect a file — in the exact same worktree. They switch the session's terminal to Regular Terminal mode, get a plain shell scoped to that worktree, run their commands, and switch back.

**Why this priority**: This is the entire reason for the feature — being able to reach a plain shell without leaving the app or losing the AI CLI's place. Nothing else in the feature has value without this.

**Independent Test**: Open a session with `claude` running, use the mode toggle to switch to Regular Terminal mode, confirm a plain shell is running with the working directory set to the session's worktree, run a shell command, and confirm it executes normally.

**Acceptance Scenarios**:

1. **Given** a session displaying its AI CLI terminal, **When** the user activates the Regular Terminal toggle, **Then** the pane now shows a plain shell process whose working directory is the session's worktree directory.
2. **Given** the terminal is in Regular Terminal mode, **When** the user types a shell command and presses Enter, **Then** the command executes and its output is rendered exactly as it would be in a standalone terminal (colors, live input, scrollback all apply, per feature 006).
3. **Given** the terminal is in Regular Terminal mode, **When** the user activates the AI CLI toggle, **Then** the pane shows the `claude` process again.
4. **Given** a session whose AI CLI process is not running (it exited, or the session has not been relaunched since the service restarted), **When** the user activates the Regular Terminal toggle, **Then** a plain shell starts and is shown, exactly as it would be for a session whose `claude` is running — and the AI CLI is not started as a side effect. *(Added by BUG-001.)*

---

### User Story 2 - The `claude` conversation survives round-trips (Priority: P1)

A developer switches to Regular Terminal mode mid-conversation, runs a few commands, and switches back to AI CLI mode. The `claude` conversation is exactly where they left it — no restarted session, no lost context, no re-shown startup banner for an already-active conversation.

**Why this priority**: If switching away risks or costs the AI CLI conversation, the feature actively works against its own purpose (the user description explicitly requires the conversation to "survive round-trips"). This is as critical as Story 1.

**Independent Test**: Start a `claude` conversation, exchange at least one message, switch to Regular Terminal mode, run a command, switch back to AI CLI mode, and confirm the conversation view shows the same history with no new/duplicate session and no interruption to a turn that was in progress.

**Acceptance Scenarios**:

1. **Given** an active `claude` conversation with history, **When** the user switches to Regular Terminal mode and back to AI CLI mode, **Then** the same conversation (same session id) is shown with its full prior history intact.
2. **Given** `claude` is mid-turn (actively generating a response) when the user switches to Regular Terminal mode, **When** the user switches back to AI CLI mode, **Then** the turn has continued uninterrupted in the background and its output is visible.
3. **Given** the `claude` process happens to have exited while Regular Terminal mode was active, **When** the user switches back to AI CLI mode, **Then** the same conversation is automatically resumed (not started fresh) with no extra action required from the user.

---

### User Story 3 - Always know which process is listening (Priority: P2)

A developer glances at the terminal pane and can immediately tell, without typing anything, whether their next keystrokes will go to `claude` or to a plain shell.

**Why this priority**: Without a clear indicator, a user could type commands intended for one process into the other (e.g., shell commands into `claude`, or a `claude` slash command into a shell) — a usability and trust problem, but the feature is still functional without it, unlike Stories 1–2.

**Independent Test**: Toggle between modes and confirm the bottom status bar's toggle button icon is a distinct, unambiguous indicator of the active mode at all times, updating immediately on switch.

**Acceptance Scenarios**:

1. **Given** the terminal is in AI CLI mode, **When** the user looks at the bottom status bar, **Then** the toggle button's icon clearly identifies the active mode as the AI CLI.
2. **Given** the terminal is in Regular Terminal mode, **When** the user looks at the bottom status bar, **Then** the toggle button's icon clearly identifies the active mode as a regular/plain terminal.
3. **Given** the user activates the mode toggle button, **When** the switch completes, **Then** the button's icon updates immediately to match the new mode.

---

### Edge Cases

- What happens if the user closes or deletes a session/worktree while its regular-terminal shell process is running in the background? The shell process MUST be terminated as part of that session's teardown, the same way the AI CLI process is today.
- What happens if the plain shell exits (e.g., the user types `exit`) or crashes while Regular Terminal mode is active? The pane reflects a not-running shell with a manual restart affordance — the shell process never auto-restarts, unlike the AI CLI process's crash-loop behavior — and the app does not force a switch back to AI CLI mode.
- What happens if the platform's default shell cannot be determined or fails to launch? The terminal shows the same kind of failure/error state used for an AI CLI launch failure, rather than crashing or silently doing nothing.
- What happens to focus gating, the reserved focus-release shortcut, and copy/paste behavior (feature 006) while in Regular Terminal mode? They behave identically to AI CLI mode — this feature only changes which process the pane is attached to, not the pane's interaction model.
- What happens if the user switches modes rapidly back and forth several times? Each switch just reattaches to an already-running process, so it is immediate and never triggers a relaunch of either process.
- What happens if the user runs `claude` manually from inside the regular shell? It is treated as an ordinary command the shell runs; it is not connected to the session's own AI CLI process or its mode state.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST let the user switch a session's embedded terminal between AI CLI mode (running `claude`) and Regular Terminal mode (running a plain shell) via a single icon button in the terminal's bottom status bar (the bar that already shows the session name and lifecycle status).
- **FR-002**: The mode toggle button MUST be reachable whenever a session's terminal is displayed, in either mode.
- **FR-003**: Switching to Regular Terminal mode MUST start a plain shell process, scoped to the session's worktree directory, if one is not already running for that session.
  - **Clarified by BUG-002 (this feature's BUG-001)**: "if one is not already running" is the only
    precondition. In particular this MUST NOT depend on the session's *AI CLI* process running —
    a session whose `claude` has exited, failed to start, or has not been relaunched since a
    service restart still gets a shell on the toggle. Nor may switching to Regular Terminal mode
    start the AI CLI as a side effect: resuming a conversation is what switching to *AI CLI* mode
    does (FR-005), and doing it unasked would resume a conversation the user did not ask for.
- **FR-004**: If a shell process was already started for the session during a prior switch, switching back to Regular Terminal mode MUST reattach to that same running process rather than starting a new one, preserving whatever shell state (working directory, in-shell environment, scrollback) it had accumulated.
- **FR-005**: Switching to AI CLI mode MUST reattach to the session's existing `claude` process if it is still running, or resume the same `claude` conversation (using the session's own id) if the process had exited, rather than ever starting a new conversation for that session.
- **FR-006**: Switching modes MUST NOT terminate either the AI CLI process or the shell process as a side effect; whichever one is not currently attached to the visible pane keeps running in the background.
- **FR-007**: At any given time, exactly one of the two processes (AI CLI or shell) MUST be attached to the visible terminal pane, receiving keystrokes and rendering output; the other MUST NOT receive input and MUST NOT be rendered.
  - **Clarified by BUG-001**: this is a statement about the *system*, not about either half of it.
    A mode switch that the display accepts but the process host does not — the indicator showing
    Regular Terminal mode while no shell exists — violates this requirement just as much as two
    processes being attached at once. A switch that cannot be honoured MUST fail visibly rather
    than leave the two halves disagreeing.
- **FR-008**: All real-terminal behavior defined for the embedded terminal (colored/styled output, live per-keystroke input, scrollback, mouse/selection handling, copy/paste, focus gating) MUST apply identically in Regular Terminal mode as it does in AI CLI mode.
- **FR-009**: The mode toggle button's icon (and accessible label/tooltip) MUST always reflect which mode is currently active, updated immediately on every switch, so it also serves as the pane's mode indicator.
  - **Clarified by BUG-001**: "which mode is currently active" means the mode the session is
    actually in, not the mode that was last requested. The indicator MUST NOT advance on a switch
    that did not take effect.
- **FR-010**: Each session's mode MUST be tracked independently; switching one session's mode MUST NOT affect any other session's mode or processes.
- **FR-011**: A session's current mode MUST be persisted and restored — reopening a session, including after an application restart, MUST show its terminal in the mode it was last in.
- **FR-012**: Closing a session, or the AI CLI process's own crash-triggered auto-restart, MUST act on whichever of a session's processes (AI CLI, shell, or both) are currently running — closing stops both if both are running; a crash-triggered auto-restart of the AI CLI process never touches the shell process. (This is existing session-close/crash-restart behavior extended to cover two processes, distinct from FR-013's new manual shell-restart affordance below.)
- **FR-013**: An exited shell process (whether from an intentional exit or a crash) MUST be reflected in the terminal as a not-running state with a manual restart affordance, without forcing the pane back to AI CLI mode. Unlike the AI CLI process, the shell process MUST NOT be automatically restarted on unexpected exit — restarting it is always a manual, user-triggered action, separate from FR-012's session-close/crash-restart behavior.
- **FR-014**: Deleting or otherwise tearing down a session MUST terminate both its AI CLI process and its shell process, whichever are running.
- **FR-015**: Regular Terminal mode MUST NOT alter the AI CLI session's identity, transcript, or sidebar label in any way — the shell process is entirely separate from the `claude` conversation it runs alongside. In particular, no code path introduced by this feature reads or writes the AI CLI provider's transcript file or session-title parsing (`src/provider.rs`) — the shell process's spawn/output/exit handling is fully independent of that logic.

### Key Entities

- **Terminal Mode**: Per-session state with two values, AI CLI and Regular Terminal, identifying which process is currently attached to a session's visible terminal pane. Persisted with the session and restored across app restarts.
- **Regular Shell Process**: A new background process kind, one per session, running the platform's plain interactive shell scoped to the session's worktree directory — parallel to, and independent of, the session's existing AI CLI process. Its lifecycle is simpler than the AI CLI process's: running or not-running, with restart always manual (no crash-loop / auto-restart-attempt tracking).
- **Session** *(existing, extended)*: Gains the persisted Terminal Mode value and may now have up to two live background processes (AI CLI and shell) at once instead of exactly one.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Switching a session's terminal between AI CLI mode and Regular Terminal mode completes with no perceptible relaunch delay (under 500ms) whenever the target process is already running in the background.
- **SC-002**: 100% of in-progress `claude` conversations resume with their full prior history and no duplicate/new session after any number of round-trips through Regular Terminal mode.
- **SC-003**: Users can identify the active mode correctly from the visual indicator alone, without issuing any command, in every observed case.
- **SC-004**: Shell state changes made before switching away (e.g., a `cd` into a subdirectory) are still in effect when the user returns to Regular Terminal mode later in the same application run.
- **SC-005**: Switching modes on one session produces zero observable effect (process state, output, or indicator) on any other concurrently open session, on Linux, macOS, and Windows alike.
- **SC-006**: 100% of Regular Terminal toggles either show a running shell or report why they could
  not; 0% leave the mode indicator claiming a mode the session is not in. *(Added by BUG-001.)*

## Assumptions

- The "regular shell" is the platform's standard interactive default shell (e.g., the user's configured `$SHELL` on Linux/macOS, the platform default on Windows) — the same shell a standalone terminal application would launch. This feature does not add a shell-selection UI.
- Both the AI CLI process and the shell process for a session are kept running in the background across mode switches rather than being killed and relaunched. This mirrors how a standalone multi-tab terminal keeps inactive tabs' processes alive, and is what makes the conversation "survive round-trips" (Story 2) and shell state persist (SC-004) without relying on `claude --resume` (or a fresh shell) on every single toggle. `--resume` remains the fallback specifically for the case where a process has independently exited while not attached to the pane.
- At most one shell process and one AI CLI process exist per session at a time — this feature does not add support for multiple simultaneous regular-terminal instances per session (e.g., terminal tabs). That is out of scope.
- The mode toggle is a manual, explicit user action; there is no automatic/heuristic switching between modes.
- Because operating-system processes cannot survive an application restart, "restoring the last-used mode" (FR-011) means the terminal reopens in that same mode with a freshly (re)started process of the appropriate kind — consistent with how the AI CLI process is already restored today via `--resume`.
- Regular Terminal mode is available under the same conditions AI CLI mode is available today (an active session with a live worktree); it is not offered for archived or already-deleted sessions.

**Bugfix**: 2026-08-04 — BUG-001 Switching a session to Regular Terminal mode did nothing at all,
silently, whenever the session's primary process was not running: the daemon spawned the shell and
then dropped it on the floor without registering it, returning success, and the attach that followed
no-opped for the same reason — while the client advanced its own mode indicator regardless. FR-003
clarified (a shell does not depend on the AI CLI running, and must not start it), FR-007 clarified
(the requirement binds the system, not each half separately), FR-009 clarified (the indicator
follows the actual mode, not the requested one), US1 acceptance scenario 4 added, SC-006 added.
