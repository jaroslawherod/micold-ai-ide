---
description: "Task list for autopilot supervisor M4: route session questions to the user in one batch (#1815)"
---

# Tasks: Autopilot supervisor M4: route session questions to the user in one batch

**Input**: `specs/1815-supervisor-question-routing/` (plan.md, spec.md, research.md, data-model.md, contracts/routing-cli.md, quickstart.md)

**Tests**: Constitution I: every test task comes before the task that makes it pass, and is seen to fail first. Tests are bash cases in `scripts/tests/supervisor.test.sh` and `scripts/tests/autopilot.test.sh`, fixtures only (no `gh`, MCP or live session). Skill and task text is checked by grep cases, as in #1813.

**Documentation**: tooling only, no user-guide page (plan, Constitution VII). The skill text is the documentation; the README pointer and `rules/supervised.md` change in the story that needs them.

**Format**: `- [ ] T### [P?] [US?] Description with file path`. `[P]`: different files, no dependency on an unfinished task.

## Phase 1: Setup

- [x] T001 Search `scripts/tests/supervisor.test.sh` and `scripts/tests/autopilot.test.sh` for existing fixture helpers (ledger init, assert, temp dir); add one shared helper in `scripts/tests/supervisor.test.sh` that writes an escalation JSON file (`category`, `banner`, `questions[{number, question, options[{label,description}], recommendation, evidence}]`, as quickstart A2) and a session-list JSON file with an optional `parent` field.

## Phase 2: Foundational (blocks every story)

- [x] T002 Failing case in `scripts/tests/supervisor.test.sh`: a ledger with a `## Routing` section still parses in the existing `ledger table`, `ledger set` and `next` paths (research D2), `route table` on a ledger without `## Routing` prints nothing, and the first `route add` gains an empty `## Routing` section; a usage error and a malformed ledger exit 2.
- [x] T003 Add the `route` subcommand dispatch and the Routing section reader/writer to `scripts/autopilot/supervisor.sh` (sharing its ledger reader and `die`; bash 3.2, no associative arrays): rows keyed `R<issue>.Q<n>`, cells State, Session, Via, Question, Options, Rec, Evidence, Answer, Sent (data-model.md), every cell stored pipe-free (`|` stored as `/`, newlines to spaces, options joined by ` // `), `route table` prints the section. If the route code passes about 150 lines, move it to `scripts/autopilot/supervisor-route.sh` sourced by `supervisor.sh` (plan, Structure Decision).

**Checkpoint**: `route table` works on any ledger; existing supervisor cases still pass.

## Phase 3: User Story 1 - Two sessions ask at once, the user answers once (P1) 🎯 MVP

**Goal**: two `asking` runs produce one prompt and one notification; each answer reaches only its own session (SC-001, SC-004).

**Independent test**: quickstart A2 (two-session fixture batch) and the `route` cases below.

