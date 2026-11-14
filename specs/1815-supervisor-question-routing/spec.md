# Feature Specification: Autopilot supervisor M4: route session questions to the user in one batch

**Feature Branch**: `claude/project-thread-0lx36q`

**Created**: 2026-11-14

**Status**: Draft

**Input**: User description: "autopilot supervisor M4: route session questions to the user in one batch (issue #1815)"

## Background

Builds on #1812 (`run-status.sh --json`: state `asking`, JSON `escalation` with `category`, `banner`,
`questions[]`; `answer.sh` records an answer) and #1813 (`autopilot-supervisor` skill: queue, start,
Restart, Report, Watcher). Today the supervisor's *Report* only tells the person about an `asking`
run and never answers it ("Never answer a question ... yourself"). This feature replaces that for
`asking` runs: the supervisor resolves what the repo answers, batches the rest into one question to
the user, and sends each answer back to the session that asked.

Dependency decision (orchestrator, recorded in the ledger): the parent/sibling visibility rule and
the `parent` field of the session list (#1858, #1857 FR-015 to FR-017) live only in open PR #1873,
not on main. Everything below is specified against what main offers today, except the upward relay
(US4), which is conditional on the daemon reporting the supervisor's parent. Nothing here depends
on code from #1873. The addressing rule of #1857 FR-015 to FR-017 (a session sees and addresses
only its parent, siblings and direct children; anything else is `not_found`) is design intent, not
main behaviour.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Two sessions ask at once, the user answers once (Priority: P1)

Two supervised sessions reach `asking` at about the same time. The supervisor collects both open
escalations and puts them in one `AskUserQuestion` (up to 4 questions), each naming its issue and
answerable cold, with a recommended option first and the banner format of `tasks/ask.md`, plus a
`PushNotification`. After the answer, each answer goes to its own session and both resume.

**Why this priority**: this is the feature's "done when" and its reason to exist: one interruption
instead of one per session.

**Independent Test**: fixture `run-status.sh --json` answers for two `asking` runs and a fake
`send_session_input`; assert one batch with two questions, and one send per session carrying that
session's answer only.

**Acceptance Scenarios**:

1. **Given** two `running` rows whose run-status state is `asking` with category-1 escalations,
   **When** the supervisor routes questions, **Then** it makes one `AskUserQuestion` with two
   questions, each prefixed `#<issue>`, option 1 labelled `(Recommended)`.
2. **Given** the user's answers to both, **When** the supervisor relays them, **Then** each session
   receives `resume <answer>` (By `user`) (a batch escalation: one `<n>. <answer>` line per question) through
   `send_session_input`, never the other session's answer.
3. **Given** the answers were sent, **When** the supervisor re-reads each session's status, **Then**
   neither is `asking` any more (its *Open escalation* is `None.` and *Decisions* has the row).
4. **Given** five `asking` runs needing the user, **When** the supervisor routes, **Then** the first
   four go out in one call in ascending issue order, and the fifth goes in the next batch after the
   first answer.

---

### User Story 2 - A routine question never reaches the user (Priority: P1)

An `asking` run whose escalation is something `rules/escalation.md` says is never the human's (fmt,
clippy or test failure in its own code, a review finding, a merge conflict or flaky rerun, an
equivalent implementation, naming, file layout, a question the repo answers) is answered by the
supervisor from repo evidence and the answer is sent to the session. The row is recorded as
resolved by the supervisor.

**Why this priority**: the second "done when"; without it the batch fills with noise.

**Independent Test**: fixture escalations of a "never ask" kind and of a real category; assert the
first is answered with a cited evidence path and not batched, the second is batched.

**Acceptance Scenarios**:

1. **Given** an escalation asking whether to fix a clippy warning in the flow's own code, **When**
   the supervisor routes, **Then** it sends `resume <answer>` with the evidence it found and adds
   nothing to the user batch.
2. **Given** a "never ask" escalation for which the repo holds no evidence, **When** the supervisor
   routes, **Then** it does not guess: the question goes to the user batch.
3. **Given** an escalation labelled category 1 (or 5 or 6) whose subject is a *Never ask* item
   (for example a clippy failure written up as non-convergence) and the repo answers it, **When**
   the supervisor routes, **Then** it classifies by subject, overrides the label, and answers it.
4. **Given** an escalation of category 2, 3 or 4, or a category 1, 5 or 6 one whose subject is not
   a *Never ask* item, **When** the supervisor routes, **Then** it never answers it itself,
   whatever the evidence.
