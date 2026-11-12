# Feature Specification: Autopilot supervisor M2: supervisor skill that queues issues and starts sessions

**Feature Branch**: `feat/1813_autopilot-supervisor-m2-supervisor-skill-that`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "autopilot supervisor M2: supervisor skill that queues issues and starts sessions (issue #1813)"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a queue of issues under a cap (Priority: P1)

A person gives the supervisor a list of issues, or the label `autopilot:queue`, in one session at
the project root. The supervisor starts one autopilot session per issue, never more than the cap
at once (default 3), starts the next queued issue when a running one finishes, and finishes when
the queue is empty and every session has ended.

**Why this priority**: this is the feature's "done when": three issues, a cap of two, two start
first and the third starts when one finishes.

**Independent Test**: run the supervisor's decision step against fixture queue state, fixture
`run-status.sh --json` answers and a fake session starter; assert which issues it starts.

**Acceptance Scenarios**:

1. **Given** three queued issues, a cap of two and nothing running, **When** the supervisor
   advances, **Then** it starts exactly two sessions and leaves the third `queued`.
2. **Given** two running sessions and one queued issue, **When** one session's status becomes
   `done`, **Then** the supervisor starts the queued issue.
3. **Given** the label `autopilot:queue` and no explicit list, **When** the supervisor starts,
   **Then** it queues the open issues carrying that label, in ascending issue number.
4. **Given** a started issue, **When** the supervisor starts it, **Then** it creates the worktree
   for the issue and starts the session with the prompt `/speckit-autopilot #<n>` and the
   supervised marker from M1 (`AUTOPILOT_SUPERVISED`).

---

### User Story 2 - Resource gate before every start (Priority: P1)

Before it starts any session the supervisor checks the machine: free memory, load average and the
cap on parallel sessions. If any check fails it starts nothing and says which check failed and
why, then tries again on its next advance.

**Why this priority**: unattended sessions that exhaust memory take the whole machine down.

**Independent Test**: feed the gate fixture readings (memory, load, running count) and assert pass
or the named failing check.

**Acceptance Scenarios**:

1. **Given** free memory below the threshold, **When** the gate runs, **Then** it fails naming
   memory and no session starts.
2. **Given** a load average above the threshold, **When** the gate runs, **Then** it fails naming
   load.
3. **Given** the number of running sessions equal to the cap, **When** the gate runs, **Then** it
   fails naming the cap.
4. **Given** all readings within limits, **When** the gate runs, **Then** it passes.

---

### User Story 3 - Sequence issues that touch the same files (Priority: P2)

Two issues that will edit the same files must not run in parallel. The supervisor reads declared
file overlap from the issue (a `Touches:` line) or infers it from the `flows` and spec paths, and
holds the later issue `queued` until the earlier one no longer occupies a slot (it is `done`, `failed`, `lost` or `skipped`), even when the cap has room.

**Why this priority**: parallel edits of the same files produce rebase conflicts the sessions
cannot settle alone; correct without it, but wasteful.

**Independent Test**: fixture issues with overlapping and disjoint touch sets, cap 3; assert only
the disjoint ones start together.

**Acceptance Scenarios**:

1. **Given** issues A and B that declare an overlapping path, **When** the supervisor advances,
   **Then** it starts A and holds B until A is `done`.
2. **Given** issues with disjoint paths, **When** the supervisor advances, **Then** it starts them
   in parallel up to the cap.
3. **Given** an issue with no declared paths and no inferable ones, **When** the supervisor
   advances, **Then** it treats it as disjoint and says so in the ledger.

---

### User Story 4 - A restarted supervisor resumes from its ledger (Priority: P1)

The supervisor records the queue and each session's id in a supervisor ledger file, updated
before each start and after each state change. A restarted supervisor reads that file, matches
each recorded session against `run-status.sh --json`, and continues: it does not restart an issue
that already has a live session and it does not lose the queue.

**Why this priority**: the supervisor's own session can end; without the ledger every restart
would rebuild state from memory and risk starting an issue twice.

**Independent Test**: build a ledger with one running, one done and one queued issue, restart the
supervisor, and assert it starts only the queued one (cap permitting).

**Acceptance Scenarios**:

1. **Given** a ledger with issue 1 running (session id and worktree path recorded) and issue 2 queued, **When** the
   supervisor restarts, **Then** it keeps issue 1 as running and does not start a second session
   for it.
2. **Given** a ledger whose running issue's status is now `done`, **When** the supervisor
   restarts, **Then** it marks it `done` and counts the freed slot.
3. **Given** a recorded issue whose worktree directory is gone, or whose session id `list_sessions`
   (metadata only) no longer lists while its status is not `done`, **When** the supervisor
   restarts, **Then** it marks the issue `lost` and reports it instead of silently restarting it.
4. **Given** a session started a moment ago whose status is still `dead-no-ledger` (its autopilot
   has not written a ledger yet), **When** the supervisor advances within the grace period after
   the start, **Then** it counts the issue as `running` and raises no alarm; past the grace period
   it reports it as dead.

---

### User Story 5 - Small supervisor context (Priority: P2)

The supervisor reads only `run-status.sh --json` per session and the ledger. It never reads a
session transcript, and its output per advance is a short table.

**Why this priority**: the supervisor lives for hours; a growing context makes every call dearer
(CLAUDE.md, *Keep the context small*).

**Independent Test**: inspect the skill text and the advance step: no instruction reads session
output; the status read is `--json`.

