# Implementation Plan: Autopilot supervisor M1: machine-readable run status and non-blocking escalation

**Branch**: `feat/1812_autopilot-supervisor-m1-machine-readable-run` | **Date**: 2026-11-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1812-supervisor-run-status/spec.md`

## Summary

Add `scripts/autopilot/run-status.sh [--json] [<worktree>]`, a read-only command that reads a run's
ledger and GitHub and prints one of seven states with issue, flow, phase, next step, PRs, time in
state and, for `asking`, the escalation fields. Add a supervised mode: a marker (env var or ledger
field) makes an escalation write a structured *Open escalation*, print the banner and end the turn;
`scripts/autopilot/answer.sh` records the `resume <answer>` reply in *Decisions* and clears the
escalation. Everything is bash + `jq` + `git` + `gh`, in the existing `scripts/autopilot/` helper
set, with prose changes in the `speckit-autopilot` skill. Ledger lookup, PR-number extraction and
the handoff check are reused (the first two moved into the sourced `ledger-of.sh`); dwell limits
come from `pr-state.sh` of #1806 and are defined nowhere else.

## Technical Context

**Language/Version**: bash 3.2-compatible (macOS), `jq`, `git`, `gh`; `python3` for file mtime
(already a dependency: `scripts/autopilot/brief.py`)

**Primary Dependencies**: `scripts/autopilot/ledger-of.sh` (extended), `resume.sh` (refactored to
source it), `handoff-check.sh` (called, unchanged), `pr-state.sh` (#1806, PR #1810, open; called
through `AUTOPILOT_PR_STATE`, default `scripts/autopilot/pr-state.sh`)

**Storage**: the ledger markdown files (`specs/*/autopilot.md` and variants); no new store

**Testing**: shell tests in `scripts/tests/` run by `mise run gate`; `gh` and `pr-state.sh` stubbed
(see Test strategy)

**Target Platform**: Linux and macOS developer machines running autopilot (no GNU-only `date`,
`sed`, `stat` flags)

**Project Type**: CLI helper scripts plus agent-skill prose (no Rust, no GUI)

**Performance Goals**: SC-005: under 5 s for one worktree with GitHub reachable (one `gh pr view`
per ledger PR via `pr-state.sh`, one `gh pr list`)

**Constraints**: bash 3.2: no `mapfile`, no `${var,,}`, no `"${arr[@]}"` of a possibly empty array under `set -u`; read-only (FR-008); no copy of resume or handoff logic (FR-007); unsupervised
behaviour byte-identical (FR-013)

**Scale/Scope**: one worktree per call; ledgers of tens of lines per section

## Constitution Check

*GATE: passes before Phase 0; re-checked after Phase 1 design: unchanged.*

- [x] **I. Test-First**: every script change is preceded by a failing case in
  `scripts/tests/run-status.test.sh` or `scripts/tests/autopilot.test.sh` (Test strategy). The
  skill prose has no executable behaviour and is checked by `pressure-scenarios.md` additions.
- [x] **II. Multi-Session**: state is per ledger and per worktree; the command resolves the ledger
  from the given worktree's branch only and never reads another's (edge case: concurrent runs).
- [x] **III. Worktree Integration**: takes a worktree directory, runs `git -C`/`cd` into it; asks
  nothing of the user in git.
- [x] **IV. Local-First**: ledger and git are local. GitHub is already a dependency of every
  autopilot helper; with `gh` unreachable the PR is `UNKNOWN` and the run is never `done`; the
  command still prints.
- [x] **V. Rust + iced**: N/A, no application code. The workspace's helper scripts are shell by
  precedent (`scripts/autopilot/*`). The run state is a closed set of seven strings, validated by
  the test (SC-001); no Rust type is possible in shell.
- [x] **VI. Cross-Platform Parity**: bash 3.2 (no `mapfile`, no `${var,,}`), portable `date`/`sed`;
  Windows is not an autopilot platform (the scripts already require bash and `gh`).
- [x] **VII. Documentation**: no user-guide surface (internal tooling). The contract is documented
  in `contracts/` and, as shipped, in `tasks/ask.md`, `tasks/resume.md`, the ledger template and
  the skill README.
- [x] **VIII. Reusable UI**: N/A, no UI.

## Project Structure

### Documentation (this feature)

```text
specs/1812-supervisor-run-status/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── run-status.md        # CLI, exit codes, text and JSON output
│   └── supervised-escalation.md   # ledger escalation format, marker, answer message, answer.sh
└── tasks.md                 # /speckit-tasks
```

### Source Code (repository root)

```text
scripts/autopilot/
├── ledger-of.sh        # EDIT: add ledgers_of (paths relative to the toplevel; ledger_of keeps its absolute prefix),
│                       #       ledger_prs, phase_of, ledger_on_main <branch>, final_pr_open <branch>,
│                       #       pr_state_of <n> (moved from resume.sh / new, see research D2)
├── resume.sh           # EDIT: source ledger-of.sh for those helpers; report output byte-identical
├── run-status.sh       # NEW: the status command
├── supervised.sh       # NEW: exit 0 when a run is supervised (env or ledger field)
├── answer.sh           # NEW: record an answer, clear the escalation (the only writer)
├── pr-state.sh         # #1806 (not ours); called, never copied
└── handoff-check.sh    # unchanged; run inside the target worktree
scripts/tests/
├── lib/autopilot-fixtures.sh  # NEW: gh stub, new_repo, ledger, check — extracted from autopilot.test.sh
├── autopilot.test.sh   # EDIT: source the library (no case changes); add answer/supervised cases
└── run-status.test.sh  # NEW: one fixture per state, text and JSON
.claude/skills/speckit-autopilot/
├── templates/autopilot-ledger.md  # EDIT: Supervised field, structured Open escalation
├── tasks/ask.md        # EDIT: supervised branch, answer message (FR-011, FR-011a-c)
├── tasks/resume.md     # EDIT: `resume <answer>`, re-banner (FR-011b), status pointer
├── rules/unit.md, rules/escalation.md  # EDIT: write the structured escalation
├── tests/pressure-scenarios.md    # EDIT: supervised-escalation scenario
└── README.md           # EDIT: list the three scripts
```

**Structure Decision**: single-purpose scripts beside the existing helpers; a sourced library is the
repo's idiom for shared helpers (`ledger-of.sh`).

## Requirement map

| Requirement | Where |
|---|---|
| FR-001, FR-001a | `run-status.sh` state resolver; contracts/run-status.md *State order* |
| FR-001b | resolver `done` branch; research D4 |
| FR-002, FR-003 | contracts/run-status.md *JSON*; data-model *Run state*, *Escalation* |
| FR-004 | `pr_state_of <n>` in `ledger-of.sh` (`gh pr view <n> --json state -q .state`, `UNKNOWN` on failure) is the only source of PR state; the ledger supplies numbers only; `pr-state.sh` is called for open PRs only, to decide `stuck` |
| FR-005 | `handoff-check.sh` in the target worktree |
| FR-006, FR-006a | `pr-state.sh` call; research D3; data-model *Dwell* |
| FR-007 | `ledger-of.sh` helpers, `handoff-check.sh`, `pr-state.sh` reused; research D2 |
| FR-008 | no writes in `run-status.sh`; test asserts `git status` and ledger bytes unchanged |
| FR-009 | `supervised.sh`; contracts/supervised-escalation.md *Marker* |
| FR-010, FR-011b, FR-011c | `tasks/ask.md`, `tasks/resume.md`, `rules/unit.md`; contracts *Escalation format* |
| FR-011, FR-011a | contracts *Answer message*; `tasks/ask.md` |
| FR-012, FR-013 | `answer.sh`; supervised branch gated by `supervised.sh`, no edit to the unsupervised path |
| FR-014 | `run-status.test.sh`, `autopilot.test.sh` |
| SC-001..005 | Test strategy |

## Test strategy

All layers are shell: this feature has no core, client, geometry or visual layer, so no quickstart
§B visual pass applies. `mise run gate` runs `scripts/tests/*.test.sh`.

| Requirement | Layer |
|---|---|
| FR-001, FR-001a, FR-001b, FR-005, SC-001 | `run-status.test.sh`: fixture repo (bare origin + clone, `gh` stub, stub `pr-state.sh` via `AUTOPILOT_PR_STATE`) per state, plus precedence cases (several states match) |
| FR-002, FR-003 | same file, `--json` parsed with `jq -e`; text form grepped |
| FR-004 | ledger says merged, `gh` stub says OPEN: PR reported OPEN |
| FR-006, FR-006a | stub `pr-state.sh` prints OK/STUCK lines; case with no `pr-state.sh` (`dwell_source` falls to ledger commit); no-open-PR run is never stuck |
| FR-008 | before/after checksum of ledger, `git rev-parse HEAD`, `git status --porcelain` |
| Edge cases | several ledgers (error exit), non-worktree path, partial escalation, no `gh`, ledger only on `origin/main`, concurrent runs (two ledgers, ask about one) |
| FR-009, FR-013 | `autopilot.test.sh`: `supervised.sh` with env, with ledger field, with neither |
| FR-010, FR-011a, FR-011b, FR-012 | `autopilot.test.sh`: `answer.sh` single, batched, no open escalation (ignored with note), missing number (escalation kept); prose steps in `tasks/ask.md` and `tasks/resume.md` checked by `pressure-scenarios.md` |
| SC-002 | `run-status.sh --json` output is the only input of the tests' assertions (no ledger text read) |
| SC-003, SC-004 | `answer.sh` cases (section cleared, row recorded, a second `answer.sh` is ignored so the answer is not asked again) plus pressure scenario for the orchestrator not calling `AskUserQuestion` |
| FR-013 | existing `autopilot.test.sh` cases unchanged and green; `supervised.sh` exit 1 leaves `tasks/ask.md`'s unsupervised path untouched |
| several-ledgers | fixture: one `done` plus one open ledger on the same branch resolves to the open one; two open ledgers is exit 3 |
| SC-005 | measured once in quickstart (not a gate) |

Test-first order: fixtures library extraction first (existing suite stays green), then one failing
case per state before each branch of the resolver.

## Complexity Tracking

None. One departure to note: `run-status.sh` is not read-only in the strictest sense because
`handoff-check.sh` runs `git fetch -q origin` (as `resume.sh` already does). It updates remote
tracking refs only; no ledger, branch or PR changes (FR-008 holds as written).
