# Tasks: Autopilot supervisor M2: supervisor skill that queues issues and starts sessions

**Input**: `specs/1813-supervisor-skill/` (spec.md, plan.md, data-model.md, contracts/supervisor-cli.md, research.md)
**Tests**: required (Constitution I). Every test task precedes the implementation it covers, and all
cases live in `scripts/tests/supervisor.test.sh`, so test tasks are never `[P]` with each other.

## Format: `[ID] [P?] [Story] Description` (file path in the description)

## Phase 1: Setup

- [ ] T001 Add `/.autopilot/` to `.gitignore`
- [ ] T002 Create `scripts/tests/supervisor.test.sh` skeleton (sources `scripts/tests/lib/autopilot-fixtures.sh`, `tmp`, pass/fail counters; model on `scripts/tests/run-status.test.sh`) with a case asserting `git check-ignore .autopilot/supervisor.md`

## Phase 2: Foundational (the supervisor ledger)

- [ ] T003 Add ledger test cases to `scripts/tests/supervisor.test.sh`: `ledger init` (creates, refuses an existing file), `ledger owner`, `ledger set/get` per cell, unknown `State` rejected, malformed ledger exits 2, `ledger table` is short
- [ ] T004 Create `scripts/autopilot/supervisor.sh` with argument parsing and the `ledger init|owner|set|get|table` subcommands (bash 3.2, `jq`-free text handling of the markdown table, data-model.md)

## Phase 3: User Story 1 - Start a queue of issues under a cap (P1) 🎯 MVP

**Independent test**: with fixtures, `next` admits at most cap minus occupied issues, in ascending order.

