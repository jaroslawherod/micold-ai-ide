# Feature Specification: Autopilot supervisor M5: verify handoff and report completed runs

**Feature Branch**: `feat/1816_autopilot-supervisor-m5-verify-handoff-and-report`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "autopilot supervisor M5: verify handoff and report completed runs (issue #1816)"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A finished run is verified without opening its session (Priority: P1)

When a run reports `done`, the supervisor checks the run itself: the handoff check on the run's
ledger in the run's worktree (clean tree, commits on `origin/main`, PRs merged), and that the
run's GitHub issue is closed with its labels. The user sees a verdict per run, not a session to open.

**Why this priority**: it is the issue's "done when": a run is only done when it is true, and a
run with an unmerged PR is never reported as done.

**Independent Test**: run the verifier against fixture runs (all merged, one PR open, dirty tree,
issue open) and read the verdict and the list of what remains.

**Acceptance Scenarios**:

1. **Given** a run whose ledger reads `done`, whose PRs are merged, whose tree is clean and whose
   issue is closed, **When** it is verified, **Then** the verdict is `VERIFIED`.
2. **Given** a run whose ledger reads `done` and whose last PR is open or unmerged, **When** it is
   verified, **Then** the verdict is `NOT DONE`, the PR is listed as what remains, and the run is
   not reported as done anywhere.
3. **Given** a run with a dirty tree, or commits not on `origin/main`, **When** it is verified,
   **Then** the verdict is `NOT DONE` with one line per thing that remains.
4. **Given** a run whose work is all merged but whose issue is still open, or still carries `in-progress` or lacks its `flow:<flow>` label, **When** it is verified, **Then** the verdict is `NOT DONE` naming the issue and the
   missing state.
5. **Given** the verifier is run from another worktree than the run's, **When** it verifies, **Then**
   the checks run in the run's worktree, never the caller's.

---

### User Story 2 - NOT DONE is sent back, then escalated (Priority: P1)

On `NOT DONE`, the supervisor tells the run's session what remains, or resumes it, and checks
again. After a cap of attempts without convergence it escalates to the user as non-convergence
(category 5 of `rules/escalation.md`) instead of looping.

**Why this priority**: without it the user still has to open the session to push a run over the line.

**Independent Test**: feed a fixture run that stays `NOT DONE`; confirm the follow-ups are counted,
each carries the remaining items, and the cap yields one escalation and no further follow-up.

**Acceptance Scenarios**:

1. **Given** a `NOT DONE` run with a live session, **When** the supervisor acts, **Then** it sends
   the session the remaining items, as one message.
2. **Given** a `NOT DONE` run whose session is stopped or dead, **When** the supervisor acts,
   **Then** it asks for the session to be resumed from the ledger instead of sending text.
3. **Given** a run that has had the cap of follow-ups and is still `NOT DONE`, **When** the
   supervisor acts, **Then** it records a non-convergence escalation (what remains, what was sent)
   and sends nothing more.
4. **Given** a run that turns `VERIFIED` after a follow-up, **When** it is checked again, **Then**
   it is reported done and its follow-up count stops.

---

### User Story 3 - One completion report across runs (Priority: P2)

The user reads one report covering every completed run: its issue, its PRs, the decisions made by
the user versus those resolved by the supervisor or agents, the interventions made on it, and its
tokens. Runs that are `NOT DONE` or escalated are listed apart, never in the done list.

**Why this priority**: it replaces opening each session to learn what happened.

**Independent Test**: build a report from two fixture ledgers and a fixture supervisor ledger and
compare it to the expected text.

**Acceptance Scenarios**:

1. **Given** two verified runs, **When** the report is written, **Then** it has one entry per run
   with issue, PRs and merge state, a count of decisions by user and by agent, the interventions,
   and tokens.
2. **Given** a run with an unmerged PR, **When** the report is written, **Then** it appears under
   not done, never under completed.
3. **Given** a run with no recorded interventions or tokens, **When** the report is written,
   **Then** the entry says none or unknown, not zero by guess.

---

### User Story 4 - Worktrees safe to remove are listed, not removed (Priority: P2)

The report lists worktrees that are safe to remove: the run is `VERIFIED`, nothing is uncommitted,
nothing is unpushed and no session is live in it. Removal stays the user's click. Only when the
user has opted in does the supervisor remove them.

**Why this priority**: it finishes the close-out, but removing is destructive, so it is last.

**Independent Test**: run the lister against fixture worktrees in each condition; only the
fully clean verified one is listed; with the opt-in off nothing is removed.

**Acceptance Scenarios**:

1. **Given** a verified run with a clean tree and no live session, **When** the list is made,
   **Then** its worktree is listed as safe to remove.
2. **Given** a worktree with uncommitted work, an unmerged commit, or a live session, **When** the
   list is made, **Then** it is not listed, and the reason is given.
3. **Given** the user has not opted in, **When** the list is made, **Then** no worktree is removed.
4. **Given** the user has opted in, **When** a worktree is safe to remove, **Then** the removal is
   requested through the app's own removal, which still asks confirmation when nobody has
   pre-approved it.

### Edge Cases

