# Autopilot ledger — #938 structured-session-mode

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 938 — issue #938 "Structured session mode: run claude with stream-json next to the PTY mode"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #938
- **Worktree branch**: claude/project-thread-cl0lw3
- **Started**: 2026-10-18
- **Phase**: M4
- **Next step**: M4 PR to open (title/body in the session scratchpad), then CI and merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1035 | design PR (spec, plan, research, contracts, tasks) | merged (rebase) | 0d6ced4e44dcf63df729fc15bdd8765cdf045a8d |
| #1044 | M1 parser, fixtures, kind persisted | merged (rebase) | 613faba7880da73fd7e6228cda6c8af60d7ea85c |
| #1056 | M2 daemon runs a structured session, streams events | merged (rebase) | 4793cbab05478fe50b1434ab61a1c99c717701e5 |
| #1064 | M3 first prompt and user messages | merged (rebase) | 1fab9ea568ba3d8e00c60cb14c16f3277120fecf |
| #1070 | design revision: event history saved under Save terminal history (M7) | merged (rebase) | bd88174731b916d4fe47de2f16f4865dc197a3eb |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | parser types every recorded fixture line; kind persisted | #1044 | merged |
| M2 | T011–T021 | full | daemon runs a structured session and streams events; lifecycle parity | #1056 | merged |
| M3 | T022–T028 | full | first prompt and user messages | #1064 | merged |
| M4 | T029–T030 | full | resume | — | in progress |
| M5 | T031–T032 | full | late and multiple viewers | — | planned |
| M6 | T033–T036 | full | terminal-only requests refused | — | planned |
| M7 | T049–T055 | full | event history saved under Save terminal history, restored after a daemon restart | — | planned |
| M8 | T037–T047, T056 | full | chevron entry, row marker, log pane, user guide, setting note | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Permission behaviour of a structured session before #942 | Same permission configuration as a terminal session, no override; approvals the CLI would need are refused and show as events (FR-006a) | agent | crates/micold-core/src/provider.rs ToolServerSupport; spec FR-006 |
| 2 | clarify | How the structured kind is chosen; pane before #939 | Per-session, start-session chevron, Claude only; plain read-only line-per-event log (FR-022, FR-022a) | user (confirmed, final) | worktrees-and-sessions.md chevron |
| 3 | clarify | Does event history survive a daemon restart | Yes, saved under "Save terminal history" like terminal sessions, restored after a daemon restart; off = memory only (FR-021a–c, US7, SC-007; research R14; new milestone M7, UI milestone renumbered M8) | user (final; jaro tapped "Save to disk", replacing the provisional "daemon memory only") | clarify card |
| 4 | clarify | Where the structured choice is offered | "Claude Code (structured)" entry in the start-session chevron, chevron shown even with one CLI; other entry points create terminal sessions | agent | worktrees-and-sessions.md#choosing-which-ai-cli-a-session-runs |
| 5 | tasks | Milestone cut | M1 = Setup + Foundational + US2 parser (P1); US1 split along scenarios 1–3 / 4–5 (M2, M3), as US1 with the foundation exceeds ~10 tasks; US5 lifecycle tests moved into M2 where the Proc enum is built; added US6 (P3) to spec for FR-022/022a/023 acceptance scenarios; M2 at 11 and M7 at 11 tasks kept whole (no smaller observable split) | agent | speckit-analyze C1, milestones.md rules 1–3 |
| 6 | tasks | Plan fixes from speckit-analyze | `start_new_of_kind`/`restored_of_kind` delegation instead of changing 300 callers (I1); PROTOCOL_VERSION bumped per wire-changing milestone (M2, M3, M6), since each merges separately; `OversizedOnWire` variant added to data-model; FR-014 names daemon notes | agent | tests/schema_hash.rs; analyze I1/F4 |
| 7 | tasks | Fixture capture needs a logged-in claude (R11) | M1 unit escalates if none; claude 2.1.302 is installed in the container | agent | analyze F6 |
| 8 | M1 | Real CLI writes line types the model does not type (`active_goal`, `autocompact_state`, `rate_limit_event`, `system` status/post_turn_summary/task_summary) | T008 amended: no `Unreadable`, and no `Unrecognised` for a known type; the others are `Unrecognised` (FR-012), as SC-001 says "each known event type recognised" | agent | recorded fixtures, SC-001 |
| 9 | M1 | `structured_args` "default `None`" vs `AiCliProvider`'s rule "No method has a default" (feature 026 FR-021) | trait method without a default, implemented in all six impls | agent | provider.rs trait doc |
| 10 | M1 | Fixtures keep `\r\n` byte for byte under `* text=auto eol=lf` | `.gitattributes`: `crates/micold-core/tests/fixtures/stream-json/*.jsonl -text` | agent | tolerance.jsonl CRLF line |
| 11 | M4 | T030 predicted a code change (restore drops the kind) | None needed: M1's `StoredSession::into_session` already restores the kind and M2's start path shares the terminal `LaunchSpec`; T029's tests are test-after, each pinned by a deliberate mutant (cycle-log 9) | agent | store.rs into_session; tests/structured_resume.rs |

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
| M2 review A (code-review high) | 1 | 6ff1da44088f251211d8c2ac4981f0dbf7e7529d:3f735466dce7f7ee16e1f6f946425617725d6029 | 10 findings: 3 fixed (F1 structured teardown under the state lock, F5 child left running when a reader fails to start, F6 reader counted done on unwind), 7 declined |
| M2 review B (conformance, sonnet) | 1 | 6ff1da44088f251211d8c2ac4981f0dbf7e7529d:3f735466dce7f7ee16e1f6f946425617725d6029 | CHANGES: 1 MAJOR declined (T032), 2 MINOR (F3 test-list fixed; F2 declined, T032) |
| M2 review A (code-review high, sonnet) | 2 | 826ed0589dfaa8c7d6ce31d4a56ced5687973387:93b677536b14171260bd556c4c255b3ea4aee2cc | CLEAN (5 MINOR, not fixed: worktree retry count ignores structured kills, `end_removed` on async callers, doc wording, detached reader on spawn failure, test-wide lint allow) |
| M3 review A (code-review high) | 1 | a88fe9259fe8470c367121d7f982506069cbe4be:2c9fe052b0e42a603aa745424724d5a5fdbe8410 | 9 findings: 2 fixed (F1 per-message spawn_blocking reordered turns — accept now on the connection loop, write off it; F2 turn guard held across `on_event` at EOF), 7 declined |
| M3 review A (code-review high, sonnet) | 2 | 83111196a2ccba7189d864995a4bc4576a907f2c:25ea31c8f8fcdbd6212cf65e8452d43a890b4b71 | CLEAN (3 MINOR, not fixed: `running` read before the turn lock at EOF, refusal answered via spawn_blocking, no dedicated order test — the refusal test covers it) |
| M3 review B (conformance, sonnet) | 1 | 42359e030cca65c2395966e97e553098c6ef5fa9:8b37008a71fb40230b6fe64505177caaf1cde800 | CLEAN (2 MINOR: T024 red was a compile error, right reason shown by mutation; Verify read from the full gate log) |
| Revision review spec | 1 | e0ff3114af0e20d59a968874d970b56ded98c2fa:1b32faee25fb96fda33a6c60adb7b7a6f022e6f6 | CHANGES: 2 MAJOR (F1 scenarios for FR-021b startup cleanup and FR-023 note; F2 concurrency edge cases — fixed), 2 MINOR (F3 Q-numbering fixed; F4 declined) |
| Revision review plan | 1 | e0ff3114af0e20d59a968874d970b56ded98c2fa:1b32faee25fb96fda33a6c60adb7b7a6f022e6f6 | CLEAN (1 MINOR declined) |
| Revision review tasks | 1 | e0ff3114af0e20d59a968874d970b56ded98c2fa:1b32faee25fb96fda33a6c60adb7b7a6f022e6f6 | CLEAN (3 MINOR fixed: T056 test first, T054 startup sweep/race/saver-pass cases, ID-gap note) |
| M4 review A (code-review high) | 1 | 6a0863fabf5c27b5ea526eb41d05e07bf2d31892:6574ac82466bbcc1f1f37374b317511a52d9889a | 8 findings: 1 MAJOR fixed (F1 first-prompt-once check rested on a sleep — now ordered by a later user message, mutant-checked), 2 MINOR fixed (F4 seqs across the resume, F6 T030 text amended), 1 moot (F7 Handover reset), 4 declined |
| M4 review A (code-review high, sonnet) | 2 | 98bcb395bfd2300f09a61eb563ade201255d60af:cea110aaea3aabc4c2f93cc0e196486797906f49 | CLEAN |
| M4 review B (conformance, sonnet) | 1 | 6a0863fabf5c27b5ea526eb41d05e07bf2d31892:6574ac82466bbcc1f1f37374b317511a52d9889a | CLEAN |
| Revision review spec (sonnet) | 2 | 7497d055378edd3e6c97a8eea28e417fb1c95801:1b32faee25fb96fda33a6c60adb7b7a6f022e6f6 | CLEAN (1 MINOR fixed: FR-021b and scenario 7 cover purge at start with saving off) |

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
| M2 | A F2 / B F1 | `event_streams` entry never dropped on delete | T032 (M5): "drop the history map entry on session delete" |
| M2 | A F4 / B F2 | snapshot cloned under the history lock; `Lagged` skips events | T032 (M5): chunked snapshots and re-snapshot on `RecvError::Lagged` |
| M2 | A F3 | history's 64 MiB bound counts `raw` only | research R5 bounds "retained line text" |
| M2 | A F7 | Windows `claude.cmd` not resolved by `Command` | recorded under *Follow-ups not done* (not this milestone) |
| M2 | A F8 | crash restart of a structured session uses `--resume` | same as a terminal session's restart (FR-009 parity); resume is M4 |
| M2 | A F9, F10 | old readers may push after a timed-out teardown; `ReadersDone` could be a channel | MINOR; the teardown shape copies `PtySession`'s |
| M3 | A 1 | F3 unbounded stdin write holds the turn | a stalled CLI is ended by teardown, which kills first and breaks the pipe; the reset to `Idle` only follows a failed write, when nothing was delivered |
| M3 | A 1 | F4 unknown session gets NotRunning | the refusal set is fixed by the contract (FR-008); MINOR |
| M3 | A 1 | F5 blank first prompt refused for structured | FR-008 refuses blank messages and the refusal reaches the caller; runs.rs already rejects a blank run prompt upstream |
| M3 | A 1 | F6 mixed poison idioms | taste; MINOR |
| M3 | A 1 | F7 `unwrap_or_default` in encode | serializing `&str` fields cannot fail; MINOR |
| M3 | A 1 | F8 sleeps in negative-delivery tests | an absence check needs a time bound; margins match the M2 tests; MINOR |
| M3 | A 1 | F9 user text in a `Debug` ClientMsg | `SessionInput` carries typed terminal bytes the same way; the daemon never logs a `ClientMsg` with `?` |
| M4 | A 1 | F2 `CLAUDE_CONFIG_DIR` temp dir never removed | a static is never dropped, so a `TempDir` there leaks the same; one small directory per test binary run, keyed by pid; MINOR |
| M4 | A 1 | F3 `started_times(dir, 2)` relies on `clear_records` keeping `starts-record` | that is `clear_records`' documented contract; the case waits for the first run's start before the stop, so the count cannot come from the wrong process; MINOR |
| M4 | A 1 | F5 gone-conversation case compares to the terminal result and calls `start_session` directly | US3 scenario 4 is parity with a terminal session; the terminal side is pinned by the message and the mutant shows the structured side differs when gated; MINOR |
| M4 | A 1 | F8 stand-ins stopped only at each test's end | a failing case ends its test process's runtime and the daemon's process groups with it, as in the M2/M3 structured tests; MINOR |
| Revision | spec 1 | F4 User Story 7 before User Story 6 | US6's number is referenced by merged tasks, plan and PRs; renumbering churns merged artifacts; tasks.md notes the order |
| Revision | plan 1 | F1 `format.rs` split has no task | T052 already names `terminal_history/format.rs` and the pinned fixture test |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Windows (after M2): `std::process::Command` in `structured.rs` does not resolve `claude.cmd` on
  `PATH` via `PATHEXT` as the PTY spawn's `CommandBuilder` does; a structured start on Windows
  needs the same resolution.
