---

description: "Task list for autopilot supervisor M1: machine-readable run status and non-blocking escalation"
---

# Tasks: Autopilot supervisor M1: machine-readable run status and non-blocking escalation

**Input**: `specs/1812-supervisor-run-status/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)
**Prerequisites**: plan.md, spec.md
**Tests**: mandatory and first (Constitution I): every script change is preceded by a failing case. All code is bash 3.2-compatible (no `mapfile`, no `${var,,}`, no `"${arr[@]}"` of a possibly empty array under `set -u`).

## Format: `- [ ] [TaskID] [P?] [Story?] Description with file path`

## Phase 1: Setup

- [x] T001 Extract the `gh` stub, `new_repo`, `ledger` and `check` helpers from `scripts/tests/autopilot.test.sh` into `scripts/tests/lib/autopilot-fixtures.sh` and source it from there; no case changes, the existing suite stays green (`scripts/tests/autopilot.test.sh`)

## Phase 2: Foundational (blocks US1 and US2)

- [x] T002 Add failing cases for the new `ledger-of.sh` helpers (`ledgers_of`, `ledger_prs`, `phase_of`, `ledger_on_main <branch>`, `final_pr_open <branch>`, `pr_state_of <n>` with `UNKNOWN` on `gh` failure) in `scripts/tests/autopilot.test.sh`
- [x] T003 Extend `scripts/autopilot/ledger-of.sh` with those helpers (`ledger_of` keeps its absolute prefix; `ledgers_of` prints paths relative to the toplevel); T002 goes green
- [x] T004 Refactor `scripts/autopilot/resume.sh` to source the helpers from `scripts/autopilot/ledger-of.sh`; its report output stays byte-identical (existing resume cases in `scripts/tests/autopilot.test.sh` unchanged and green)

**Checkpoint**: shared helpers exist, resume behaviour unchanged.

## Phase 3: User Story 1 - One machine-readable state per run (Priority: P1) 🎯 MVP

**Note**: `pr-state.sh` of #1806 (PR #1810, open) is not on `main`: the tests stub it through `AUTOPILOT_PR_STATE`; with the helper absent or failing the run is never `stuck` (research D3). T019 (ledger template, Phase 5) ships in M1 so the escalation format that T013 parses is defined where it is first read.

**Goal**: `run-status.sh [--json] [<worktree>]` prints one of seven states with issue, flow, phase, next step, PRs, dwell and escalation fields.
**Independent Test**: `scripts/tests/run-status.test.sh` passes, one fixture per state, text and JSON.

### Tests for US1 (write first, watch them fail)

- [x] T005 [US1] Create `scripts/tests/run-status.test.sh` (sources `scripts/tests/lib/autopilot-fixtures.sh`; stub `pr-state.sh` through `AUTOPILOT_PR_STATE`; `AUTOPILOT_NOW`) with usage cases: exit 2 for a non-worktree path, exit 3 listing each ledger when two open ledgers name the branch, one `done` plus one open ledger resolves to the open one
- [x] T006 [US1] In `scripts/tests/run-status.test.sh` add cases for `dead-no-ledger` (no ledger; ledger only on `origin/main`), `done` (handoff-check passes; refused when it fails, with the handoff lines in `notes`; `resume.sh` exit-4 mapping per data-model *State transitions*) and `running` (FR-001, FR-001b, FR-005)
- [x] T007 [US1] In `scripts/tests/run-status.test.sh` add cases for `waiting-ci` (open PR listed with its GitHub state), GitHub winning over the ledger (ledger says merged, `gh` stub says OPEN: PR reported OPEN, FR-004), `asking` and `blocked` (`blocked by work outside my flow`) with `escalation.questions[]` fields `number, question, options[{label,description}], recommendation, evidence` separate and partial escalations giving `""`/`[]`, never dropped (FR-002, FR-003)
- [x] T008 [US1] In `scripts/tests/run-status.test.sh` add cases for `stuck` (stub `pr-state.sh` prints a `STUCK` line, `underlying_state` `waiting-ci`, `since = now - DWELL`), the ledger-commit/mtime fallback when no `pr-state.sh` exists (`dwell_source` `ledger`), a run with no open PR never `stuck`, and precedence when several states match (FR-006, FR-006a, FR-001a)
- [x] T009 [US1] In `scripts/tests/run-status.test.sh` add cases for output shape and safety: `--json` is one line parsed with `jq -e` and every assertion reads only that JSON (SC-002); text form (`STATE <state>` first line, `key: value`, `PR #<n> <state>`, `question N:`, `option N.k:`, `recommended N:`, `evidence N:`); `gh` unavailable gives PR `UNKNOWN` and never `done`; ledger bytes, `git rev-parse HEAD` and `git status --porcelain` unchanged after a call (FR-008); concurrent runs (two ledgers, ask about one)

