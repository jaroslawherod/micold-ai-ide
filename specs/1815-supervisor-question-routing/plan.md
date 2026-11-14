# Implementation Plan: Autopilot supervisor M4: route session questions to the user in one batch

**Branch**: `claude/project-thread-0lx36q` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1815-supervisor-question-routing/spec.md`

## Summary

Extend the `autopilot-supervisor` skill so an `asking` run is routed instead of only reported. The
split of #1813 stays: decisions are pure bash in `scripts/autopilot/supervisor.sh` (new `route`
subcommand family, reproducible from fixtures); the LLM session does the effects (`AskUserQuestion`,
`PushNotification`, `send_session_input`, `list_sessions`) and the one judgment a script cannot make:
whether an escalation's *subject* is a *Never ask* item the repo answers (FR-003). The script
enforces the hard rule (category 2, 3, 4 never answered by the supervisor), batching (4 per call,
ascending issue order), per-question completeness (FR-008a), routing state (FR-011) and the upward
target (FR-012). Two worker-side edits ship with it: `answer.sh --by <who>` plus the `resume --by`
pass-through in `tasks/resume.md` (FR-005a), and `tasks/ask.md` step 0 sending no notification when
supervised (FR-006a). Nothing here uses code from open PR #1873 (ledger D1).

## Technical Context

**Language/Version**: bash 3.2-compatible (macOS), `jq`; markdown skill and task text. No Rust.

**Primary Dependencies**: `scripts/autopilot/run-status.sh --json` (#1812; `escalation.category`
string, `banner`, `questions[{number, question, options[{label,description}], recommendation,
evidence}]`), `answer.sh`, `supervisor.sh` and `supervisor-watch.sh` (#1813), `context.py`; MCP tools
`send_session_input`, `list_sessions` (called by the skill, never by scripts); `AskUserQuestion`,
`PushNotification`.

**Storage**: a new `## Routing` section in `.autopilot/supervisor.md` (gitignored, local file); the
worker's own ledger *Decisions* via `answer.sh`. No remote service.

**Testing**: `scripts/tests/supervisor.test.sh` (route subcommands and skill grep cases),
`scripts/tests/autopilot.test.sh` (`answer.sh --by`, ask.md and resume.md text); both run by the
gate (`scripts/tests/*.test.sh`). Fixtures only: no `gh`, MCP or live session.

**Target Platform**: Linux and macOS (Principle VI); no OS-specific calls (spec Edge Cases).

**Project Type**: skill + shell helper (tooling, no GUI).

**Performance Goals**: one prompt and one notification per batch (SC-001); no transcript reads.

**Constraints**: bash 3.2 (no associative arrays); pipe-free ledger cells (`\|` escaping as in
`answer.sh`); existing `supervisor.sh` row parsing must not see Routing rows (research D2).

**Scale/Scope**: single-digit cap, so at most a few open questions; 4 per `AskUserQuestion`.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1: unchanged.*

- [x] **I. Test-First**: each `route` subcommand and the `--by` flag is preceded by a failing case
  (tasks order tests first); the skill and task text edits are checked by grep cases, as in #1813.
- [x] **II. Multi-Session**: routing state is per issue and per question; an answer carries its
  session id and goes only to it (SC-004, `route compose` is keyed by issue); a new `asking` run
  during an answer waits for the next batch; the ledger owner check of #1813 gates who routes.
- [x] **III. Worktree**: N/A to file operations; the supervisor stays at the project root (Default
  location); no manual git steps. Worker edits touch only that worktree's ledger.
- [x] **IV. Local-First**: all state in a local file; works offline (no `gh` in any routing path).
- [x] **V. Rust + iced**: N/A, no application code. Invalid routing states are rejected by
  `route state` the way `ledger set State` rejects unknown states.
- [x] **VI. Cross-Platform**: bash 3.2 and `jq` only, as `supervisor.sh` today. Windows is out, as in
  #1813 research D6.
- [x] **VII. Documentation**: the skill text is the documentation; `speckit-autopilot/README.md`
  pointer line updated; `rules/supervised.md` and `tasks/ask.md`, `tasks/resume.md` updated in the
  same change. No user-guide page (tooling only).
