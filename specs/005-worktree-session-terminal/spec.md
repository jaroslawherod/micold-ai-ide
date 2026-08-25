# Feature Specification: Worktree & Session Navigation with Embedded Terminal

**Feature Branch**: `005-worktree-session-terminal`

**Created**: 2026-07-15

**Status**: Closed (implemented and shipped. The manual quickstart pass — V1–V10 plus the SC-001/SC-002/SC-004 timings — ran 2026-08-21 on Linux, headlessly; evidence: `evidence/T061-manual-validation.md`. Eight scenarios pass outright, V3 passes but for a rollback clause only reachable through `FakeGit`, and V7's crash-loop guard fails → one open defect, `bugs/BUG-004.md`. T058's performance pass ran 2026-08-25 and **passes** — coalescing and the per-session scrollback cap are now measured by `crates/micold-daemon/tests/frame_coalescing.rs` against real flooded PTYs (20,000 lines in 48–62 frames against ceilings of 66–81; 5,000 lines into a 100-line cap retained exactly cap+screen), evidence: `evidence/T058-performance-pass.md`. Every task is now done. macOS/Windows parity is unrun.)

**Input**: User description: "Add the ability to open an existing project and manage its Claude Code worktrees and sessions through a Material Design navigation sidebar. Top-level navbar items are worktrees for the selected project; sub-items are sessions. Users can add a new worktree by name, which creates a workspace at .claude/worktrees/<name> relative to the project dir. Selecting a worktree lets the user start a session; an active session shows an embedded terminal on the right running claude in that worktree."

## Clarifications

### Session 2026-07-15

- Q: Are worktree/session removal and cleanup in scope for this feature? → A: Session close/stop is in scope; worktree removal (deleting the git worktree + branch) is deferred to a later feature.
- Q: What does "restore a session across app restart" mean? → A: Restore the session entry and, when reopened, re-launch `claude --resume <session-id>` in the session's worktree so it resumes the prior conversation. Live terminal scrollback is not persisted/replayed.
- Q: When the user switches to another session, do background sessions keep running? → A: Yes — background sessions keep their `claude` process running; switching only changes which terminal is displayed (truly concurrent, no cap).
- Q: Can a single worktree host multiple concurrent sessions? → A: Yes — a worktree may have multiple concurrent sessions, since the user may run non-interfering tasks in parallel. Coordinating overlapping edits within a shared worktree is the user's responsibility.
- Q: How are illegal/separator characters in the ticket or name handled? → A: Auto-sanitize (slugify) the ticket and name into characters valid for both a directory and a git branch name, then derive the directory and branch from the sanitized values.

### Session 2026-07-15 (2)

- Q: What happens when a session's `claude` process exits, crashes, or is terminated externally? → A: Auto-restart the `claude` process immediately using `claude --resume <session-id>` without user action; guard against rapid crash loops by stopping auto-restart after repeated quick failures and surfacing an error.
- Q: How are sessions labeled/distinguished in the sidebar? → A: The session label is extracted from `claude` (its session name/title), the same way the session id is obtained — not user-entered. A placeholder is shown until `claude` provides a name.
- Q: What happens to running sessions when the active project is switched or closed? → A: Stop that project's `claude` session processes on close/switch, but keep the sessions persisted so they reappear and resume via `claude --resume` when the project is reopened.
- Q: How are worktrees that are missing or invalid on disk handled? → A: Show them in the sidebar flagged as unavailable/invalid and disable starting sessions on them until resolved (do not silently hide them).

### Session 2026-07-16 (bugfix BUG-001)

- Q: Should empty sessions (started but never used, so `claude` recorded no conversation) be persisted and resumed across restarts? → A: No. Only sessions that have a recorded `claude` conversation are persisted and resumed. Empty sessions are not written to the store and are pruned on load, so a restart never attempts to resume a nonexistent conversation.

### Session 2026-07-17 (bugfix BUG-002)

- Q: The sidebar label never matches the AI CLI's own session name — it stays on the placeholder ("New session") forever. What is expected? → A: The system MUST actively read the provider-assigned session name/title while the session runs and reconcile the sidebar label to it (placeholder → provider name), updating whenever the provider's name changes. This label-sync flow must actually run at runtime, not merely be represented in the model.
- Q: The requirements and contracts name `claude` directly everywhere. Should the AI CLI be abstracted so other AI CLI providers can be supported later? → A: Yes. Treat the AI CLI as an abstract **AI CLI provider** behind a single seam; all provider-specific details (id ownership, resume mechanism, conversation-transcript location, session-title record format) live behind that abstraction. `claude` (Claude Code) is the default and first provider. Throughout this spec, existing references to `claude` are to be read as "the configured AI CLI provider" (see FR-024), with `claude` as the concrete default; they are not rewritten inline to keep this bugfix minimal.

