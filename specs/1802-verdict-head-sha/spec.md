# Feature Specification: Review verdicts bound to the PR head SHA

**Feature Branch**: `feat/1802_autopilot-bind-review-verdicts-to-the-pr-head-sha`

**Created**: 2026-11-10

**Status**: Closed 2026-11-11 — shipped in PRs #1818, #1821, #1825, #1831

**Input**: Issue #1802, adapted from ai-sdlc RFC-0061 (review verdict bound to the head commit).
"Approved" in the autopilot ledger is not tied to the commit it reviewed; after a rebase or
force-push the merge step can act on a verdict for a different head.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Refuse a merge on a stale verdict (Priority: P1)

The orchestrator is about to merge a PR. The ledger holds an approved review verdict, but the PR's
head has moved since (force-push, rebase, extra commit) and the new head changed the code. The
merge must stop with a message that names the reviewed SHA, the current SHA and what to do.

**Why this priority**: it is the defect in the issue; without it a reviewed-clean claim can ship
unreviewed code.

**Independent Test**: record an approved verdict for SHA A, move the PR head to SHA B with a
different patch, run the merge guard: it exits non-zero and prints both SHAs and "re-review".

**Acceptance Scenarios**:

1. **Given** an approved verdict recorded for head A and a PR whose head is B with a different
   diff, **When** the merge guard runs, **Then** it refuses, exit non-zero, naming A, B and
   "re-review required".
2. **Given** an approved verdict recorded for head A and a PR whose head is A, **When** the merge
   guard runs, **Then** it allows the merge.
3. **Given** a ledger with no verdict for a required review of the PR, **When** the merge guard
   runs, **Then** it refuses with "no approved verdict for <head>".
4. **Given** the latest verdict for the head is `CHANGES`, **When** the guard runs, **Then** it
   refuses.

---

### User Story 2 - Carry a verdict across a clean replay (Priority: P2)

After a rebase onto a newer `main` (or a force-push with an amended message), the code under review
is the same. The guard carries the verdict forward to the new head instead of forcing a re-review,
and records that it did.

**Why this priority**: without it every routine rebase costs a full review round.

**Independent Test**: approve at head A, rebase so the head becomes B with an identical patch,
run the guard: it allows the merge and a PR comment records "verdict carried A -> B".

**Acceptance Scenarios**:

1. **Given** an approved verdict at head A and a new head B whose patch-id set equals A's over the
   same changed files (clean replay), **When** the guard runs, **Then** it posts a PR comment
   `verdict carried A -> B` and allows the merge.
2. **Given** a new head B whose patch differs in any hunk (including a conflict resolution), **When**
   the guard runs, **Then** no carry-forward happens and the merge is refused.
3. **Given** a `CHANGES` verdict at A and a clean replay B, **When** the guard runs, **Then** it
   refuses: only approved verdicts carry.
4. **Given** a head that was carried once and then rebased again with identical patch, **When** the
   guard runs, **Then** it carries again from the original reviewed SHA (chain, not hop).

---

### User Story 3 - Record the head with every verdict (Priority: P1, built first: US1 and US2 depend on it)

Each time a review round ends, the unit records the verdict in the ledger together with the PR head
SHA (or, before a PR exists, the commit) the reviewer covered.

**Why this priority**: the guard has nothing to compare without it.

**Independent Test**: run the record helper for a round; the ledger row holds review, round,
snapshot, verdict and head SHA.

**Acceptance Scenarios**:

1. **Given** a finished round, **When** the unit records it, **Then** the *Review rounds* row has
   `Scope` and `Head` with the full 40-character SHA.
2. **Given** a malformed SHA or an unknown verdict word, **When** recording, **Then** the helper
   refuses.

### Edge Cases

- A ledger written before this feature (no `Head` column) in a flow still in flight: refused once,
  with "re-review"; the record helper upgrades the table header in place on first use, and the run
  records fresh rows. No silent pass.
- Force-push with only a different commit message: patch-ids equal, carried.
- Base moved but the PR's own commits replay unchanged: ranges start at each side's merge-base, so
  it carries; a conflict resolution changes a patch-id and does not.
- `CHANGES` at head B then a clean replay C of B: not approved, not carried; the latest round of
  each required review decides.
- Concurrency (Principle II): the guard reads one PR and one ledger and writes only a PR comment on
  a carry, never a file; two worktrees never share a PR or a ledger.
- Cross-platform (Principle VI): bash plus `git` and `gh`, like the other `scripts/autopilot/`
  helpers; no GNU-only flags.
- `gh` failing or the PR head unreadable: refuse with the error.
- Several reviews on one PR (spec review, review B per round): all required reviews must pass.

## Requirements *(mandatory)*

### Binding model

A verdict covers the **reviewed head** `H`: the commit `git rev-parse HEAD` names when the unit
records the verdict, after the review's fixes are committed and before the commit that holds the
ledger row. The ledger itself (the four `ledger-of.sh` patterns, `LEDGER_PATHSPECS` in the plan) is excluded from every comparison, so the
ledger commit that records a verdict, and any later ledger-only commit, never makes a verdict stale.
`Snapshot` (`<tree>:<head>`) stays what it is, the dispatch-time input of `review-snapshot.sh diff`;
`Head` is a separate, later fact (the head at record time) and the only one the guard reads.

Verdict vocabulary (the `Verdict` column starts with one of): `CLEAN`, `MINOR-ONLY`, `CHANGES`,
`pending`. Free text may follow. Approved means `CLEAN` or `MINOR-ONLY`.

