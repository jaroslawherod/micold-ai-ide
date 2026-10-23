# Autopilot ledger — #942 structured-permission-prompts

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #942 "Structured session: permission prompts via permission-prompt-tool" (epic #945; depends on #939 transcript view and #941 composer/busy state, neither on main yet; #943 depends on this). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #942
- **Worktree branch**: claude/project-thread-tt60gq
- **Started**: 2026-10-21
- **Phase**: M8 implement
- **Next step**: M8 (split: T064, T065, T067, T068, T069): implement, gate, review, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1197 | Design: spec, clarify, plan, research, contracts, tasks | merged (rebase) | 51e571f4603dfa1cb9091f7f82f32062b9ca6e8a |
| #1201 | M1: core request kinds, answers, rules, modes, control lines | merged (rebase; rebased onto main first, one conflict in `tests/support/stream_json.rs` with #944, both lists kept) | a689c4c4a89cd1ab9da5206afd4f0c913e94437d |
| #1204 | M2: daemon holds and decides prompt requests (switch off) | merged (rebase) | 0c33edd153cba2c02c9f41dfe584253b6a15056b |
| #1208 | M3: daemon waiting activity and withdrawal | merged (rebase) | 802f46ebcbd3379fb50758ac94fde18e757e11b2 |
| #1224 | M4: daemon mode persisted, created, launched | merged (rebase) | 4780eb3eaf8e292a0d9258ded2a646ea855a3756 |
| #1231 | M5: mode change while running, following Claude, plan approval sets the mode | merged (rebase; CI's one red job `sandbox_real_provision_own_image_without_ssh_fails_in_reach`, a Docker Hub 502 pulling alpine:3, passed on one re-run) | 1403972ac7d6e4cc845004c89b558d8253ab33c7 |
| #1263 | M6: client render-free prompt state and create-flow mode | merged (rebase; after CI went green main moved and the PR conflicted in `crates/micold-client/src/app.rs` `forget_structured` with #944 structured_insights; the orchestrator rebased keeping both forgets, the full gate passed again, then merged) | b7f1b704e184ad5b1e0c905f4fb3d6e35569839d |
| #1266 | M7: requests, plans and questions in the transcript | merged (rebase) | b636391e9e89f14bd664f2f7a867048c1ad1b89d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T023 | full | Core: request kinds, answers, rules, modes, control lines and parse against recorded fixtures; capture script | #1201 | merged |
| M2 | T024–T034 | full | Daemon holds and decides prompt requests (switch off), wire Permission*, stand-in prompt | #1204 | merged |
| M3 | T035–T038 | full | Daemon: waiting activity and withdrawal | #1208 | merged |
| M4 | T039–T042 | full | Daemon: mode persisted, created, launched | #1224 | merged |
| M5 | T043–T048 | full | Daemon: mode change while running, following Claude, plan approval sets mode | #1231 | merged |
| M6 | T049–T055 | full | Client render-free prompt state and create-flow mode | #1263 | merged |
| M7 | T056–T063 | full | Window transcript side (ChoiceList, PromptCard, request/plan/question views); needs #939 M5–M6 on main (I-1, met: #1203, #1206) | #1266 | merged |
| M8 | T064, T065, T067, T068, T069 | full | Create-flow mode rows, Bypass confirmation and session-bar label, prompts switched on, user guide, Part B without the composer (split, decision 18) | — | in progress |
| M9 | T071, T072, T066, T073 | full | Composer side: mode Select and *Waiting for you* in #941's composer; needs #941 M6 on main (I-2) | — | pending (held on I-2) |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 18 | M8 | #941's composer (PR #1255) is not on main and #943 waits on prompts being on (its SC-007): wait for #941 or split M8? | Split (orchestrator): M8 builds what needs no composer (T067 switch on, the create-flow mode choice, `ui/confirm_bypass.rs`, the Bypass label, T068, T069 for what it draws); the composer's mode Select (FR-015 shown in the view, FR-017), T066 *Waiting for you* and their quickstart steps move to a new M9 (T071–T073, T066), held on #941 M6 (I-2) | orchestrator | tasks.md *Phase 10b*, *M9* |
| 19 | M8 | The create flow's mode `Select` inside the start list, a `MenuOverlay` of items | Four **Permissions: <mode>** rows under the offered structured entry, the chosen one marked with the switcher's active marker; a row press sends `StartModeChosen` and leaves the list open; Bypass is confirmed when the structured entry is pressed (M6's `start_gate`). A `Select`'s own dropdown would be a menu inside the menu | agent | `ui/mod.rs` `start_mode_items`; `menu_anatomy.rs` `start_list_items` |
| 20 | M8 | Where the *Bypass: Claude runs every tool without asking* label shows without #941's composer | First row of the session bar at the top of the transcript (feature 944), warning icon and words in the error role, while the session's summary mode is Bypass; the Bypass confirmation for a mode change (`ConfirmBypassModeDialog`) is registered now so M9 only adds the Select | agent | `ui/transcript/session_bar.rs` `mode_label`; showcase *the session bar of a session in Bypass* |
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
| Review A M8 (code-review high) | 1 | ee0e359b8647849961a2f14424cf9c9eab0298f1:2ecf9e9eaa23625479047f65c2f6a7f2625b7c2b | CHANGES: 8 findings; F1 (a `PromptId` is unique per daemon only, so the focus release could miss) MAJOR fixed (`FieldId::Prompt*` carry the session); F2 mode confirmation opens without `clear_for_dialog` fixed (shell clears before it opens); F3 release lives in root `app.rs` moved to `features/window.rs` (manual call sites kept, declined part); F4 mode looked up in the active project only fixed (`workspace.find_session`); F5 stale R14 docs fixed; F6 a Bypass pick stuck for later starts fixed (reset to Default after a confirmed Bypass start); F7 doc comment fixed; F8 declined |
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
| Review B M6 (conformance, sonnet) | 1 | a5abcb82e42d6a8e874f7972bd53b5f53eb5886e:b0449c8a6fcee16f488b7ee9e86ee9c8eafb9157 | CHANGES: 2 MAJOR (F1 the additional-session confirmation created in the create flow's current mode, so a Bypass chosen after a gated start skipped its confirmation — fixed, the mode travels in `PendingOp::CreateSession` and `AdditionalOpened`, red shown; F2 two parts of cycle 9 had no red — recorded as an explicit exception in the cycle log, the restore of main's sources being refused here), 1 MINOR (F3 route tests outside the Verify command — fixed, prose only: T053 and Verify name `--bin micold-ai-ide permission_prompts`). Verify given inline, 0 permission denials |
| Review B M6 (conformance, sonnet) | 2 | efcb81b437004bf9df6e6fa0c6d1d48cd93c8857:afca60ee94cc3e8d5cdec06e07fe2d026838ff97 | CLEAN (fix diff; Verify re-run inline: shell routes 9, `structured_start_menu` 12, `features_permission_prompts` 22 passed), 0 permission denials |
| Review A M7 (code-review high) | 1 | 257c2b6c54f8b83d657915841ee59e69982a4940:41a267c065996875862907ef31fd18f54bd4722d | CLEAN: 3 MINOR (F1 structured tool input shown twice; F2 the entry row changes shape when a request appears; F3 question rows pressable while sending — all declined). `claude -p` with the brief and diff inline, 0 permission denials. Before it: rebased onto main 79db5768, resolving `ui/transcript/mod.rs` and `ui/terminal.rs` against #944's `view_with_insights` (new `view_full`: session bar + insights + prompts; Insights messages to `SessionMsg::Insights`) |
| Scoped gate M7 | 1–3 | adcfbcfc | red twice: `field_focus_call_sites` (the deny-message and plan-note fields did not track focus — fixed: `FieldId::PromptDenyMessage`/`PromptNote`, `focus::TrackFocusAs<M>`, `focused`/`on_focus` on `PromptsView`) and `root_vocabulary_is_cross_cutting` (`PermissionPrompts` is now emitted — its `NO_OWNER` entry removed); third run green (fmt, clippy -D warnings, 241 test binaries; whole `layout_snapshot` 86 passed) |
| Review A M7 (scoped, sonnet) | 2 | eb1bba267c813a3566418f65dec5286d903479a7:adcfbcfcf041461d22670f5dd38852b27019ac5a | CLEAN on the gate-fix diff: 1 MINOR (F1 a focused request field is not cleared when its request ends — declined, M8). Not counted (red gate's fixes) |
| Review B M7 (conformance, sonnet) | 1 | eb1bba267c813a3566418f65dec5286d903479a7:adcfbcfcf041461d22670f5dd38852b27019ac5a | CLEAN: 2 MINOR (F1 tool input twice — same as A F1, declined; F2 cycle 11's unit-test exception — accepted as logged). Verify given inline: `showcase_completeness` 46, `material_builder_api` 13, `layout_snapshot` 86, `transcript_permission_view` 12 passed; 0 permission denials |
| Visual pass M7 | 1 | adcfbcfc | PASS light and dark, offscreen (tiny-skia, after `layout::settle`; the worker's `claude -p` had every build refused, and Xvfb here has no xdotool): the three `PromptCard` poses keep their action rows inside the card (Allow/Deny/Always allow + rule note; Approve and accept edits/Approve/Keep planning; Submit disabled while a question is unanswered/Decline), `ChoiceList` rings with a dot when chosen, checkbox glyphs ticked when chosen, Other row with its field; no overlap. MINOR noted: the checkbox glyph reads smaller than the radio ring. Evidence `visual-pass/m7-*.png`. A first capture without a redraw drew every button disabled — a harness artefact |

## Declined review findings

| Phase | Review | Finding | Why declined |
|---|---|---|---|
| M8 | Review A r1 | F3 (part) MINOR: the focus release must be called by hand after each path that ends a request | Every path that removes a pending request is one of `resolved`, `connection_lost`, `forget`, and each now calls it; reading the field as unfocused when its request is gone would touch every key-routing read of `focused_field` for one feature |
| M8 | Review A r1 | F8 MINOR: `start_dialog`/`mode_dialog` are near copies, and `EFFECT` repeats the label's words | Two six-line lookups over different state with different messages share `modal` already; the dialog's sentence and the bar's label are separate texts the spec gives separately (FR-014 confirmation, US2 scenario 4 label) |
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
| M7 | Review A r1 / B r1 | F1 MINOR: a tool request with no held diff shows its input as the summary line and again as pretty JSON | Deliberate (FR-006): the summary is the one-line command the user decides on, the JSON what the tool receives in full; for `Bash` they differ (description, timeout) |
| M7 | Review A r1 | F2 MINOR: the entry row changes shape when a request appears under its card | Unconfirmed by the reviewer; the card's fold state lives in `State::is_open`, not in the widget tree, so at most transient widget state (a text selection) could reset; revisit if M8's live pass shows it |
| M7 | Review A r1 | F3 MINOR: question rows stay pressable while an answer is sending | The reducer keeps the draft and `send` refuses while `sending`; edits made then are not sent. M8's live pass decides whether rows should grey out |
| M7 | Review A r2 | F1 MINOR: a focused deny/note field keeps `focused_field` after its request ends, so terminals do not take keys until a click | Unreachable while the switch is off; M8 turns prompts on and owns the keyboard side (composer, T064–T066) — listed in *Follow-ups* for it |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M6's first final gate failed once in `tests::pr_status::pr_list_read_a_second_ask_shares_the_read_under_way` (`the work an update returned has to finish: Elapsed`, the 10 s bound of `main_tests::messages`; untouched by #942). Isolated it passes 5 of 5 but takes 10.05 s each run, right at that bound: a load flake to watch.

- M2's first final gate failed once in `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` (`left: []`, no notice; untouched by #942); 5 of 5 isolated runs and the rerun gate passed. A load flake to watch. M4's final gate hit it once more (5 of 5 isolated runs and the rerun passed).

- Local builds in this cloud container fill the disk with the default `line-tables-only` debug info (the first M1 gate died at `ld` Bus error, 0 bytes free); M1's green gate ran with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` after `rm -rf target-shared/debug`.

- `claude -p --agent autopilot-reviewer` in this container (workspace not trusted, so the project's
  `permissions.allow` is ignored) refuses every Bash call and every Read outside tracked files,
  and still answers `VERDICT: CLEAN`. Check `permission_denials` in `--output-format json`; M4's
  review A got a real round only with the brief and diffs inline in its prompt.

- ~~For M8 (switch on): when a request ends while its deny-message or plan-note field holds the keyboard, clear `window.focused_field` (M7 review A r2 F1).~~ Done in M8: `State::release_ended_prompt_focus`.