### Session 2026-07-21 (bugfix 002/BUG-001)

- Q: A store-level fault (or the 002/BUG-001 per-project storage split's own failure mode) can
  leave a project with no persisted session records even though the AI CLI provider still has real
  conversation transcripts for it on disk. Should the app do anything beyond isolating the fault to
  that one project? → A: Yes. On project open, reconcile the session list against the provider's
  own transcripts for the project's supported session locations — its root directory and every
  worktree — and reconstruct a session entry for any transcript found with no matching persisted
  record. This is a supplement to normal restore (FR-020/FR-021/FR-023a), not a replacement.

### Session 2026-07-23 (bugfix BUG-003)

- Q: Closing a session (FR-015a) deletes its record, but FR-020b's reconciliation then
  reconstructs it from its still-existing `claude` transcript on next project open — silently
  undoing the close. Should close instead keep the record, and should there be a separate,
  stronger "permanently forget this" action? → A: Yes to both. Close now archives (kills the
  process, keeps the record hidden from the sidebar, never shown again) instead of deleting. A
  new, distinct **Remove** action (confirm-gated) permanently deletes the record. Both MUST
  durably block reconciliation from ever reconstructing that session id again — durably meaning
  independent of the app's own store, since a corrupted/lost store is exactly the scenario
  FR-020b exists to route around, so a flag living only in that same store wouldn't survive the
  scenario it's meant to guard against.
- Q: Should archived (closed) sessions be browsable anywhere — an "archived" list, an unarchive
  action? → A: No. Archiving is an invisible tombstone: the session disappears from the sidebar
  exactly as close did before, with no browsing UI and no way back. Tombstone records accumulate
  indefinitely; this is an accepted trade-off, not a defect (they are cheap: an id, a label, and
  a flag — no conversation content).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Open a project and browse its worktrees (Priority: P1)

A developer opens an existing project by selecting its directory. The application makes that project the active context and presents a Material Design navigation sidebar whose top-level items are the worktrees belonging to the project. Each worktree can be expanded to reveal its sessions as sub-items.

**Why this priority**: Without opening a project and seeing its worktrees, no other capability in this feature is reachable. This is the entry point and the minimum viable slice.

**Independent Test**: Open a project directory that already contains one or more worktrees under `.claude/worktrees/` and confirm the sidebar lists each worktree as a top-level item that expands to show its sessions. Delivers value by giving a navigable overview of an existing project's worktrees.

**Acceptance Scenarios**:

1. **Given** no project is open, **When** the user selects an existing directory that is a git repository, **Then** that project becomes the active context and the sidebar shows its worktrees as top-level items.
2. **Given** no project is open, **When** the user selects a directory that is not a git repository, **Then** the directory is rejected as a project, no project is opened, and the user is shown a clear message that only git repositories can be opened.
3. **Given** a project with existing worktrees is open, **When** the user expands a worktree item, **Then** its sessions appear as sub-items beneath it.
4. **Given** a project with no worktrees is open, **When** the sidebar renders, **Then** it shows the project as active with an empty worktree list and an affordance to add a worktree.
5. **Given** a project is open, **When** the sidebar renders in either light or dark mode, **Then** it follows the existing Material Design theming of the app shell.

---

### User Story 2 - Create a new worktree (Priority: P1)

From the sidebar, the developer adds a new worktree via a form. The form lets the developer select a **type** following the Conventional Commits vocabulary (for example `fix`, `feat`, `chore`, `docs`), enter an **optional ticket reference**, and enter a **name**. From these inputs the application derives a directory name of the form `${type}-${ticket}-${name}` (the `${ticket}` segment is omitted when no ticket is provided) and a git branch of the form `${type}/${ticket}-${name}`. It creates the branch and a git worktree bound to it under `.claude/worktrees/`, and the new worktree appears as a top-level item in the sidebar. These naming formats are fixed for the initial version but are intended to be user-configurable in the future.

**Why this priority**: Creating worktrees is the core management action of the feature and is required before a developer can start isolated sessions on a fresh line of work. It is equally essential to the MVP as opening a project.

**Independent Test**: With a project open, invoke "add worktree", pick a type, optionally enter a ticket, enter a name, submit the form, and confirm a worktree directory named `${type}-${ticket}-${name}` is created under `.claude/worktrees/`, a git branch named `${type}/${ticket}-${name}` is created and bound to it, and a top-level item appears in the sidebar. Delivers value by letting the developer provision consistently-named isolated workspaces without manual git steps.

**Acceptance Scenarios**:

1. **Given** a project is open, **When** the user opens the add-worktree form, **Then** the form presents a type selector populated with the Conventional Commits types (e.g. `fix`, `feat`, `chore`, `docs`), an optional ticket reference field, and a name field.
2. **Given** the add-worktree form, **When** the user selects a type, enters ticket `ABC-123`, enters name `login`, and submits, **Then** a git branch `${type}/ABC-123-login` is created and a git worktree bound to it is created at `.claude/worktrees/${type}-ABC-123-login` relative to the project directory, and the worktree appears as a top-level sidebar item.
3. **Given** the add-worktree form, **When** the user selects a type, leaves the ticket empty, enters name `cleanup`, and submits, **Then** the ticket segment is omitted, producing branch `${type}/cleanup` and directory `.claude/worktrees/${type}-cleanup`.
4. **Given** the add-worktree form, **When** the user enters a ticket or name containing separators or illegal characters, **Then** the inputs are sanitized (slugified) into valid characters and the derived directory and branch names are shown to the user before creation.
4a. **Given** the add-worktree form, **When** no type is selected or the name is empty after sanitization, **Then** submission is blocked and the user is shown a clear validation message.
5. **Given** a worktree with the derived directory name already exists, or a git branch with the derived name already exists, **When** the user submits the form, **Then** creation is prevented and the user is informed the name is already in use.
6. **Given** worktree creation fails partway (for example a git error while creating the branch or worktree), **When** the failure occurs, **Then** the user is shown an error, no partial branch or worktree is left behind, and the sidebar does not display a broken or half-created worktree.

---

### User Story 3 - Start a session and interact with the embedded terminal (Priority: P1)

The developer selects a worktree and starts a session on it. The session appears as a sub-item under its worktree. When a session is active, the right side of the window shows an embedded terminal running `claude` (the Claude Code CLI) with its working directory set to that worktree. Selecting a different session switches the right-side terminal to that session.

**Why this priority**: The embedded terminal running `claude` in the worktree is the payoff of the feature — it is why worktrees and sessions exist here. Without it, the feature does not deliver its intended value.

**Independent Test**: Select a worktree, start a session, and confirm an embedded terminal appears on the right running `claude` in that worktree's directory; start a second session and confirm switching between them swaps the visible terminal. Delivers value by letting the developer run Claude Code interactively inside an isolated worktree from within the app.

**Acceptance Scenarios**:

1. **Given** a worktree is selected, **When** the user starts a session, **Then** a new session sub-item appears under that worktree and the session becomes active.
2. **Given** a session is active, **When** the session view renders, **Then** the right side of the window shows an embedded terminal running `claude` with its working directory set to the session's worktree.
3. **Given** multiple sessions exist, **When** the user selects a different session, **Then** the right-side terminal switches to that session's terminal and every other session's `claude` process keeps running uninterrupted in the background.
4. **Given** sessions and worktrees are listed in the sidebar, **When** their state changes, **Then** the sidebar reflects whether each is active or inactive.
5. **Given** an active session with a running terminal, **When** the user types input, **Then** the input is delivered to the `claude` process and its output is displayed in the embedded terminal.

---

### Edge Cases

- What happens when the user tries to open a directory that is not a git repository (opening must be refused)?
- What happens when a git branch with the derived name already exists but no matching worktree exists (or vice versa)?
- What happens when the ticket reference or name itself contains a separator character (`-` or `/`) or other illegal characters? (Resolved: inputs are sanitized/slugified into valid characters before deriving names; a name that is empty after sanitization is rejected.)
- What happens when two different raw inputs sanitize to the same derived directory/branch name (collision after slugify)?
- What happens when the `.claude/worktrees/` directory does not yet exist on first worktree creation?
- What happens when a worktree directory exists on disk but is not a valid/registered git worktree? (Resolved: shown flagged as unavailable/invalid; starting sessions on it is disabled until resolved.)
- What happens when the `claude` CLI is not installed or not found on PATH when starting a session?
- What happens when the `claude` process in a session exits, crashes, or is terminated externally? (Resolved: auto-restart via `claude --resume`; a crash-loop guard stops restarting after repeated quick failures and surfaces an error.)
- How does the system handle a worktree whose underlying directory was deleted outside the application? (Resolved: flagged as unavailable/invalid in the sidebar; session creation disabled until resolved.)
- What happens to active sessions and their terminals when the user closes or switches the active project? (Resolved: the project's `claude` session processes are stopped, but sessions persist and resume via `claude --resume` when the project is reopened.)
- What happens to a session that was started but never used (no `claude` conversation) on restart? (Resolved — bugfix BUG-001: empty sessions are not persisted; they are excluded on save and pruned on load, so a restart never resumes a nonexistent conversation.)
- What happens when the AI CLI provider assigns or later changes a session's name after the session is already shown in the sidebar? (Resolved — bugfix BUG-002: the label is actively reconciled with the provider's current session name at runtime — placeholder → provider name, and updated on any subsequent change — so the displayed name never diverges from the provider's.)
- What happens when the provider has not yet supplied a session name, or the name cannot be read? (Resolved — bugfix BUG-002: the placeholder / last-known label is kept and the read is retried opportunistically; a failed or absent read never fails the session.)
- How does the system handle a very large number of worktrees or sessions in the sidebar?
- What happens when a project's persisted session list is missing, empty, or was just reset (e.g. by the 002/BUG-001 storage-fault-isolation fix), but the AI CLI provider still has real conversation transcripts on disk for that project? (Resolved — bugfix 002/BUG-001: opening the project scans the provider's transcript directory for the project's root directory and every worktree, and reconstructs a session entry for any transcript found with no matching persisted record, so a lost or corrupted store does not orphan a real, resumable conversation.)
- What happens when the user closes or removes a session whose `claude` transcript still exists on disk, and the project is later reopened — does reconciliation (FR-020b) bring it back? (Resolved — bugfix BUG-003: no. Both close and remove record a durable, provider-side marker (FR-020c) that reconciliation checks and skips, independent of the app's own store, so an intentionally closed/removed session never resurfaces even if the app's own persisted record of the closure is itself lost.)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST allow the user to open an existing project by selecting its directory and make that project the active context.
- **FR-001a**: System MUST verify the selected directory is a git repository and MUST refuse to open a non-git directory as a project, informing the user that only git repositories can be opened.
- **FR-002**: System MUST present a Material Design navigation sidebar whose top-level items are the worktrees belonging to the active project.
- **FR-003**: System MUST display each worktree's sessions as sub-items nested under their worktree, and allow worktree items to be expanded and collapsed.
- **FR-004**: The sidebar MUST follow the existing Material Design light/dark theming used by the application shell.
- **FR-005**: System MUST allow the user to add a new worktree through a form that captures a type, an optional ticket reference, and a name.
- **FR-005a**: The type field MUST offer the Conventional Commits type vocabulary (at minimum `fix`, `feat`, `chore`, `docs`) for selection.
- **FR-005b**: The ticket reference field MUST be optional; when omitted, the ticket segment MUST be dropped from both the derived directory name and branch name (no empty separators left behind).
- **FR-006**: System MUST derive the worktree directory name as `${type}-${ticket}-${name}` and the git branch name as `${type}/${ticket}-${name}` from the form inputs, and MUST, when the worktree is added, create the derived git branch and create a git worktree bound to that branch at `.claude/worktrees/${type}-${ticket}-${name}` relative to the active project directory, without requiring the user to run manual git commands.
- **FR-006a**: The directory and branch naming formats MUST be defined in a single place and MUST be structured so they can become user-configurable in a future version without changing the surrounding creation flow. (Configurability itself is out of scope for this version; the formats above are the fixed defaults.)
- **FR-006b**: If branch or worktree creation fails, System MUST roll back any partial changes (created branch and/or directory) so no orphaned branch or worktree remains.
- **FR-007**: System MUST display a newly created worktree as a top-level item in the sidebar, labeled by its derived name.
- **FR-008**: System MUST sanitize (slugify) the ticket and name inputs into characters valid for both a directory name and a git branch name, and derive the directory and branch names from the sanitized values. System MUST still reject inputs that cannot yield a valid worktree — e.g. no type selected, or a name that is empty after sanitization — with a clear message.
- **FR-008a**: System SHOULD show the user the derived directory and branch names (the result of sanitization) before creation so the outcome is predictable.
- **FR-009**: System MUST prevent creation of a worktree whose derived directory name duplicates an existing worktree or whose derived branch name duplicates an existing git branch, and inform the user.
- **FR-010**: System MUST allow the user to start a new session on a selected worktree.
- **FR-010a**: System MUST allow a single worktree to host multiple concurrent sessions, each with its own independent `claude` process and terminal, listed as separate sub-items under that worktree. Coordinating overlapping file edits within a shared worktree is the user's responsibility and is not enforced by the system.
- **FR-011**: System MUST display each started session as a sub-item under its worktree in the sidebar.
- **FR-011a**: System MUST label each session in the sidebar using the session name/title extracted from the AI CLI provider (obtained the same way as the provider's session id), not a user-entered name. Until the provider supplies a name, System MUST show a placeholder label, and MUST update the label once the name becomes available. The extracted name MUST be persisted alongside the session id. **The label MUST be actively reconciled with the provider's current session name while the session is running (bugfix BUG-002): System MUST read the provider-supplied name at runtime and keep the sidebar label in sync with it — updating the label whenever the provider assigns or changes the name — so the displayed name never diverges from the provider's session name. A failed or absent read MUST NOT fail the session (the label simply stays at its last known value / the placeholder).**
- **FR-012**: System MUST, when a session is active, show an embedded terminal on the right side of the window.
- **FR-013**: The embedded terminal MUST run the `claude` CLI with its working directory set to the session's worktree directory.
- **FR-014**: System MUST allow the user to send interactive input to, and view output from, the `claude` process through the embedded terminal.
- **FR-015**: System MUST switch the right-side terminal to the corresponding session when the user selects a different session.
- **FR-015b**: System MUST keep the `claude` process of every non-displayed (background) session running when the user switches sessions; switching MUST only change which terminal is displayed, never suspend or stop other sessions. There is no fixed cap on the number of concurrent running sessions.
- **FR-015a**: ~~System MUST allow the user to close/stop an active session, terminating its `claude` process and removing the session from the sidebar.~~ (Superseded — bugfix BUG-003, 2026-07-23: "removing... from the sidebar" was read as permanent, but nothing distinguished it from FR-020b's reconciliation later reconstructing that same session from its still-existing `claude` transcript — the user's close was silently undone on next project open.) System MUST allow the user to **close** an active session: terminate its `claude` process, keep its persisted record (so FR-020b's transcript-based reconciliation never reconstructs it again — see FR-020c), and hide it from the sidebar. A closed session is not browsable or re-openable through the UI (an "invisible tombstone" — bugfix BUG-003); its record simply stops appearing. (Worktree removal — deleting the git worktree and branch — is out of scope for this feature and deferred to a later feature.)
- **FR-015c**: System MUST allow the user to **remove** a session — a distinct, permanent-delete action from **close** (FR-015a) — terminating its `claude` process (if running) and deleting its persisted record entirely, behind a confirmation step (bugfix BUG-003; mirrors the existing worktree-delete confirmation, feature 008 FR-018/FR-019). Remove is reachable only from a currently-visible (not-yet-closed) session; there is no UI path to remove an already-closed session, since closed sessions are not shown (FR-015a).
- **FR-016**: System MUST reflect worktree and session state (active/inactive) in the sidebar.
- **FR-017**: System MUST report errors from worktree creation or session/terminal startup to the user and MUST NOT leave broken or half-created worktrees or sessions represented in the sidebar.
- **FR-018**: System MUST discover and display worktrees already present under the active project's `.claude/worktrees/` directory when the project is opened.
- **FR-018a**: System MUST detect worktrees that are missing or invalid on disk (e.g. the directory was deleted externally, or exists but is not a valid/registered git worktree), display them in the sidebar flagged as unavailable/invalid rather than hiding them, and disable starting new sessions on them until the condition is resolved.
- **FR-019**: Sessions MUST be isolated from one another such that no session's terminal or state affects another session.
- **FR-020**: System MUST persist each session's identity (including a reference to the underlying `claude` session id) and its association to a worktree locally, and MUST restore session entries in the sidebar across application restarts. **Empty sessions — those for which `claude` has recorded no conversation — MUST NOT be persisted** (bugfix BUG-001): they are excluded when saving and pruned when loading, so no never-used session is restored.
- **FR-020a**: System MUST determine whether a session has a recorded `claude` conversation before persisting/restoring it (e.g. by the presence of the session's `claude` conversation transcript). Only sessions with a recorded conversation are persisted and restored (bugfix BUG-001).
- **FR-021**: System MUST, when the user reopens a persisted session after a restart, re-launch the `claude` CLI with the `--resume <session-id>` flag pointing to that session, in the session's worktree directory, so the prior conversation resumes. Because only sessions with a recorded conversation are persisted (FR-020/FR-020a), a resumed session always has a conversation to resume. Live terminal scrollback/output is not required to be persisted or replayed.
- **FR-022**: When an active session's `claude` process exits, crashes, or is terminated externally, System MUST automatically restart it using `claude --resume <session-id>` in the session's worktree directory, without requiring user action.
- **FR-022a**: System MUST guard against crash loops: after a bounded number of failed restarts within a short interval, System MUST stop auto-restarting that session and surface a clear error, leaving the session in the sidebar so the user can retry manually.
- **FR-023**: ~~When the user closes the active project or switches to a different project, System MUST stop all of that project's running session `claude` processes, while preserving the sessions' persisted identity, `claude` session id, and name.~~ (Superseded in part — spec/code alignment 2026-07-20. The **switch** half is reversed by feature 008 FR-001, which requires sessions to keep running across a project switch; that is the whole point of background project switching. The **close** half is untouched by 008 and is restated below.) When the user closes the active project, System MUST stop all of that project's running session `claude` processes, while preserving the sessions' persisted identity, `claude` session id, and name. Switching to a different project MUST NOT stop any session (feature 008 FR-001).
  - **Status**: NOT IMPLEMENTED and currently unreachable — the application exposes no "close project" action, so this requirement has no trigger. `Session::stop_for_project_change` exists in the code with zero call sites. This is a known gap, not drift: the requirement stands and is deliberately left open rather than deleted. Implementing a close-project action MUST implement this stop behaviour with it.
- **FR-023a**: When a previously-open project is reopened, System MUST restore its persisted sessions in the sidebar (consistent with FR-020/FR-021), resuming a session's `claude` process via `claude --resume <session-id>` when it is reopened. The crash-loop auto-restart of FR-022 applies only to unexpected process exits, not to processes intentionally stopped on project close. (Amended 2026-07-20: previously read "project close/switch"; sessions are no longer stopped on switch per feature 008 FR-001. On switch, sessions keep running and are re-attached rather than resumed — feature 008 FR-003.)
- **FR-020b**: When a project is opened, System MUST reconcile its session list against the AI CLI
  provider's own conversation records for that project's supported session locations — the
  project's root directory and every worktree under `.claude/worktrees/` (bugfix 002/BUG-001).
  For each conversation transcript found (named by its session id) with no corresponding persisted
  session record, System MUST reconstruct a session entry using that session id, the location it
  was found in, and the provider-supplied title from the transcript if available (falling back to
  the `Pending` placeholder otherwise). This reconciliation supplements, but never replaces, normal
  persisted-session restoration (FR-020/FR-021/FR-023a); it exists so a lost, corrupted, or
  just-emptied session store does not orphan a real, resumable conversation.
- **FR-020c**: Reconciliation (FR-020b) MUST NOT reconstruct a session that the user closed
  (FR-015a) or removed (FR-015c) (bugfix BUG-003). This suppression MUST be durable against the
  app's own persisted store being corrupted, missing, or entirely lost — the same failure class
  FR-020b itself exists to route around — so it MUST be recorded via the AI CLI provider seam
  (FR-024) as a marker independent of `projects.json` and any per-project state file (e.g. a
  small file recorded alongside the session's transcript, in the provider's own storage), not
  solely as a flag inside the app's own store. The app's own store MAY additionally track a
  closed session's state (e.g. for fast in-memory sidebar filtering without touching disk), but
  that MUST NOT be the only record — the provider-side marker is authoritative for whether
  reconciliation reconstructs a given session id.
- **FR-024**: System MUST treat the underlying AI CLI as an abstract **AI CLI provider** rather than hard-coding one tool (bugfix BUG-002). All provider-specific behaviour MUST be defined in a single place and accessed through one seam, including: the executable/launch command, how the app-owned session id is passed, how a session is resumed, where the conversation transcript lives, how "a conversation was recorded" is detected (FR-020a), how the session name/title is extracted (FR-011a), and how a closed/removed session is durably marked and checked (FR-020c, bugfix BUG-003). `claude` (Claude Code) MUST be the default and only provider shipped in this version; adding another provider MUST NOT require changes to the session model, persistence, sidebar, or terminal wiring — only a new provider definition. (Multiple providers and provider selection UI are out of scope for this version; this requirement only mandates the seam, mirroring the configurable-naming approach of FR-006a.)

### Key Entities *(include if feature involves data)*

- **Project**: An existing git repository directory opened by the user, serving as the active context. Non-git directories cannot be opened as projects. Owns a collection of worktrees and is the root relative to which `.claude/worktrees/` is resolved.
- **Worktree**: An isolated workspace located at `.claude/worktrees/${type}-${ticket}-${name}` under the project and bound to a dedicated git branch `${type}/${ticket}-${name}`. Created from a type (Conventional Commits vocabulary), an optional ticket reference, and a name. Appears as a top-level sidebar item, has a validity/active state, and owns a collection of zero or more concurrent sessions.
- **Worktree Naming Convention**: The rule set that maps `(type, ticket, name)` inputs to a directory name (`${type}-${ticket}-${name}`) and a branch name (`${type}/${ticket}-${name}`), dropping the ticket segment when absent. Fixed defaults in this version; designed to become user-configurable later.
- **Session**: A unit of work bound to a single worktree. Appears as a sub-item under its worktree, has an active/inactive state, and is associated with an embedded terminal. Persists its identity, the underlying AI CLI provider's session id, and the provider-supplied session name/title (used as its sidebar label) so it can be restored (via the provider's resume mechanism, e.g. `claude --resume <session-id>`) and re-labeled after an application restart. The label is kept in sync with the provider's current session name while running (FR-011a, bugfix BUG-002).
- **Embedded Terminal**: The interactive terminal surface shown on the right for an active session, running the AI CLI provider's CLI (default `claude`) in the session's worktree directory and relaying input and output between the user and that process.
- **AI CLI Provider** (bugfix BUG-002): The abstraction over the AI coding CLI that backs a session. Defines the launch command, how the app-owned session id is passed, how a session is resumed, where the conversation transcript is stored, how a recorded conversation is detected, and how the session name/title is extracted for the sidebar label. `claude` (Claude Code) is the default and only provider in this version; the abstraction exists so other providers can be added later without touching the session/persistence/UI layers (FR-024).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can open a project and see its existing worktrees in the sidebar within 3 seconds of selecting the directory.
- **SC-002**: A user can create a new worktree by name in under 30 seconds without leaving the application or running any manual git commands.
- **SC-003**: 100% of successfully created worktrees exist on disk at `.claude/worktrees/<name>` relative to the project, are bound to their dedicated git branch, and appear as top-level sidebar items.
- **SC-003a**: 100% of attempts to open a non-git directory as a project are refused with a clear message and no project is opened.
- **SC-003b**: For 100% of successfully created worktrees, the directory name equals `${type}-${ticket}-${name}` and the branch name equals `${type}/${ticket}-${name}` for the given inputs, with the ticket segment correctly omitted when no ticket was provided.
- **SC-004**: A user can start a session and see an interactive `claude` terminal for the correct worktree within 5 seconds of starting the session.
- **SC-005**: Switching between two active sessions displays the correct session's terminal on the first attempt in 100% of switches, with no cross-session output leakage.
- **SC-006**: 100% of invalid or duplicate worktree name attempts are rejected with a clear message and produce no directory or sidebar artifacts.
- **SC-007**: The sidebar accurately reflects the active/inactive state of every worktree and session at all times.
- **SC-008**: After an application restart, 100% of previously persisted sessions reappear in the sidebar, and reopening one resumes its prior `claude` conversation via `claude --resume`. Only sessions with a recorded `claude` conversation are persisted; empty (never-used) sessions do not reappear (bugfix BUG-001).
- **SC-009**: For 100% of sessions whose AI CLI provider has assigned a session name, the sidebar label matches that provider-supplied name (not the placeholder), and updates to reflect any later change to the provider's name — the displayed name never stays diverged from the provider's session name (bugfix BUG-002).
- **SC-010**: When a project is opened, 100% of AI CLI provider conversation transcripts found under its root directory or any of its worktrees, with no matching persisted session record, are reconstructed as sessions in the sidebar (bugfix 002/BUG-001).
- **SC-011**: 100% of sessions closed or removed do not reappear after the project is closed and reopened, even when the app's own persisted store (catalog or per-project state) is deleted entirely between the close/remove and the reopen (bugfix BUG-003).

## Assumptions

- The active project MUST be a git repository; opening a non-git directory as a project is refused outright (not a supported flow), which guarantees git worktree and branch creation are always possible.
- Each worktree is bound to its own git branch created at worktree-creation time; the directory name (`${type}-${ticket}-${name}`) and branch name (`${type}/${ticket}-${name}`) are derived deterministically from the form inputs, with the ticket segment omitted when no ticket is given. The base ref the branch is created from (for example the current HEAD) is an implementation detail deferred to planning.
- The Conventional Commits type list, the naming formats, and the ability to customize them are fixed defaults in this version; making them user-configurable is explicitly deferred to a future version and out of scope here.
- The `.claude/worktrees/` directory is created on demand under the project if it does not already exist.
- The `claude` CLI is installed and available on the user's PATH; its absence is surfaced as an error when starting a session.
- The AI CLI provider is abstracted behind a single seam (FR-024); `claude` (Claude Code) is the default and only provider in this version. Provider selection and additional providers are deferred to a future version — the abstraction exists so they can be added without reworking the session, persistence, or UI layers (bugfix BUG-002).
- This feature builds on the existing project/workspace management and Material Design app shell already present in the application (specs 001–004).
- Worktree names map directly to directory names under `.claude/worktrees/`; no separate display-name-to-directory mapping is assumed for the initial version.
- Session and worktree state is persisted locally, consistent with the project's local-first storage principle; the precise persistence format is an implementation detail deferred to planning.
- A single project is active at a time in the sidebar; multi-project simultaneous views are out of scope for this feature.
- Closing/stopping a session is in scope; removing a worktree (deleting its git worktree and branch) is deferred to a later feature and not covered here.
- Empty sessions (started but with no `claude` conversation recorded) are not persisted; only sessions with a recorded conversation survive a restart (bugfix BUG-001).
- Reconciling a project's sessions against the AI CLI provider's transcripts (FR-020b) is a
  best-effort discovery pass at project-open time, not continuous background monitoring; it scans
  the project's root directory and its currently-discovered worktrees only (bugfix 002/BUG-001).
- Closed sessions are invisible tombstones with no browsing or unarchive UI (bugfix BUG-003);
  their records (id, last-known label, closed flag) accumulate indefinitely in the app's own
  store. This is an accepted trade-off — the records are small and hold no conversation content
  (that stays with `claude`, under `~/.claude`) — not a scope gap to fill later.

**Bugfix**: 2026-07-16 — BUG-001 Empty sessions are no longer persisted or resumed on restart. FR-020 amended, FR-020a added, FR-021/SC-008 clarified, plus a Clarifications entry and edge case.

**Bugfix**: 2026-07-17 — BUG-002 Session name kept in sync with the AI CLI provider's session name, and `claude` references abstracted behind an AI CLI provider seam. FR-011a amended (active label reconciliation), FR-024 added (AI CLI provider abstraction), SC-009 added, an "AI CLI Provider" key entity added, Session/Embedded Terminal entities reworded provider-neutral, plus a Clarifications entry, two edge cases, and an assumption. Existing inline `claude` references are read as "the configured AI CLI provider" (default `claude`) per FR-024 rather than rewritten, to keep the patch minimal.

**Alignment**: 2026-07-20 — Spec/code alignment audit. FR-023 split: its **switch** half is superseded by feature 008 FR-001 (sessions keep running across a project switch — the point of background switching), while its **close** half is restated and explicitly marked NOT IMPLEMENTED, since the application exposes no close-project action and `Session::stop_for_project_change` has zero call sites. The requirement is deliberately kept open rather than deleted. FR-023a's "project close/switch" narrowed to "project close". No behaviour change from this amendment.

**Bugfix**: 2026-07-21 — 002/BUG-001 A store-level fault could wipe every open project's sessions
with no way to recover them, because sessions lived embedded in the same shared, whole-file-fate
`projects.json` the known-projects catalog uses (see `specs/002-project-workspace-management`
BUG-001 for the storage-split half of this fix). This spec's half of the fix: FR-020b added — on
project open, reconcile the session list against the AI CLI provider's own transcripts for the
project's root directory and every worktree, reconstructing any session whose transcript exists
but whose persisted record does not. New Clarifications entry, edge case, SC-010, and an
assumption added. `contracts/storage-schema.md` updated accordingly.

**Bugfix**: 2026-07-23 — BUG-003 FR-020b's reconciliation (above) had no way to tell a session the
user intentionally closed apart from one simply missing from a lost/corrupted store — so closing
a session was silently undone by reopening the project. FR-015a split: close now archives
(process killed, record kept, hidden from the sidebar, never browsable again) instead of deleting.
New FR-015c (Remove: a distinct, confirm-gated permanent delete). New FR-020c: suppression from
both close and remove MUST be durable against the app's own store being corrupted or lost —
recorded via the AI CLI provider seam (FR-024), not solely inside the app's own store, since a
flag living only there wouldn't survive the exact failure class FR-020b exists to route around.
New Clarifications entries, edge case, SC-011, and an assumption (tombstones are not browsable and
accumulate indefinitely — accepted trade-off). `contracts/claude-cli.md` and
`contracts/storage-schema.md` updated accordingly.
