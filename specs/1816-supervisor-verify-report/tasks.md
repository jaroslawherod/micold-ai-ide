# Tasks: Autopilot supervisor M5: verify handoff and report completed runs

**Input**: `specs/1816-supervisor-verify-report/` (spec, plan, research, data-model, contracts, quickstart)
**Tests**: required (Constitution I): every script task is preceded by its failing case in
`scripts/tests/close-out.test.sh`. No Rust, no GUI, no user-guide surface (internal tooling).

## Phase 1: Setup

- [ ] T001 Create `scripts/tests/close-out.test.sh` skeleton: source `scripts/tests/lib/autopilot-fixtures.sh`, a fixture repo writer (bare origin + clone + run ledger), `check` summary; picked up by `mise run gate`
- [ ] T002 [P] Extend `scripts/tests/lib/autopilot-fixtures.sh` with a ledger writer and `issue-<n>.json` support in the `gh` stub only if missing (grep first; reuse otherwise)

## Phase 2: Foundational

None: the verdict reuses `run-status.sh` and `handoff-check.sh` (#1812); the supervisor-ledger readers are first needed by US2 (T011).

## Phase 3: User Story 1 - A finished run is verified without opening its session (P1) 🎯 MVP

**Goal**: `verify-run.sh <worktree>` prints `VERIFIED` or `NOT DONE` plus remaining lines; read-only.
**Independent test**: `bash scripts/tests/close-out.test.sh` verdict cases.

- [ ] T003 [US1] Test: all merged, clean, issue closed with flow label, gives `VERIFIED`, exit 0 in `scripts/tests/close-out.test.sh` (US1.1, FR-001)
- [ ] T004 [US1] Tests: open PR, `UNKNOWN` PR, dirty tree, unmerged commit, phase not `done`, no ledger, no PRs, several ledgers, not a worktree each give `NOT DONE` with the data-model line, exit 1 / usage exit in `scripts/tests/close-out.test.sh` (US1.2, US1.3, FR-004)
- [ ] T005 [US1] Tests: issue open, `in-progress` left, `flow:` label missing, issue state `UNKNOWN` (no fixture) in `scripts/tests/close-out.test.sh` (US1.4, FR-003)
- [ ] T006 [US1] Tests: verifying from a second dirty clone reflects the target (US1.5); checksums of ledger, `HEAD`, `git status --porcelain` unchanged (FR-005) in `scripts/tests/close-out.test.sh`
- [ ] T007 [US1] Implement `scripts/autopilot/verify-run.sh` per `contracts/verify-run.md`: `run-status.sh --json <worktree>`, issue step via `gh issue view`, closed verdict pair, exit codes (FR-001..FR-005, FR-002)
- [ ] T008 [US1] Add `verify-run.sh` to the script list in `.claude/skills/speckit-autopilot/README.md`

**Checkpoint**: a run is verified by one command from any worktree.

## Phase 4: User Story 2 - NOT DONE is sent back, then escalated (P1)

**Goal**: `follow-up.sh` turns `NOT DONE` into SEND / RESUME, counts, escalates at the cap.

- [ ] T009 [US2] Tests: live session gives `SEND` with remaining lines, stopped gives `RESUME`, count increments in the supervisor ledger *Follow-ups* (US2.1, US2.2, FR-006) in `scripts/tests/close-out.test.sh`
- [ ] T010 [US2] Tests: after 3 follow-ups the next call gives `ESCALATE` once with a category 5 block exactly once in the ledger, then `HOLD`; same `--check` id gives `SKIP`; `VERIFIED` after a follow-up gives `DONE`, count unchanged (US2.3, US2.4, FR-007, SC-004) in `scripts/tests/close-out.test.sh`
- [ ] T011 [US2] Implement `scripts/autopilot/supervisor-ledger.sh` (sourced): readers of `Auto-remove`, *Interventions*, *Follow-ups*, *Escalations*, and the *Follow-ups* / *Escalations* writer, per `data-model.md`
- [ ] T012 [US2] Implement `scripts/autopilot/follow-up.sh` per `contracts/follow-up.md`: six closed action words, cap 3, bash 3.2 safe

**Checkpoint**: a NOT DONE run is sent back at most 3 times, then escalated once.

## Phase 5: User Story 3 - One completion report across runs (P2)

- [ ] T013 [US3] Tests: two verified runs give one entry each (issue, PRs, decisions agent/user, interventions, tokens); open-PR run only under Not done; escalated section; no interventions/tokens give `none` / `unknown`; zero runs and absent supervisor ledger give empty sections, exit 0 and a run that regresses after being reported done appears under Not done (US3.1..3, edge case, FR-008..FR-010, SC-002) in `scripts/tests/close-out.test.sh`
- [ ] T014 [US3] Implement `scripts/autopilot/close-out-report.sh` per `contracts/close-out-report.md`, classifying each run via `verify-run.sh` (FR-004) (markdown on stdout, `--supervisor-ledger`, `--no-live-sessions`), sourcing `supervisor-ledger.sh`

**Checkpoint**: one command reports many runs; unmerged runs never appear as completed.

## Phase 6: User Story 4 - Worktrees safe to remove are listed, not removed (P2)

- [ ] T015 [US4] Tests: clean verified is `SAFE`; dirty, unmerged commit, live session, not verified, no session list, several ledgers, not a worktree are `KEEP <reason>`; opt-in off gives zero `REMOVE`, opt-in on gives `REMOVE` only for SAFE; the report's Safe to remove section lists them; text guard: script has no worktree removal command and no `rm` (US4.1..4, FR-011..FR-013, SC-005) in `scripts/tests/close-out.test.sh`
- [ ] T016 [US4] Implement `scripts/autopilot/safe-to-remove.sh` per `contracts/close-out-report.md`; wire its section into `close-out-report.sh`
- [ ] T017 [US4] Test: the rules file names the four scripts (FR-015) in `scripts/tests/close-out.test.sh`
- [ ] T018 [US4] Write `.claude/skills/speckit-autopilot/rules/supervisor-close-out.md` (verify, follow-up, report, safe list; supervisor makes the Micold calls) and a close-out scenario in `.claude/skills/speckit-autopilot/tests/pressure-scenarios.md`
- [ ] T019 [US4] List the other three scripts and the rules file in `.claude/skills/speckit-autopilot/README.md`

**Checkpoint**: the supervisor has the full close-out step documented and scripted; nothing removes a worktree.

## Dependencies

T001 first. Each test task precedes its implementation (T003-T006 before T007, T009-T010 before T011-T012, T013 before T014, T015 before T016, T017 before T018). T014 needs T011. T016 needs T011.

## Implementation Strategy

MVP = M1 (verifier). Then follow-up, report, lister. The quickstart "By hand" commands (a command walkthrough, no visual pass) in the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Verify a run 🎯 MVP

- **Tasks**: T001–T008
- **Deliverable**: `scripts/autopilot/verify-run.sh <worktree>` prints `VERIFIED` or `NOT DONE` with the remaining items, read-only, from any worktree.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002, FR-003, FR-004, FR-005
- **Verify**: `bash scripts/tests/close-out.test.sh` (verdict cases); quickstart "By hand" first command
- **Depends on**: —
- **Tier**: full

### M2 — Follow up and escalate

- **Tasks**: T009–T012
- **Deliverable**: `scripts/autopilot/follow-up.sh` returns SEND / RESUME / ESCALATE / HOLD / SKIP / DONE and counts follow-ups in the supervisor ledger, escalating once at the cap.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-006, FR-007, SC-004
- **Verify**: `bash scripts/tests/close-out.test.sh` (follow-up cases)
- **Depends on**: M1
- **Tier**: full

### M3 — Close-out report

- **Tasks**: T013–T014
- **Deliverable**: `scripts/autopilot/close-out-report.sh` writes one markdown report over many runs, with completed, not done and escalated sections.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-008, FR-009, FR-010, SC-002
- **Verify**: `bash scripts/tests/close-out.test.sh` (report cases)
- **Depends on**: M1, M2
- **Tier**: full

### M4 — Safe-to-remove list and close-out rules

- **Tasks**: T015–T019
- **Deliverable**: `scripts/autopilot/safe-to-remove.sh` lists SAFE / KEEP worktrees (REMOVE requests only on opt-in, never removing), and `rules/supervisor-close-out.md` documents the whole close-out step.
- **Satisfies**: US4 acceptance scenarios 1–4; FR-011, FR-012, FR-013, FR-014, FR-015, SC-005
- **Verify**: `bash scripts/tests/close-out.test.sh` (safe-list and rules cases); quickstart "By hand" last two commands
- **Depends on**: M1, M2, M3
- **Tier**: full
