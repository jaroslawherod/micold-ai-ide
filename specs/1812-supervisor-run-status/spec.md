# Feature Specification: Autopilot supervisor M1: machine-readable run status and non-blocking escalation

**Feature Branch**: `feat/1812_autopilot-supervisor-m1-machine-readable-run`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "autopilot supervisor M1: machine-readable run status and non-blocking escalation (issue #1812)"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One machine-readable state per run (Priority: P1)

A supervisor watching many autopilot sessions asks, for one worktree, "what state is this run in?"
and gets one answer it can parse, without reading a terminal or a ledger by hand. The answer comes
from the run's ledger and from GitHub, and where they disagree GitHub wins, as `resume.sh` already
does.

**Why this priority**: every later supervisor milestone reads this output. Nothing else works
without it.

**Independent Test**: run the status command against a fixture ledger and fixture GitHub answers
for each state; each returns the expected state and fields.

**Acceptance Scenarios**:

1. **Given** a worktree whose ledger has an open PR with checks pending, **When** status is asked,
   **Then** the state is `waiting-ci` and the PR is listed with its GitHub state.
2. **Given** a ledger whose *Open escalation* holds a question, **When** status is asked, **Then**
   the state is `asking` and the question, its options and the recommendation are separate fields.
3. **Given** a ledger that reads `done` and a handoff check that passes, **When** status is asked,
   **Then** the state is `done`.
4. **Given** a worktree on a branch no ledger names, **When** status is asked, **Then** the state
   is `dead-no-ledger`.
5. **Given** a ledger marking a PR merged while GitHub reports it open, **When** status is asked,
   **Then** the PR is reported open (GitHub wins).
6. **Given** a run in one state for longer than the dwell limit, **When** status is asked, **Then**
   the state is `stuck`.
7. **Given** a run blocked by work outside its flow, **When** status is asked, **Then** the state
   is `blocked`.
8. **Given** a run with work in progress and none of the above, **When** status is asked, **Then**
   the state is `running`.

---

### User Story 2 - Supervised escalation ends the turn instead of blocking (Priority: P1)

When a run is started as supervised, an escalation does not open an interactive question that
holds the session until someone answers. It records the full escalation in the ledger, prints the
banner, and ends the turn. The supervisor sees the run as `asking`, gets the answer from the
human, and sends it to the session, which resumes from the ledger and continues.

**Why this priority**: a blocked session cannot be supervised; this is the second half of the
issue's "done when".

**Independent Test**: start a supervised run that escalates; check that the turn ends, the ledger
holds the complete escalation, and that sending `resume` plus the answer moves the question to
*Decisions* and continues.

**Acceptance Scenarios**:

1. **Given** a supervised run, **When** a unit escalates, **Then** *Open escalation* holds the
   question, every option, the recommendation and the evidence, the banner is printed, and the turn
   ends without an interactive question.
2. **Given** a run that is not supervised, **When** a unit escalates, **Then** behaviour is
   unchanged from today.
3. **Given** a supervised run that ended its turn on an escalation, **When** the session receives
   `resume` plus the answer, **Then** the answer is recorded in *Decisions* as decided by the user,
   *Open escalation* is cleared, and the run continues from the ledger's next step.
4. **Given** a supervised run waiting on an answer, **When** status is asked, **Then** the state is
   `asking` with the same question, options and recommendation.

---

### User Story 3 - Every state has a fixture and a test (Priority: P2)

A maintainer can run the tests and see each state, plus the supervised escalation shape, covered by
a fixture ledger.

**Why this priority**: it is the issue's proof, but it only checks stories 1 and 2.

**Independent Test**: the tests under `scripts/tests/` pass, with one passing case per state in
text and JSON form.

**Acceptance Scenarios**:

1. **Given** the fixture set, **When** the tests run, **Then** one case per state passes, in text
   and in JSON form.

### Edge Cases

- No `gh` access or a PR GitHub cannot find: the PR is reported `UNKNOWN`, and the run is not
  declared `done`.
- Several ledgers name one branch, or a worktree path that is not a git worktree: a clear error,
  not a guessed state.
- An escalation that is only partly written (question but no options): reported `asking` with the
  missing fields empty, not dropped.
- A ledger at `done` whose last PR is unmerged: not `done` (handoff check fails).
- Concurrent runs: asking about one worktree never reads or writes another's ledger.
- A supervised marker set on a run whose ledger predates it: the run is treated as supervised.
- An answer sent while no escalation is open: ignored with a note, nothing is recorded.
- Paths and timestamps work on Linux and macOS (Principle VI): no GNU-only date or sed flags.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A status command MUST print one state for one run, chosen from `running`,
  `waiting-ci`, `asking`, `blocked`, `stuck`, `done`, `dead-no-ledger`, for the given worktree
  directory or, with none given, the current one.