**Acceptance Scenarios**:

1. **Given** the skill, **When** a reader looks for transcript reads (`read_session_output`),
   **Then** none is instructed.
2. **Given** a session whose status is `asking`, **When** the supervisor advances, **Then** it
   reports the question from the JSON `escalation` field and does not read the session.

### Edge Cases

- An *advance* is one read-decide-start pass. The supervisor waits between advances without a
  polling loop in its own context; what wakes it is an open question for the clarify unit.

- Empty input: no issues and no `autopilot:queue` label: the supervisor reports "nothing to do"
  and ends; it creates no ledger entry.
- An issue number that is closed, missing or already has a worktree/ledger of its own: it is
  skipped with a reason in the ledger, not started twice.
- Failure: `create_worktree` or `create_session` fails: the issue goes back to `queued` with the
  error recorded, and the failure counts as a try; after 3 tries it is `failed` (no endless loop).
- Concurrency (Principle II): two supervisors on the same ledger. The ledger names the supervisor
  session that owns it; a second supervisor refuses to start while the owner session is listed by `list_sessions`, and
  says which session owns the ledger. Issues already in the ledger count as claimed.
- A session in state `asking`, `stuck` or `blocked` keeps its slot; the supervisor reports it and
  never answers for the human.
- Cross-platform (Principle VI): free memory and load average are read through a helper that works
  on Linux and macOS (`/proc/meminfo` or `vm_stat`; `uptime`/`getloadavg`).
- Restart while a start was in flight: the ledger notes `starting` before the MCP calls, so the
  restarted supervisor looks for an existing worktree/session for the issue before starting again.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The supervisor MUST accept either an explicit list of issue numbers or the label
  `autopilot:queue`, and build one queue from it.
- **FR-002**: Before starting a session the supervisor MUST apply a gate of free memory, load
  average and a cap on parallel sessions (default 3, overridable), and MUST start nothing and name
  the failing check when any fails.
- **FR-003**: For each issue it starts, the supervisor MUST create the worktree with
  `create_worktree` (`github_issue` set) and then start the session with `create_session` using the
  prompt `/speckit-autopilot #<n>` and the supervised marker.
- **FR-004**: The supervisor MUST NOT start an issue while another issue with overlapping touched
  files occupies a slot (any ledger state except `done`, `failed`, `lost`, `skipped`), and MUST take the touched files from the issue's declaration first, and from
  `flows`/spec paths otherwise.
- **FR-005**: The supervisor MUST record the queue, each issue's state (`queued`, `starting`,
  `running`, `done`, `failed`, `lost`, `skipped`) and each session id in a supervisor ledger file,
  and MUST update it before each start and after each observed state change.
- **FR-006**: A restarted supervisor MUST continue from the ledger, matching each recorded issue's
  worktree against `run-status.sh --json <worktree-dir>`, and MUST NOT start an issue that already has a live session.
- **FR-007**: The supervisor MUST read run state only through `run-status.sh --json` (plus
  `list_sessions` metadata for liveness) and MUST NOT read session transcripts.
- **FR-008**: The supervisor MUST count a session in any state except `done` (and `lost`,
  `failed`) as occupying a slot, and MUST start the next queued issue when a slot frees.
- **FR-009**: The supervisor MUST report `asking`, `stuck`, `blocked` and `dead-no-ledger` runs to
  the person with the JSON's question or reason, and MUST NOT answer or restart them itself,
  except that `dead-no-ledger` within a grace period after the start counts as `running`.
- **FR-011**: The mapping from the run-status states to the supervisor ledger states MUST be
  defined: `done` to `done`; `dead-no-ledger` past the grace period to `lost`; every other state
  keeps the issue `running` (and its slot).
- **FR-010**: The decisions (gate, overlap, which issues to start) MUST be reproducible from fixture
  inputs without live sessions.

### Key Entities

- **Supervisor ledger**: a file listing the owner session, the cap, and one row per issue: number,
  state, touched paths, worktree, session id, tries, note.
- **Queue entry**: an issue and its state in the ledger.
- **Gate reading**: free memory, load average and running-session count against their limits.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With three issues and a cap of two, the first advance starts exactly two sessions and
  a later advance, after one reports `done`, starts the third.
- **SC-002**: A restarted supervisor reproduces the same queue and starts zero duplicate sessions
  for issues already recorded as running.
- **SC-003**: A failing gate check results in zero session starts and one line naming the check.
- **SC-004**: One advance reads each running issue's status once and no session
  transcript.

## Assumptions

- M1 (#1812) is merged: `scripts/autopilot/run-status.sh --json` and `supervised.sh` exist, and
  `AUTOPILOT_SUPERVISED` marks a supervised run.
- The micold MCP tools `create_worktree` and `create_session` are available to the supervisor
  session, as `CLAUDE.md` describes for "Start work on issue".
- A session's slot is released when its status is `done`; a merged PR is handled by that session.
- [NEEDS CLARIFICATION: where does the supervisor ledger live and how is it kept out of feature
  branches: a gitignored file under the project root, or committed on main?]
- [NEEDS CLARIFICATION: how does the supervisor learn the `done` of a session started in another
  worktree: run `run-status.sh <worktree-dir>` for each recorded worktree path?]
- [NEEDS CLARIFICATION: default memory and load thresholds when the person gives none.]

## Out of Scope

- Answering escalations, resuming sessions after `asking` (a later milestone).
- Merging PRs or changing the sessions' own flow.
- A GUI for the queue.
