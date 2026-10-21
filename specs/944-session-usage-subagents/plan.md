# Implementation Plan: Structured Session — Usage, Context Meter, Subagents and Background Tasks

**Branch**: `claude/project-thread-lllxzt` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/944-session-usage-subagents/spec.md`

## Summary

Add, to #939's structured-session transcript, the numbers and the hidden work around it. A new
render-free core module, `micold_core::structured::insights`, takes the same `StructuredEvents`
messages #939's `Transcript` takes and derives, incrementally and equal to a fresh build: the
context reading (latest main response's usage against its model's `contextWindow`, the CLI's
`autocompact_state` threshold), each turn's tokens and its cost and per-model figures as the
difference of the CLI's cumulative figures (FR-009, D5), session totals, a task table fed by the
CLI's `task_*` and `background_tasks_changed` lines (background tasks, subagent state), and a
subagent thread index (attribution by `parent_tool_use_id`, placement at any depth, placeholders).
A thread's content is #939's `Transcript::nested(id)` built from `subagent_events(id)`, only while
the thread is open. The daemon turns each `rate_limit_event` a running structured CLI writes into a
feature 490 `UsageReading` (D4), and launches the CLI with `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`
so subagent text and thinking reach the stream. The client keeps a per-window feature module
beside #939's and draws a session bar, footer figures, threads under cards, task states and the
background-task list through five named extension points of #939's view. No new wire message or
protocol version. Details: [research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain)

**Primary Dependencies**: existing only — `serde_json` (reading `raw` of `Assistant`, `Result` and
`Unrecognised` lines), `tokio` (the daemon's plan-usage feed channel), `iced` 0.14 and the shared
`ui/material` library. No new crate.

**Storage**: none. Insights and fold state are per window, in memory; the event history and its
saving are #938's (spec *Assumptions*).

**Testing**: `cargo test` (through `mise run gate`) — core fixture tests
(`crates/micold-core/tests/structured_insights.rs`, `plan_usage_rate_limit_event.rs`) on #938's
recorded fixtures plus four new recordings (research R11); a daemon test against the stand-in CLI
(`crates/micold-daemon/tests/plan_usage_structured.rs`); client reducer tests
(`crates/micold-client/tests/features_structured_insights.rs`); geometry gates for the extended
`ProgressBar` and the session bar; quickstart Part B for draw code.

**Target Platform**: Linux, macOS, Windows (client, daemon, core)

**Project Type**: desktop app — iced client and local daemon over the shared render-free core

**Performance Goals**: meter and figures follow an event within 1 s (FR-005) — O(1) per event,
O(digests held) only on eviction; folded threads cost nothing (no nested transcript built); the
plan-usage reading reaches every window within 10 s (FR-010) — parsed on the reader thread,
broadcast at once.

**Constraints**: no protocol change; #938's seven structured arguments unchanged (research R5);
#939's event → entry rules not duplicated (E-2); terminal sessions untouched (FR-025, SC-007);
warning and failure never by colour alone (FR-003).

**Scale/Scope**: a handful of structured sessions per window; up to `MAX_EVENTS` (5,000) held
events per session; subagent threads of a few hundred events, nested a few levels.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after
Phase 1: unchanged.

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every rule (context reading, FR-009 differences, totals, task transitions, attribution and placement, rate-limit parsing, figure text, fold and thread state) is in `micold-core` or render-free client/daemon code, driven by failing tests first on recorded fixtures. Only `ui/transcript/session_bar.rs`, the draw additions at X1–X5 and the `ProgressBar` paint step use the GUI exception, validated by [quickstart.md](quickstart.md) Part B and geometry gates. |
| **II. Multi-Session** | PASS | `SessionInsights` and view state are keyed by `SessionId` per window; an isolation test drives two sessions. Plan-usage readings from all sessions feed the one shared reading, newest wins, as feature 490 does. Nothing new is persisted. |
| **III. Worktree Integration** | PASS | No file or VCS operation; the session's location is #938's. |
| **IV. Local-First** | PASS | Every figure is read from the local CLI's own output. The plan-usage source reads no credential and makes no request (feature 490 FR-013/FR-014 hold). Nothing leaves the device. |
| **V. Rust + iced** | PASS | iced only. `TaskState`, `TaskKind`, `Owner`, `ContextState`, `SubagentState` are closed enums; a task's id and a thread's parent are fixed at construction; `ContextState::Measured` cannot exist without a window, so a percentage of an unknown window is unrepresentable (FR-004). |
| **VI. Cross-Platform** | PASS | Core never branches on the OS; figures are formatted by one core function on all three (research R9); the env var is set through `Command::env` on every platform; CI runs the gate on all three. |
| **VII. Documentation** | PASS | `docs/user-guide/settings.md` *Claude plan usage* and `worktrees-and-sessions.md` *Claude plan usage* say structured sessions are a source (M2, with the feed); *Structured sessions* describes the meter, figures, threads and background tasks in M6, when they become visible (FR-027). |
| **VIII. Reusable UI** | PASS | The meter extends the shared `material::ProgressBar` with `.marker(Ratio)` and `.warning(bool)` builder steps (showcase entry updated); threads and placeholders reuse #939's `DisclosureCard`; the task list reuses `AccordionSection`; details reuse `Tooltip`; labels reuse `IconLabel` and `Text`. No feature-local widget. |

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/944-session-usage-subagents/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── insights-model.md        # core model: input, event → effect rules, guarantees I1–I6, #939 items used
│   ├── plan-usage-source.md     # rate_limit_event → UsageReading, the daemon feed (amends 490 FR-012)
│   └── transcript-extensions.md # X1–X5: where #944 draws into #939's view
└── tasks.md                     # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── format.rs                         # NEW: group_digits (moved from client worktree_form.rs, R9)
├── lib.rs                            # + pub mod format
├── plan_usage.rs                     # + pub mod rate_limit_event; + limit_window (shared, R4)
├── plan_usage/status_line.rs         # window() → plan_usage::limit_window
├── plan_usage/rate_limit_event.rs    # NEW: parse(raw, obtained_at) (contracts/plan-usage-source.md)
├── structured/mod.rs                 # + pub mod insights; + mod sync
├── structured/sync.rs                # NEW: Continuity — replace-or-continue, shared with Transcript (R1)
├── structured/transcript/mod.rs      # apply() uses sync::Continuity (no behaviour change; #939's G3 tests stay green)
└── structured/insights/              # NEW, render-free
    ├── mod.rs                        # SessionInsights: apply, bound, digest log, refold (R8)
    ├── digest.rs                     # event → Digest, tolerant reads of raw (R2, R3, R6, R7)
    ├── usage.rs                      # Tokens, ModelFigures, TurnUsage, SessionTotals (R3)
    ├── context.rs                    # ContextReading, ContextState (R2)
    ├── tasks.rs                      # TaskTable, Task, TaskKind, TaskState (R6)
    ├── subagents.rs                  # ThreadIndex, ThreadSummary, Owner, ThreadView (R7)
    └── figures.rs                    # display strings (data-model *Formatting*)
crates/micold-core/src/provider.rs    # + AiCliProvider::structured_env() (default &[]; Claude: forward subagent text, R5)
crates/micold-core/tests/
├── structured_insights.rs            # NEW: I1–I6 over the fixtures
├── plan_usage_rate_limit_event.rs    # NEW
└── fixtures/stream-json/             # + subagent-turn, nested-subagent, background-command,
                                      #   background-subagent (recorded), insights-tolerance (synthetic); README rows
scripts/capture-stream-json.sh        # + CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1 (R5, R11)
scripts/tests/capture-stream-json.test.sh  # asserts the variable is set

crates/micold-daemon/src/
├── structured.rs                     # spawn: + provider.structured_env(); on_event unchanged in shape
├── state.rs                          # spawn_structured: rate_limit_event → feed; + offer_plan_usage; feed channel + task
└── hooks.rs                          # status handler → DaemonState::offer_plan_usage (shared)
crates/micold-daemon/tests/
├── plan_usage_structured.rs          # NEW: stand-in CLI replaying text-turn; on / off / restored
└── structured_session.rs             # + the env var reaches the stand-in

crates/micold-client/src/
├── features/structured_insights.rs   # NEW render-free: per-session SessionInsights, open threads, task-list fold
├── features/worktree_form.rs         # thousands → micold_core::format::group_digits
├── features/mod.rs                   # + module
├── app.rs                            # State.structured_insights
├── shell/daemon_sync.rs              # StructuredEvents → structured_insights after structured_transcript; forget paths
├── shell/…                           # InsightsMsg handling
├── icons.rs                          # + Subagent, BackgroundTask
├── ui/material/progress_bar.rs       # + .marker(Ratio), .warning(bool)
├── ui/transcript/session_bar.rs      # NEW draw code (X1)
├── ui/transcript/{mod,entries}.rs    # X1–X5 hooks (contracts/transcript-extensions.md)
├── ui/settings/environment.rs        # plan-usage note: structured sessions are a source too (FR-027)
└── showcase/sections/                # ProgressBar marker/warning; session bar over a fixture
crates/micold-client/tests/
├── features_structured_insights.rs   # NEW reducer tests
├── icons.rs                          # + codepoints
└── gates/session_bar_fits.rs         # NEW, wired into tests/layout_snapshot.rs
docs/user-guide/settings.md, docs/user-guide/worktrees-and-sessions.md  # FR-027
```

