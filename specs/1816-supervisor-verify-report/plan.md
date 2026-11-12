# Implementation Plan: Autopilot supervisor M5: verify handoff and report completed runs

**Branch**: `feat/1816_autopilot-supervisor-m5-verify-handoff-and-report` | **Date**: 2026-11-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1816-supervisor-verify-report/spec.md`

## Summary

Add four bash helpers beside the existing `scripts/autopilot/` set: `verify-run.sh` (one verdict
per run, read-only), `follow-up.sh` (turns `NOT DONE` into a send-text or resume request, counts it
in the supervisor ledger, escalates at the cap), `close-out-report.sh` (one report over many runs)
and `safe-to-remove.sh` (the worktrees that may be removed, and removal requests when the user
opted in). The scripts decide and print; the supervisor makes the Micold tool calls
(`send_session_input`, resume, `delete_worktree`), so nothing here touches the app. The verdict
reuses `run-status.sh --json` (which runs `handoff-check.sh` in the run's worktree) and adds the
GitHub issue check; the rules for the supervisor go in one new rules file.

## Technical Context

**Language/Version**: bash 3.2-compatible (macOS), `jq`, `git`, `gh`; no GNU-only `date`/`sed`/`stat` flags

**Primary Dependencies**: `scripts/autopilot/run-status.sh` (#1812, merged; called with `--json`,
never copied), `handoff-check.sh` (reached through it), `ledger-of.sh` (sourced for `ledger_prs`,
`phase_of`), `supervised.sh` (its exact-line field match is the idiom the `Auto-remove` opt-in
copies in shape only), `gh issue view`

**Storage**: markdown ledgers: the run's `specs/*/autopilot.md` (read) and the supervisor ledger
(read; `follow-up.sh` is its only writer here). No new store, nothing remote beyond GitHub reads.

**Testing**: `scripts/tests/close-out.test.sh` run by `mise run gate`; `gh` and `pr-state.sh`
stubbed through `scripts/tests/lib/autopilot-fixtures.sh`

**Target Platform**: Linux and macOS developer machines running autopilot

**Project Type**: CLI helper scripts plus agent-skill prose (no Rust, no GUI)

**Performance Goals**: SC-006: under 10 s per run with GitHub reachable (one `run-status.sh`,
which makes two `gh pr view` per ledger PR when the phase is `done` (its own loop and `handoff-check.sh`; `pr-<n>.json.<k>` fixtures count both) plus one `gh pr list`, plus one `gh issue view`)

**Constraints**: read-only verifier (FR-005); no copy of handoff or run-status logic (FR-002);
bash 3.2: no `mapfile`, no `${var,,}`, no `"${arr[@]}"` of a possibly empty array under `set -u`;
no script removes a worktree itself (FR-013)

**Scale/Scope**: a handful of runs per report; ledgers of tens of lines

## Constitution Check

*GATE: passes before Phase 0; re-checked after Phase 1 design: unchanged.*

- [x] **I. Test-First**: every script is preceded by a failing case in `close-out.test.sh` (Test
  strategy, test-first order). The rules file has no executable behaviour; a scenario added to
  `.claude/skills/speckit-autopilot/tests/pressure-scenarios.md` checks it.
- [x] **II. Multi-Session**: state is per run and per worktree; the verifier resolves the ledger from
  the given worktree's branch only; the supervisor ledger keeps one row per run, so runs never
  share a count; a live session in a worktree blocks its removal.
- [x] **III. Worktree Integration**: every check runs in the run's worktree, never the caller's
  (US1-5). Removal is requested through the app's `delete_worktree`, never a manual git step
  (FR-013).
- [x] **IV. Local-First**: ledgers and git are local; GitHub is already a dependency of every
  autopilot helper. With `gh` unreachable the PR or issue is `UNKNOWN` and the run is not done; the
  scripts still print.
- [x] **V. Rust + iced**: N/A, no application code. Helper scripts are shell by precedent
  (`scripts/autopilot/*`). The verdict is a closed pair and the follow-up action a closed set of
  six words, both asserted by the tests.
- [x] **VI. Cross-Platform Parity**: bash 3.2, portable `sed`/`awk`/`date`; Windows is not an
  autopilot platform (the scripts already require bash and `gh`).
- [x] **VII. Documentation**: internal tooling, no user-guide surface. The contracts are in
  `contracts/` and, as shipped, in `rules/supervisor-close-out.md` and the skill README.
- [x] **VIII. Reusable UI**: N/A, no UI.

## Project Structure

### Documentation (this feature)

```text
specs/1816-supervisor-verify-report/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── verify-run.md         # verdict CLI, exit codes, output
│   ├── follow-up.md          # action words, cap, supervisor-ledger parts it reads and writes
│   └── close-out-report.md   # report layout and safe-to-remove.sh
└── tasks.md                  # /speckit-tasks
```

### Source Code (repository root)

```text
scripts/autopilot/
├── verify-run.sh          # NEW: verdict for one run (read-only)
├── follow-up.sh           # NEW: next action for a NOT DONE run; counts and escalates
├── close-out-report.sh    # NEW: one report over many runs
├── safe-to-remove.sh      # NEW: SAFE / KEEP per worktree; REMOVE requests on opt-in
├── supervisor-ledger.sh   # NEW, sourced: readers (and the follow-up writer) of the supervisor ledger
├── run-status.sh, handoff-check.sh, ledger-of.sh, supervised.sh   # #1812: called or sourced, unchanged
scripts/tests/
├── close-out.test.sh      # NEW: all verdicts, cap, report sections, safe list
└── lib/autopilot-fixtures.sh   # EDIT only if a fixture-ledger writer is missing (the gh stub already serves issue-<n>.json)
.claude/skills/speckit-autopilot/
├── rules/supervisor-close-out.md   # NEW: the close-out step the supervisor skill (#1813) loads
├── tests/pressure-scenarios.md     # EDIT: close-out scenario
└── README.md                       # EDIT: list the four scripts
```

**Structure Decision**: single-purpose scripts beside the existing helpers; the supervisor-ledger
readers live in one sourced file so `follow-up.sh`, `close-out-report.sh` and `safe-to-remove.sh`
share them (precedent: `ledger-of.sh`).

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 | `verify-run.sh`; contracts/verify-run.md *Output*, *Exit* |
| FR-002 | `verify-run.sh` calls `run-status.sh --json <worktree>`, which runs `handoff-check.sh` in that worktree; research D1 |
| FR-003 | `verify-run.sh` issue step; research D2; data-model *Verdict* |
| FR-004 | any remaining line (PR not `MERGED`, `UNKNOWN` included) is `NOT DONE`; `close-out-report.sh` puts only `VERIFIED` runs under Completed |
| FR-005 | no write in `verify-run.sh`; test checks ledger bytes, `HEAD` and `git status --porcelain` unchanged (`handoff-check.sh`'s `git fetch` updates remote refs only, as in #1812) |
| FR-006 | `follow-up.sh` actions `SEND` and `RESUME`; contracts/follow-up.md |
| FR-007 | `follow-up.sh` count in the supervisor ledger *Follow-ups* table, cap 3, `ESCALATE` writes a category 5 block; research D3 |
| FR-008 | `close-out-report.sh`; data-model *Report entry* |
| FR-009 | `Tokens` field reader; `unknown` otherwise; research D5 |
| FR-010 | report sections Completed / Not done / Escalated; contracts/close-out-report.md |
| FR-011 | `safe-to-remove.sh`; research D4 |
| FR-012, FR-013 | `Auto-remove` reader in `supervisor-ledger.sh`; `safe-to-remove.sh` prints `REMOVE <path>` only for SAFE worktrees and only on opt-in; it never runs a removal itself |
| FR-014 | `close-out.test.sh` |
| FR-015 | `rules/supervisor-close-out.md` |
| SC-001..SC-006 | Test strategy |

## Test strategy

All layers are shell: no core, client, geometry or visual layer exists, so no quickstart §B visual
pass applies. `mise run gate` runs `scripts/tests/*.test.sh`.

| Requirement | Layer |
|---|---|
| FR-001, FR-002, FR-004, SC-001 | `close-out.test.sh`: fixture repo (bare origin + clone, `gh` stub, `AUTOPILOT_PR_STATE` stub) per verdict: all merged, open PR, `UNKNOWN` PR, dirty tree, unmerged commit, issue open, `in-progress` left, `flow:` label missing, no PRs, several ledgers, not a worktree |
| US1-5 (caller's worktree) | run from a second fixture clone with a dirty tree; the verdict reflects the target |
| FR-003 | `issue-<n>.json` fixtures through the `gh` stub; no fixture means `UNKNOWN` |
| FR-005 | checksums of ledger, `git rev-parse HEAD`, `git status --porcelain` before and after |
| FR-006, FR-007, SC-004 | `follow-up.sh` fixtures: live session -> `SEND` with the remaining lines; stopped -> `RESUME`; after 3 follow-ups the next call -> `ESCALATE` once, then `HOLD`; the escalation block is in the ledger exactly once; same `--check` id twice -> `SKIP`; `VERIFIED` after a follow-up -> `DONE`, count unchanged |
| FR-008, FR-009, FR-010, SC-002 | `close-out-report.sh` against two fixture ledgers plus a supervisor ledger, compared to an expected text; an open-PR run is never under Completed; no interventions or tokens -> `none` / `unknown`; zero runs and absent supervisor ledger -> empty sections, exit 0 |
| FR-011, FR-012, FR-013, SC-005 | `safe-to-remove.sh` per condition (clean verified, dirty, unmerged commit, live session, not verified, no session list); opt-in off -> zero `REMOVE` lines; opt-in on -> `REMOVE` only for SAFE; a text guard asserts the script contains no worktree removal command and no `rm` |
| FR-015 | `pressure-scenarios.md` scenario; the test greps the rules file for the four script names |
| SC-003 | quickstart: one command from a worktree path |
| SC-006 | measured once in quickstart (not a gate) |

Test-first order: fixtures for the verdict first (one failing case per remaining-item kind before
each branch of `verify-run.sh`), then `follow-up.sh`, then the report and the lister.

## Complexity Tracking

None. One note: `verify-run.sh` is read-only for ledger, branch, PR and issue (FR-005), but
`handoff-check.sh` runs `git fetch -q origin` (remote-tracking refs only), as `run-status.sh` and
`resume.sh` already do.
