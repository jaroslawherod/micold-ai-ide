# Feature Specification: Multiple Regular Terminal Instances per Session

**Feature Branch**: `feat/allow-multiple-regular-terminals` (spec `012-multiple-regular-terminals`)

**Created**: 2026-07-20

**Status**: Draft

**Input**: User description: "Extend the existing session mode toggle (see specs/010-regular-terminal-mode) so a session can run many concurrent Regular Terminal instances instead of just one. Today TerminalMode toggles a single session between its AI CLI (Claude) process and one Regular Terminal process. This feature removes the 'at most one shell process per session' limitation that specs/010-regular-terminal-mode/spec.md explicitly calls out as a non-goal, and supersedes it. From within a single session, users should be able to: open additional Regular Terminal instances alongside the existing one, each an independent shell process; see and switch between all open Regular Terminal instances for that session, reusing the 'list of instances + one active index' pattern already used for switching between sessions; toggle between the AI CLI pane and whichever Regular Terminal instance was last active for that session, preserving today's single mode-toggle icon-button as the primary toggle, with terminal-instance switching as a secondary control visible only once more than one instance is open; close an individual terminal instance without affecting siblings or the AI CLI process, with closing the last remaining instance falling back to today's single-terminal close behavior; and have each terminal instance independently track its own shell lifecycle and be independently restartable. Non-goals: no change to the number of AI CLI processes per session (still exactly one); no change to cross-session behavior; no new persistence of terminal instances across app restart beyond whatever already exists for the single-terminal case today."

## Clarifications

### Session 2026-07-20

- Q: When the user closes the Regular Terminal instance that is currently the visible/active one, and at least one sibling instance remains open for that session, which instance should become the new visible/active one? → A: The next instance in the list order (the one after the closed instance); if the closed instance was last in the list, fall back to the new last instance.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run more than one shell at once in a session (Priority: P1)

A developer is using Regular Terminal mode for a session and needs a second, independent shell running alongside the first — for example, to tail a log in one shell while running commands in another, both scoped to the same session's working directory. They open an additional Regular Terminal instance without disturbing the first one, and now have two independent shells they can work in.

**Why this priority**: This is the core capability the feature adds — without it, nothing else in the feature (switching, closing, independent lifecycle) has anything to act on.

**Independent Test**: With a session already showing one Regular Terminal instance, open a second instance, confirm both are independent shell processes scoped to the session's working directory, and confirm a running command in one is unaffected by input typed into the other.

**Acceptance Scenarios**:

1. **Given** a session in Regular Terminal mode with one shell instance open, **When** the user opens an additional Regular Terminal instance, **Then** a second, independent shell process starts, scoped to the same session working directory, and the first instance's process and state are unaffected.
2. **Given** a session with two or more open Regular Terminal instances, **When** the user runs a long-running command in one instance, **Then** switching to and typing in another instance has no effect on the running command in the first.
3. **Given** a session with multiple open Regular Terminal instances, **When** the user checks the session's AI CLI process, **Then** exactly one AI CLI process exists for that session, unaffected by how many Regular Terminal instances are open.
4. **Given** a session in Regular Terminal mode, **When** the user presses the open-new-terminal-instance keyboard shortcut (Ctrl+Shift+T, or Cmd+Shift+T on macOS), **Then** a new Regular Terminal instance opens for that session, the same as if the user had used the on-screen affordance.

---

### User Story 2 - See and switch between all open terminal instances (Priority: P1)

A developer has opened several Regular Terminal instances in a session and wants to glance at what's open and jump directly to any one of them, the same way they already switch between a project's multiple sessions from the sidebar.

**Why this priority**: Opening multiple instances (Story 1) has little practical value if the user cannot see what is open or get back to a specific one; this makes the capability usable.

**Independent Test**: With three or more Regular Terminal instances open for a session, use the instance-switching control to select each one in turn and confirm the visible pane shows the correct instance's process and output each time.

**Acceptance Scenarios**:

