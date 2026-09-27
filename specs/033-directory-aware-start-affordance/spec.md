# Feature Specification: The start affordance answers for its own directory

**Feature Branch**: `fix/a-project-local-cli-stays-unreachable-in-that-project`

**Created**: 2026-09-27

**Status**: Draft

**Input**: User description: "bug: The start affordance's chevron, and its primary press, read one
global AI-CLI availability set rather than the answer for the row's own directory — so after a
reconnect or after Settings is opened, that set holds the home directory's answer, and a CLI that
only a project-local environment-include script puts on PATH can stay unreachable in the very
project whose script provides it." Raised by review A of
[029 BUG-001](../029-pi-cli-provider/bugs/BUG-001.md) and declined there
([ledger, *Declined review findings*](../029-pi-cli-provider/bugs/BUG-001.autopilot.md)): feature
029's FR-003b names exactly two directory-aware placements — the per-session override and the
missing-CLI list — and a directory-aware affordance done naively would be a `PATH` lookup per
sidebar row per render, which 029's SC-006 and feature 026's research R11 forbid. Making the
affordance itself answer for its own directory is behaviour 029 never intended, so it is specified
here rather than patched into 029.

## Background: what happens today

Reproduced at code level on `main` at `23a0e0ee` (details in the ledger, D1):

- The application holds **one** answer to "which AI CLIs can a session start with?" for the whole
  window. It is asked for on connecting to the session service and on opening Settings — both times
  about the **home directory** — and on opening a row's "start on…" list, about **that row's
  directory**. Each answer replaces the last.
- Every sidebar row's start affordance reads that one answer. The chevron (the "choose which CLI"
  half) is drawn only when the answer lists two or more CLIs, and the primary press starts the
  default only when the answer lists it.
- So a user whose home directory resolves one CLI (say `claude`) while a project's
  environment-include script adds a second (say `pi`) sees **no chevron** on that project's rows.
  The only thing that would ask about the project's directory is opening the list the chevron opens,
  and the chevron is not drawn. The second CLI cannot be started in the one project that provides
  it, short of changing the stored default in Settings — where it is not offered either, because
  Settings answers for the home directory.
- The reverse also happens: after the list is opened in one worktree, every other row reads that
  worktree's answer until the next connect or Settings open.
- One case already recovers: when the stored default is missing from the home answer but present in
  the project, the primary press opens the list, the list asks about the project's directory, and
  the default appears one press later.

## Clarifications

### Session 2026-09-27

