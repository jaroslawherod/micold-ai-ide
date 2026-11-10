# Feature Specification: Staged review for speckit-autopilot

**Feature Branch**: `feat/1804_autopilot-staged-review-facts-by-code-plan-by`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "autopilot: staged review (facts by code, plan by strong model, probes by cheap workers). Adapted from ai-sdlc RFC-0052. autopilot-reviewer spends most of its tokens reading files, not judging; cache reads dominate cost. Stage 1 (code, no model): diff facts, changed symbols, callers, coverage of changed lines, written to a facts file. Stage 2: strong model writes a short review plan from the facts, risk and acceptance criteria. Stage 3: cheaper-model workers run the plan's probes in parallel and return evidence. Stage 4: strong model reads the evidence and writes the verdict in the existing format. Update autopilot-reviewer.md, tasks/review.md, tasks/review-rounds.md. Keep the verdict format and the ledger unchanged. Run it as a shadow beside the current review on a few PRs and compare findings and tokens before replacing it."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Facts file produced by code (Priority: P1)

An autopilot unit about to dispatch a code review runs one script. It writes a facts file for the
diff without calling any model: changed files and line ranges, changed symbols, their callers, and
whether each changed line is covered by tests.

**Why this priority**: every later stage reads this file instead of the source tree; without it the
reviewer still spends its tokens reading files.

**Independent Test**: run the script on a branch with a known diff and compare the facts file to the
expected content. No model is involved.

**Acceptance Scenarios**:

1. **Given** a branch with changes to two Rust files, **When** the script runs, **Then** the facts
   file lists each changed file with its changed line ranges and each changed function, type or
   constant by name.
2. **Given** a changed symbol that other code calls, **When** the script runs, **Then** the facts
   file lists those callers by file and line.
3. **Given** changed lines that no test executes, **When** coverage data is available, **Then** the
   facts file marks those lines uncovered.
4. **Given** coverage data is unavailable or the change has no Rust code, **When** the script runs,
   **Then** the facts file says coverage is unknown for that part and the script still succeeds.

---

### User Story 2 - Planned, parallel-probed review (Priority: P1)

The unit dispatches a strong-model reviewer that writes a short review plan from the facts file,
the risk and the acceptance criteria. The unit runs the plan's probes as parallel cheaper-model
`autopilot-worker` tasks, each returning evidence. A strong-model reviewer reads the evidence and
writes the verdict.

**Why this priority**: this is the cost reduction the issue asks for.

**Independent Test**: run the staged review on a fixture diff containing one known defect (an unchecked
error path in a changed function) and check the verdict reports it with evidence from a probe; the
plan and evidence stages are also checked against their size limits.

**Acceptance Scenarios**:

1. **Given** a facts file, **When** the plan stage runs, **Then** it returns a plan of at most 6
   probes, each with a question, the files or symbols to inspect and the evidence to
   return.
2. **Given** a plan with several probes, **When** stage 3 runs, **Then** each probe runs in its own
   worker on a cheaper model, in parallel, and returns evidence of at most 40 lines.
3. **Given** the evidence, **When** the verdict stage runs, **Then** the verdict is in the existing
   `VERDICT:` / `F<n>` format with at most 8 findings, and the ledger format is unchanged.

---

### User Story 3 - Shadow run and switch decision (Priority: P2)

The staged review runs beside the current single-agent review on a few PRs. Findings and tokens of
both are recorded and compared. The decision to switch or not is recorded.

**Why this priority**: the issue is done only when the staged review is shown equal or better.

**Independent Test**: after at least three shadow runs, a recorded comparison table and decision
exist in the feature directory.

**Acceptance Scenarios**:

1. **Given** a review dispatched in shadow mode, **When** both reviews finish, **Then** only the
   current review's verdict drives the unit's actions and both results are recorded.
2. **Given** several recorded shadow runs, **When** staged findings are equal or better and
   cache-read tokens are lower, **Then** the decision to switch is recorded with the numbers; if not,
   the decision to keep the current review is recorded with the reason.

### Edge Cases

- Empty diff or a diff with only docs: the facts file is produced, the plan stage may return no
  probes, and the verdict is `CLEAN` without workers.
- A diff with no Rust (scripts, markdown, YAML): symbol and caller facts are skipped and marked
  as such; the rest is produced.
- Coverage tooling absent or slow: coverage is marked unknown; it never blocks the review.
- A worker fails or returns no evidence: the verdict stage sees that probe as unanswered and either
  reports it or falls back to the single-agent review; it never reports clean on missing evidence.