1. **Given** a session with only one open Regular Terminal instance, **When** the user looks at the terminal area, **Then** no instance-switching control is shown, matching today's single-terminal experience.
2. **Given** a session with two or more open Regular Terminal instances, **When** the user looks at the terminal area, **Then** an instance-switching control is visible, listing every open instance for that session.
3. **Given** the instance-switching control is visible, **When** the user selects a different instance from the list, **Then** the visible pane switches to that instance's shell process and output, and the previously-visible instance keeps running unattended in the background.
4. **Given** a session in AI CLI mode with two or more Regular Terminal instances open in the background, **When** the user activates the primary AI-CLI/Regular toggle, **Then** the pane shows whichever Regular Terminal instance was last active for that session.

---

### User Story 3 - Close one instance without disturbing the rest (Priority: P2)

A developer is done with one of several open Regular Terminal instances in a session and closes just that one, expecting every other open instance and the AI CLI process to keep running exactly as they were.

**Why this priority**: Without a clean way to close individual instances, open instances would accumulate indefinitely; this is necessary for the feature to be usable over a real working session, but the feature already delivers value via Stories 1–2 without it.

**Independent Test**: With three open Regular Terminal instances, close one that is not currently visible, then close the one that is currently visible, confirming in each case that the remaining sibling instances and the AI CLI process are unaffected.

**Acceptance Scenarios**:

1. **Given** a session with three open Regular Terminal instances, **When** the user closes an instance that is not the currently visible one, **Then** that instance's shell process terminates, its entry disappears from the instance-switching control, and the currently visible instance and all other siblings are unaffected.
2. **Given** a session with three open Regular Terminal instances, **When** the user closes the currently visible instance, **Then** that instance's shell process terminates and the pane automatically shows the next instance in the list (or the new last instance, if the closed one was last in the list), with the remaining sibling instances unaffected.
3. **Given** a session with exactly one open Regular Terminal instance, **When** the user closes it, **Then** the session falls back to today's single-terminal close behavior: the mode reverts to AI CLI, matching current behavior with no Regular Terminal instances open.

---

### User Story 4 - Each instance's lifecycle and restart are independent (Priority: P3)

A developer has several Regular Terminal instances open, one of them exits (the user typed `exit`, or it crashed), and they restart just that one instance while the others and the AI CLI process continue undisturbed.

**Why this priority**: Refines the robustness of a multi-instance session; the feature is already useful without it since a user could otherwise just open a fresh instance, but per-instance restart matches the independent-lifecycle expectation set by Stories 1–3 and today's single-instance behavior.

**Independent Test**: With multiple Regular Terminal instances open, cause one to exit (e.g., type `exit`), confirm it shows a not-running state with a manual restart affordance while siblings and the AI CLI process are unaffected, then restart just that instance and confirm it resumes as a fresh shell without touching any sibling.

**Acceptance Scenarios**:

1. **Given** multiple open Regular Terminal instances, **When** one instance's shell process exits or crashes, **Then** only that instance shows a not-running state with a manual restart affordance; sibling instances and the AI CLI process keep running unaffected.
2. **Given** an instance in a not-running state, **When** the user triggers its restart affordance, **Then** only that instance starts a fresh shell process; sibling instances are not restarted and the AI CLI process is not restarted.

---

### Edge Cases