### Implementation for US1

- [x] T010 [US1] Create `scripts/autopilot/run-status.sh`: argument parsing (`--json`, `<worktree-dir>`), exit codes 0/2/3, ledger resolution through `ledger-of.sh`, `dead-no-ledger`; T005 and the `dead-no-ledger` case of T006 go green
- [x] T011 [US1] In `scripts/autopilot/run-status.sh` add the resolver for `done` (calls `handoff-check.sh` inside the target worktree), `blocked`, `asking`, `waiting-ci`, `running`, with PR state only from `pr_state_of` (GitHub wins); T006 and T007 state cases go green
- [x] T012 [US1] In `scripts/autopilot/run-status.sh` add dwell and `stuck`: `pr-state.sh` called for open PRs only through `AUTOPILOT_PR_STATE`, `since`/`seconds_in_state`/`dwell_source`, ledger commit time (or mtime when uncommitted, via `python3`) fallback, `AUTOPILOT_NOW`; T008 goes green
- [x] T013 [US1] In `scripts/autopilot/run-status.sh` add the text and JSON printers and the parser of the structured *Open escalation* (single and `### Q<n>` batch, per `contracts/supervised-escalation.md`); T007 field cases and T009 go green

**Checkpoint**: `scripts/tests/run-status.test.sh` and `scripts/tests/autopilot.test.sh` green; US1 works on its own.

## Phase 4: User Story 3 - Every state has a fixture and a test (Priority: P2)

**Goal**: a maintainer sees each of the seven states covered by a fixture, in text and JSON. Ships with M1: the cases themselves are written in Phase 3, because tests travel with the code (milestones rule 7).
**Independent Test**: `scripts/tests/run-status.test.sh`.

- [x] T014 [US3] In `scripts/tests/run-status.test.sh` add a closing coverage check that iterates the seven state names and fails when any has no passing text case or no passing JSON case, and asserts `state` is one of the seven in every JSON result (SC-001, FR-014)

## Phase 5: User Story 2 - Supervised escalation ends the turn instead of blocking (Priority: P1)

**Goal**: in a supervised run an escalation writes a structured *Open escalation*, prints the banner and ends the turn; `resume <answer>` records the answer and continues.
**Independent Test**: `scripts/tests/autopilot.test.sh` supervised and answer cases; quickstart step 4.

### Tests for US2 (write first, watch them fail)

- [ ] T015 [US2] In `scripts/tests/autopilot.test.sh` add failing cases for `scripts/autopilot/supervised.sh` (exit 0 with `AUTOPILOT_SUPERVISED` set, non-empty and not `0`; exit 0 with ledger `- **Supervised**: yes`; exit 1 with neither; no output) (FR-009, FR-013)
- [ ] T016 [US2] In `scripts/tests/autopilot.test.sh` add failing cases for `scripts/autopilot/answer.sh <ledger> <answer-text>`: single answer (one *Decisions* row by `user`, `D<n>` one past the highest, section reset to `None.`, rows printed, exit 0), batched answer (one row per question), missing or repeated question number (exit 2, ledger unchanged), no open escalation (exit 3, `ignored: no open escalation`, ledger unchanged, so a second call is ignored), `|` written `\|` and newlines to spaces (FR-010, FR-011a, FR-012, SC-003, SC-004)

### Implementation for US2

