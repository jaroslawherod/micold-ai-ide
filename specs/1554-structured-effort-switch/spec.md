# Feature Specification: Structured Session Reasoning Effort — pick, show and keep the effort level

**Feature Branch**: `claude/project-thread-carlvm`

**Created**: 2026-10-27

**Status**: Draft

**Input**: GitHub issue #1554 "Structured view: model reasoning effort cannot be changed". "The
structured session view has no way to change the model's reasoning effort (low / medium / high /
xhigh / max). Model switching exists (#943), but effort does not: there is no control for it, and
`/effort` does not work from the structured composer the way it does in the PTY terminal view.
Steps to reproduce: 1. Open a session in the structured view. 2. Look for an effort setting next to
the model switcher, or type `/effort high` in the composer. Expected: The effort level can be picked
in the structured view, e.g. next to the model switcher, and `/effort <level>` works in the
composer. The current effort level is shown, and the new level applies to the next turn. The choice
persists for the session, same as the model. Actual: There is no effort control, and `/effort` has
no effect."
labels: bug, flow:bug (entered as a bug; switched to the feature flow, see the ledger)

## In short (for non-technical readers)

Claude Code can think harder or less hard before it answers: its **reasoning effort**, one of
`low`, `medium`, `high`, `xhigh` or `max`. More effort gives better answers to hard problems and
takes longer; less is quicker for simple ones. In Claude Code's own text screen (the terminal view)
the user changes it by typing `/effort high`. In the app's structured view (#938, #943), which
shows the conversation as readable messages, there is no way to change it: there is no control and
`/effort` does nothing. This feature adds an effort control beside the model switcher (#943),
makes `/effort <level>` work in the message box, shows the level in use, applies a change from
the next answer on, and keeps the choice with the session, as the model choice is kept.

## Context

| Feature | Delivers | State |
|---|---|---|
| #938 | The structured session kind, event model, daemon side, resume | merged (`specs/938-structured-session-mode`) |
| #941 | The composer: sending, the queue, idle/busy | merged |
| #943 | Slash commands and their palette, **model switching**, switching kind | merged (`specs/943-structured-commands-fallback`) |
| **#1554 (this)** | Reasoning effort: the control, `/effort`, showing it, keeping it | |

**Boundaries.** #943 owns the model picker, the command palette and how an app-handled command is
recognised; this feature adds a second app-handled command (`/effort`) and a second session
setting beside the model, through the same paths. #943 listed "changing reasoning effort" as out
of scope (`specs/943-structured-commands-fallback/spec.md#Out of Scope`); this feature is that
follow-up and does not change anything else #943 specified.

## Terms

- **Effort level** (level): one of `low`, `medium`, `high`, `xhigh`, `max`, in that order from
  least to most reasoning, as Claude Code names them.
- **Effort choice**: the level the user picked for a session, or **Default** (no level set by the
  app: Claude Code uses its own default, as configured for that location).
- **Supported levels**: the levels the CLI reports the session model accepts. The CLI reports, per
  model, whether it supports effort at all and which levels (see Assumptions).
- **Offered levels**: the levels the picker lists after Default: the supported levels when the
  CLI reports some, otherwise all five.
- **Effective level**: the level a session actually runs with: the effort choice when the session
  model supports it (or support is unknown), otherwise none, so Claude Code's default applies.
- **Effort picker**: the control in the structured view that shows the effort choice and changes
  it.
- **Structured session**, **terminal session**, **turn**, **event history**: as feature 938
  defines them. **Composer**, **draft**, **queue**, **busy**, **idle**: as feature 941 defines them.
  **Command palette**, **app-handled command**, **model choice**, **session model**, **switching
  kind**: as feature 943 defines them.

## Clarifications

### Session 2026-10-27

- Q: When the session model does not support effort, can the user still change the effort choice,
  or is the picker unavailable? → A: It can be changed. The picker says the model does not support
  effort and still opens, listing Default and the five levels; a pick is stored, sends nothing to
  the CLI, and applies when the session moves to a model that supports it. Edge case "Model without
  effort support" and FR-003 amended. _(agent-resolved:
  specs/1554-structured-effort-switch/spec.md#FR-002 — "change … at any time"; #Edge Cases
  "Session not running" — a choice is stored and used later; ledger D2)_
- Q: When a launch (restart, restore, switch of kind) carries a stored level the session model is
  known not to support, what happens? → A: The session launches anyway: a stored choice never
  stops or fails a launch. The app names the level at launch only when the latest supported levels
  known for that model include it, or none are known; otherwise it names no level, so Claude
  Code's default applies, and the view says so (FR-003). Added FR-010a. _(agent-resolved:
  ledger D2 — "CLI default applies until a supporting model"; spec.md#FR-011 — naming no level
  leaves Claude Code's default)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pick the effort level from the view (Priority: P1)

The user is about to ask a hard design question in a structured session and wants Claude to think
harder. Beside the model switcher they see the effort level in use, open the effort picker and pick
`high`. Their next message is answered with high effort; the conversation goes on.

**Why this priority**: It is the issue's core: today the level cannot be changed at all in the
structured view, short of switching the session to the terminal.

**Independent Test**: With the stand-in CLI, pick `high` for an idle session: the CLI is asked to
use high effort for later turns and the conversation is kept; the view and a second test client
show `high`; the next turn is sent after the change took effect.

**Acceptance Scenarios**:

1. **Given** a structured Claude Code session, **When** its view is shown, **Then** the effort
   picker is shown beside the model switcher, showing the effort choice (`Default` or a level).
2. **Given** the effort picker, **When** it opens, **Then** it lists Default and the supported
   levels of the session model in order from least to most, and marks the current choice.
3. **Given** an idle running session, **When** the user picks a level, **Then** the next turn runs
   with that level, the conversation is kept, and every window viewing the session shows the new
   choice.
4. **Given** a busy session, **When** the user picks a level, **Then** the choice is accepted, the
   turn in progress is not interrupted and keeps its level, and the new level applies from the next
   turn on.
5. **Given** a level change, **When** the transcript is shown, **Then** a marker at that point says
   which effort level applies from there on.
6. **Given** a session with a level chosen, **When** the user picks Default, **Then** the session
   goes back to Claude Code's own default from the next turn, and the view shows `Default`.
7. **Given** a change the CLI refuses, **When** it is refused, **Then** the view shows the reason,
   the effort choice is unchanged, and the session keeps running.

---

### User Story 2 - Type `/effort <level>` in the composer (Priority: P1)

The user is used to the terminal view and types `/effort max` in the message box. It does what it
does in the terminal: the level changes, and nothing is sent to Claude as a message.

**Why this priority**: Named in the issue; today the command is sent as text and has no effect,
which is worse than nothing because it looks as though it worked.

**Independent Test**: With the stand-in CLI, send `/effort max`, `/effort`, `/effort Max` and
`/effort extreme` from the composer: the first and third set `max`, the second opens the picker,
the fourth shows the accepted levels; none of them reaches the CLI as a user message.

**Acceptance Scenarios**:

1. **Given** the composer, **When** the user sends `/effort <level>` with a level name in any
   letter case, **Then** it is treated as picking that level (User Story 1, scenarios 3 to 7) and
   no user message reaches the CLI.
2. **Given** the composer, **When** the user sends `/effort` with no argument, **Then** nothing
   reaches the CLI and the effort picker opens.
3. **Given** the composer, **When** the user sends `/effort default` or `/effort auto`, **Then**
   it is treated as picking Default.
4. **Given** the composer, **When** the user sends `/effort` with an argument that is not a level
   or `default`, **Then** nothing reaches the CLI, the draft is kept, and the view says which
   values are accepted.
5. **Given** the command palette, **When** the user types `/eff`, **Then** `/effort` is listed as
   an app-handled command with a description saying it sets the reasoning effort, once (not twice
   when the CLI also reports it).
6. **Given** a busy session, **When** the user sends `/effort <level>`, **Then** it is acted on at
   once, not queued (as `/model` is, feature 943 Edge Cases), and applies from the next turn.

---

### User Story 3 - The choice stays with the session (Priority: P2)

The user set `high` on a session yesterday. Today, after the app and the daemon restarted, the
session still runs with `high`; switching it to the terminal and back keeps it too.

**Why this priority**: The issue asks for it ("persists for the session, same as the model"), but
the level can be re-picked by hand, so losing it is an annoyance, not a block.

**Independent Test**: Pick `xhigh`, restart the session: the CLI is launched with `xhigh` and
resumes the same conversation; restart the daemon and check again; switch to the terminal and
check the terminal launch uses `xhigh`; switch back and check again.

**Acceptance Scenarios**:

1. **Given** a session with an effort choice, **When** it is restarted, restored after a daemon
   restart, or switched to the terminal and back, **Then** it runs with the same choice.
2. **Given** a session with an effort choice, **When** it runs as a terminal session, **Then** the
   terminal's Claude Code runs with that level.
3. **Given** a new session, **When** it starts, **Then** its effort choice is Default.
4. **Given** two sessions, **When** the level of one is changed, **Then** the other's is unchanged
   (Principle II).

---

### Edge Cases

- **Empty input**: `/effort` alone opens the picker (US2 scenario 2); `/effort` followed only by
  spaces is the same. `/effort` inside a longer message (not at the start) is not a command and is
  sent as text (feature 943 Edge Cases).
- **Model without effort support**: when the CLI reports that the session model does not support
  effort, the picker says so in words and Claude Code's default applies; the picker still opens,
  listing Default and the five levels, and a pick (or `/effort <level>`) is stored and applies when
  the session moves to a model that supports it (see Clarifications).
- **Model change to one that does not support the chosen level**: the choice is kept; while the
  session model does not support it, the view says the level is not available for this model and
  Claude Code's default applies; it applies again when the session moves to a model that supports
  it.
- **No supported levels reported** (the CLI did not report the model list, or the session never
  started): the picker lists Default and the five levels.
- **Session not running**: the choice can still be changed; it is stored and used at the next
  launch. Nothing is sent until then.
- **Level picked while a permission request is pending**: the request stays pending; the new level
  applies from the next turn.
- **Two windows pick different levels at once** (Principle II): the last choice to reach the daemon
  wins and every window shows it; no window shows a choice other than the one the daemon holds. A
  choice is shown as soon as the daemon holds it; the transcript marker (FR-004) sits where it
  starts to apply.
- **Several sessions** (Principle II): effort choices are per session; changing one never affects
  another, in the same worktree or not.
- **Sandboxed sessions** (CLI in a container): picking and keeping the level works the same.
- **Effort changed inside the terminal** (the user ran `/effort` in the terminal screen): the app
  cannot see it; on switching back, the session's effort choice is used, and the guide says so.
- **Terminal session** (a pick reaching the daemon for a session currently in the terminal kind,
  e.g. from a window not yet updated): the choice is stored and used at the next launch, and the
  answer says so (as a model pick does, feature 943); nothing is sent to the running terminal.
- **Sessions of other AI CLIs**: they show no effort picker, and a request to set one is refused
  with a reason.
- **Failure**: a change the CLI refuses, or one that cannot be delivered because the CLI process
  ended, leaves the choice unchanged and shows why (US1 scenario 7).
- **Platforms** (Principle VI): the picker, `/effort` and keeping the choice behave the same on
  Linux, macOS and Windows.
- **Light and dark theme**: the picker, the level shown, the unavailable state and the transcript
  marker are readable in both themes, never conveyed by colour alone.

## Requirements *(mandatory)*

### Functional Requirements

**Showing and picking**

- **FR-001**: A structured Claude Code session's view MUST show its effort choice (`Default` or the
  level) beside the model switcher.