- What happens when the user opens several Regular Terminal instances in rapid succession? Each instance MUST start independently, and each MUST appear in the instance-switching control without interfering with instances still starting.
- What happens if a background (non-visible) instance's shell process crashes? It MUST be reflected as not-running in the instance-switching control (no auto-restart, matching today's single-instance behavior) without changing which instance is currently visible.
- What happens if the user closes or deletes the session (worktree/Default teardown) while multiple Regular Terminal instances are running? All of that session's Regular Terminal instances MUST be terminated as part of that teardown, the same way the AI CLI process is today.
- What happens to shell state (working directory, in-shell environment, scrollback) already accumulated in an instance that isn't currently visible? It MUST be preserved exactly as today's single instance already preserves it — opening or closing sibling instances MUST NOT reset or otherwise touch it.
- What happens if the user reopens the app while a session had several Regular Terminal instances open in the prior run? Consistent with the non-goal of adding no new persistence, the session resumes in whatever mode it was last in with at most one (freshly started) Regular Terminal instance, the same as today's single-instance restart behavior — the prior instance count is not restored.
- What happens to focus gating, the reserved focus-release shortcut, and copy/paste behavior across multiple instances? They behave identically and independently per instance, exactly as they already do for the single instance today.
- What happens if the user presses the open-new-terminal-instance keyboard shortcut while the session is in AI CLI mode? Nothing happens — the shortcut only opens a new Regular Terminal instance when Regular Terminal mode is already active for that session; it does not also switch the session's mode.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST let the user open an additional Regular Terminal instance for a session at any time the session is in Regular Terminal mode, regardless of how many instances (including exactly one) are already open for that session.
- **FR-002**: Each Regular Terminal instance MUST run as an independent shell process with its own pty, scoped to the session's working directory (its worktree or the project's Default root, per the session's existing location).
- **FR-003**: Opening a new Regular Terminal instance MUST NOT affect the session's AI CLI process or any other already-open Regular Terminal instance for that session.
- **FR-004**: The system MUST let the user see a list of all currently open Regular Terminal instances for a session and switch the visible pane to any one of them.
- **FR-004a** (bugfix BUG-001; container clauses superseded by BUG-002): The instance-switching control MUST present its entries as **tabs**. ~~every entry — active and inactive alike — MUST render inside a container of the same shape and size, so the control reads as a tab strip rather than as loose text in the status bar.~~ Within a tab, the instance's label MUST be horizontally centred and its close control MUST sit at the tab's trailing (right) edge, not immediately adjacent to the label. A tab's size MUST NOT depend on whether it is the active one, so changing which instance is active MUST NOT reflow the row. ~~The active/inactive distinction (FR-004, SC-004) MUST be carried by the containers' emphasis, never by one entry having a container and another having none.~~ *(The two struck clauses are superseded — BUG-002: they were written to rule out the original container-against-bare-text defect and over-corrected, forbidding the intended idiom along with it. A tab strip carries no container; it marks the active tab with an **indicator** — see FR-004b. The layout and no-reflow clauses above are unaffected and still binding.)*
- **FR-004c** (bugfix BUG-002, added during implementation): Every tab MUST have the **same fixed width**, independent of its label's content. This is what makes the indicator possible at all: an indicator is a rule, and a rule spans the width it is given — sized against a content-width tab it resolves against whatever space the bar happens to offer, stretching the active tab and resizing it on activation, which is the SC-008 reflow. A fixed width makes SC-008 hold by construction rather than by arithmetic. A label longer than the tab MUST ellipsise rather than widen it, which is also what a tab must do once instances can be renamed.
- **FR-004b** (bugfix BUG-002): The active tab MUST be marked by an **active indicator** — an accent bar spanning the tab's width, of a thickness that reads at a glance rather than a hairline — and the active tab's label MUST take the accent colour, so the cue is carried by both weight and colour (SC-004). Inactive tabs MUST be low-emphasis labels with no container and no indicator. The indicator MUST sit at the tab's **top** edge: this control is anchored to the bottom of the window, so the content a tab selects lies above it, and an indicator on the bottom edge would point away from what it marks.
- **FR-005**: The instance-switching control MUST be visible only once a session has more than one open Regular Terminal instance; it MUST remain hidden while the session has zero or one open instance, matching today's single-terminal experience.
- **FR-006**: The existing primary AI-CLI/Regular mode-toggle control MUST continue to work as it does today: a single icon-button that switches the visible pane between the session's AI CLI process and Regular Terminal mode.
- **FR-007**: Activating the primary toggle to switch a session into Regular Terminal mode MUST show whichever Regular Terminal instance was last active for that session, or start a first instance if the session has never had one, never an arbitrary instance.
- **FR-008**: Each Regular Terminal instance MUST independently track its own shell lifecycle (not-started, starting, running, exited), matching the lifecycle states already defined for today's single instance, with no automatic restart on unexpected exit.
- **FR-009**: A transition in one Regular Terminal instance's lifecycle (starting, running, exiting) MUST NOT cause a lifecycle transition in any sibling instance or in the session's AI CLI process.
- **FR-010**: The system MUST let the user manually restart an individual Regular Terminal instance after it has exited, restarting only that instance without affecting sibling instances or the AI CLI process.
- **FR-011**: The system MUST let the user close an individual Regular Terminal instance; closing MUST terminate only that instance's shell process and MUST NOT affect any sibling instance or the AI CLI process.
- **FR-011a** (bugfix BUG-001): A control nested inside a tab — the close control, and the per-instance restart affordance of FR-010 — MUST take its colour from the tab it sits in, so it stays legible on every tab state. In particular the close control on the **active** (highlighted) tab MUST read at the same emphasis as that tab's own label; it MUST NOT keep a colour chosen for the surrounding bar's background.
- **FR-012**: When the user closes the Regular Terminal instance that is currently visible and at least one sibling instance remains open, the system MUST automatically make the next instance in the list the new visible instance (or, if the closed instance was last in the list, the new last instance), so the pane is never left showing a closed instance.
- **FR-013**: Closing the last remaining Regular Terminal instance for a session MUST revert that session to AI CLI mode, matching today's single-terminal close behavior.
- **FR-014**: All real-terminal behavior already guaranteed for a Regular Terminal instance (colored/styled output, live per-keystroke input, scrollback, mouse/selection handling, copy/paste, focus gating) MUST apply identically and independently to every open instance of a session.
- **FR-015**: Each session's set of open Regular Terminal instances, and the switching/closing/restarting of them, MUST be fully independent of every other session — actions on one session's instances MUST have no observable effect on any other session's AI CLI process or Regular Terminal instances.
- **FR-016**: The number of AI CLI processes per session MUST remain exactly one; this feature MUST NOT create, allow, or expose more than one AI CLI process for a session.
- **FR-017**: Reopening a session, including after an application restart, MUST NOT restore more than one Regular Terminal instance automatically — a session found in Regular Terminal mode resumes with at most one (freshly started) instance, regardless of how many instances were open in a prior run.
- **FR-018**: Deleting or otherwise tearing down a session MUST terminate the AI CLI process and every open Regular Terminal instance belonging to that session.
- **FR-019**: The system MUST provide a keyboard shortcut (Ctrl+Shift+T, or Cmd+Shift+T on macOS) that opens a new Regular Terminal instance for the current session whenever that session is in Regular Terminal mode, equivalent to using the on-screen affordance from FR-001.

