# Autopilot ledger — #944 session-usage-subagents

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 944 — issue #944 "Structured session: usage, context meter, subagents and background tasks"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #944
- **Worktree branch**: claude/project-thread-lllxzt
- **Started**: 2026-10-21
- **Phase**: verify M2
- **Next step**: M2 verify — review B and full gate, then PR files

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1194 | Design PR (spec, clarifications, plan, research, contracts, tasks) | merged (rebase) | 5f95db21d6b0ba7fcf87cb4bdaf647178349b702 |
| #1199 | M1 — core insights | merged (rebase) | 558cb726092ef4b32a536179ccbe0a0f575c1135 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Core insights: context reading, turn/FR-009 cost, totals, display strings; capture env var + 4 recordings | #1199 | merged |
| M2 | T016–T022 | full | Plan-usage indicator fed by structured sessions' `rate_limit_event`; subagent text forwarded; 490 Settings note + guide | — | in progress (T016–T022 ticked) |
| M3 | T023–T028 | full | Core task table + subagent thread index (I4, I5) on the recordings | — | todo (needs M1) |
| M4 | T029–T033 | full | `ProgressBar::marker/warning`, two icons, showcase | — | todo (main as is) |
| M5 | T034–T036 | full | Client `features::structured_insights` state + routing + `InsightsMsg` | — | blocked on **#939 M3** (and M3) |
| M6 | T037–T044 | full | Session bar + X1–X5 drawing, layout gate, guide *Structured sessions*, Part B | — | blocked on **#939 M4, M5, M6** (and M4, M5) |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Depends on #939 (not on main) | Spec builds on #939's transcript view, tool-call cards and turn footer; extends the footer with tokens, does not re-specify it | orchestrator | unit prompt |
| D2 | spec | Plan-usage source for structured sessions | The CLI's `rate_limit_event` (no status line in -p mode) feeds feature 490's reading — confirmed by D4 | agent | crates/micold-core/tests/fixtures/stream-json/text-turn.jsonl `rate_limit_event` |
| D3 | clarify | Q2: cost for subscription-plan accounts (FR-026) | Show for every account, labelled an estimate at API list prices, not what a plan bills; no plan detection | agent | issue #944 acceptance; code.claude.com/docs/en/agent-sdk/cost-tracking |
| D4 | clarify | Q1: may the plan-limit event feed plan usage (FR-010, amends 490 FR-012) | Yes: 490 FR-012 amended to accept the documented `rate_limit_event`; windows read tolerantly from undocumented `unifiedWindows.five_hour/seven_day`; other shapes give no reading, no error | user (jaro, 2026-10-21) | decision card; code.claude.com/docs/en/agent-sdk/typescript#sdkratelimitevent |
| D5 | clarify | Are result cost and per-model figures per turn? (FR-006, FR-009) | No: running totals of the CLI session (incl. subagents, restored on resume); `usage` is per turn, main loop only. Turn figures = difference from previous held result; a drop restarts the count; previous evicted → absent | agent | agent-sdk/cost-tracking#track-costs-in-streaming-input-mode; fixtures resume-init vs text-turn (0.0954 vs 0.0371 USD) |
| D6 | clarify | Does the spec agree with #939's nesting seam? | Yes: threads built via `Transcript::subagent_events(id)` + `Transcript::nested(id)`; FR-017 placeholders replace #939's single notice; footer cost: superseded by D7 | agent | specs/939-structured-transcript-view/contracts/transcript-model.md, research.md R7 |
| D7 | plan | Who shows a turn's own cost? (FR-006, FR-009, FR-026) | #944: #939's footer shows `total_cost_usd` labelled "session total" with no per-turn math (its M5); #944 derives the per-turn cost (FR-009 rules) and draws it beside it. Spec Context, Terms, US2, FR-006, FR-026, Out of Scope and Clarifications updated | coordinator (agreed with #939, 2026-10-21) | coordinator message during plan unit |
| D8 | plan | How do subagent text and thinking reach the stream? (US3 s2, FR-015) | Structured Claude launch sets `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1` (new `AiCliProvider::structured_env`); #938's seven args unchanged; flag rejected (older CLI refuses unknown flag) | agent | code.claude.com/docs/en/headless#follow-subagent-messages; 2.1.306 probe (research R5) |
| D9 | plan | Token-count digit grouping (spec Edge Cases, Principle VI) | Group by thousands with the app's one count format (core `format::group_digits`); spec bullet amended from "user's locale" — the app has no locale layer | agent | crates/micold-client/src/features/worktree_form.rs `thousands`; research R9 |
| D10 | tasks | Milestone cut | Plan's six layered milestones kept (not one per story): stories share the core digest, and UI waits on #939 (rule 6). `rate_limit_event` parse + `limit_window` moved M1 → M2 (plan *Delivery* updated) so M1 stays core usage/context. M1 kept whole at 15 tasks: Setup+Foundational are prerequisites and US1/US2 share the `Result` digest (window comes from it), so no split leaves both halves a deliverable. Polish T045–T046 change no code → close unit | agent | tasks.md *Milestones*; milestones.md rules 3, 4, 6 |
| D11 | implement M1 | Nested subagent recording, and R7's assumption | 2.1.306 refuses a subagent's `Agent` call at the default spawn depth 1; `nested-subagent.jsonl` was recorded with `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=2` (README row says so). The recording shows each level's messages carry their **own** parent call (`spawn_depth` 1 and 2), so R7 holds and the spec's fallback is not needed for M3 | agent | `nested-subagent.jsonl`: inner `Read` under `toolu_014…`, which the relay's `Agent` call opened under `toolu_01P…` |
| D12 | implement M1 | Totals cost text, and a result line that does not type | `totals_line` uses #939's `format_cost` unchanged (`"$0.1400 est. · 3 turns"`; T012's `"$0.14"` was illustrative, R9 names `format_cost`). A `result` line with a token count as a string is `Unrecognised` in #938's parse, so it is no digest (no turn), as #939 shows no footer for it; `insights-tolerance.jsonl` covers it | agent | `transcript/figures.rs` `format_cost`; `parse.rs` `ResultLine` |
| D13 | implement M2 | Feed shape, the shared rejection log, a scaled share beyond `u32` | `Feed` (sender + once-taken receiver) and `Rejections` live in a new `micold-daemon/src/plan_usage.rs`, held by `DaemonState`; `server::spawn_plan_usage_feed` drains it (tests call it, as `service()` starts no tasks). The status route's once-per-answer log moved there too, so a structured session's unreadable event (`"unreadable plan-limit event"`) and a status answer share one per-session set cleared by `offer_plan_usage`. `limit_window` drops a window whose tenths exceed `u32` (contract *Parse*), where `status_line` saturated before; no 490 test covers saturation | agent | contracts/plan-usage-source.md *Parse*, *Feed* 3; research R4 |
| D14 | implement M2 | The daemon test's reset times | `text-turn.jsonl`'s 5-hour reset (2026-10-18) is already past, so `plan_usage_structured.rs` replays a copy with both reset times moved to after the test's clock; utilizations, keys and every other byte unchanged | agent | `date -u -d @1792363200` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 4df620b3eaf8443d5d4dd2bd3d01ab3d1af32bdd:9ad0504870e12886d671ea3f4124162315386f78 | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Spec review | 2 | f68bdebe77fa6265dba0196024706159034d89aa:d9b6e5a168ff412b1c82182fe83426b7f01d88cc | CLEAN |
| Plan review | 1 | 6f1501def4c33357f5c87ab3c095f6d2355879e7:0153e692fd2d790dfd46dceb6612281f428234a8 | CLEAN; 1 MINOR (footer label dependency row) fixed, prose only |
| speckit-analyze | 1 | — | 0 CRITICAL, 0 HIGH; 4 MEDIUM fixed (M6 Satisfies, T039 #939 footer precondition, T003 escalation path, FR cites); LOW T1 (task size) not acted on |
| Tasks review | 1 | 95d88fdccb0d89fd5614e3eab661ae7ae4eb7836:05fc6fbe5aecacd794c413bcf2ff70a83c337b00 | CLEAN; 3 MINOR fixed (checklist ticked after reviewer confirmed no markers; T003 after T002; T036 routing test) |
| Review A M1 (code-review high) | 1 | 77e2180ce36783dd94cad9a7a7b89e5e6bf07015:3df85b84533bf6775439c57d7c3207cde3b648c9 | CLEAN; 3 MINOR: F2 (`totals_line` → `Option`, note only with a cost) and F3 (bound tracks held seqs, live seq gap) fixed with a test; F1 declined |
| Review B M1 (conformance, sonnet) | 1 | dcf8a4a7ee3df67c8cba53abd724893bd1ad62a9:520a46447de01102919168516cd1f256e52b8a2d | CHANGES: F1 MAJOR (no `tdd/cycle-log.md`) fixed — red reproduced by stubbing, `tdd/cycle-log.md` added (evidence only, no code change, so no re-run); F2 MINOR (declined finding outside the table) fixed. Verify: `structured_insights` 26/26, `structured_transcript` 37/37, lib filter 9/9, capture test 11/11 |
| Review A M2 (code-review high) | 1 | 8098161906187c26deea72c8fb8a0d72fcd6e31a:45524d887c4a62cfbb6fefcf136251f2febbeaa6 | No severities given (9 items, none a BLOCKER/MAJOR on verification). Fixed: offers serialised so pushes follow the keep order (`plan_usage_offer` lock); unreadable-event test waits for 2 with the later event as barrier; restored-history test uses a new CLI event as barrier instead of a 2 s silence; shared `limit_window` beyond-`u32` rule tested; terminal-launch assertion skipped when the test env sets the variable. Declined 4 (see table) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A (code-review high) | F1 [MINOR] digest a `result` line #938's parse leaves `Unrecognised` (a string token count) | D12: #939 shows no footer for such a line, so its per-turn figures would have nowhere to show, and counting it in the totals while the transcript shows no turn would disagree with what the user sees; `insights-tolerance.jsonl` pins it |
| M2 | A (code-review high) | Feed only drained when `spawn_plan_usage_feed` runs; call `offer_plan_usage` directly | research R4: `spawn_structured(&self, …)` holds no `Arc<DaemonState>`, so the channel is the plan's default; `server::run` spawns the drain |
| M2 | A (code-review high) | `offer_line` parses a failing line twice | Only for lines containing `rate_limit_event` that fail to read (rare); keeping `parse -> Option` matches the contract's signature |
| M2 | A (code-review high) | `Rejections` entries not cleared when a session is removed | Same as feature 490's status route before this change (an entry per (session, answer), bounded by answers × sessions); out of M2's scope |
| M2 | A (code-review high) | Empty-key filter and `share × 100` then `× 10` scaling | Empty keys are dropped as `status_line` drops them (a window needs a non-empty key, data-model); `limit_window(key, used_percent, …)` is the plan's signature, and recorded shares are two-decimal values that scale exactly enough to round alike |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- #939's transcript tests do not hold over #944's four new recordings (kept out of `RECORDED`, in
  `RECORDED_944`): `every_event_of_every_recorded_fixture_is_accounted_for` fails at
  `subagent-turn` seq 33 (a parented user text has no entry) and
  `replaying_a_recorded_fixture_one_event_at_a_time_changes_only_the_streamed_entry` at
  `background-command` ("SC-002, no text duplicated or lost"). For #939's flow.
- #939's footer cost: resolved by D7 (#939 labels it "session total"; #944 owns the per-turn
  cost). Nothing left for #939's flow.