- No GitHub access, or a PR or issue GitHub cannot find: reported `UNKNOWN`, the run is not done.
- A ledger at `done` with no PRs (a chore closed without one): verified on the other checks.
- Several ledgers name one branch, or the path is not a git worktree: a clear error per run, the
  other runs are still reported.
- A session that cannot receive input (stopped) while a follow-up is due: resume is asked instead.
- Two supervisors or two reports over the same runs: read-only checks give the same answer; a
  follow-up is not sent twice for the same remaining set within one check.
- A run that goes `NOT DONE` again after being reported done (a revert, a new commit): it moves back
  to not done in the next report.
- Zero runs, or an absent or empty supervisor ledger: an empty report with its sections present and
  exit 0; follow-up count read as 0.
- Paths and dates work on Linux and macOS (Principle VI): no GNU-only flags.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A verifier MUST take a run's worktree and print `VERIFIED` or `NOT DONE` with one line
  per thing that remains, exiting non-zero on `NOT DONE`.
- **FR-002**: The verifier MUST run the existing handoff check on the run's ledger inside the run's
  worktree, and MUST NOT copy its logic.
- **FR-003**: The verifier MUST also check that the run's issue is closed and carries its flow
  labels, read from GitHub, and list any miss as remaining.
- **FR-004**: A run with any PR not `MERGED`, including `UNKNOWN`, MUST NOT be reported as done in
  any output.
- **FR-005**: The verifier MUST be read-only: no ledger, branch, PR or issue changes.
- **FR-006**: On `NOT DONE`, the supervisor-side tooling MUST produce the follow-up: send-text with
  the remaining items for a live session, or a resume request for a stopped one.
- **FR-007**: Follow-ups per run MUST be counted in the supervisor's ledger and capped; at the cap
  the tooling MUST record a category 5 escalation (`rules/escalation.md`) and send no more. The cap
  is [NEEDS CLARIFICATION: how many follow-ups before escalating? Default proposed: 3, matching "CI
  red after 3 fix attempts" in category 5].
- **FR-008**: A report command MUST write one report over the given runs with, per run: issue, PRs
  and their GitHub states, decisions by user and by agent (from the ledger's *Decisions* **By**
  column), interventions (from the supervisor ledger), and tokens.
- **FR-009**: Tokens MUST be reported only from a recorded source; where none exists the entry says
  `unknown`. [NEEDS CLARIFICATION: where do tokens come from? Options: the ledger's last
  `checkpoint.sh` CONTEXT line (one unit's context, not run total), the metrics of #1803 when it
  lands, or omit the column until then. Default proposed: print `unknown` unless the ledger has a
  `Tokens` field.]
- **FR-010**: The report MUST list not-done and escalated runs in a separate section from completed
  ones.
- **FR-011**: A lister MUST name worktrees safe to remove: verified run, clean tree, nothing
  unpushed, no live session; and give the reason for each worktree left out.
- **FR-012**: Removal MUST stay a user action unless the user opted in. [NEEDS CLARIFICATION: how is
  the opt-in given? Options: an environment variable, a field in the supervisor ledger, or an
  argument per report. Default proposed: a supervisor-ledger field `- **Auto-remove**: yes`.]
- **FR-013**: Even with opt-in, removal MUST go through the app's own worktree removal, and MUST
  NOT remove a worktree that is not on the safe list.
- **FR-014**: Tests under `scripts/tests/` MUST cover each verdict (merged, open PR, dirty tree,
  unmerged commits, issue open, label missing, `UNKNOWN`), the cap, the report sections and the
  safe-to-remove list, with fixture ledgers and fixture GitHub answers.
- **FR-015**: The supervisor's rules MUST document this close-out step in a rules file, so the
  supervisor skill (#1813) can load it.

### Key Entities

- **Verdict**: `VERIFIED` or `NOT DONE`, with the remaining items, for one run.
- **Follow-up**: one message or resume request sent for a `NOT DONE` run, counted per run.
- **Completion report**: per-run entries (completed, not done, escalated) and the safe-to-remove list.
- **Supervisor ledger**: the supervisor's file recording its queue, session ids, interventions and
  follow-up counts (defined by #1813 and #1814; read here).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For fixture runs of each verdict kind, the verdict is correct (all kinds).
- **SC-002**: A run with an unmerged PR appears in the completed section of 0 reports.
- **SC-003**: A finished run is verified and reported by one command from the worktree path alone,
  with no session input and no ledger read by the user.
- **SC-004**: A run that never converges produces exactly one escalation after the cap and no
  follow-up after it.
- **SC-005**: With opt-in off, 0 worktrees are removed in any test.
- **SC-006**: Verifying one run takes under 10 seconds with GitHub reachable.

## Assumptions

- #1812's run status (`run-status.sh --json`) is the source of a run's state and PR list.
- The supervisor ledger of #1813/#1814 records interventions and session ids; until it lands the
  tooling reads a fixture of the same shape, and a missing file means no interventions recorded.
- The session actions (`send_session_input`, resume, `delete_worktree`) are made by the supervisor
  with the Micold tools; the scripts here produce the verdict, the message and the lists.
- Out of scope: the supervisor skill and its queue (#1813), the watch loop and recovery (#1814),
  batching questions (#1815), computing the metrics of #1803, any change to how a single run is driven.