### Key Entities

- **Regular Terminal Instance**: One of possibly several per session; an independent shell process with its own pty and its own working directory (inherited from the session at creation). Each instance independently tracks its own shell lifecycle (not-started, starting, running, exited) and is independently restartable and closeable. Supersedes the single, at-most-one shell process per session from the prior feature.
- **Session** *(existing, extended)*: Now holds an ordered collection of open Regular Terminal instances plus a record of which one is currently active — mirroring the existing "list of items + one active index" pattern already used elsewhere for switching between a project's sessions. Still has exactly one AI CLI process and one Terminal Mode value.
- **Terminal Mode** *(existing, unchanged)*: The session-level two-value switch (AI CLI / Regular Terminal). When set to Regular Terminal, the visible pane shows whichever Regular Terminal Instance is currently active for that session.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From a session already showing one Regular Terminal instance, a user can open a second, fully independent instance and begin using it in under 500ms, with zero interruption to the first instance's running process or output.
- **SC-002**: Switching the visible pane among any number of open Regular Terminal instances for a session completes with no perceptible delay (under 500ms) whenever the target instance is already running.
- **SC-003**: Closing one Regular Terminal instance never stops, restarts, or otherwise visibly affects any sibling instance's output or the AI CLI conversation, in 100% of observed cases.
- **SC-004**: Users can identify, from the instance-switching control alone, every currently open instance and which one is currently active, without issuing any command, in every observed case where more than one instance is open.
- **SC-005**: An exit, crash, or manual restart of one Regular Terminal instance never disrupts any sibling instance or the AI CLI process, in 100% of observed cases.
- **SC-006**: Opening, switching, or closing Regular Terminal instances in one session produces zero observable effect on any other concurrently open session, on Linux, macOS, and Windows alike.
- **SC-007** (bugfix BUG-001): Every open instance's close control is legible on the tab it belongs to — including the active, highlighted tab — in every observed case, in both the light and the dark theme. SC-004 alone does not cover this: a user can tell *which* tab is active while still being unable to see how to close it.
- **SC-008** (bugfix BUG-001): Switching which instance is active leaves every tab's position and size unchanged, so no tab moves under the pointer when the user selects one.
- **SC-009** (bugfix BUG-002): The active tab is identifiable at a glance from its indicator and label colour together, in both themes, without a container to distinguish it — the cue survives being read quickly and at the small size a status bar allows.

