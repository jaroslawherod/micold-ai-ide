# Feature Specification: Multiple Regular Terminal Instances per Session

**Feature Branch**: `feat/allow-multiple-regular-terminals` (spec `012-multiple-regular-terminals`)

**Created**: 2026-07-20

**Status**: Closed

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
- What happens if the user presses the open-new-terminal-instance keyboard shortcut while the session is in AI CLI mode? ~~Nothing happens — the shortcut only opens a new Regular Terminal instance when Regular Terminal mode is already active for that session; it does not also switch the session's mode.~~ *(Answered the other way by feature 027 FR-004, 2026-08-21: it opens an instance and switches to it. The original answer was consistent with a bar whose "+" was only visible in Regular Terminal mode; since 026 both panes share one bar, and a shortcut that silently does nothing is the defect, not the safeguard.)*

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001** *(amended by feature 027 FR-004, 2026-08-21)*: The system MUST let the user open an
  additional Regular Terminal instance for a session at any time, ~~the session is in Regular
  Terminal mode,~~ regardless of how many instances (including exactly one) are already open for
  that session. *(The mode precondition is struck. It made sense when the AI pane hid the strip and
  its "+"; 026 gave both panes one bar, so the control is on screen while the AI pane is showing and
  a "+" that does nothing where it is drawn is a defect. 027 FR-004 requires it to work from either
  pane, opening the instance and switching to it.)*