- **FR-002**: A user MUST be able to change a structured session's effort choice at any time from
  its view; the picker MUST offer Default and the supported levels of the session model, in order
  from least to most, or all five levels when none are reported, and MUST mark the current choice.
- **FR-003**: When the CLI reports that the session model does not support effort, or does not
  support the chosen level, the view MUST say so in words; the stored choice MUST be kept, and the
  user MUST still be able to change it (the change is stored and applies on a model that supports
  it).

**Applying**

- **FR-004**: A change MUST apply from the next turn, MUST keep the conversation, MUST NOT interrupt
  or alter a turn in progress, and MUST be shown in every window viewing the session within 1
  second of the app accepting it, with a marker in the transcript where it starts to apply.
- **FR-005**: A change the CLI refuses, or one that cannot be delivered, MUST leave the effort
  choice unchanged and show the reason; the session MUST keep running.
- **FR-006**: Picking Default MUST return the session to Claude Code's own default effort from the
  next turn.

**Composer command**

- **FR-007**: Sending `/effort <level>` (any letter case), or `/effort default` or `/effort auto`
  (both meaning Default), MUST be treated as picking that value, and MUST send nothing to the CLI as a user message; sending `/effort` with no
  argument MUST open the picker; any other argument, including a level outside those the picker
  offers for the session model, MUST be refused in the view, naming the accepted values, with the
  draft kept.