## Assumptions

- The primary AI-CLI/Regular mode-toggle icon-button from the prior feature is unchanged by this feature; only a new secondary control for listing/switching/opening Regular Terminal instances is added alongside it.
- An affordance to open a new Regular Terminal instance is available whenever a session is in Regular Terminal mode, independent of the instance-switching control's visibility — so a user can always go from one instance to two, even though the switching/list portion of the control only appears once two or more instances exist.
- Regular Terminal instances are identified in the switching control by their creation order (e.g., sequentially numbered), since — unlike AI CLI sessions — a shell process has no independent title source to display.
- There is no artificial cap on the number of concurrent Regular Terminal instances a session may have open; the practical limit is whatever the host system's resources allow, consistent with there being no such cap on the number of sessions today.
- New Regular Terminal instances are appended to the end of the instance list in the order they are opened; closed instances are removed from the list and their position is not reused by later instances.
- As today, only the persisted Terminal Mode value survives an application restart; the set and count of open Regular Terminal instances is not persisted — this matches the non-goal of adding no new persistence beyond what already exists for the single-terminal case.

**Bugfix**: 2026-08-14 — BUG-001 The instance switcher did not read as tabs and its close control was
illegible on the active tab. Only the active entry had a container (a filled pill); every inactive
entry was bare text with a loose close glyph beside it, and inside a tab the label and close sat
adjacent rather than centred/trailing. The close control also kept the surrounding bar's foreground
colour instead of the tab's, so on the active tab's fill it was near-invisible. FR-004a added (tabs:
uniform containers for active and inactive alike, centred label, trailing close, active-independent
size) and FR-011a added (a control nested in a tab takes that tab's colour), plus SC-007 (close
control legible on every tab, both themes) and SC-008 (no reflow on activation). The root cause was
an under-specified contract: `contracts/terminal-instance-switcher-ui.md` said "one small entry per
instance" and delegated appearance to a `TreeItem::selected` analogy, which does not transfer from a
full-width sidebar row to a strip of short numeric labels; nested-control tint was never constrained
because the nesting was not anticipated. That contract's "Instance-switcher row" section is updated
accordingly. The same bug report also retires the bottom bar's release-focus affordance — specified
in `023-terminal-focus-flow` FR-021 and `006-real-terminal-emulator` `contracts/focus-model.md`, and
amended there.


**Bugfix**: 2026-08-16 — BUG-002 "Tab" meant a Material **primary tab** — bare label plus an active
indicator — not the container-per-entry strip FR-004a specified. BUG-001 read the original defect
(one filled pill among loose numbers) as "the entries need containers", which the visible symptom
supported; the missing container was half the bug, and the half never stated anywhere is that a tab
strip marks its active member with an indicator. FR-004a's two container clauses struck, FR-004b
added (indicator plus accent label, no container on any tab), SC-009 added. **The indicator sits on
the tab's top edge**, against Material's placement: this control is anchored to the bottom of the
window, so the content a tab selects is above it and a bottom indicator would point away from what
it marks. FR-004a's layout clauses — centred label, trailing close, size independent of activation —
and FR-011a's nested-control colour rule are untouched and matter more without a container to carry
the emphasis. Two gates added by BUG-001 encode the superseded rule and are replaced rather than
deleted: a test that pins a decision should fail when the decision changes, and the replacement pins
the indicator instead. Deferred but recorded, because it constrains this fix: a terminal instance
should become renameable from a right-click menu, so a tab must be able to show a *name* rather than
an ordinal — every tab has one fixed width and a longer label ellipsises inside it rather than widening the strip
(FR-004c, added during implementation after the visual pass found the indicator stretching its own
tab).