- **FR-002**: With `--json` the command MUST print one JSON object with: state, issue, flow, phase,
  next step, the PRs with their GitHub state, and time in the current state. Without it, the same
  data as readable text.
- **FR-003**: When the state is `asking`, the object MUST carry the question, the options and the
  recommendation as separate fields.
- **FR-004**: GitHub MUST win over the ledger for every PR state.
- **FR-005**: `done` MUST be reported only when the existing handoff check passes.
- **FR-006**: `stuck` MUST be reported when the run has stayed in one state longer than the dwell
  limit [NEEDS CLARIFICATION: dwell limit per state; the PR state machine of #1806 defines dwell
  times and may supply it]. `stuck` replaces the underlying state in `state` and the underlying
  state stays available in a separate field.
- **FR-006a**: Time in state MUST be measured from a "since" time taken from the #1806 state
  machine when present, otherwise from the time of the last ledger commit or change; the source
  used is reported.
- **FR-001a**: When several states match, the first in this order wins: `dead-no-ledger`, `done`,
  `blocked`, `asking`, `stuck`, `waiting-ci`, `running`. `blocked` is a ledger *Open escalation*
  that carries the words `blocked by work outside my flow`; any other non-empty *Open escalation* is
  `asking`.
- **FR-001b**: The ledger outcomes of `resume.sh` map as follows: a ledger at `done` whose last PR
  is unmerged or open, or with an uncommitted or unpushed tree, is not `done`: it is
  `waiting-ci` when a PR is open, else `running`; a ledger only on `origin/main` is `dead-no-ledger`
  with a note that `branch-start.sh` is next; several ledgers is an error, not a state. A ledger at
  `done` is found by a lookup that includes `done` ledgers, and the handoff check runs in the target
  worktree, not the caller's directory.
- **FR-007**: The command MUST reuse the existing ledger lookup, resume and handoff-check logic and
  the PR-state helper of #1806, not copy them.
- **FR-008**: The command MUST be read-only: it changes no ledger, branch or PR.
- **FR-009**: A run MUST be markable as supervised, by an environment variable or a ledger field.
- **FR-010**: In a supervised run an escalation MUST write the complete *Open escalation* (question,
  options, recommendation, evidence), print the banner, and end the turn without an interactive
  question.
- **FR-011**: The way an answer comes back MUST be defined and documented in `tasks/ask.md`:
  [NEEDS CLARIFICATION: exact message shape of the answer, for example `resume` followed by the
  chosen option or free text].
- **FR-011a**: A supervised escalation holding several batched questions MUST take one answer per
  question, in one message, matched by question number.
- **FR-011b**: A supervised `resume` that finds a non-empty *Open escalation* and no answer MUST
  print the banner again and end the turn, never block on an interactive question.
- **FR-011c**: A supervised question raised before any ledger exists (the flow question at entry)
  has nowhere to be recorded and is out of scope; it stays an interactive question.
- **FR-012**: On receiving it, the run MUST record the answer in *Decisions* as decided by the
  user, clear *Open escalation*, and continue.
- **FR-013**: Unsupervised escalation behaviour MUST NOT change.
- **FR-014**: Tests under `scripts/tests/` MUST cover every state with a fixture ledger, in text
  and JSON, and the supervised escalation shape.

### Key Entities

- **Run state**: one of the seven states, with issue, flow, phase, next step, PR list, time in
  state, and for `asking` the escalation fields.
- **Supervised marker**: a flag on a run, set at start, telling an escalation to end the turn.
- **Open escalation**: the ledger section holding the unanswered question, options, recommendation
  and evidence.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a fixture of each of the seven states, the JSON output names the right state
  (7 of 7).
- **SC-002**: Reading a run's state takes 0 reads of terminal output or ledger text by the caller.
- **SC-003**: A supervised run that escalates ends its turn with an *Open escalation* holding
  question, options, recommendation and evidence (0 interactive blocks).
- **SC-004**: After the answer is sent, the run records it and continues from the ledger's next
  step without the answer being asked again.
- **SC-005**: Status for one worktree returns in under 5 seconds with GitHub reachable.

## Assumptions

- The ledger format of `templates/autopilot-ledger.md` is the source for phase, next step, PRs and
  escalation.
- #1806 (PR state machine, open as PR #1810) provides the PR-state helper and dwell times; until it
  lands, the command uses the same GitHub query `resume.sh` uses, behind one function.
- `blocked` is read from a ledger marker for work outside the flow (the escalation wording
  `blocked by work outside my flow`).
- Out of scope: the supervisor itself, notifying humans, restarting stuck or dead runs, any
  change to how unsupervised runs are driven.