- **FR-008**: `/effort` MUST be acted on at once whether the session is idle or busy, never queued.
- **FR-009**: The command palette MUST list `/effort` once, as an app-handled command, with a
  description, whether or not the CLI also reports it.

**Keeping the choice**

- **FR-010**: The effort choice MUST be persisted with the session and used on every later launch
  of it, in either kind (restart, restore after a daemon restart, switch of kind).
- **FR-010a**: A stored effort choice MUST never stop or fail a launch: when the latest supported
  levels known for the session model exclude the stored level, the launch MUST name no level, so
  Claude Code's default applies (FR-003).
- **FR-011**: A new session's effort choice MUST be Default, and launching with Default MUST name
  no level to the CLI, so that Claude Code's own default and the user's own Claude Code settings
  apply as today.
- **FR-012**: Effort choices MUST be per session; sessions of other AI CLIs MUST NOT offer an
  effort picker, and a request to set one MUST be refused with a reason.

**Testing and documentation**

- **FR-013**: Picking a level, picking Default, a refused change, `/effort` with a level, with no
  argument and with a wrong argument, a relaunch with a stored choice, and a relaunch with a stored
  level the session model does not support (FR-010a) MUST be covered by tests
  that run the daemon against the stand-in CLI.
- **FR-014**: The user guide's structured-sessions section MUST describe the effort picker,
  `/effort`, what the levels mean in a sentence each, and that a change made inside the terminal
  screen is not seen, in the same change that ships them.