- [ ] T017 [US2] Create `scripts/autopilot/supervised.sh [<ledger>]` per `contracts/supervised-escalation.md` *Marker*; T015 goes green
- [ ] T018 [US2] Create `scripts/autopilot/answer.sh` (the only writer of the answer rows; batch versus single set by the number of `### Q<n>` subsections); T016 goes green
- [x] T019 [P] [US2] (ships in M1) Edit `.claude/skills/speckit-autopilot/templates/autopilot-ledger.md`: add the `- **Supervised**:` header line after **Effort** and the structured *Open escalation* format (Category, Question, Options, Recommended, Evidence, Banner; `### Q<n>` batch)
- [ ] T020 [P] [US2] Edit `.claude/skills/speckit-autopilot/tasks/ask.md`: supervised branch (print banner and notification, no `AskUserQuestion`, end the turn; no ledger yet keeps the interactive question, FR-011c), the `resume <answer>` message format and *After the answer* (FR-010, FR-011, FR-011a; unsupervised path untouched, FR-013)
- [ ] T021 [P] [US2] Edit `.claude/skills/speckit-autopilot/tasks/resume.md`: `resume <answer>` calls `answer.sh` then continues from *Next step*; `resume` alone with a non-empty escalation re-prints the banner and ends the turn, supervised only (FR-011b); pointer to `run-status.sh`
- [ ] T022 [P] [US2] Edit `.claude/skills/speckit-autopilot/rules/unit.md` and `.claude/skills/speckit-autopilot/rules/escalation.md`: a unit writes the structured *Open escalation* (FR-010, FR-011c)
- [ ] T023 [P] [US2] Edit `.claude/skills/speckit-autopilot/tests/pressure-scenarios.md`: supervised-escalation scenario (orchestrator must not call `AskUserQuestion`, a repeated answer is not asked again) (SC-003, SC-004)
- [ ] T024 [P] [US2] Edit `.claude/skills/speckit-autopilot/README.md`: list `run-status.sh`, `supervised.sh` and `answer.sh`

**Checkpoint**: supervised escalation works end to end in the fixture; `mise run gate` green.

## Dependencies & Execution Order

- Milestone order, not priority order, sets the file order: T019 belongs to M1; T023 is written before T020–T022 so the prose is checked against a scenario.
- SC-005 (under 5 s) is measured once in quickstart step 2 by the close unit; it is not gated.
- T001 -> T002 -> T003 -> T004 (Foundational) -> US1 (T005-T013) -> T014.
- US2 (Phase 5) needs T001 and T003 only; it follows US1 in delivery order. T019-T024 are `[P]` (disjoint files) and follow T017/T018 only in that prose names the scripts.
- Within a story: tests before the code they cover.

## Implementation Strategy

MVP first: Setup, Foundational, US1 and the coverage check (M1) give the read-only status command every later supervisor milestone consumes. Then US2 (M2) adds the supervised escalation. The `Supervised` ledger header added by T019 is inert until M2 ships `supervised.sh`. T014 is a guard that may pass at once; run it once with a state's case removed to see it fail. M1 and M2 touch no user-guide surface (internal tooling, plan Constitution VII): the `docs-not-needed` label applies. No Polish phase ships code: quickstart results and the SC-005 timing are the close unit's work.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Run status command 🎯 MVP

- **Tasks**: T001–T014, T019
- **Deliverable**: `scripts/autopilot/run-status.sh [--json] [<worktree>]` prints one of seven run states with issue, flow, phase, next step, PRs, dwell and escalation fields, and `scripts/tests/run-status.test.sh` covers every state in text and JSON.
- **Satisfies**: US1 acceptance scenarios 1–8; US3 acceptance scenario 1; FR-001, FR-001a, FR-001b, FR-002–FR-008, FR-014; SC-001, SC-002, SC-005
- **Verify**: `scripts/tests/run-status.test.sh` then `scripts/tests/autopilot.test.sh` (quickstart steps 1–3)
- **Depends on**: —
- **Tier**: full

### M2 — Supervised escalation

- **Tasks**: T015–T018, T020–T024
- **Deliverable**: with `AUTOPILOT_SUPERVISED` or `- **Supervised**: yes`, an escalation writes the structured *Open escalation*, prints the banner and ends the turn; `answer.sh` records `resume <answer>` in *Decisions* and clears it, and `run-status.sh` reports `asking` with the question, options and recommendation.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-009, FR-010, FR-011, FR-011a–c, FR-012, FR-013; SC-003, SC-004
- **Verify**: `scripts/tests/autopilot.test.sh` supervised and answer cases; quickstart step 4
- **Depends on**: M1
- **Tier**: full
