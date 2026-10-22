# Autopilot ledger — #942 structured-permission-prompts

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #942 "Structured session: permission prompts via permission-prompt-tool" (epic #945; depends on #939 transcript view and #941 composer/busy state, neither on main yet; #943 depends on this). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #942
- **Worktree branch**: claude/project-thread-tt60gq
- **Started**: 2026-10-21
- **Phase**: M6 verify
- **Next step**: full gate and review B of M6, then PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1197 | Design: spec, clarify, plan, research, contracts, tasks | merged (rebase) | 51e571f4603dfa1cb9091f7f82f32062b9ca6e8a |
| #1201 | M1: core request kinds, answers, rules, modes, control lines | merged (rebase; rebased onto main first, one conflict in `tests/support/stream_json.rs` with #944, both lists kept) | a689c4c4a89cd1ab9da5206afd4f0c913e94437d |
| #1204 | M2: daemon holds and decides prompt requests (switch off) | merged (rebase) | 0c33edd153cba2c02c9f41dfe584253b6a15056b |
| #1208 | M3: daemon waiting activity and withdrawal | merged (rebase) | 802f46ebcbd3379fb50758ac94fde18e757e11b2 |
| #1224 | M4: daemon mode persisted, created, launched | merged (rebase) | 4780eb3eaf8e292a0d9258ded2a646ea855a3756 |
| #1231 | M5: mode change while running, following Claude, plan approval sets the mode | merged (rebase; CI's one red job `sandbox_real_provision_own_image_without_ssh_fails_in_reach`, a Docker Hub 502 pulling alpine:3, passed on one re-run) | 1403972ac7d6e4cc845004c89b558d8253ab33c7 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T023 | full | Core: request kinds, answers, rules, modes, control lines and parse against recorded fixtures; capture script | #1201 | merged |
| M2 | T024–T034 | full | Daemon holds and decides prompt requests (switch off), wire Permission*, stand-in prompt | #1204 | merged |
| M3 | T035–T038 | full | Daemon: waiting activity and withdrawal | #1208 | merged |
| M4 | T039–T042 | full | Daemon: mode persisted, created, launched | #1224 | merged |
| M5 | T043–T048 | full | Daemon: mode change while running, following Claude, plan approval sets mode | #1231 | merged |
| M6 | T049–T055 | full | Client render-free prompt state and create-flow mode | — | in progress |
| M7 | T056–T063 | full | Window transcript side (ChoiceList, PromptCard, request/plan/question views); needs #939 M5–M6 on main (I-1) | — | pending |
| M8 | T064–T069 | full | Window composer side, switch on, user guide, Part B; needs #941 on main (I-2) | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 17 | M3 | How a session's stdout reader withdraws its requests on a `result`, `TurnEndedWithoutResult` or end of output, with no `Arc<DaemonState>` and a teardown that may join it under the state lock | The reader signals `permission::Endings` (its own small mutex: `TurnEnded` on a result or the cut-off note, `ProcessEnded` at end of output); each waiting prompt call watches its session's next end and runs `withdraw_session_prompts`. Stop and remove withdraw directly (`SessionStopped`, `SessionRemoved`), before the kill. Rules drop on every reason but `TurnEnded` and at each structured spawn | agent | `permission.rs` `Endings`, `serve_prompt`; `state.rs` `spawn_structured`; tests `a_request_is_withdrawn_when_*` |
| 16 | M2 | Always allow on a request that offers no rule (a `Bash` call without `command`) | Refused `WrongKind`, nothing changes; the request's `always_allow` is `None`, so a window never offers it | agent | data-model *AllowRule*; `permission.rs` `decide` |
| 15 | M2 | Is the prompt tool offered to a structured caller while the switch is off? | No: `ListScope::Structured` only when prompts are on, else the model could call the listed tool and raise a request nobody can answer (R14 "behaves exactly as feature 938") | agent | `DaemonState::prompt_scope`; test `a_terminal_sessions_credential_cannot_list_or_call_the_prompt_tool` |
| 14 | M2 | How the router hands over a prompt call (it needs `_meta`) | New `Route::Prompt { id, arguments, meta }`, only for a structured scope; a terminal caller's call stays `CallTool` and fails `unknown_tool` | agent | `core/src/mcp/jsonrpc.rs`; `mcp_jsonrpc.rs` |
| 1 | clarify r1 | Can a running session's mode change? | Any time, via stream-json control request, effective next tool request, no restart; Bypass keeps confirmation | decided by user | spec Clarifications; FR-017 |
| 2 | clarify r1 | Always-allow scope | This session only (`session` destination); no Claude settings file written | decided by user | spec Clarifications; FR-010 |
| 3 | clarify r2 | Mode change vs a pending request; while not running | Pending unaffected; saved for next launch | agent | spec FR-017, FR-003, FR-015 |
| 4 | clarify r2 | Always-allow rule with no CLI suggestions | Deferred to plan research (fixtures, FR-020) | agent | deferral |
| 5 | plan | Always-allow rule with no CLI suggestions (item 4) | Daemon holds session-scoped exact-input rules (Bash command, file path, WebFetch host, else tool name) and answers matches itself; dropped at process end | agent | research R9; probe P-11 (MCP path sends no suggestions, ignores updatedPermissions) |
| 6 | plan | Transport: MCP prompt tool vs stdio control channel | MCP prompt tool (issue acceptance); control channel only for set_permission_mode | agent | research R1; P-1, P-7 |
| 7 | plan | CLI tool-call timeout vs FR-003 | Structured binding gets per-server timeout 2147483647 ms | agent | research R3; P-5, P-12, P-13 |
| 8 | plan | Runtime switch to Bypass | Every structured launch adds --allow-dangerously-skip-permissions | agent | research R6; P-9, P-10 |
| 13 | M1 | Fixture layout for runs with several calls or launches | `<case>.json` keeps `call`/`answer`/`tool_result` for the first call and `later` for the rest; `mode-change.json` holds `runs` (flags, control request/response pairs); its stdout is two files, `prompt-mode-change.jsonl` (launched without the bypass flag) and `prompt-mode-change-bypass.jsonl` | agent | permission-prompt/README.md |
| 12 | M1 | The 2.1.306 recording honoured the always-allow `updatedPermissions` (P-11 said ignored) | Kept R9 (daemon keeps its own rule); since a CLI may honour the answer's rule, `AllowRule::cli_rule` names one only where it is no broader than the daemon's (Bash command without `*?[`, WebFetch `domain:<host>`, tool name), none for file paths or exact URLs; contract and research R9 updated | agent | fixture `always-allow` (1 prompt call for 2 identical Bash calls) vs a control run with plain allow (2 calls) |
| 11 | M1 | `ControlOutcome::Success(Value)` cannot cross the postcard wire (`Value` needs `deserialize_any`) | `Success(String)`: the `response` object's JSON text, as `ContentBlock::ToolUse.input` already is | agent | event.rs; structured_wire.rs round-trips it |
| 10 | tasks | M1 has 23 tasks (over rule 3's ~10) | Kept whole: its only split (Phase 2 vs Phase 3) leaves a Foundational-alone milestone (rule 1); 13 of its tasks are tests, fixtures or capture scripts, and the core code is small pure functions | agent | tasks review r1 F3 |
| 9 | tasks | Milestones by layer, not by story (rule 2 deviation) | Every story spans core/daemon/window and the window needs #939/#941; the plan's six layers kept, the two daemon steps each split in two along acceptance scenarios (US1 vs US5/US6; US2 sc1–3 vs sc5+US3) for the ~800-line budget → M1–M8 | agent | tasks.md *Milestones*; plan *Delivery* |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | e384ed34098b974a962950905d077d5e736d9aec:9ad0504870e12886d671ea3f4124162315386f78 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec review | 2 | 004bb5b1f949c41b2cca2730ff420b7af0b6099f:f13d3e811a532c30a69e9475c01265080385e3f8 | CLEAN: 3 MINOR (fixed, prose only) |
| Plan review | 1 | f2b7b32bfd45c20bf89137e325eab23124ae2413:bdd0ffab5e0b7317d1c13b816299b6c68b23a5bf | CLEAN: 1 MINOR (fixed, prose only) |
| Tasks review | 1 | 606e053a7f9c9c7632da9f594a373a2fbb5a824d:dffdf2b12eb552819450d6edf043665419ab018e | CHANGES: 2 MAJOR ([P] on shared files, fixed), 2 MINOR (F3 ledger decision 10; F4 checklist ticked) |
| Tasks review | 2 | 9798b34af9176881119e5e988c331a2bed97b955:18723871b14cc61aac3015421e5aea161167f6bb | CLEAN |
| Review A M1 (code-review high) | 1 | 166c2275321aa52bd9e60dbe67abaf1a1e4d7a61:a2a02be619ce58df6f96999e95bc220a90d9fea4 | CHANGES: 1 MAJOR (Bash rule ignored dangerouslyDisableSandbox/run_in_background — fixed: shell rules match every non-cosmetic field), 3 MINOR (PowerShell/Task AnyInput — PowerShell now a shell rule, rest declined; PROTOCOL_VERSION 58 — fixed; preview clone — fixed) |
| Review B M1 (conformance, sonnet) | 1 | 166c2275321aa52bd9e60dbe67abaf1a1e4d7a61:a2a02be619ce58df6f96999e95bc220a90d9fea4 | CLEAN but incomplete (its `claude -p` sandbox refused every command, Verify not run): 2 MINOR (T005 text — fixed; no tdd/cycle-log.md — added with stub-based red evidence) |
| Review A M1 (sonnet) | 2 | eaf8fcf201e09294a5e358f8e9a654abbbbe907f:23f1ca80d17fd747754fd5b488d3730d0f06197a | CLEAN |
| Review B M1 (conformance, sonnet) | 2 (full; r1 ran no command) | eaf8fcf201e09294a5e358f8e9a654abbbbe907f:23f1ca80d17fd747754fd5b488d3730d0f06197a | CLEAN; Verify `INTEG_EXIT=0` (37, 11, 9, 39, 5 passed), `LIB_EXIT=0` (52), capture test 6/0; 1 MINOR (cycle log red taken afterwards on stubs — follow-up) |
| Review A M2 (code-review high) | 1 | 09a697c663e494eb41ad33a7fad9e966d9108097:69239a2b67091c928e63a2cdb59f70e7d3154e19 | CLEAN: 3 MINOR (F1 doc comment displaced — fixed; F2 rules never dropped — declined, M3; F3 entry left pending if the wait is dropped — fixed, drop guard) |
| Review B M2 (conformance, sonnet) | 1 | 09a697c663e494eb41ad33a7fad9e966d9108097:69239a2b67091c928e63a2cdb59f70e7d3154e19 | CLEAN; Verify `DAEMON_EXIT=0` (13, 14 passed), `CORE_EXIT=0` (8, 15, 54); 2 MINOR (F1 hang-up withdrawal untested — test added, cycle 4; F2 = A's F1, fixed) |
| Review A M3 (code-review high) | 1 | edf8b82347414b400af69e760bf7e3b7553198b0:275254ee26bcb14aaa85d5153e121405c7ada690 | CLEAN: 3 MINOR (F1 activity applied off the registry lock may land out of order — declined; F2 a late waiter withdraws a next turn's requests — declined; F3 a hang-up applies no activity — declined, per R8) |
| Review B M3 (conformance, sonnet) | 1 | edf8b82347414b400af69e760bf7e3b7553198b0:275254ee26bcb14aaa85d5153e121405c7ada690 | CLEAN; Verify 23 passed, 0 failed; 1 MINOR (F1 ledger 17 "before the kill" — declined) |
| Review A M4 (code-review high) | 1 | f8700e0a018d6a318ee5f4a40732448129c58b9c:3196a9e7f9874a86042c6a3a1ed57e42270b8ef0 | CLEAN: 1 MINOR (F1 `stop_session` called on the async test thread — declined). Its first two dispatches had every Bash and scratchpad Read refused and returned an empty CLEAN; the counted round had the diff inline |
| Review B M4 (conformance, sonnet) | 1 | f8700e0a018d6a318ee5f4a40732448129c58b9c:3196a9e7f9874a86042c6a3a1ed57e42270b8ef0 | CLEAN: 2 MINOR (F1 cycle log uncommitted — fixed, committed; F2 Verify not run, its `cargo test` refused — the unit ran Verify: 5 passed; 8 and 39 passed) |
| Review A M5 (code-review high) | 1 | f87c133e430c27b98cb5cc16d987e545768d27ea:4780eb3eaf8e292a0d9258ded2a646ea855a3756 | CLEAN: 3 MINOR (F1 a stale `status` report applied after a change — comment, declined; F2 plan resolution broadcast after the mode change — comment, declined; F3 stand-in formats JSON by hand — declined). `claude -p` with the brief and diff inline, 0 permission denials |
| Review B M5 (conformance, sonnet) | 1 | f87c133e430c27b98cb5cc16d987e545768d27ea:4780eb3eaf8e292a0d9258ded2a646ea855a3756 | CLEAN: 1 MINOR (F1 T048 text still said `mode_change` — fixed, prose only). Verify output given inline (13 and 29 passed), 0 permission denials |
| Review A M6 (code-review high) | 1 | f57630e6ecbab071c1f308ae038e7bea8ae2dbae:80b87f4423a2d01f7ebc2f4c90c51f1b52183f67 | CLEAN: 3 MINOR (F1 a refused Show all cleared an answer's `sending` — fixed, `input_failed`; F2 the Bypass gate runs for an automatic start — declined; F3 no shell test of the start gate — added `a_bypass_start_is_created_only_by_its_confirmation`). `claude -p` with the brief and diff inline, 0 permission denials |

## Declined review findings

| Phase | Review | Finding | Why declined |
|---|---|---|---|
| tasks | speckit-analyze | C1 MEDIUM: sandboxed-session edge case has no task | a sandboxed session runs the daemon and its CLI in the same container (feature 027), so the prompt call is the same loopback tool-server call with the same credential that T029 covers; reachability in the container is already the sandbox suite's (`sandbox_real_mcp.rs`) |
| tasks | speckit-analyze | C4 LOW: no real `SessionInterrupt` test | #941's; plan I-4: an interrupt ends the turn with a `result`, which T037 covers |

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | Review A r1 | F2 MINOR (part): `Task`/`Agent` and other unnamed tools get an any-input rule | data-model *AllowRule* fixes "any other: the same tool name" (MCP tools are the case it serves); only `PowerShell`, a shell like `Bash`, was moved to the command rule |
| M2 | Review A r1 | F2 MINOR: a session's always-allow rules are never dropped | T038 (M3) drops them at process end, beside the withdrawals; nothing in M2 removes a session's process state yet |
| M3 | Review A r1 | F1 MINOR: the Notification/PreToolUse activity runs after the registry lock is released, so a decision racing the first request's open can apply first | The window is the few instructions between `drop(inner)` and `note_activity`'s own lock, and the next hook or turn end corrects it; a full fix needs the activity FSM under the registry's lock step, more than a few minutes |
| M3 | Review A r1 | F2 MINOR: a waiter woken late by a turn end could withdraw a next turn's requests | The next turn's request needs a user message and a new tool call after the `result`, while the waiter's wake-up is immediate on the runtime; turn generations would add state for a window no run can hit in practice |
| M3 | Review A r1 | F3 MINOR: an agent hang-up withdraws with no activity change | R8 and T036: "a withdrawal applies nothing" — the turn's own events decide the activity |
| M3 | Review B r1 | F1 MINOR: ledger 17 says stop withdraws before the kill, the code after | `stop_session_gated` only takes the live entry out of the map under the lock; `end_processes`, which kills, runs after `withdraw_session_prompts` |
| M4 | Review A r1 | F1 MINOR: `each_mode_on_create_is_listed_launched_and_kept_across_a_restart` calls `stop_session` on the async test thread | Taste: `structured_resume.rs` does the same in its `multi_thread` tests, and the stand-in exits at once on stdin close |
| M5 | Review A r1 | F1 MINOR: a `status` report the CLI wrote just before a change's `success` can be applied after the change and revert it | The CLI writes `status` with the new mode right after its `success` (P-9), which restores it; ordering reports against control answers would need sequence numbers across the feed. Commented in `spawn_structured` |
| M5 | Review A r1 | F2 MINOR: an approved plan's `PermissionResolved` waits for the mode change (≤ 10 s) | Deliberate: the broadcast names the mode the session has after the change (`Resolution::PlanApproved { mode }`); another client's decision meanwhile is refused as no longer pending, as for any request decided first. Commented in `decide_prompt` |
| M5 | Review A r1 | F3 MINOR: the stand-in formats `control_response` JSON by hand | The stand-in is compiled by plain `rustc` with no crates (no serde); the tests pass it only plain text |
| M6 | Review A r1 | F2 MINOR: the Bypass gate runs for an automatic start, which would pop the confirmation uninvited | The auto-start always asks for a terminal session (`StartRequested` doc, feature 938 FR-022), whose gate is `Create(Default)`; for a structured one, an unconfirmed Bypass create is what FR-014 forbids, so the confirmation is the right answer |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M2's first final gate failed once in `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` (`left: []`, no notice; untouched by #942); 5 of 5 isolated runs and the rerun gate passed. A load flake to watch. M4's final gate hit it once more (5 of 5 isolated runs and the rerun passed).

- Local builds in this cloud container fill the disk with the default `line-tables-only` debug info (the first M1 gate died at `ld` Bus error, 0 bytes free); M1's green gate ran with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` after `rm -rf target-shared/debug`.

- `claude -p --agent autopilot-reviewer` in this container (workspace not trusted, so the project's
  `permissions.allow` is ignored) refuses every Bash call and every Read outside tracked files,
  and still answers `VERDICT: CLEAN`. Check `permission_denials` in `--output-format json`; M4's
  review A got a real round only with the brief and diffs inline in its prompt.