- Q: Is a worktree row answered for its own directory or for its project's root? → A: Its own
  directory. A worktree session is spawned in the worktree (constitution Principle III), 029
  FR-003b decides availability against the environment a session started in that directory would
  receive, and the session service's environment-include cache is keyed by the spawn directory, so
  FR-006's "the resolution a session spawn in that directory uses" is the worktree's own. Rows of
  the project root ("Default") are answered for the root. _(agent-resolved:
  specs/029-pi-cli-provider/spec.md#FR-003b; .specify/memory/constitution.md#III;
  crates/micold-daemon/src/state.rs#env_include_vars_for)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A CLI one project provides can be started in that project (Priority: P1)

A developer's home shell puts `claude` on `PATH`. One project's environment-include script also puts
`pi` there (a Node version manager activated per directory, for example). In that project's sidebar
rows they can choose `pi` for a new session, and in a project without the script they are not
offered it.

**Why this priority**: It is the reported defect. Today the CLI is unreachable from the one place
that provides it, and nothing on screen says why.

**Independent Test**: Configure a home environment with one AI CLI and a project whose
environment-include script adds a second. Open the project and, without opening Settings or the
list first, bring a project row to the point where its own answer is asked for (FR-006) and check
that it offers the choice and that starting the second CLI works;
open a second project without the script and check that its rows offer no choice.

**Acceptance Scenarios**:

1. **Given** the home directory resolves only `claude` and project P's environment-include script
   also resolves `pi`, **When** P is open and its rows are shown, **Then** each of P's rows offers
   the choice of CLI and the list it opens includes `pi`.
2. **Given** the same setup and a second project Q without the script, **When** Q is open, **Then**
   Q's rows offer no choice (one CLI available) and the primary press starts `claude`.
3. **Given** P's rows offer `pi`, **When** the user opens and closes Settings, **Then** P's rows
   still offer `pi` throughout — the home-directory answer Settings asks for never replaces P's.
4. **Given** P's rows offer `pi`, **When** the user reconnects to the session service, **Then** P's
   rows offer `pi` again as soon as P's re-asked answer arrives, with no user action (FR-011).
5. **Given** the stored default is `pi` and only P resolves it, **When** the user presses the
   primary half on one of P's rows, **Then** a `pi` session starts in that row's directory with no
   intermediate list.

---

### User Story 2 - One row's answer never leaks into another's (Priority: P2)

A project with several worktrees, where the worktrees resolve different CLIs (one worktree pins a
tool version the others do not). Each row's affordance reflects its own directory, whatever list
was opened last.

**Why this priority**: The leak is the same defect seen from the other side: it can offer a CLI
that will fail to start in the row the user presses, or hide one that would succeed.

**Independent Test**: Two worktrees of one project with different environment-include results. Open
the start list on worktree A, close it, and check that worktree B's affordance still shows B's
answer.

**Acceptance Scenarios**:

1. **Given** worktree A resolves `claude` and `pi` and worktree B resolves only `claude`, **When**
   the user opens A's start list and closes it, **Then** B's row offers no choice and its primary
   press starts `claude`.
2. **Given** the same worktrees, **When** the user opens B's start list, **Then** A's row still
   offers the choice.

---

### User Story 3 - The answer follows the events that change it (Priority: P3)

The developer edits the environment-include settings (a different script, the feature switched off)
or deletes and recreates a worktree. The affordances reflect the new environment without a restart,
and without the application doing any periodic work while nothing changes.

**Why this priority**: Without it the per-directory answer would be a new way to be stale; with it
the feature costs nothing while idle.

**Independent Test**: With P's rows offering `pi`, switch environment-include off in Settings and
save; check P's rows stop offering `pi`. Switch it back on; check they offer it again. Leave the
application idle and confirm it schedules no timers or wakeups for this feature.

**Acceptance Scenarios**:

1. **Given** P's rows offer `pi` through the script, **When** the user switches environment-include
   off and saves Settings, **Then** P's rows stop offering `pi` without any other action.
2. **Given** a row whose answer is known, **When** nothing that can change it happens, **Then** the
   application asks nothing and schedules nothing for that row, however long it stays open.
3. **Given** the user installs a CLI while the application is open, **When** they open a row's start
   list, **Then** that row's answer is refreshed and the new CLI is offered there.

---

### Edge Cases

- **Answer not known yet** (a row has just appeared, the service has not replied): the row's
  affordance behaves as it does today, from the home-directory answer, and changes once its own
  answer arrives — it is never blank, disabled or spinning because of this feature.
- **Slow or failing environment-include script**: the row keeps the home-directory answer until its
  own arrives; a script that times out or fails yields the answer a session spawned there would get
  (the service's own `PATH`), exactly as the spawn does. No error is shown on the row for this.
- **Stale answer at press time**: a row can still be out of date by a press (a CLI uninstalled since
  its answer). The launch-time check (026 FR-010) still reports the missing CLI; the affordance
  never starts a different CLI from the one the user asked for.
- **No CLI in the home directory, one in the project**: the project's rows read the project's
  answer, so the primary press starts the default if the project provides it and otherwise opens
  the project's list with the default marked unavailable (FR-010). It does not fall through to
  "nothing available".
- **Many rows**: a project with dozens of worktrees must not make opening the project, scrolling
  the sidebar or redrawing the window cost one availability check per row per frame (see FR-006).
- **Several sessions and windows** (Principle II): answers are per directory, so two rows for the
  same directory — or two clients asking about it — see the same answer; answers for different
  directories never overwrite each other, whichever arrives last.
- **Sessions run in a container** (feature 027): the answer is still settled where sessions run, per
  directory inside that environment; 027 FR-023b's "not in this image" wording is unchanged.
- **Reconnect to a different service** (a restart, a switch between host and container): every
  held answer is dropped and re-asked, since it described a different place.
- **Platforms** (Principle VI): Linux, macOS and Windows behave identically; the presence check and
  the environment-include resolution are the existing per-platform ones, unchanged.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The start affordance on each sidebar row MUST decide both whether to show the choice
  of CLI and what the primary press does from the availability answer **for that row's directory**
  — the directory a session started from that row would run in (029 FR-003b's rule, extended to the
  affordance). [NEEDS CLARIFICATION: Should the affordance become directory-aware at all, or is a
  cheaper product answer preferred — (a) a per-directory answer as specified here; (b) keep the one
  window-wide answer but always draw the chevron whenever any CLI is known, so the list (already
  directory-aware) is always one press away; (c) leave the behaviour as 029 shipped it and document
  the Settings workaround?]
- **FR-002**: An answer for one directory MUST NOT replace or be read as the answer for another. In
  particular, the home-directory answer asked for on connecting and on opening Settings MUST NOT
  replace any row's own answer.
- **FR-003**: The application MUST hold at most one current answer per directory, in memory only,
  and MUST NOT persist any availability answer (026 research R11). This widens R11's timing, which
  was "probed when the choice is offered, and again at launch": a row that *could* offer the choice
  is now asked about ahead of the press, on the FR-004 events. The "never persisted" half is kept
  unchanged.
- **FR-004**: A row's answer MUST be refreshed on, and only on, these events: the row's first ask
  (when that happens is FR-006's question; a row's directory *appears* when a project is opened or a
  worktree is added or discovered, never by scrolling into view); (re)connecting to the session
  service; a saved change to the environment-include settings; the row's directory being deleted or
  recreated; and the user opening that row's start list. No other event and no timer triggers a refresh.
- **FR-005**: Until a row's own answer is known, its affordance MUST behave as it does today, from
  the home-directory answer, and MUST switch to the row's answer when it arrives without any user
  action.
- **FR-006**: Drawing, scrolling or resizing the sidebar MUST NOT cause any availability check or
  environment resolution; the cost of a redraw MUST NOT grow with the number of rows beyond reading
  held answers. Rows for the same directory MUST share one answer and one resolution, and the
  resolution MUST be the one a session spawn in that directory uses (029 FR-003b, SC-006a).
  [NEEDS CLARIFICATION: When is a row's first answer asked for — (a) eagerly for every row as soon
  as the project opens (one environment-include resolution per directory, up front); (b) lazily, the
  first time a row is pointed at, focused or pressed, keeping the home answer until then; (c) only
  for the selected row and the row the user interacts with?]
- **FR-007**: Worktree rows MUST be answered for their own directory, not their project's root;
  rows of the project root ("Default") are answered for the root. One resolution per distinct
  directory, shared with the session spawns there (see Clarifications).
- **FR-008**: The Settings default and its "not installed" sentence MUST keep answering for the home
  directory (029 FR-003b); the per-session override list and the missing-CLI list MUST keep
  answering for their own directory, and MUST update that directory's held answer rather than the
  window-wide one.
- **FR-009**: An answer that arrives after a newer answer for the **same** directory was asked for
  MUST be discarded; answers for **different** directories MUST all be kept, in whatever order they
  arrive.
- **FR-010**: The primary press MUST never start a CLI other than the stored default or the one the
  user chose; when the row's answer lacks the default, the press MUST open the row's list with the
  default marked unavailable (026 FR-002), exactly as today.
- **FR-011**: On reconnecting to the session service, every held answer MUST be discarded and the
  answers re-asked for the rows that had one, by the same rule as a first ask (FR-006), since the service may now describe a different place.

### Key Entities

- **Directory availability answer**: which AI CLIs a session started in one directory would find,
  and what that answer describes (this computer or a named image). One
  per directory; in memory; replaced only by a newer answer for the same directory.
- **Home-directory answer**: the answer for the user's home directory, used by Settings and as the
  fallback for a row whose own answer is not known yet.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user whose second AI CLI is provided only by one project's environment-include
  script can start a session on it from that project's rows in at most two presses, without opening
  Settings or changing their installation — and is offered it in no other project.
- **SC-002**: After a reconnect, a Settings open, or a start list opened on another row, 100% of
  rows on screen still offer exactly the CLIs their own directory provides (once each row's answer
  has arrived).
- **SC-003**: With the application idle, this feature schedules zero timers and zero periodic
  wakeups, and redrawing the sidebar performs zero availability checks, whatever the number of rows
  — verified structurally, as 029's SC-006 is.
- **SC-004**: Opening a project with N rows over D distinct directories causes at most D
  environment resolutions for this feature, each shared with the session spawns in that directory,
  and none on any later redraw.
- **SC-005**: A saved change to environment-include is reflected on every row on screen without a
  restart and without any user action beyond saving.

## Assumptions

- The session service's per-directory availability question and its per-directory environment
  cache (features 011 and 029) are reused as they are; this feature changes who asks and what is
  kept, not how the service answers.
- Installing or removing a CLI outside the application is not observed (that would need polling,
  which SC-003 forbids). It is picked up by the next refreshing event, and the launch-time check
  (026 FR-010) covers a press in between.
- The fallback while a row's answer is pending is the home-directory answer, which is exactly
  today's behaviour, so the feature is never worse than `main` for any row.
- The visual form of the affordance (split button, chevron, list) is unchanged; only which answer
  it reads changes.

## Out of Scope

- Making the Settings default directory-aware or offering a per-project default CLI.
- Observing CLI installs and uninstalls as they happen.
- The pre-existing race and missing single-flight in the environment-include cache recorded in
  [029 BUG-001's follow-ups](../029-pi-cli-provider/bugs/BUG-001.autopilot.md) (feature 011's
  work), except where a plan for this feature finds it must be fixed to meet SC-004.
- Any change to how the service decides availability (the presence check, 029 FR-003a).