**Structure Decision**: the existing three-crate split. Rules in core (fixture-testable before
#939's client lands), the plan-usage feed in the daemon, view state in a render-free client
feature, drawing in `ui/transcript/` at #939's extension points.

## Dependency on #939

#939's design is on main; its code lands in six milestones (its ledger: M1–M2 core on main, M3–M6
client pending). #944 uses, by name:

| #939 item | Module | On main | #944 milestone |
|---|---|---|---|
| `Transcript`, `Transcript::nested`, `subagent_events`, `entry`, `rebuilds`, `omitted_notice`, `apply`'s rule | `micold_core::structured::transcript` | yes (M1–M2) | M1 (Continuity refactor), M3 |
| `ToolCard.status`, `EntryKind::{Footer, SubagentNotice, ToolCall}`, `EntryKey`, `format_cost` | same | yes | M3, M5 |
| `features::structured_transcript` (`SessionTranscript`, routing in `shell/daemon_sync.rs`, forget paths) | client | no (#939 M3) | M5 |
| `material::DisclosureCard`, `AccordionSection` slots, `Text::monospace`, `Icon::ToolFailed` | client | no (#939 M4) | M6 |
| `ui::transcript::view`, `entries.rs` per-entry draw and footer/header view model (T032), `BlockRenderers` | client | no (#939 M5) | M6 |
| the pane showing the transcript, user-guide *Structured sessions* | client, docs | no (#939 M6) | M6 |
| `TurnFooter`, `footer_figures`, and the footer's cost relabelled "session total" | core (`figures.rs`) on main without the label; the label is agreed (D7) but not yet in #939's artifacts — #939 M5 draws it | label: no | M6 (X2) |

The footer's "session total" is #939's (drawn in its M5); the turn's own cost is #944's (D7). If
#939 ships a different footer, M6's X2 integration adapts to it (contracts/transcript-extensions.md
E-5); the core figures do not change.

## Requirement Coverage

Test layers: **core** = `micold-core` unit/fixture tests; **daemon** = `micold-daemon`
integration tests against the stand-in CLI; **client** = `micold-client` reducer tests;
**geometry** = the material library's in-crate gates and `tests/gates/`; **§B** = quickstart Part
B visual pass; **docs** = diff review of the user guide.

| FR / SC | Design | Test layer |
|---|---|---|
| FR-001 | R2; `ContextReading`, `ContextState::Measured` | core I2 (every fixture: used, window, percent) |
| FR-002 | R2 threshold; R12 `ProgressBar::marker` | core (threshold from `autocompact_state`; disabled → none); geometry (marker inside track); §B 1 |
| FR-003 | R2 warning rule; R12 icon + text | core (at threshold; 90% without one); geometry (`composition_contrast` warning fill); §B 7, 8 |
| FR-004 | `ContextState::{NoData, TokensOnly}` | core (no response; window unknown before first result; unknown model) ; §B 2 |
| FR-005 | R2 latest response; O(1) per event | core (a larger, then a smaller response move the reading; compaction) |
| FR-006 | R3 `TurnUsage`; X2 | core I2; client (footer figures for the footer entry's `seq`); §B 1 |
| FR-007 | R3 `SessionTotals` | core I3 |
| FR-008 | R3 counted/left-out, `partial` | core (no usage; ended without result; evicted → partial) |
| FR-009 | R3 difference, restart, evicted-previous | core I3 (`text-turn` + `resume-init`; restart; eviction of the previous result; 50 turns) |
| FR-010 | R4; contracts/plan-usage-source.md | core (parse); daemon (on → reading in a client) |
| FR-011 | `offer_plan_usage` switch check | daemon (off → none, nothing kept) |
| FR-012 | R4 tolerant parse; shared logging | core (malformed shapes); daemon (current reading unchanged) |
| FR-013 | R4: only `on_event` of a running CLI | daemon (restored history → no reading) |
| FR-014 | R7 index; #939's nested scope | core I4; client (open thread holds exactly its events) |
| FR-015 | R7 summary, X3 folded `DisclosureCard`, R10 fold set | core (summary fields); client (default folded, toggle); §B 3 |
| FR-016 | R7 `owner` by `tool_owner` | core I4 on `nested-subagent` |
| FR-017 | R7 `Unknown`; X5 | core (parent evicted; never written); client (placeholder list) |
| FR-018 | R6 task state; `TurnEndedWithoutResult`, `Init`, overlay | core; client (CLI not running → ended) |
| FR-019 | R10 per-window fold | client (two windows' states independent) |
| FR-020 | R6 `background_tasks`, `running_background` | core I5; client (list hidden with none); §B 4 |
| FR-021 | R6 `task_for_tool`; X4 | core; §B 4 |
| FR-022 | R6 `Init`, live set, overlay | core I5; client |
| FR-023 | R8 I1; R1 same message | core I1 |
| FR-024 | tolerant digests | core I6 |
| FR-025 | no terminal path changed | `mise run gate` (whole suite) |
| FR-026 | R12 "est." label and note in details | core (figure strings carry the label and the note) ; §B 1 |
| FR-027 | docs and Settings note | docs (M2: 490 pages and note; M6: *Structured sessions*) |
| FR-028 | R11 fixtures; daemon feed test | core I2–I6 on the recordings; daemon |
| SC-001 | R2, R3 | core I2 |
| SC-002 | R3 | core I3 |
| SC-003 | R7 | core I4 |
| SC-004 | R4 | daemon |
| SC-005 | X1 session bar always shown | §B 1 |
| SC-006 | R8 tolerance | core I6 |
| SC-007 | no change to existing paths | `mise run gate` |

Geometry gates: `ProgressBar`'s marker lies inside the track and its warning fill passes
`composition_contrast` in both schemes (in-crate); `tests/gates/session_bar_fits.rs` (NEW): the
meter, totals and task count stay on one row inside the pane at the narrowest pane width, the
warning label included; `tests/layout_snapshot.rs` gains a transcript pane state with the session
bar and an open thread.

## Delivery

Six milestones; the first four need nothing beyond what is on main today.

| ID | Deliverable | Needs on main | User-visible |
|---|---|---|---|
| **M1** | Core: `sync::Continuity` (extracted from `Transcript::apply`), fixture recordings (R11) and the capture script's env var, `insights` usage + context + figures, `format::group_digits` | main as is | no |
| **M2** | Core `plan_usage::rate_limit_event` + shared `limit_window`; daemon: plan-usage feed from structured sessions (`offer_plan_usage`, feed task), `structured_env` with `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`; feature 490's Settings note and both guide sections (FR-027) | M1 | yes: the app-bar indicator learns from structured sessions |
| **M3** | Core: task table and thread index (I4, I5) on the new recordings | M1; #939 M1–M2 (on main) | no |
| **M4** | Shared UI: `ProgressBar::marker`/`warning`, icons, showcase | main as is | showcase only |
| **M5** | Client: `features::structured_insights` (state, `InsightsMsg`, open threads, routing after `structured_transcript`, forget paths) | M3; **#939 M3** | no |
| **M6** | Client: session bar and X1–X5 drawing, layout gate, user guide *Structured sessions* (FR-027), Part B | M4, M5; **#939 M4, M5, M6** | yes: meter, figures, threads, background tasks |

M2 ships the plan-usage feed before any view (it needs no view). The meter and the rest become
visible only with M6, after #939's pane shows the transcript, so the user guide's *Structured
sessions* text changes with M6.

## Complexity Tracking

None.