- **FR-002**: Each Regular Terminal instance MUST run as an independent shell process with its own pty, scoped to the session's working directory (its worktree or the project's Default root, per the session's existing location).
- **FR-003**: Opening a new Regular Terminal instance MUST NOT affect the session's AI CLI process or any other already-open Regular Terminal instance for that session.
- **FR-004**: The system MUST let the user see a list of all currently open Regular Terminal instances for a session and switch the visible pane to any one of them.
- **FR-004a** (bugfix BUG-001; container clauses superseded by BUG-002): The instance-switching control MUST present its entries as **tabs**. ~~every entry — active and inactive alike — MUST render inside a container of the same shape and size, so the control reads as a tab strip rather than as loose text in the status bar.~~ Within a tab, the instance's label MUST be horizontally centred and its close control MUST sit at the tab's trailing (right) edge, not immediately adjacent to the label. *(bugfix BUG-005: "centred" is measured against the tab, so the leading spacer that balances the trailing edge MUST balance **everything** on that edge. Today it is one close control wide, which centres the label on a tab whose only trailing child is the close — and leaves it off centre by the width of a restart affordance on a tab that has one. The requirement is unchanged, and once FR-010b moves that affordance out of the tab the spacer balances the whole trailing edge again — the close control is all that is left on it. Recorded because the clause was briefly false and nothing said so, and because it becomes false again the next time a tab gains a trailing child.)* A tab's size MUST NOT depend on whether it is the active one, so changing which instance is active MUST NOT reflow the row. ~~The active/inactive distinction (FR-004, SC-004) MUST be carried by the containers' emphasis, never by one entry having a container and another having none.~~ *(The two struck clauses are superseded — BUG-002: they were written to rule out the original container-against-bare-text defect and over-corrected, forbidding the intended idiom along with it. A tab strip carries no container; it marks the active tab with an **indicator** — see FR-004b. The layout and no-reflow clauses above are unaffected and still binding.)*
- **FR-004c** (bugfix BUG-002, added during implementation): Every tab MUST have the **same fixed width**, independent of its label's content. This is what makes the indicator possible at all: an indicator is a rule, and a rule spans the width it is given — sized against a content-width tab it resolves against whatever space the bar happens to offer, stretching the active tab and resizing it on activation, which is the SC-008 reflow. A fixed width makes SC-008 hold by construction rather than by arithmetic. A label longer than the tab MUST ellipsise rather than widen it, which is also what a tab must do once instances can be renamed. ~~independent of its label's content~~ *(bugfix BUG-005: the struck phrase named the wrong subject. A tab's width has to be independent of **all** its content, and the label is the smaller half — the per-instance restart affordance of FR-010 is a second content-dependent child, roughly seven times the width of an ordinal, and it was not considered when this requirement was written. The fixed width was chosen against the three tab states the BUG-002 visual pass drew, none of which was exited.)* **The figure MUST be derived from the constants a tab's widest arrangement requires — its own padding, a leading spacer and a close control (each at the minimum interactive target), the gaps between children, ~~the restart affordance,~~ and a minimum legible label — and MUST NOT be a number chosen to make one observed arrangement look right.** *(The restart affordance is struck from the list by the decision recorded in FR-010a: deriving the width with it in produces a tab of 204dp against today's 128, and three instances would take 628dp of a 1014dp bar — the tab strip would crowd out the bar it lives in, and it would do so on every tab, for a child that most tabs never draw. The affordance moves out of the tab instead. The derivation rule is unchanged and is the durable half: it is what would have caught this at the time.)* A width chosen rather than derived is silently wrong the first time a tab gains a child, and wrong in the one way layout does not report: iced satisfies a shortfall by shrinking the trailing children, so the control disappears instead of the row overflowing (BUG-005).
- **FR-004b** (bugfix BUG-002): The active tab MUST be marked by an **active indicator** — an accent bar spanning the tab's width, of a thickness that reads at a glance rather than a hairline — and the active tab's label MUST take the accent colour, so the cue is carried by both weight and colour (SC-004). Inactive tabs MUST be low-emphasis labels with no container and no indicator. The indicator MUST sit at the tab's **top** edge: this control is anchored to the bottom of the window, so the content a tab selects lies above it, and an indicator on the bottom edge would point away from what it marks.
- **FR-005** *(**superseded** by feature 026 FR-003, 2026-08-20)*: The instance-switching control
  MUST be visible only once a session has more than one open Regular Terminal instance; it MUST
  remain hidden while the session has zero or one open instance, matching today's single-terminal
  experience.

  **This no longer holds, and deliberately.** Feature 026 puts the session's AI CLI process in the
  strip as a tab of its own, so the strip always has something in it and is drawn whenever a session
  is displayed — including at zero and one instance. The requirement above was written when the
  strip's only members were instances, and "hidden below two" was the right answer to "what does a
  switcher with nothing to switch between look like". It is the wrong answer to "what does a strip
  of every displayable pane look like", which is what this control now is. See 026's spec for the
  reversal and its reasoning.

  **Two other things about these tabs changed with it**, and a reader of this spec should not be
  surprised by them. Every tab gained a **leading state mark** — a small ring in the error role when
  its process is not running (026 FR-012c), drawn in the spacer FR-004a already reserved, so no tab
  grew and FR-004c's derived width is untouched. And the strip they sit in **scrolls horizontally**
  under overflow (026 FR-002a) instead of letting the bar shrink its own controls, which is a defect
  that was live here: past about five instances the mode toggle was laid out at 0.0dp wide and
  nothing reported it.
- **FR-006** *(**superseded** by feature 027 FR-001, 2026-08-21)*: ~~The existing primary
  AI-CLI/Regular mode-toggle control MUST continue to work as it does today: a single icon-button
  that switches the visible pane between the session's AI CLI process and Regular Terminal mode.~~

  **There is no toggle any more.** Feature 026 put the AI CLI process in this strip as a tab of its
  own, which left the button naming a destination the strip already named — and naming it worse,
  since a toggle can only say "the other one". 027 deletes it, and the strip becomes the sole route
  between panes. What this requirement was protecting — that switching to the AI pane stays
  available and one press away — is carried by 027 FR-002, which pins the AI tab last in the bar.
- **FR-007** *(amended by feature 027, 2026-08-21)*: Switching a session into Regular Terminal mode
  MUST show whichever Regular Terminal instance was last active for that session, or start a first
  instance if the session has never had one, never an arbitrary instance. *(The clause read
  "activating the primary toggle to switch"; the toggle is gone per FR-006 above. The rule is
  unchanged and now applies to the route that replaced it — pressing a terminal tab, which per 027
  FR-005 sets the mode **and** the instance in one press, so "which instance" is answered by the
  tab itself. It still governs any switch that names no instance.)*
- **FR-008**: Each Regular Terminal instance MUST independently track its own shell lifecycle (not-started, starting, running, exited), matching the lifecycle states already defined for today's single instance, with no automatic restart on unexpected exit.
- **FR-009**: A transition in one Regular Terminal instance's lifecycle (starting, running, exiting) MUST NOT cause a lifecycle transition in any sibling instance or in the session's AI CLI process.
- **FR-010**: The system MUST let the user manually restart an individual Regular Terminal instance after it has exited, restarting only that instance without affecting sibling instances or the AI CLI process.
- **FR-010a** (bugfix BUG-005): The per-instance restart affordance MUST be reachable — present at its full size and pressable — for **every** instance that offers it, including one that is not the active instance. Restarting a background instance without first selecting it is the whole point of addressing the restart message by instance id rather than to the attached process; a control laid out at zero width satisfies every structural claim about it (it exists, it is conditioned correctly, it dispatches the right message) while satisfying none of FR-010.
- **FR-010b** (bugfix BUG-005): The affordance MUST NOT live inside the tab. It MUST be offered from a **context menu on the tab**, opened by a secondary (right) click, listing restart for an instance whose own lifecycle offers it and close for any instance. A tab is a fixed width shared by every tab (FR-004c), so a child only one tab ever draws is paid for by all of them: carrying it costs 76dp on every tab for a control most never show, and at three instances the strip would take about 62% of the bar. The trade this makes is discoverability — a menu is less findable than a visible button — and it is accepted because restart is a recovery action taken deliberately after a shell has died, not something a user hunts for mid-task, and because a tab context menu is where renaming an instance is already headed.
- **FR-011**: The system MUST let the user close an individual Regular Terminal instance; closing MUST terminate only that instance's shell process and MUST NOT affect any sibling instance or the AI CLI process.
- **FR-011a** (bugfix BUG-001): A control nested inside a tab — the close control, and the per-instance restart affordance of FR-010 — MUST take its colour from the tab it sits in, so it stays legible on every tab state. In particular the close control on the **active** (highlighted) tab MUST read at the same emphasis as that tab's own label; it MUST NOT keep a colour chosen for the surrounding bar's background.
- **FR-012**: When the user closes the Regular Terminal instance that is currently visible and at least one sibling instance remains open, the system MUST automatically make the next instance in the list the new visible instance (or, if the closed instance was last in the list, the new last instance), so the pane is never left showing a closed instance.
- **FR-013**: Closing the last remaining Regular Terminal instance for a session MUST revert that session to AI CLI mode, matching today's single-terminal close behavior.
- **FR-014**: All real-terminal behavior already guaranteed for a Regular Terminal instance (colored/styled output, live per-keystroke input, scrollback, mouse/selection handling, copy/paste, focus gating) MUST apply identically and independently to every open instance of a session.
- **FR-015**: Each session's set of open Regular Terminal instances, and the switching/closing/restarting of them, MUST be fully independent of every other session — actions on one session's instances MUST have no observable effect on any other session's AI CLI process or Regular Terminal instances.
- **FR-016**: The number of AI CLI processes per session MUST remain exactly one; this feature MUST NOT create, allow, or expose more than one AI CLI process for a session.
- **FR-017**: Reopening a session, including after an application restart, MUST NOT restore more than one Regular Terminal instance automatically — a session found in Regular Terminal mode resumes with at most one (freshly started) instance, regardless of how many instances were open in a prior run.
- **FR-018**: Deleting or otherwise tearing down a session MUST terminate the AI CLI process and every open Regular Terminal instance belonging to that session.
- **FR-019** *(amended by feature 027 FR-004, 2026-08-21)*: The system MUST provide a keyboard
  shortcut (Ctrl+Shift+T, or Cmd+Shift+T on macOS) that opens a new Regular Terminal instance for
  the current session ~~whenever that session is in Regular Terminal mode~~, equivalent to using the
  on-screen affordance from FR-001 — which now means from either pane, for the reason recorded
  against FR-001.

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
- **SC-010** (bugfix BUG-005): No control inside a tab is laid out narrower than the size it asks for, in any arrangement of tabs the feature can produce — including a tab that offers a restart, which is the widest. Stated as its own criterion because it is the one thing the layout fixture cannot report by itself: a squeezed child still contains its own children, still escapes nothing, and still overflows nothing, so the whole gate suite is green over a control that has been reduced to nothing.
- **SC-009** (bugfix BUG-002): The active tab is identifiable at a glance from its indicator and label colour together, in both themes, without a container to distinguish it — the cue survives being read quickly and at the small size a status bar allows.

## Assumptions

- ~~The primary AI-CLI/Regular mode-toggle icon-button from the prior feature is unchanged by this feature; only a new secondary control for listing/switching/opening Regular Terminal instances is added alongside it.~~ *(No longer true: the secondary control absorbed the primary one. Feature 026 made the AI pane a tab in this strip and 027 deleted the toggle — see FR-006.)*
- An affordance to open a new Regular Terminal instance is available ~~whenever a session is in Regular Terminal mode~~ whenever a session is displayed, independent of the instance-switching control's visibility — so a user can always go from one instance to two, even though the switching/list portion of the control only appears once two or more instances exist. *(Widened by 027 FR-004; the strip itself is now always drawn, per FR-005 above.)*
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


**Bugfix**: 2026-08-18 — BUG-005 The tab that offers a restart cannot fit one. `TAB_WIDTH` gives a
tab's content row 112dp and a restartable tab's children ask for 166.3, so iced shrinks the trailing
two: the restart button lays out **0.0dp wide** and the close control at 45.2, below the 48dp minimum
interactive target feature 018 FR-027 sets. A background instance that exits cannot be restarted
from its own tab, which is the one thing addressing the restart message by instance id was built to
allow. Classified as a **spec conflict** rather than drift: FR-004c fixes every tab's width against
*the label's* content, which is the case it was written for, and FR-011a puts the restart affordance
inside the tab as a second and much larger content-dependent child. Each is right alone; the two
cannot both hold. FR-004c's "independent of its label's content" struck and replaced with a rule
that the width be **derived** from what the widest tab must contain rather than chosen; FR-010a
added (the affordance must be reachable, not merely present); SC-010 added (no control inside a tab
is laid out narrower than it asks for); FR-004a annotated, because the leading spacer that centres
the label balances only the close control and so the label is off centre by a restart button's width
on any tab that has one. No task was falsely completed — T029 built the affordance correctly and its
condition is still right, and T056 chose a width that solved the defect its visual pass could see,
against three tab states none of which was exited. What made this invisible afterwards is that the
strip was in **no covered state at all** until T063 registered one; every gate is green over a
zero-width control, because a squeezed child still contains its own children, escapes nothing and
overflows nothing.

**Decision (BUG-005, during implementation)**: the fix is FR-010b — the restart affordance leaves the
tab for a context menu on it — not a wider tab. Deriving the width with the affordance in, as FR-004c
first required, gives **204dp** against today's 128; three instances then take **628dp of a 1014dp
bar**, and a fourth does not fit beside the title, the status text, the "+" and the mode toggle. That
cost falls on every tab, for a child only a stopped instance ever draws. The measured alternatives
were recorded before choosing: balancing the leading spacer against the whole trailing group (FR-004a
read strictly) gives 264dp and 808dp at three instances, which is unusable; dropping the spacer and
centring the label in what is left of the tab gives 152dp but amends FR-004a's centring clause to
mean something weaker than it says. Moving the affordance out keeps every tab at **136dp** — the sum of a
tab's own padding, two 48dp touch targets (the close control and the leading spacer balancing it),
the gaps, and a 16dp floor for the label — returns the close control to its full 48dp target, and
puts restart beside the rename that is already headed for the same menu. It was expected to land
back on the written 128 and did not, by 8dp: the literal had reserved about 8dp for the label, which
is narrower than the two digits an ordinal already reaches, and nobody noticed because a label
smaller than its reserve just leaves the tab looking roomy. Landing on 128 would mean declaring that
a tab reserves less room for its name than `99` needs. The sum stands; choosing the reserve to
reproduce the old number is the move FR-004c was rewritten to forbid. What it spends is discoverability, which is stated in FR-010b rather than left to be
noticed later.