5. **Given** a resolved question, **When** it is answered, **Then** the supervisor ledger's row for
   the issue records the question, the answer, `resolved by supervisor` and the evidence path, and
   the worker's own ledger *Decisions* row has By `supervisor` with that evidence (not `user`).

---

### User Story 3 - Over the context cap, ask again after `/clear` (Priority: P2)

When the supervisor's own context is over the cap, calling `AskUserQuestion` would block the
session and re-write a large context hours later. The supervisor writes the pending batch to its
ledger, sends the banners and the notification, and ends the turn. After `/clear` it re-asks from
the ledger.

**Why this priority**: keeps a long-lived supervisor cheap; correct without it, but costly.

**Independent Test**: run the routing step with the context check reporting `OVER`; assert no
`AskUserQuestion` call, the batch in the ledger, and a re-ask from the ledger on the next run.

**Acceptance Scenarios**:

1. **Given** `AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py` prints `OVER`, **When** the
   supervisor routes, **Then** it makes no `AskUserQuestion` call, records the batch in the ledger,
   sends the banners with the line `Cheaper: /clear, then /autopilot-supervisor — it asks this again
   from the ledger.` and the notification, and ends the turn.
2. **Given** a ledger holding a pending batch, **When** the supervisor starts (Restart), **Then** it
   asks that batch again before any new start, without re-reading the sessions' transcripts.
3. **Given** a pending batch whose session has since left `asking` (the user answered it in the
   session directly), **When** the supervisor re-asks, **Then** that question is dropped.

---

### User Story 4 - A supervisor below the top tier relays the question upward (Priority: P3)

A supervisor that is itself a session of a parent (not in `default`) and cannot answer a question
from the repo passes it to its own parent and relays the answer back down. A worker's escalation
reaches the user only through its supervisor (FR-006a), never directly or to a session outside its scope. The rule is conditional: it applies only when the
daemon reports the supervisor's parent. With no parent reported (main today), the supervisor
treats itself as top tier and asks the user.