- [ ] T005 [US1] Add `next` test cases to `scripts/tests/supervisor.test.sh`: three queued, cap 2 gives two lowest; `starting` and `running` occupy slots; `done|failed|lost|skipped` release; nothing printed when none (US1 1-2, SC-001, FR-008)
- [ ] T006 [US1] Implement `supervisor.sh next` (cap and slots only; overlap is added in US3)
- [ ] T007 [US1] Add skill-text test cases to `scripts/tests/supervisor.test.sh` (red before T008/T009): `.claude/skills/speckit-autopilot/SKILL.md` documents the trailing `supervised` argument and the literal `- **Supervised**: yes` line `supervised.sh` matches; `.claude/skills/autopilot-supervisor/SKILL.md` names the prompt `/speckit-autopilot #<n> supervised` and never names `read_session_output`
- [ ] T008 [US1] Document the trailing `supervised` argument (writes the ledger's `- **Supervised**: yes` line) in `.claude/skills/speckit-autopilot/SKILL.md`
- [ ] T009 [US1] Create `.claude/skills/autopilot-supervisor/SKILL.md`: *Build the queue* (list or `autopilot:queue`, sorted), *Start an issue* (`create_worktree` with `github_issue`, `create_session` prompt `/speckit-autopilot #<n> supervised`, ledger write before and after; a closed or missing issue, or one that already has a worktree or ledger, gets `ledger set <n> State skipped` with the reason in Note; a failed create is noted in Note; the tries counter and `failed` after 3 tries are enforced in M2, T014-T015), the advance loop (`next`, start, an empty queue says nothing to do, stop when no slot is occupied), finishing report
- [ ] T010 [US1] Add a pointer to the supervisor skill in `.claude/skills/speckit-autopilot/README.md`

## Phase 4: User Story 2 - Resource gate before every start (P1)

**Independent test**: `gate` returns PASS or the first failing check for each fixture reading.

- [ ] T011 [US2] Add `gate` test cases to `scripts/tests/supervisor.test.sh`: cap, memory below 4096 MiB, load above cores, PASS, failure order, readings from `AUTOPILOT_SUP_MEM_MIB|LOAD|CORES` (US2 1-4, SC-003)
- [ ] T012 [US2] Implement `supervisor.sh gate` with Linux (`/proc/meminfo`, `nproc`) and macOS (`vm_stat`, `sysctl`) readers and `uptime` load
- [ ] T013 [US2] Add the gate step to `.claude/skills/autopilot-supervisor/SKILL.md`: run `gate --running <occupied + starts so far>` before each `create_worktree`; on FAIL start nothing, report which check, retry next advance

## Phase 5: User Story 4 - A restarted supervisor resumes from its ledger (P1)

**Independent test**: given run-status fixtures, `map` and `check-owner` reproduce each state and a restart starts nothing twice.

- [ ] T014 [US4] Add test cases to `scripts/tests/supervisor.test.sh`: `map` per data-model.md table (done, dead-no-ledger within and past Grace, asking/stuck/blocked/waiting-ci/running, `--exit 2|3` gives `lost`), tries and `failed` after 3, `skipped` via `ledger set`, `check-owner` OWN/FREE/HELD, owner set after FREE (US4 1-4, SC-002, FR-011)
- [ ] T015 [US4] Implement `supervisor.sh map` and `check-owner`, plus the tries counter rule
- [ ] T016 [US4] Add watcher test cases to `scripts/tests/supervisor.test.sh`: exits `CHANGED <issue> <old> <new>` on a changed state and `CEILING` at the ceiling, stub run-status via `AUTOPILOT_RUN_STATUS`, `--interval 1 --ceiling 2`
- [ ] T017 [US4] Create `scripts/autopilot/supervisor-watch.sh` (state-change or 10 minute exit, `AUTOPILOT_WATCH_CEILING`)
- [ ] T018 [US4] Add *Restart*, *Report* (`asking`, `stuck`, `blocked`, `dead-no-ledger` runs, from the JSON `escalation`) and the background watcher via `run_in_background` to `.claude/skills/autopilot-supervisor/SKILL.md`, including the owner check at start (`check-owner`, then `ledger owner <id>` after `FREE`), `list_sessions` liveness for `lost`, and the `starting` row with empty cells recovery (FR-006, FR-009)

## Phase 6: User Story 3 - Sequence issues that touch the same files (P2)

**Independent test**: two issues with overlapping `Touches:` run one after the other even with cap room.

- [ ] T019 [US3] Add test cases to `scripts/tests/supervisor.test.sh`: `touches` declared line, inferred from `flows` and spec paths, none; `overlap yes|no`; `touches` returns `none` for an issue with nothing inferable; `next` holds the later overlapping issue and releases it once the earlier is released (US3 1-3, FR-004)
- [ ] T020 [US3] Implement `supervisor.sh touches` and `overlap`, and make `next` skip candidates overlapping an occupying row or an earlier pick
- [ ] T021 [US3] Add the Touches step to `.claude/skills/autopilot-supervisor/SKILL.md` (read the issue body with `gh issue view`, run `touches`, store in the ledger; for `none`, write in Note that the issue is treated as disjoint)

## Phase 7: User Story 5 - Small supervisor context (P2)

**Independent test**: grep cases over the skill text.

- [ ] T022 [US5] Add skill-text test cases to `scripts/tests/supervisor.test.sh`: `run-status.sh --json` is the only run-state read and is named once per advance row (SC-004), the advance reports through `ledger table`, and `next` and `gate` agree on the slot count for the same fixture (US5, FR-007, FR-010)
- [ ] T023 [US5] Fix `.claude/skills/autopilot-supervisor/SKILL.md` until the T022 cases pass and the advance output is the short `ledger table`

## Dependencies and order

Phases run in order; within a phase, tests before implementation. `supervisor.sh` and
`supervisor.test.sh` are shared, so tasks touching them are sequential.

## Implementation Strategy

MVP is M1 (ledger, `next` under a cap, the resource gate, the skill that starts sessions; restart and double-start protection (US4) is M2, so M1 is a single foreground run; the gate is in M1 because the spec runs it before every start and M1 starts sessions unattended; M1 has 13 small tasks, under the 800 line limit). Each later milestone adds
one story to the same helper and skill. Quickstart Part B (live MCP calls) is run by the close unit.
No user-guide task: tooling only, the PR carries the `docs-not-needed` label.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Supervisor ledger and capped queue start 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: `/autopilot-supervisor` starts one supervised autopilot session per queued issue up to the cap, only when the resource gate passes, recording each in `.autopilot/supervisor.md`.
- **Satisfies**: US1 acceptance scenarios 1–2; US2 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-005, FR-008; SC-001, SC-003
- **Verify**: `bash scripts/tests/supervisor.test.sh`
- **Depends on**: —
- **Tier**: full

### M2 — Resume and report

- **Tasks**: T014–T018
- **Deliverable**: a restarted supervisor maps run-status JSON onto its ledger, starts nothing twice, reports asking/stuck/blocked/dead runs, and a watcher wakes it on a state change.
- **Satisfies**: US4 acceptance scenarios 1–4; FR-006, FR-009, FR-011; SC-002
- **Verify**: `bash scripts/tests/supervisor.test.sh` (map, check-owner, watcher cases)
- **Depends on**: M1
- **Tier**: full

### M3 — Overlap sequencing

- **Tasks**: T019–T021
- **Deliverable**: issues with overlapping touched files are never started together.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-004
- **Verify**: `bash scripts/tests/supervisor.test.sh` (touches, overlap, next cases)
- **Depends on**: M1
- **Tier**: full

### M4 — Small context guarantees

- **Tasks**: T022–T023
- **Deliverable**: the skill text is proven to read only `run-status.sh --json` and the ledger and to print a short table.
- **Satisfies**: US5 acceptance scenarios; FR-007, FR-010; SC-004
- **Verify**: `bash scripts/tests/supervisor.test.sh` (skill-text cases)
- **Depends on**: M1, M2, M3
- **Tier**: light