Required reviews for a PR: the ledger's *Review rounds* rows whose `Scope` (new column: a PR number
or a milestone ID, `design` for the spec/plan/tasks reviews) matches the PR; for each review name
the latest round counts. The PR number is recorded when the PR opens; rows recorded earlier carry
the milestone ID and the guard maps it through the *Milestones* table's PR column. `design` maps to the *Pull requests* row whose Purpose is Design and is required only for that PR; later PRs need only their own milestone's rows.

### Functional Requirements

- **FR-001**: The ledger's *Review rounds* table MUST have `Scope` and `Head` columns after
  `Round`; each recorded verdict MUST carry its scope and the full 40-character reviewed head `H`.
- **FR-002**: A helper in `scripts/autopilot/` MUST record a verdict row (review, round, scope,
  snapshot, head, verdict) and MUST refuse a non-SHA head or a verdict outside the vocabulary.
- **FR-003**: A guard helper `scripts/autopilot/verdict-guard.sh <pr>` MUST decide merge-or-refuse:
  it reads the PR's current head SHA fresh from GitHub, finds the ledger of the PR's head branch
  (`ledger_of` on that branch's checkout, else the pushed branch's copy), and allows only when every
  required review's latest row is approved for that head or validly carried (FR-005).
- **FR-004**: Every merge goes through `wait-merge.sh`, which calls the guard before merging:
  `wait-merge.sh <pr>` rebases as today, and a new `--squash` mode (replacing the hand-run API call
  in `tasks/ci.md` *Merge problems*) guards, then squash-merges the same head SHA; both modes pass `--match-head-commit <P>`. `gate-hook.sh`
  denies every direct `gh pr merge` (new: today it denies only the admin and branch-delete flags and merge API calls) and keeps denying merge API calls (it cannot see inside the scripts),
  so no unguarded path remains (`--no-merge` merges nothing and skips the guard); the hook test is part of FR-010.
- **FR-005**: Carry-forward rule. When the current head `P` differs from `H`, an **approved** verdict
  carries to `P` only when, with ledger files excluded: neither range contains a merge commit or is
  empty; the ordered list of `git patch-id --stable` values of the non-empty commits in
  `merge-base(P,base)..P` equals that of `merge-base(H,base)..H`; and the touched-file sets (`git diff --name-status -M`, rename = old and new path) are equal. Anything else MUST NOT carry. `CHANGES` and `pending` never carry. Whitespace
  or line-number-only drift is invisible to patch-id and carries; this is the accepted limit.
- **FR-006**: A difference limited to ledger files (`H` and `P` have equal ledger-excluded
  patches) is the same head: allowed, nothing recorded. A real carry MUST NOT touch the branch (a
  push would move the head and restart CI): the guard records it as a PR comment `verdict carried
  <original H> -> P` and prints it. Later carries compare against the original `H` found in the
  ledger, so chains do not drift (the ledger row is never rewritten).
- **FR-007**: The refusal MUST be a distinct last line and exit code (7, see contracts/cli.md) naming the reviewed SHA, the
  current SHA and the action (`re-review`), covering: stale, no row, `CHANGES`/`pending`, unreadable
  PR head, and a ledger of the flow with no `Head` column (legacy; in-flight runs re-review once).
  `tasks/merge.md` MUST map it in its table.
- **FR-008**: A PR whose head branch no flow ledger names (a PR outside autopilot) is not guarded:
  the guard prints `UNGUARDED <pr> no ledger` and allows; `gh` failing is a refusal, never a pass.
- **FR-009**: The rule is written once, in `tasks/merge.md`; `tasks/review.md` and
  `tasks/review-rounds.md` point to it and say to record verdicts with the helper after the
  review's fixes are committed; `templates/autopilot-ledger.md` shows the new columns.
- **FR-010**: Script tests in `scripts/tests/verdict-guard.test.sh` (new file; `gh` stubbed on the
  pattern of `autopilot.test.sh`, real throw-away git repos) MUST cover: stale refused, current
  allowed, ledger-only commit allowed, clean replay carried, changed patch refused, merge commit and
  empty range refused, legacy ledger refused, `CHANGES` not carried, chained carry, no-ledger PR
  unguarded, `gh` failure refused, `design` scope mapped to the design PR, bugs-ledger exclusion, ledger unreadable (fetch fails) refused, carry comment posted once per (H,P), header upgrade of a legacy table, `--squash` guarded, and `gate-hook.sh` still denying direct merge commands.

### Key Entities

- **Verdict row**: review, round, scope, snapshot, head `H`, verdict, optional carried-from.
- **Reviewed head**: `H` above; not the snapshot head.
- **Patch identity**: ordered `git patch-id --stable` list plus touched-file set, ledger excluded.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the script test, 100% of merges with a stale, missing or `CHANGES` verdict are
  refused with the message naming both SHAs (0 merges issued).
- **SC-002**: A rebase with an identical patch merges with no new review round (0 reviewer
  dispatches) in the test.
- **SC-003**: Any change that alters a commit's patch-id after approval forces a re-review in 100% of tested cases (whitespace-only drift is the documented exception).
- **SC-004**: `mise run gate` passes with the new test included.

## Assumptions

- Merge modes are `wait-merge.sh <pr>` (rebase) and `wait-merge.sh <pr> --squash`; both guard.
- A verdict covers the head at record time (Binding model); the snapshot head is only the
  dispatch-time input of `review-snapshot.sh diff`. No change to the reviewer protocol.
- Unpushed review rounds (spec/plan before the design PR) record the local HEAD; the guard checks
  them against the PR head at merge.

## Out of Scope

- Changing what reviewers check or the round limit.
- Enforcing verdicts in CI or on GitHub (branch protection); this is autopilot-local.
- Verdicts for PRs not opened by autopilot.