- [x] **VIII. Reusable UI**: N/A, no UI.

## Project Structure

### Documentation (this feature)

```text
specs/1815-supervisor-question-routing/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/routing-cli.md    # supervisor.sh route, answer.sh --by, resume text
└── tasks.md                    # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
scripts/autopilot/supervisor.sh                       # + `route` family (add, next, state, answer, compose, overdue, target, gate, table, drop)
scripts/autopilot/answer.sh                           # + `--by <who>` (default user)
.claude/skills/autopilot-supervisor/SKILL.md          # + Route questions; Report and Restart edited
.claude/skills/speckit-autopilot/tasks/resume.md      # resume --by pass-through
.claude/skills/speckit-autopilot/tasks/ask.md         # step 0: supervised sends banner only, no PushNotification
.claude/skills/speckit-autopilot/rules/supervised.md  # one line: who notifies
.claude/skills/speckit-autopilot/README.md            # pointer line
scripts/tests/supervisor.test.sh                      # + route cases, skill grep cases (edit the "Never answer" case)
scripts/tests/autopilot.test.sh                       # + answer.sh --by, ask.md and resume.md cases
```

**Structure Decision**: no new script file: `route` is a subcommand of `supervisor.sh` beside `ledger`
and `next`, sharing its ledger reader and `die`. If it passes ~150 lines it moves to
`scripts/autopilot/supervisor-route.sh` sourced by `supervisor.sh` (decided at tasks time, no
interface change).

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-014 | SKILL *Route questions* step 1 reads the Restart step-3 `<n>.json`; `route add` ingests its `escalation` object |
| FR-002, FR-004 | SKILL step 2 (judge subject, cite evidence); no evidence: `route add` as `pending` for the user |
| FR-003 | `route gate <category>` (`user-only` for 2, 3, 4; `judge` for 1, 5, 6) plus SKILL subject-override rule |
| FR-005 | SKILL step 5 sends `route compose` output as `resume --by supervisor <answer>`; `route state ... resolved` |
| FR-005a | `answer.sh --by`; `tasks/resume.md` (contracts/routing-cli.md) |
| FR-006, FR-007 | `route next` (cap 4, ascending issue); SKILL banner, `PushNotification`, one `AskUserQuestion` |
| FR-006a | `tasks/ask.md` step 0, `rules/supervised.md` |
| FR-008 | `route state ... answered` + `route sent`; confirm on `CHANGED`; `route overdue` for the ceiling |
| FR-008a | `route compose` exits 1 listing the missing question numbers |
| FR-009 | SKILL step 1 on re-ask: `route drop <issue>` when the run is no longer `asking` |
| FR-010 | SKILL step 3 (context check) + Routing rows holding the full question, options, recommendation |
| FR-011 | Routing `State` column: `pending asked answered resolved` (data-model.md) |
| FR-012, FR-013 | `route target`; SKILL step 6; a `not_found` send is recorded with `route state` and asked of the user |
| FR-015 | SKILL *Report* now routes `asking`; `stuck`, `blocked`, `lost` stay reported |

## Test strategy

| Requirement | Layer |
|---|---|
| FR-003, FR-006, FR-008, FR-008a, FR-009 (drop), FR-011, FR-012 target | `supervisor.test.sh`: fixture escalation JSON and fixture session-list JSON, `route` outputs and exits |
| FR-005a | `autopilot.test.sh`: `answer.sh --by supervisor` writes `supervisor` in By; default stays `user`; exit codes unchanged |
| FR-002, FR-004, FR-005, FR-007, FR-010, FR-013, FR-015 (LLM steps) | grep cases over SKILL, ask.md, resume.md: the sentence, the order, the forbidden phrases (no `read_session_output`, no re-send loop); quickstart A walks them by hand with a fake `send_session_input` |
| FR-006a | `autopilot.test.sh` grep over ask.md step 0 |
| SC-001..SC-004 | quickstart A (two-session fixture batch), plus `route compose` keyed-by-issue case for SC-004 |
| Geometry gates, §B visual pass | none: no GUI |
