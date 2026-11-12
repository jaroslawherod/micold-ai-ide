# Implementation Plan: Autopilot supervisor M2: supervisor skill that queues issues and starts sessions

**Branch**: `feat/1813_autopilot-supervisor-m2-supervisor-skill-that` | **Date**: 2026-11-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1813-supervisor-skill/spec.md`

## Summary

Add a project skill `.claude/skills/autopilot-supervisor/SKILL.md` and one decision helper,
`scripts/autopilot/supervisor.sh`. The helper is pure bash: it reads the supervisor ledger
(`.autopilot/supervisor.md`, gitignored), run-status JSON and machine readings, and prints
decisions (gate verdict, state mapping, which issues to start). The skill (an LLM session) does the
effects the helper cannot: `gh` reads, `create_worktree`, `create_session`, `list_sessions`, the
background watcher. Splitting them makes FR-010 true: every decision is reproducible from fixtures
with no live session. Run state comes only from `run-status.sh --json <worktree>` (M1, #1812).

## Technical Context

**Language/Version**: bash 3.2-compatible (macOS), `jq`, `git`, `gh`; markdown skill text

**Primary Dependencies**: `scripts/autopilot/run-status.sh --json` and `supervised.sh` (M1, merged);
micold MCP tools `create_worktree`, `create_session`,
`list_sessions` (called by the skill, never by the script); `gh issue view/list`

**Storage**: `.autopilot/supervisor.md` under the project root, a local markdown file, added to
`.gitignore` as `/.autopilot/`; no remote service

**Testing**: `scripts/tests/supervisor.test.sh`, run by `mise run gate` (the gate runs
`scripts/tests/*.test.sh`); fixtures through env overrides; `gh`, MCP and `run-status.sh` are never
called by the helper's decision commands, so no stubs of them are needed

**Target Platform**: Linux and macOS (Principle VI); the memory/load reader branches on `uname`

**Project Type**: skill + shell helper (no Rust, no GUI)

**Performance Goals**: one advance reads each running issue's status once (SC-004); the advance
output is a short table

**Constraints**: no transcript reads (`read_session_output` never named in the skill); no polling
in the supervisor's context; bash 3.2 (no associative arrays)

**Scale/Scope**: a queue of tens of issues, a cap of single digits

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design: unchanged.*

- [x] **I. Test-First**: every helper subcommand is preceded by a failing case in
  `supervisor.test.sh` (tasks order tests first). The skill text is checked by grep cases in the
  same test (no `read_session_output`, `--json` only, supervised marker named).
- [x] **II. Multi-Session Support**: the ledger names its owner session; a second supervisor refuses
  while the owner is listed (FR in Edge Cases); each issue has its own session id and worktree.
- [x] **III. Worktree Integration**: each issue gets `create_worktree` (`github_issue`) then a
  session in it; the supervisor itself runs at the project root, the sanctioned Default location.
  No manual git steps.
- [x] **IV. Local-First**: all state is a local file; the only network use is `gh`, already the
  autopilot's dependency, and it is not needed for gate, state mapping or overlap decisions.
- [x] **V. Rust + iced**: N/A, no application code changes; the helper is tooling in
  `scripts/autopilot/` beside its siblings. Invalid states are narrowed by `supervisor.sh`
  rejecting an unknown ledger state.
- [x] **VI. Cross-Platform**: memory via `/proc/meminfo` (Linux) or `vm_stat` + `sysctl hw.memsize`
  (macOS); load via `uptime`; cores via `nproc` or `sysctl -n hw.ncpu`. Windows is out: the
  autopilot scripts are bash-only already (research.md D6).
- [x] **VII. Documentation**: the skill is the documentation; `README.md` of speckit-autopilot gains
  a pointer, and `docs/` has no user-facing change (tooling only).
- [x] **VIII. Reusable UI**: N/A, no UI.

## Project Structure

### Documentation (this feature)

```text
specs/1813-supervisor-skill/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/supervisor-cli.md      # helper subcommands, ledger file format
└── tasks.md                         # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
.claude/skills/autopilot-supervisor/SKILL.md   # new: the loop, MCP calls, report table
scripts/autopilot/supervisor.sh                # new: gate | map | touches | overlap | next | check-owner | ledger
scripts/autopilot/supervisor-watch.sh          # new: the background watcher (exit on change / 10 min)
scripts/tests/supervisor.test.sh               # new: fixture tests, one per acceptance scenario
scripts/tests/lib/autopilot-fixtures.sh        # reused (new_repo, tmp); extended only if a helper is missing
.gitignore                                     # + /.autopilot/
.claude/skills/speckit-autopilot/README.md     # + pointer to the supervisor skill
.claude/skills/speckit-autopilot/SKILL.md      # + trailing `supervised` argument writes the ledger's Supervised line
```

**Structure Decision**: helpers beside `run-status.sh` in `scripts/autopilot/`, test beside
`run-status.test.sh`. A separate skill (not a flow of `speckit-autopilot`) because the supervisor
starts autopilot sessions and never runs a flow itself.

## Requirement mapping

| Requirement | Where |
|---|---|
| FR-001 queue from list or label | SKILL.md *Build the queue* (`gh issue list --label autopilot:queue`, sorted); `supervisor.sh ledger init` |
| FR-002 gate | `supervisor.sh gate`; contracts/supervisor-cli.md |
| FR-003 worktree + session | SKILL.md *Start an issue* (MCP calls, prompt `/speckit-autopilot #<n> supervised`, research D8) |
| FR-004 overlap | `supervisor.sh touches`, `supervisor.sh overlap`, used by `next` |
| FR-005 ledger | data-model.md; `supervisor.sh ledger set/get` |
| FR-006 resume | SKILL.md *Restart* (incl. a `starting` row with empty cells, data-model.md); `supervisor.sh map` + `check-owner` |
| FR-007 no transcripts | SKILL.md carries no `read_session_output`; grep case in the test |
| FR-008 slots | `supervisor.sh next` counts every state except the four released ones |
| FR-009 report asking/stuck/blocked/dead | SKILL.md *Report*, JSON `escalation`; `map` grace rule |
| FR-010 reproducible | all decisions in `supervisor.sh`, fixtures by env and files |
| FR-011 mapping | `supervisor.sh map` (data-model.md *State mapping*) |
| `.gitignore` (clarify) | `.gitignore` edit, a test asserts `git check-ignore .autopilot/supervisor.md` |
| Wake-up (clarify) | `supervisor-watch.sh`, started by the skill via `run_in_background` |

## Test strategy

| Layer | Covers |
|---|---|
| `supervisor.test.sh` gate cases | US2 scenarios 1-4, SC-003 |
| `supervisor.test.sh` next cases | US1 1-2, SC-001, US3 1-3, FR-004, FR-008 |
| `supervisor.test.sh` map/ledger cases | US4 1-4, SC-002, FR-011, tries/`failed`, owner refusal, `map --exit 2/3`, `skipped` via `ledger set`, owner set after `FREE` |
| `supervisor.test.sh` watcher cases | exits on a changed state and at the ceiling (`AUTOPILOT_WATCH_CEILING`), uses a stub `run-status.sh` through `AUTOPILOT_RUN_STATUS`, `--interval 1 --ceiling 2` |
| `supervisor.test.sh` skill-text cases | US5: no `read_session_output`, `--json` used, supervised marker and prompt present, speckit-autopilot SKILL.md documents the `supervised` argument and the literal `- **Supervised**: yes` line `supervised.sh` matches |
| Not tested automatically | the live MCP calls (`create_worktree`, `create_session`): quickstart.md Part B, run by hand once |
| Core / client / geometry / visual pass | not applicable: no Rust or UI change |

## Complexity Tracking

No violations.