- Two units in different worktrees review at once (Principle II): the facts file is per worktree
  and per snapshot, so reviews do not overwrite each other.
- Round 2+ re-reviews use the staged flow on the fix diff only (facts for the fix diff, same limits);
  spec, plan and tasks artifact reviews keep the single-agent path (the facts stage covers code diffs).
- HEAD moves or the snapshot goes stale mid-review: the staged result is discarded and the review
  re-runs on a fresh snapshot, as `review-rounds.md` already requires for a stale snapshot.
- Cross-platform: the script runs on Linux and macOS with only tools the repo already requires,
  tolerating BSD versus GNU utility differences; coverage tooling missing on either is "unknown".
- Subagents cannot spawn subagents (the reviewer type has no `Agent` tool): stages 2 to 4 are
  dispatched by the unit, one fresh subagent per stage, not nested.
- Other open work (head-bound verdict issue) edits the same three files: changes here must rebase
  onto it without altering the verdict format.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A script MUST produce a facts file for the current diff against `origin/main` with
  changed files and line ranges, changed symbols, callers of changed symbols, and coverage of
  changed lines, using no model.
- **FR-002**: The facts file MUST be written outside the worktree's tracked files (scratchpad) and
  keyed by the review snapshot, so concurrent reviews do not collide.
- **FR-003**: The facts file MUST state coverage as unknown, not as covered, when coverage data is
  missing.
- **FR-004**: The review plan stage MUST take only the facts file, the risk and the acceptance
  criteria, and return at most 6 probes, each naming its question, targets and evidence to
  return.
- **FR-005**: Probes MUST run as `autopilot-worker` subagents on a cheaper model, in parallel, each
  returning at most 40 lines of evidence.
- **FR-006**: The verdict stage MUST read the facts file, the plan and the evidence, and write the
  existing verdict format unchanged; the ledger format MUST be unchanged.
- **FR-007**: `.claude/agents/autopilot-reviewer.md`, `tasks/review.md` and
  `tasks/review-rounds.md` MUST describe the staged flow, the shadow mode and the fallback to the
  single-agent review.
- **FR-008**: A shadow mode MUST run the staged review beside the current review, drive the unit
  only from the current review's verdict, and record both results.
- **FR-009**: The shadow comparison MUST record, per PR, findings of each review and cache-read
  tokens of each, and the switch decision MUST be recorded with the numbers.
- **FR-010**: Round-limit and snapshot rules in `review-rounds.md` MUST apply unchanged to the
  staged review.
- **FR-011**: When the facts script fails (non-zero exit, no `origin/main`, detached HEAD), or any
  probe is unanswered or errors, the unit MUST run the single-agent review instead and MUST NOT
  report the staged result as `CLEAN`.
- **FR-012**: A shadow comparison counts a staged review as equal or better when it reports every
  BLOCKER and MAJOR (same file and same defect) the current review reports and its extra BLOCKER or
  MAJOR findings each hold up on verification by the unit; the unit adjudicates, and a miss judged
  not a real defect is recorded with the reason. At least three code-diff PRs, the next three
  milestone or chore reviews after the change merges, are compared.
- **FR-013**: The switch decision MUST state what changes: when switching, the three files make the
  staged flow the default and drop shadow mode; when not, shadow mode is removed and the finding
  recorded.

### Key Entities

- **Facts file**: per-snapshot record of diff facts for one review.
- **Review plan**: bounded list of probes derived from the facts, risk and acceptance criteria.
- **Evidence**: a probe's bounded answer.
- **Shadow record**: per-PR comparison of current and staged review findings and tokens.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Across at least three shadow PRs, the staged review reports every BLOCKER and MAJOR
  the current review reports, or the miss is explained and judged not a real defect.
- **SC-002**: Across those PRs, total cache-read tokens of the staged review are lower than the
  current review's, with the numbers recorded.
- **SC-003**: The facts file for a typical diff is produced in under one minute without any model
  call on a diff of up to 50 files.
- **SC-004**: The existing verdict reading (`VERDICT:` and `F<n>` lines) and the ledger reader accept
  staged output with no change to either; checked by running them on a staged verdict.
- **SC-005**: The decision to switch or keep the current review is recorded in the feature
  directory.

## Assumptions

- Coverage of changed lines comes from an existing coverage tool if the repo has one; otherwise it
  is reported unknown and may be refined later.
- "Strong model" is the session model and "cheaper model" is Sonnet or Haiku, per CLAUDE.md.
- Token counts for the comparison come from the subagent usage the harness reports.
- Out of scope: changing the rubrics, the verdict format, the ledger, the head-bound verdict work,
  and any non-autopilot review.
