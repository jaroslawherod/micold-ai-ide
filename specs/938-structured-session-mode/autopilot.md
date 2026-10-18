# Autopilot ledger — #938 structured-session-mode

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 938 — issue #938 "Structured session mode: run claude with stream-json next to the PTY mode"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #938
- **Worktree branch**: claude/project-thread-cl0lw3
- **Started**: 2026-10-18
- **Phase**: M1
- **Next step**: M1 — PR to open (branch pushed; title/body in the scratchpad), then CI and merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1035 | design PR (spec, plan, research, contracts, tasks) | merged (rebase) | 0d6ced4e44dcf63df729fc15bdd8765cdf045a8d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | parser types every recorded fixture line; kind persisted | to-open | gate green |
| M2 | T011–T021 | full | daemon runs a structured session and streams events; lifecycle parity | — | planned |
| M3 | T022–T028 | full | first prompt and user messages | — | planned |
| M4 | T029–T030 | full | resume | — | planned |
| M5 | T031–T032 | full | late and multiple viewers | — | planned |
| M6 | T033–T036 | full | terminal-only requests refused | — | planned |
| M7 | T037–T047 | full | chevron entry, row marker, log pane, user guide | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Permission behaviour of a structured session before #942 | Same permission configuration as a terminal session, no override; approvals the CLI would need are refused and show as events (FR-006a) | agent | crates/micold-core/src/provider.rs ToolServerSupport; spec FR-006 |
| 2 | clarify | How the structured kind is chosen; pane before #939 | Per-session, start-session chevron, Claude only; plain read-only line-per-event log (FR-022, FR-022a) | user (provisional: recommended option) | worktrees-and-sessions.md chevron |
| 3 | clarify | Does event history survive a daemon restart | No: daemon memory only; history after restart starts at the resume; #939 reads the CLI transcript (FR-021a) | user (provisional: recommended option) | — |
| 4 | clarify | Where the structured choice is offered | "Claude Code (structured)" entry in the start-session chevron, chevron shown even with one CLI; other entry points create terminal sessions | agent | worktrees-and-sessions.md#choosing-which-ai-cli-a-session-runs |
| 5 | tasks | Milestone cut | M1 = Setup + Foundational + US2 parser (P1); US1 split along scenarios 1–3 / 4–5 (M2, M3), as US1 with the foundation exceeds ~10 tasks; US5 lifecycle tests moved into M2 where the Proc enum is built; added US6 (P3) to spec for FR-022/022a/023 acceptance scenarios; M2 at 11 and M7 at 11 tasks kept whole (no smaller observable split) | agent | speckit-analyze C1, milestones.md rules 1–3 |
| 6 | tasks | Plan fixes from speckit-analyze | `start_new_of_kind`/`restored_of_kind` delegation instead of changing 300 callers (I1); PROTOCOL_VERSION bumped per wire-changing milestone (M2, M3, M6), since each merges separately; `OversizedOnWire` variant added to data-model; FR-014 names daemon notes | agent | tests/schema_hash.rs; analyze I1/F4 |
| 7 | tasks | Fixture capture needs a logged-in claude (R11) | M1 unit escalates if none; claude 2.1.302 is installed in the container | agent | analyze F6 |
| 8 | M1 | Real CLI writes line types the model does not type (`active_goal`, `autocompact_state`, `rate_limit_event`, `system` status/post_turn_summary/task_summary) | T008 amended: no `Unreadable`, and no `Unrecognised` for a known type; the others are `Unrecognised` (FR-012), as SC-001 says "each known event type recognised" | agent | recorded fixtures, SC-001 |
| 9 | M1 | `structured_args` "default `None`" vs `AiCliProvider`'s rule "No method has a default" (feature 026 FR-021) | trait method without a default, implemented in all six impls | agent | provider.rs trait doc |
| 10 | M1 | Fixtures keep `\r\n` byte for byte under `* text=auto eol=lf` | `.gitattributes`: `crates/micold-core/tests/fixtures/stream-json/*.jsonl -text` | agent | tolerance.jsonl CRLF line |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | e63810803ba37fa21cbd87694781aa84aa9312bd:8ee4531564743d6032729cd056a73f655abfb1a0 | CHANGES: 3 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | fd5463849846bf112086f77d7913c42278133170:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN (1 MINOR, fixed) |
| Review plan | 1 | 45b0fc650d313338200dd10363a89e8a9d0c79d7:62098e2ac5367f62ce4e90774162b79389a152c5 | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Review plan | 2 | 2e543df9ccb26d8f478193d8cdf4e3ebcbb65767:5233609a4f479ef50bc8ee7f209f6969d2f950a2 | CLEAN (1 MINOR, fixed, prose only) |
| Review tasks | 1 | 04c1c7e294dde5831e98cdfcec29ca4f3d9203f8:5eed6c26bf89d047cf687661f1602dd39770ae1e | CHANGES: 2 MAJOR (T042 client call sites; stale checklist — fixed, spec gained a plain-language summary), 2 MINOR (F3 fixed; F4 already noted in T029) |
| Review tasks | 2 | f741a87ee478cc18590ea2227bdf989249aa490d:96c0440120fd62354a731049322b3adec98be5e1 | CLEAN (2 MINOR: T042 test list, line wrap — fixed, prose only) |
| M1 review A (code-review high) | 1 | c931c49619d8dbc13c1b4396c5944008406defed:bf97dc4a7be50771bcceb7ce878792ec43fc39b0 | 10 findings: 4 fixed (split-delta uuid leak in fixtures + script, lenient init `tools`, `null` tool input, single `raw` copy), 6 declined |
| M1 review B (conformance, sonnet) | 1 | b697707771467cbdcf3dc27783bf157d45976967:b37bc607eaeeb1f53cde3a4d02c4ac77a1b8903d | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Review tasks 1 | F4 T029 partly green before T030 | already documented in T029; only daemon-restart cases are red-first |
| M1 | A | `Result` without `is_error`/`subtype` becomes `Unrecognised` | US2 scenario 3 / FR-012: a known type missing a field the model expects is kept as unrecognised |
| M1 | A | `StoredSession.kind` should use a `Stored*` mirror | data-model.md and plan name `StoredSession.kind: SessionKind` with `skip_serializing_if = "SessionKind::is_terminal"`; a downgrade drops an unknown key either way |
| M1 | A | `Session.kind` is a `pub` field | every `Session` field is `pub`, `provider` included (same FR-001-style "fixed for life" rule); the constructors are the only way to choose it |
| M1 | A | `structured_args` should default to `None` on the trait | Decision 9: the trait's own rule "No method has a default" (feature 026 FR-021) |
| M1 | A | refusal sentence hardcodes "Claude Code" | the sentence is specified (T004, FR-003); one CLI has the mode |
| M1 | A | `LineSplitter` buffer has no cap | research R4 rejects a maximum line length (tool results run to several MiB, lines read whole) |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