- [x] T004 [US1] Failing cases in `scripts/tests/supervisor.test.sh` for `route add` (one `R<issue>.Q<n>` per question written `pending`, free-text escalation gives `Q1`, re-add is a no-op so a restart never asks twice, a question containing `|` keeps State, Session and Answer in their cells) and `route next` (up to 4 `pending` rows, ascending issue then number, five questions give four then one, empty when none), per contracts/routing-cli.md.
- [x] T005 [US1] Failing cases in `scripts/tests/supervisor.test.sh` for `route state` (accepts `pending|asked|answered|resolved`, exit 2 on unknown state or key), `route answer <key> user|parent <text>` (State `answered`, Answer set), `route compose <issue>` (first line `session <id>`, then `<n>. <answer>` per question, bare answer for a free-text escalation; exit 1 printing `missing: <n> ...` while any question has no answer, FR-008a; two issues answered, compose of one prints only that issue's answers and its own session, SC-004) `route sent <issue> <epoch>` and `route drop <issue>` (removes only that issue's rows, `|` cells intact; a later `route add` for the same issue starts fresh).
- [x] T006 [US1] Implement `route add`, `route next`, `route state`, `route answer` in `scripts/autopilot/supervisor.sh` (or `supervisor-route.sh`) until T004 and the `state` and `answer` cases of T005 pass.
- [x] T007 [US1] Implement `route compose` (session first, held until every question of the issue is `answered` or `resolved`, exit 1 with `missing:`), `route sent` and `route drop` in the same file until T005 passes.
- [x] T008 [P] [US1] Failing grep cases in `scripts/tests/autopilot.test.sh` over `.claude/skills/speckit-autopilot/tasks/ask.md` step 0 and `rules/supervised.md` (FR-006a): a supervised run writes its banner to its own ledger and sends no notification; an unsupervised run is unchanged. Then edit `ask.md` step 0 and add the one line on who notifies to `rules/supervised.md` until they pass.
- [x] T009 [US1] Failing grep cases in `scripts/tests/supervisor.test.sh` over `.claude/skills/autopilot-supervisor/SKILL.md` (steps in order: collect each `asking` run's `<n>.json` from the Restart step 3 and never name `read_session_output`; `route add`; send the banner of `tasks/ask.md` and one `PushNotification` (text `🛑 <issues> need a decision: <topic>`); `route state <key> asked` for each row of the batch after the banners and the notification and before `AskUserQuestion` (FR-011: a restart never asks twice); one `AskUserQuestion` of at most 4; `route answer`; `route compose` then `send_session_input` only on exit 0 and only to the printed session; `route sent`; after a `CHANGED` wake shows the run left `asking`, `route drop`; the banner carries the `#<issue>` prefix, unit `supervisor`, a `(Recommended)` first option with its evidence, and is answerable cold; a `compose` exit 1 re-asks only the missing question; more than 4 questions go in the next batch after the first answer; a send that fails, not `not_found`, runs `ledger set <issue> Note`, leaves the rows `answered`, reports, and never retries in a loop; a `not_found` runs `route drop` and reports the session lost), and rewrite the existing `Never answer` case so it matches the routed *Report*; then add the *Route questions* section and edit *Report* so an `asking` run is routed (FR-015; `stuck`, `blocked`, `lost` stay reported) until they pass.
- [x] T010 [US1] Add the pointer line to `.claude/skills/speckit-autopilot/README.md`, run `bash scripts/tests/supervisor.test.sh` and `bash scripts/tests/autopilot.test.sh`, and walk quickstart Part A2 by hand against the fixtures.

**Checkpoint**: the two-session batch works end to end on fixtures.

## Phase 4: User Story 2 - A routine question never reaches the user (P1)

**Goal**: a *Never ask* question the repo answers is resolved by the supervisor and never prompts the user (SC-002); categories 2, 3, 4 always go to the user.

**Independent test**: fixture escalations of a "never ask" kind and of each category; assert the route.

- [ ] T011 [US2] Failing cases in `scripts/tests/supervisor.test.sh` for `route gate <category>` (`user-only` for a category starting `2`, `3` or `4`; `judge` for `1`, `5`, `6`; exit 2 unknown) and for `route answer <key> supervisor <text>` (State `resolved`) with `route compose` printing the leading `--by supervisor` word only when every row of the issue is `resolved` by the supervisor.
- [ ] T012 [P] [US2] Failing cases in `scripts/tests/autopilot.test.sh` for `answer.sh --by supervisor <ledger> <text>` (By cell holds `supervisor`; default stays `user`; exit codes 0, 2, 3 unchanged, per `specs/1812-supervisor-run-status/contracts/supervised-escalation.md`) and grep cases over `.claude/skills/speckit-autopilot/tasks/resume.md` (`resume --by supervisor <answer>` runs `answer.sh --by supervisor`; `resume <answer>` is By `user`).
- [ ] T013 [US2] Implement `route gate` and the `supervisor` via of `route answer` and `route compose` in `scripts/autopilot/supervisor.sh` until T011 passes.
- [ ] T014 [P] [US2] Add `--by <who>` (default `user`, written in the By cell of every row it adds) to `scripts/autopilot/answer.sh` and the `resume --by` pass-through to `.claude/skills/speckit-autopilot/tasks/resume.md` until T012 passes.
- [ ] T015 [US2] Failing grep cases in `scripts/tests/supervisor.test.sh` over SKILL.md (judge the subject by `route gate`, not the category label; cite repo evidence in Answer; no evidence goes to the user batch as `pending`; send with `resume --by supervisor`; a missing or unreadable category, where `route gate` exits 2, goes to the user), then add step 2 to the *Route questions* section of `.claude/skills/autopilot-supervisor/SKILL.md` until they pass.
- [ ] T016 [US2] Run both script tests and confirm US2 acceptance scenarios 1 to 5 by their fixtures (resolved question: one send, no `AskUserQuestion`; category 2, 3, 4 or a non-routine subject: user batch).

**Checkpoint**: routine questions resolve without the user; `answer.sh --by` merged.

## Phase 5: User Story 3 - Over the context cap, ask again after `/clear` (P2)

**Goal**: over the cap the batch survives in the ledger and is asked once after restart; a question answered meanwhile is dropped.

**Independent test**: run the routing step with the context check reporting `OVER`; no second prompt after restart.

- [ ] T017 [US3] Failing cases in `scripts/tests/supervisor.test.sh` for `route next --reask` (every `asked` row, no limit, ascending; no banner or notification text) and `route overdue --now <epoch> --ceiling <s>` (issues whose Sent is set and older than the ceiling; none when Sent is empty).
- [ ] T018 [US3] Implement `route next --reask` and `route overdue` in `scripts/autopilot/supervisor.sh` until T017 passes.
- [ ] T019 [US3] Failing grep cases in `scripts/tests/supervisor.test.sh` over SKILL.md, then edit `.claude/skills/autopilot-supervisor/SKILL.md` until they pass: the step 3 context check ends the turn on `OVER` only after the banners, the notification and `route state ... asked` (M1) and prints `Cheaper: /clear, then /autopilot-supervisor — it asks this again from the ledger.` (FR-010); *Restart* calls `route next --reask` and re-asks without a banner or notification; a session that left `asking` unanswered gets `route drop` (FR-009; the subcommand ships in M1); after the send a `CHANGED` wake showing the run left `asking` removes the rows, and still `asking` past the Watcher ceiling is listed by `route overdue` and reported, never re-sent.

**Checkpoint**: SC-003 holds on fixtures.

## Phase 6: User Story 4 - A supervisor below the top tier relays the question upward (P3)

**Goal**: with a reported parent, the question goes to the parent; no parent means the user.

**Independent test**: a session-list fixture with a `parent` field and one without.

- [ ] T020 [US4] Failing cases in `scripts/tests/supervisor.test.sh` for `route target --sessions <file> --me <id>` (`parent <id>` when that row has a non-empty `parent`; `user` when the field is empty or absent; exit 2 when the file is unreadable).
- [ ] T021 [US4] Implement `route target` in `scripts/autopilot/supervisor.sh` until T020 passes.
- [ ] T022 [US4] Failing grep cases in `scripts/tests/supervisor.test.sh` over SKILL.md, then edit `.claude/skills/autopilot-supervisor/SKILL.md` until they pass: step 6 sends the banner to the parent with `send_session_input`, holds the rows `asked` with Via `parent`, and records the reply with `route answer ... parent`; a `not_found` refusal runs `ledger set <issue> Note "<refusal>"` then asks the user (Via `user`), never retried (FR-012, FR-013); a top-tier supervisor asks the user.

**Checkpoint**: all four stories pass on fixtures.

## Dependencies

Phase 1 → Phase 2 → US1 → US2 → US3 → US4 (US3 and US4 depend only on Phase 2 and US1; they are ordered by priority). Within a story: tests before the task that makes them pass.

## Parallel opportunities

T008 with T004; T012 with T011; T013 with T014 (different files).

## Implementation Strategy

MVP is M1: the routing core, `route` for batching and compose, the ask.md change, and the skill's batch step. Each later story adds its subcommands and skill steps and merges alone.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Batch routing core 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: two `asking` fixture runs produce one batched question list, one notification and a per-session answer message; `supervisor.sh route add|next|state|answer|compose|sent|drop|table` exist, the skill's *Route questions* section routes `asking` runs, and a supervised run no longer notifies on its own.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-006, FR-006a, FR-007, FR-008 (send only to the asking session), FR-008a, FR-011, FR-014, FR-015; SC-001, SC-004
- **Verify**: `bash scripts/tests/supervisor.test.sh` and `bash scripts/tests/autopilot.test.sh` print no FAIL; quickstart Part A2
- **Depends on**: —
- **Tier**: full

### M2 — Resolve routine questions from the repo

- **Tasks**: T011–T016
- **Deliverable**: `route gate` and `answer.sh --by supervisor` exist; the supervisor answers a *Never ask* question from repo evidence with `resume --by supervisor`, and categories 2, 3, 4 always reach the user.
- **Satisfies**: US2 acceptance scenarios 1–5; FR-002, FR-003, FR-004, FR-005, FR-005a; SC-002
- **Verify**: `bash scripts/tests/supervisor.test.sh` and `bash scripts/tests/autopilot.test.sh` print no FAIL (the `route gate`, `--by` and SKILL step 2 cases)
- **Depends on**: M1
- **Tier**: full

### M3 — Survive the context cap

- **Tasks**: T017–T019
- **Deliverable**: `route next --reask` and `route overdue` exist; the skill sends banners before ending the turn on `OVER`, re-asks after restart and drops answered questions.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-008 (confirm and ceiling), FR-009, FR-010, FR-011; SC-003
- **Verify**: `bash scripts/tests/supervisor.test.sh` prints no FAIL (the `--reask`, `overdue` and SKILL step 3 cases)
- **Depends on**: M1
- **Tier**: light

### M4 — Upward relay

- **Tasks**: T020–T022
- **Deliverable**: `route target` exists; a supervisor whose session list reports a parent relays the question to it, and a `not_found` refusal falls back to the user.
- **Satisfies**: US4 acceptance scenarios 1–4; FR-012, FR-013
- **Verify**: `bash scripts/tests/supervisor.test.sh` prints no FAIL (the `route target` and SKILL step 6 cases)
- **Depends on**: M1
- **Tier**: light