**Why this priority**: part of the scope-and-relay design (#1857, #1858), but its data lives in
open PR #1873, so it cannot be exercised on main.

**Independent Test**: a fixture session list with a `parent` field and one without; assert the
question goes to the parent, or to the user, respectively.

**Acceptance Scenarios**:

1. **Given** the session list reports a parent for the supervisor and an unanswerable question,
   **When** the supervisor routes, **Then** it sends the question to that parent with
   `send_session_input` and holds the asking session until the parent's answer arrives, then relays
   it down.
2. **Given** the session list reports no parent (or lacks the field), **When** the supervisor
   routes, **Then** it asks the user.
3. **Given** the daemon refuses an address as `not_found`, **When** the supervisor sends, **Then**
   it treats the target as out of scope, records the refusal, and asks the user instead of retrying.
4. **Given** a supervisor in `default` (top tier), **When** it cannot answer, **Then** it asks the
   user directly.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The supervisor MUST collect the *Open escalation* of every `running` row whose
  `run-status.sh --json` state is `asking`, across sessions, from the JSON `escalation` object
  (`category`, `banner`, `questions`), never from a transcript.
- **FR-002**: Before asking anyone, the supervisor MUST try to resolve each escalation whose subject
  is in `rules/escalation.md` *Never ask about these*, from repo evidence, and MUST name the
  evidence path or command in the answer.
- **FR-003**: The supervisor MUST classify an escalation by its subject, not its category label.
  A subject matching a *Never ask* item overrides a label of category 1, 5 or 6 (the categories a
  run may mis-apply). It MUST NOT answer any escalation of category 2, 3 or 4, nor one of category
  1, 5 or 6 whose subject matches no *Never ask* item.
- **FR-004**: An escalation it cannot resolve with evidence MUST go to the user batch, not be
  guessed.
- **FR-005**: The supervisor MUST send each resolved answer to the asking session with
  `send_session_input` as `resume --by supervisor <answer>` (a batch escalation: one `<n>. <answer>` line per
  question), and record it as `resolved by supervisor` with question, answer and evidence.
- **FR-005a**: `answer.sh` MUST accept `--by <who>` (default `user`), write that name in the
  *Decisions* By cell, and `tasks/resume.md` MUST pass it through from `resume --by supervisor`.
  An answer relayed from the user is sent as `resume <answer>`, By `user`.
- **FR-006**: The supervisor MUST put every escalation left over in one `AskUserQuestion` call of up
  to 4 questions in ascending issue order, the rest in the next call after the answer. A single
  question goes alone, at once. Each question MUST name its issue, be answerable cold, and list the
  recommended option first labelled `(Recommended)` with its evidence.
- **FR-006a**: `tasks/ask.md` step 0 MUST change: a supervised run writes its banner to its own
  transcript but sends no `PushNotification`; the supervisor sends the only notification. Outside
  supervision, ask.md is unchanged.
- **FR-007**: Before the call the supervisor MUST send the banner of `tasks/ask.md` (unit field is
  `supervisor`) for each question, and one `PushNotification` `🛑 <issues> need a decision: <topic>`.
- **FR-008**: After the answer, the supervisor MUST send each answer only to the session that asked
  it, and MUST then re-read that session's status and confirm it left `asking`. The check runs
  on the next Watcher wake (`CHANGED`) after the worker's next turn, not inline after the send. A
  session still `asking` one Watcher ceiling after the send is reported to the user as not
  accepting input, never re-sent in a loop.
- **FR-008a**: Before sending, the supervisor MUST check that a batch answer has exactly one line
  per question number of the escalation's `questions[]`, and re-ask the user for any missing one;
  it does not rely on `answer.sh` exit 2, whose message prints in the worker.
- **FR-009**: A question that was answered while the supervisor waited (its session left `asking`)
  MUST be dropped from the batch and its answer not sent.
- **FR-010**: When `AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py` prints `OVER`, the
  supervisor MUST write the batch to its ledger, make no `AskUserQuestion` call, send the banners
  with the `Cheaper: /clear ...` line and the notification, end the turn, and re-ask from the ledger
  on its next start.
- **FR-011**: The ledger MUST record, per issue, the routing state (`pending`, `asked`, `answered`,
  `resolved`) so a restarted supervisor neither asks twice nor loses a question.
- **FR-012**: Upward relay: when the daemon reports the supervisor's parent and the supervisor is
  not in `default`, an escalation it cannot resolve MUST go to that parent, and the answer MUST be
  relayed back to the asking session. With no parent reported, the supervisor MUST act as top tier
  and ask the user. The supervisor MUST NOT read `parent` from anything but the daemon's session
  list.
- **FR-013**: Where the daemon enforces parent/sibling/child addressing, a send refused as
  `not_found` MUST be recorded and the question asked of the user, not retried.
- **FR-014**: Routing MUST work from `run-status.sh --json` and `supervisor.sh ledger table`
  alone: no session transcript is read to collect, ask or confirm a question.
- **FR-015**: After this feature, the supervisor routes an `asking` run (resolve, batch or relay)
  instead of only reporting it; `stuck`, `blocked` and `lost` runs are still only reported.

### Key Entities

- **Open escalation**: a run's pending question(s), as `run-status.sh --json` reports them.
- **Routing batch**: the user-facing questions collected in one pass, with issue, session id and
  question number per entry.
- **Routing state**: per-issue `pending | asked | answered | resolved` in the supervisor ledger.

## Success Criteria *(mandatory)*

- **SC-001**: Two sessions escalating at once produce exactly one user prompt and exactly one user
  notification, and both resume after the answer.
- **SC-002**: A routine question (clippy, fmt, review finding, conflict) produces zero user
  prompts and one send to its session.
- **SC-003**: Over the cap, zero `AskUserQuestion` calls and the batch is present in the ledger.
- **SC-004**: No answer is ever sent to a session other than the one that asked.

## Edge Cases

- No `asking` run: nothing to route; the supervisor says so.
- An `asking` run with a free-text escalation (no `### Q<n>`): one question, first line.
- A batch escalation (several `### Q<n>` in one run): all its questions go to the user, or all are
  resolved; an answer line per question number, as `answer.sh` requires. If only some resolve, the
  resolved ones are held until the rest are answered, and sent together.
- The user's answer misses a question number: the supervisor's pre-send check (FR-008a) catches it
  and re-asks only the missing question; nothing is sent until the answer is complete.
- Two supervisors: the ledger owner check of #1813 applies; only the owner routes.
- A session ended (`lost`) while its question was pending: dropped and reported.
- `send_session_input` fails: recorded in Note; the question stays `asked`, reported, not retried
  in a loop.
- Concurrency (Principle II): a new `asking` run appearing while the user is answering goes to the
  next batch.
- Cross-platform (Principle VI): decisions live in shell and skill text, no OS-specific calls.

## Out of Scope

- Answering any escalation whose subject is not in `rules/escalation.md` *Never ask about these*, and any category 2, 3 or 4 escalation, whatever the evidence.
- The daemon's `parent` field and visibility rule (#1873, #1858); only the conditional use of
  them.
- Starting sessions, the queue and the gate (#1813); status derivation (#1812).
- Answering `stuck`, `blocked` or `lost` runs.
- Changing the escalation format or categories of `rules/escalation.md`.
