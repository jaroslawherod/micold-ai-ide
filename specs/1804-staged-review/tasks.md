---
description: "Tasks: staged review for speckit-autopilot"
---

# Tasks: Staged review for speckit-autopilot

**Input**: `specs/1804-staged-review/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)

**Tests**: mandatory and first (Constitution I). Script tests live in `scripts/tests/autopilot.test.sh` and are written, and seen failing, before `review-facts.py`. The markdown flow is checked by grep assertions in the same file.

**Documentation**: no user-facing behaviour (tooling only); the contributor docs are the three skill files. No user-guide task; use the `docs-not-needed` label on the PRs.

**Cross-platform**: python stdlib and `git` only; BSD/GNU tolerant; missing coverage is `unknown` on every OS.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [x] T001 Read `scripts/autopilot/review-snapshot.sh` and `scripts/tests/autopilot.test.sh` helpers (`check`, throwaway-repo builder) and note the reuse points for the new script and tests in a comment block at the top of the new test section in `scripts/tests/autopilot.test.sh`

## Phase 2: User Story 1 - Facts file produced by code (P1) 🎯 MVP

**Independent test**: `scripts/tests/autopilot.test.sh` facts block passes; quickstart steps 1-3.

### Tests first (must fail before T008)

- [x] T002 [US1] Add a `review-facts.py` block to `scripts/tests/autopilot.test.sh`: two changed Rust files give each file with its changed line ranges and each changed fn/type/const by name, including an impl method nested in an `impl` block (US1 sc. 1, FR-001)
- [x] T003 [US1] Same file: a changed symbol called elsewhere lists callers as `file:line` with count and truncated flag (US1 sc. 2, FR-001)
- [x] T004 [US1] Same file: with an `--lcov` file (relative and absolute paths both match repo paths), uncovered changed lines are marked `uncovered`; without it coverage reads `unknown (no coverage data)` and exit is 0, never `covered` (US1 sc. 3-4, FR-003)
- [x] T005 [US1] Same file: docs-only and empty diffs exit 0 with `skipped: no Rust files` for Symbols and Callers; no `origin/main`, detached HEAD and a stale `--base-snapshot` exit 1 with a one-line stderr reason (FR-011 script side)
- [x] T006 [US1] Same file: file name is `review-facts-<tree>-<head>.md` in `--out-dir`, a rerun overwrites it, two different snapshots do not collide, stdout prints the path, the tracked tree/index/HEAD are untouched (FR-002)
- [x] T007 [US1] Same file: a 50-file diff finishes in under 60 s (SC-003), and the script imports no network module and calls no model (grep of its imports)
- [x] T007a [US1] Same file: feed a staged-format verdict (`VERDICT:` and `F<n>` lines) to `brief.py` and the `checkpoint.sh` ledger readers; they accept it unchanged (SC-004)

### Implementation

- [x] T008 [US1] Create `scripts/autopilot/review-facts.py` per `contracts/facts-file.md`: CLI (`--out-dir`, `--base`, `--base-snapshot`, `--lcov`), diff ranges via `git diff -U0` plus working tree as `review-snapshot.sh` reads it, Files section, exit codes and stderr reasons
- [x] T009 [US1] Extend `scripts/autopilot/review-facts.py`: Rust symbol extraction from changed ranges, `git grep -nw` callers with truncation (<400 lines total), `skipped:` markers for non-Rust
- [x] T010 [US1] Extend `scripts/autopilot/review-facts.py`: lcov parsing for the Coverage section, `unknown` default, make it executable; run the T002-T007a block green with `bash scripts/tests/autopilot.test.sh`

## Phase 3: User Story 2 - Planned, parallel-probed review (P1)

**Independent test**: grep assertions pass; quickstart step 4 fixture run reports the planted defect with probe evidence.

### Tests first (must fail before T012-T014)

- [x] T011 [US2] Add grep assertions to `scripts/tests/autopilot.test.sh` over `.claude/agents/autopilot-reviewer.md`, `.claude/skills/speckit-autopilot/tasks/review.md`, `.claude/skills/speckit-autopilot/tasks/review-rounds.md`: 6-probe and 40-line limits, "never `CLEAN`" on an unanswered probe, fallback to single-agent on script failure, stale-snapshot re-run (unit re-checks the snapshot after stage 4), a verdict that needs more reading falls back, token accounting (staged total = plan + workers + verdict), whether the unit passes `--lcov`, shadow mode and `shadow.md`, round-limit rules count the single-agent verdict, `--base-snapshot` for round 2+ (FR-004, FR-005, FR-007, FR-010, FR-011, FR-013)

### Implementation

- [x] T012 [P] [US2] Add a "Stage roles" paragraph to `.claude/agents/autopilot-reviewer.md` (plan stage reads only facts; verdict stage reads facts, plan, evidence); leave the verdict-format text untouched (FR-007)
- [x] T013 [P] [US2] Describe the four-stage flow in `.claude/skills/speckit-autopilot/tasks/review.md` per `contracts/staged-review.md`: stage prompts, one worker per probe on `haiku`/`sonnet` in one message, limits, fallback, shadow mode paragraph; code-diff reviews only (FR-004..007, FR-011)
- [x] T014 [P] [US2] Update `.claude/skills/speckit-autopilot/tasks/review-rounds.md`: round 2+ staged reviews use `review-snapshot.sh diff <snap>` via `--base-snapshot`; round-limit and snapshot rules apply unchanged; stale snapshot discards the staged result (FR-006, FR-010)
- [x] T015 [US2] Run the quickstart step 4 fixture (unchecked error path) once through the documented flow and record in the PR body that the verdict reports it with probe evidence; also run `brief.py` and `checkpoint.sh` readers on the fixture's verdict; run `mise run gate`

## Post-merge (not a milestone): User Story 3 - Shadow run and decision (P2)

Cannot finish in a PR from this feature: it needs the next three code-diff reviews after this change merges. Tracked by a follow-up issue (see the ledger); the close unit does not wait for it.

- [x] T016 [US3] After merge, run the staged flow in shadow beside the single-agent review on the next three milestone or chore code-diff reviews; create `specs/1804-staged-review/shadow.md` with one row per PR (findings of each, parity adjudication, cache-read tokens of each) per `data-model.md` (done in #1808: see `shadow.md`)
- [x] T017 [US3] Record the Decision section in `shadow.md` with the numbers (FR-012, FR-013, SC-001, SC-002, SC-005) and change the three skill files: staged default and shadow dropped, or shadow removed and the finding recorded (done in #1808: decision is not to switch, see `shadow.md`)

## Dependencies

T002-T007a before T008-T010. T011 before T012-T014. T012-T014 disjoint files, parallel. M2 depends on M1 (the flow cites the script). T016-T017 after M2 merges and three reviews have run.

## Implementation Strategy

MVP is the script with its tests (M1); the markdown flow (M2) then makes it used. Shadow follows in the field.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). T016–T017 are intentionally unmilestoned: they close US3, SC-001, SC-002 and SC-005 with follow-up issue #1808, not with this feature.

### M1 — Facts script 🎯 MVP

- **Tasks**: T001–T010 (incl. T007a)
- **Deliverable**: `scripts/autopilot/review-facts.py` writes a per-snapshot facts file (files and ranges, symbols, callers, coverage) with no model, and `autopilot.test.sh` covers it, including SC-003 and SC-004.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-011 (script side); SC-003, SC-004
- **Verify**: `bash scripts/tests/autopilot.test.sh` (facts block green); quickstart steps 1–3
- **Depends on**: —
- **Tier**: full

### M2 — Staged review flow documented

- **Tasks**: T011–T015
- **Deliverable**: the three skill files describe the four-stage flow, fallback and shadow mode, with grep assertions in `autopilot.test.sh`; a fixture run shows the planted defect reported with probe evidence.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-004, FR-005, FR-006, FR-007, FR-008, FR-009 (wording; record is T016), FR-010, FR-011, FR-013 (wording)
- **Verify**: `bash scripts/tests/autopilot.test.sh` (markdown assertions green); quickstart steps 4–6 (step 4 is a manual model run recorded in the PR body)
- **Depends on**: M1
- **Tier**: full