### Constraints

- **Mirror model switching**: the effort choice travels the same way the model choice of feature
  943 does (stored with the session, carried to every window, applied at launch and to a running
  session); no second mechanism is introduced where #943's can be extended.
- **Shared UI components (Principle VIII)**: the effort picker is built from, or added to, the shared
  component library, as the model picker is.
- **Terminal sessions created as such** behave exactly as today unless they carry an effort choice
  from a structured period (US3 scenario 2).

### Out of Scope

- Choosing an effort level when creating a session; a new session starts on Default and can be
  changed at once.
- Claude Code's other `/effort` arguments (`ultracode`): refused like any wrong argument (FR-007).
- Other per-session settings: output style, fast mode, turning thinking off, and anything else
  beyond the model and the effort level.
- Detecting an effort change made inside the terminal screen.
- Reasoning effort for sessions of other AI CLIs.
- Showing, per turn, which effort level a turn actually ran with when the CLI does not report it;
  the transcript marker (FR-004) records changes.

### Key Entities

- **Effort choice**: per session: Default or one of the five levels; persisted with the session,
  beside the model choice.
- **Supported levels**: per model the CLI reports: whether it supports effort, and which levels; as
  of the session's latest start.
- **Effort change marker**: a point in a structured session's event history where the effort choice
  changed, with the new value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of test runs, a level picked for an idle or busy session applies to the next
  turn and not to a turn in progress, without the conversation being lost.
- **SC-002**: Every window viewing the session shows a new effort choice within 1 second of the
  app accepting it.
- **SC-003**: In 100% of test runs, `/effort` in any of its forms (a level, `default`, no argument,
  a wrong argument) never reaches the CLI as a user message.
- **SC-004**: Across restarts, a daemon restart and a structured → terminal → structured switch, the
  effort choice is the same at every launch in 100% of test runs.
- **SC-005**: Changing the level takes the user at most two clicks from the view, or one sent
  message from the composer.
- **SC-006**: Every existing terminal-session and structured-session test passes unchanged.

## Assumptions

- The CLI's structured mode accepts a request to change the effort level of a running session for
  later turns, or the `/effort` command itself, without ending the process; if it does not, the
  plan applies a change by relaunching the session, resuming the same conversation, once it is
  idle, which still meets FR-004. Settled by the plan (research P-1, R1): a running CLI takes the
  change as a control request and keeps its process.
- The CLI accepts an effort level at launch, in both kinds, so a stored choice can be applied when
  a session starts (FR-010).
- The CLI reports, per model, whether effort is supported and which levels (recorded in
  `crates/micold-core/tests/fixtures/stream-json/command-initialize.jsonl`); the level a session currently uses is readable on request (`get_settings`, research R10), but
  the view shows the effort choice rather than a level read back from the CLI.
- The five level names are those Claude Code accepts today; a later release that adds or drops a
  level shows up through the supported levels the CLI reports.
