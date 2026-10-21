# Feature Specification: Worktree session guard

**Feature Branch**: `feat/1210_guard-stop-sessions-from-working-inside-a-worktree`

**Created**: 2026-10-21

**Status**: Draft

**Input**: User description: "Guard: stop sessions from working inside a worktree they created (issue #1210). An agent called `create_worktree`, then moved into the new worktree and did the work in its own session instead of handing it to a dedicated session via `create_session`. Per-repo rules (#944) are per-repo, miss `git -C <dir> worktree add`, do not cover `create_worktree`, and nothing blocks working in another worktree's path. Micold should enforce this itself, for every project, with no per-repo setup."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A session cannot work in another worktree (Priority: P1)

A session that Micold launched, in its own worktree or in the project's Default location, tries to
edit a file, write a file or run a shell command whose working directory or target path lies inside
a worktree that is not its own. Micold refuses the action and tells the agent to hand the work to a
dedicated session instead.

**Why this priority**: This is the guard itself. It closes the failure in the issue (an agent doing
a worktree's work from the wrong session) for every project without per-repo setup.

**Independent Test**: Launch a session, create a second worktree, then have the session attempt to
edit a file under that worktree. The edit is denied with a message naming `create_session`; the same
edit inside the session's own worktree is allowed.

**Acceptance Scenarios**:

1. **Given** a session that created a worktree through `create_worktree`, **When** it edits or
   writes a file inside that worktree, **Then** the call is denied and the message tells it to hand
   the work off with `create_session`.
2. **Given** a session, **When** it runs a shell command whose working directory is inside another
   worktree, **Then** the call is denied with the same message.
3. **Given** a session, **When** it edits a file inside its own worktree (or, for a Default session,
   inside the project root), **Then** the call is allowed.
4. **Given** a session in worktree A, **When** it targets a path in worktree B of the same project,
   **Then** the call is denied.
5. **Given** a session in a worktree, **When** it targets a path under the project root (main
   checkout) outside the allowlist, **Then** the call is denied; and **Given** a Default session,
   **When** it targets a path in any worktree, **Then** the call is denied.

---

### User Story 2 - Worktree creation by other means is blocked (Priority: P1)

Sessions Micold launches cannot create or enter a worktree by any route other than the sanctioned
one: entering a worktree with the agent's own enter-worktree tool, and every form of `git worktree
add` (including `git -C <dir> worktree add`), are denied. Creating a worktree through Micold's
`create_worktree` tool stays available.

**Why this priority**: Without it the agent sidesteps the guard by making and entering a worktree
itself, and today's per-repo rules miss the `-C` form.

**Independent Test**: In a launched session, attempt `git worktree add`, `git -C /some/dir worktree
add`, and the enter-worktree tool; each is denied. Call `create_worktree`; it succeeds.

**Acceptance Scenarios**:

1. **Given** a launched session, **When** it runs `git worktree add ...` or `git -C <dir> worktree
   add ...`, **Then** the command is denied.
2. **Given** a launched session, **When** it invokes the enter-worktree tool, **Then** it is denied.
3. **Given** a launched session, **When** it calls `create_worktree` then `create_session` for the
   new worktree, **Then** both succeed and the new session runs in that worktree.

---

### User Story 3 - `create_worktree` tells the agent to hand off (Priority: P2)

The result of `create_worktree` states that the agent must not work in the new worktree and must
hand the work to a dedicated session with `create_session`.

**Why this priority**: Steers the agent to the right behaviour before the guard has to fire.

**Independent Test**: Call `create_worktree` and read the result text.

**Acceptance Scenarios**:

1. **Given** a successful `create_worktree` call, **When** the result is returned, **Then** its text
   contains the do-not-work-here instruction and the `create_session` hand-off.

---

### User Story 4 - Legitimate cross-worktree use keeps working (Priority: P2)

Autopilot's prefetch worktree and the main checkout's shared build output directory
(`target-shared/`) remain reachable from any session.

**Why this priority**: Without the allowlist the guard would break the autopilot flow and shared
builds.

**Independent Test**: From a launched session, read or write under the prefetch worktree and under
the shared build directory; both are allowed while another worktree is still denied.

**Acceptance Scenarios**:

1. **Given** a session, **When** it targets a path in the autopilot prefetch worktree, **Then** the
   call is allowed.
2. **Given** a session, **When** it targets a path under the main checkout's shared build output,
   **Then** the call is allowed.

---

### User Story 5 - Opt out (Priority: P3)

A user who wants the previous behaviour turns the guard off with a setting; sessions then launch
without the guard and without the added deny rules.

**Why this priority**: Not everyone wants enforcement; it must be reversible.

**Independent Test**: Turn the setting off, launch a session, edit a file in another worktree; the
call is allowed. Turn it on again; new sessions are guarded.

**Acceptance Scenarios**:

1. **Given** the opt-out is set, **When** a session is launched, **Then** no guard or deny rules are
   applied to it.
2. **Given** the default setting, **When** a session is launched, **Then** the guard is applied.

### Edge Cases

- Path forms: relative paths, `..` segments, symlinks and trailing separators resolving into
  another worktree are treated by where they resolve, not how they are written.
- A shell command that changes directory part-way (`cd <other-worktree> && ...`) is judged on the
  directory it runs in where that can be determined (see FR-004).
- A path that is not in any worktree (outside the project, e.g. a temp directory or the user's home
  config) is allowed: the guard concerns only worktrees of the project.
- A worktree removed or renamed while sessions run: the guard uses the current worktree list, and a
  path in a removed worktree is no longer guarded.
- Several sessions launching at once in different worktrees: each is guarded relative to its own
  worktree; one session's state does not affect another's (Principle II).
- Guard cannot determine the session's own worktree or the worktree list (error, empty input): it
  must fail safe to the least harmful outcome and say why (see FR-009).
- Path conventions differ per platform (separators, case-insensitive filesystems, Windows drive
  letters); the same rules hold on Linux, macOS and Windows (Principle VI).
- The opt-out is toggled while sessions run: already-launched sessions keep the guard state they launched with; only sessions launched afterwards follow the new setting.
- A session the user themselves drives in the project's Default location performing its legitimate
  work in the project root is never blocked.

## Clarifications

### Session 2026-10-21

- Q: How far should shell analysis go (FR-004)? → A: Best effort: starting directory, explicit path arguments and chained `cd`/`pushd`; no expansion or nested-script analysis. _(agent-resolved: docs/user-guide/settings.md; FR-001 and the edit/write tools carry the strong guarantee)_
- Q: Scope of the opt-out (FR-008)? → A: One app-wide setting, default guarded, no per-project copy. _(agent-resolved: docs/user-guide/settings.md#L169, docs/daemon.md; FR-001 forbids per-repo configuration)_

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every session Micold launches MUST be guarded automatically, for every project, with
  no per-repo configuration, unless the user has opted out (FR-008).
- **FR-002**: The guard MUST deny any file-edit, file-write or shell-command call whose target path
  or working directory resolves into a worktree of the project other than the session's own.
- **FR-003**: A denial MUST carry a message that states the action was blocked because it targets
  another worktree and that the work must be handed to a dedicated session with `create_session`.
- **FR-004**: For shell commands the guard MUST judge the command's working directory and any path
  arguments it can identify (best effort):
  the shell's starting directory, explicit path arguments, and `cd <dir>` / `pushd <dir>` steps
  chained with `&&`, `;` or `||` ahead of a command. Paths built by expansion, substitution or
  variables, and the contents of nested scripts, are not analysed (the edit and write tools remain
  the strong guard).
- **FR-005**: Launched sessions MUST deny entering a worktree via the agent's enter-worktree tool
  and deny `git worktree add` in every form, including `git -C <dir> worktree add`.
- **FR-006**: The `create_worktree` tool MUST stay available to launched sessions, and the
  `create_worktree` then `create_session` flow MUST keep working unchanged.
- **FR-007**: The guard MUST allow targets in autopilot's prefetch worktree and in the main
  checkout's shared build output directory (`target-shared/`) from any session.
- **FR-008**: The user MUST be able to opt out of the guard and deny rules with a setting
  (one app-wide setting, no per-project copy, matching the existing *Let AI sessions manage worktrees and sessions* setting) The
  default is guarded.
- **FR-009**: When the guard cannot determine the session's own worktree or the project's worktree
  list, it MUST allow the call and record why, rather than block legitimate work. [Assumed: fail
  open; the deny rules of FR-005 still apply.]
- **FR-010**: The `create_worktree` result text MUST state that the agent must not work in the new
  worktree and must hand off with `create_session`.
- **FR-011**: A call whose target is inside the session's own worktree (or the project root for a
  Default session) MUST NOT be affected by the guard.
- **FR-012**: The guard's decision logic MUST be covered by automated tests in `micold-core`,
  including allow, deny, allowlist and opt-out cases and the path forms of FR-014.
- **FR-013**: Guard settings applied to a launched session MUST NOT overwrite or remove the
  project's own settings or rules; they are added to them.
- **FR-014**: The decision for a given logical layout MUST be identical on Linux, macOS and
  Windows, including symlinked, differently cased (on case-insensitive filesystems) and
  drive-lettered paths.

### Key Entities

- **Guarded session**: a session Micold launched, bound to one worktree or to the Default location.
- **Own worktree**: the worktree (or project root) a session is bound to.
- **Other worktree**: every location git lists for the project (project root included) except the session's own binding.
- **Allowlisted location**: the autopilot prefetch worktree and the shared build output directory.
- **Guard opt-out**: the user setting that switches the guard off.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In every test of a session editing or writing a file, or running a shell command, in a worktree other than its
  own, 100% of attempts are denied with a message naming `create_session`.
- **SC-002**: 100% of `git worktree add` forms tested (plain, `-C <dir>`) and enter-worktree
  attempts are denied, while `create_worktree` plus `create_session` completes successfully.
- **SC-003**: Edits in the session's own worktree, the prefetch worktree and the shared build
  directory are never denied in the test suite (0 false denials).
- **SC-004**: With the opt-out on, 0 guard denials occur; with it off, all of the above hold.
- **SC-005**: The guard adds no user-perceptible delay to a tool call (under 100 ms per decision).
- **SC-006**: The same tested layouts (including symlinked and differently cased paths) give the
  same decision on all three supported platforms.
- **SC-007**: In a test, a launched session's pre-existing project rules and settings are all still
  in effect after the guard is applied.

## Assumptions

- "Launched by Micold" covers sessions started from the app and through the tool server.
- The guard is relative to the session's own worktree, so a session in the project root (Default)
  is denied other worktrees as well.
- Worktrees are those git reports for the project; the prefetch worktree is identified by Micold's
  existing autopilot convention.
- Per-repo rules such as #944 stay valid and are not removed.
- The Principle III limits on Default sessions (no rename, modify or removal of worktrees) are unchanged by this feature.

## Out of Scope

- Blocking reads (viewing files) in other worktrees.
- Constraining sessions Micold did not launch.
- Changing how `create_worktree` or `create_session` themselves work, beyond the result text.
- Removing existing per-repo rules.
