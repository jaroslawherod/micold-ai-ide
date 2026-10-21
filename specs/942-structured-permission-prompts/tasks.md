# Tasks: Structured Session Permission Prompts — approve, deny, plan approval and questions

**Input**: Design documents from `specs/942-structured-permission-prompts/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Mandatory (Constitution I). In every phase a test task comes before the implementation
task it covers and must fail first. Draw code is the GUI exception, validated by quickstart Part B.

**Documentation**: `docs/daemon.md` (prompt tool, wire) in Phase 4 (M2); the user guide (FR-021)
in Phase 10 (M8), the phase that makes the feature user-visible.

**Organization**: every user story spans the core, the daemon and the window, and the window side
needs work not on `main` (#939's transcript view, I-1; #941's composer, I-2). Phases therefore
follow the layers of plan.md *Delivery*, one phase per milestone, and each task carries the story
it serves. Phase 2 holds the core types every story needs.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US6) the task serves; Setup and Foundational tasks have none

Paths: `core` = `crates/micold-core`, `daemon` = `crates/micold-daemon`,
`client` = `crates/micold-client`.

---

## Phase 1: Setup — recorded fixtures (FR-020, research R12)

- [x] T001 [P] Write `scripts/tests/capture-permission-prompt.test.sh`: feeds a sample prompt
  fixture holding a scratch path, a home path and a session uuid through the capture script's
  scrub step and asserts `<CWD>`, `<HOME>`, `<SESSION_ID>` replace them and the JSON stays valid;
  it fails until T003 exists
- [x] T002 [P] Write the stub MCP server `scripts/capture/prompt-stub.py`: loopback HTTP JSON-RPC
  serving `initialize`, `tools/list` (the `permission_prompt` tool of
  [contracts/claude-permission.md](contracts/claude-permission.md)) and `tools/call`; logs each
  call's `arguments` and `_meta` and answers from a scripted list of answer JSONs; stdlib only
- [x] T003 Write `scripts/capture-permission-prompt.sh <case>`: runs the real `claude` with feature
  938's structured argv plus `--permission-prompt-tool mcp__micold__permission_prompt` and a
  binding with `"timeout": 2147483647` pointing at the stub; per case writes
  `core/tests/fixtures/permission-prompt/<case>.json` (`call`: the recorded arguments and `_meta`,
  `answer`: what the stub returned, `tool_result`: the CLI's resulting tool-result text, or the
  control lines for `mode-change`) and the run's stdout as
  `core/tests/fixtures/stream-json/prompt-<case>.jsonl`; **sources** `scrub_stream_json` from
  `scripts/capture-stream-json.sh` (never copies it); T001 passes
- [x] T004 Record the five cases with T003 against a logged-in CLI: `tool-request` (Bash, allow),
  `always-allow` (Bash, always-allow answer, the identical second call asked again — R9),
  `plan-approval` (`--permission-mode plan`, `ExitPlanMode` approved, `status` line to `default`),
  `question-multi` (`AskUserQuestion`, two questions, one `multiSelect`), `mode-change`
  (`set_permission_mode` `acceptEdits` success and `bypassPermissions` with and without
  `--allow-dangerously-skip-permissions`); add `core/tests/fixtures/permission-prompt/README.md`
  naming the CLI version, the argv and how to re-record

---

## Phase 2: Foundational — core types every story needs (blocking)

- [x] T005 [P] Tests in `core/tests/structured_parse.rs` over `prompt-*.jsonl`: `Init.permission_mode`
  read from `system`/`init` `permissionMode`; `system`/`status` → `Status { permission_mode, raw }`;
  `control_response` → `ControlResponse { request_id, outcome }` with `ControlOutcome::Success(String)` (the `response` object's JSON text; ledger decision 11)
  and `ControlOutcome::Error { message, code: Option<String> }` (`error_code`
  `bypass_not_launched`); every line still makes exactly one event (feature 938 FR-014)
- [x] T006 Implement T005 in `core/src/structured/event.rs` and `core/src/structured/parse.rs`
- [x] T007 [P] Tests in `core/tests/structured_transcript.rs`: `Status` and `ControlResponse` events
  are absorbed (no entry, no raw row), and a saved history containing them restores to the same
  transcript; unit test in `core/src/structured/log_line.rs` for each one's line text
- [x] T008 Implement T007 in `core/src/structured/transcript/build.rs` and
  `core/src/structured/log_line.rs`
- [x] T009 [P] Unit tests in `core/src/structured/control.rs`: `encode_set_mode(request_id, mode)`
  gives exactly the one-line JSON of the contract ending in `\n`; request ids are `micold-<n>`,
  increasing per process counter
- [x] T010 Implement `core/src/structured/control.rs` (T009) and register it in
  `core/src/structured/mod.rs`
- [x] T011 [P] Unit tests in `core/src/structured/permission/request.rs`: `parse_call(arguments,
  meta)` yields `PromptCall { tool_name, input, tool_use_id }`, taking `tool_use_id` from
  `arguments.tool_use_id`, else `_meta["claudecode/toolUseId"]`; `Malformed { reason }` is exactly
  `"no tool name"` (missing or not a string) or `"the input is not an object"`
- [x] T012 Create `core/src/structured/permission/{mod.rs,request.rs}` with `PromptCall`,
  `Malformed`, `parse_call` (T011) and an empty `RequestKind` skeleton; register in
  `core/src/structured/mod.rs`
- [x] T013 [P] Serde round-trip tests in `core/tests/protocol_roundtrip.rs` for the wire types
  `PromptId(u64)`, `PermissionRequest { id, session, tool_use_id, tool_name, input_preview,
  input_bytes, kind, always_allow }`, `PromptChoice` (all seven variants), `QuestionAnswer
  { labels, other }`, `Resolution` (all eight) and `WithdrawReason` (all five); no message variant
  yet
- [x] T014 Add those types to `core/src/protocol/messages.rs` (T013)

---

## Phase 3: Core decisions for every request kind and mode (M1)

- [x] T015 [P] [US1] Fixture tests in `core/tests/structured_permission.rs` (new): `tool-request` and
  `always-allow` calls parse and classify `Tool`; `ExitPlanMode` with a blank or missing `plan`,
  and `AskUserQuestion` with no options, one option or a non-string `question`, classify `Tool`
  (never dropped); `answer()` for Allow equals the recorded `{"behavior":"allow","updatedInput":…}`
  with the input unchanged; Deny with no message gives "The user denied this request."; AlwaysAllow
  adds `updatedPermissions` `addRules` with `"destination":"session"` and equals the recorded answer
- [x] T016 [P] [US1] Unit tests in `core/src/structured/permission/decision.rs`: `validate` admits
  Allow, AlwaysAllow, Deny for `Tool` only (else `WrongKind`); each `DecisionRefusal` (`NotPending`,
  `OtherSession`, `WrongKind`, `Incomplete`, `NotStructured`) has its user sentence; the preview is
  at most `PREVIEW_BYTES` (64 KiB) of the input's JSON text, cut at a char boundary (a multi-byte
  char straddling the limit), with `input_bytes` the whole size
- [x] T017 [P] [US1] Unit tests in `core/src/structured/permission/rules.rs`, one per row of
  data-model *AllowRule*: Bash same `command`; Edit, MultiEdit, Write same `file_path`; NotebookEdit
  same `notebook_path`; WebFetch same lower-cased host (text between `://` and the next `/`, `?`
  or `#`), an unparseable URL the same URL; any other tool the tool name; a Bash call without
  `command` gets no rule; `describe()` gives `Bash: "npm test"`
- [x] T018 [US1] Implement `classify` (the `Tool` branch), `core/src/structured/permission/decision.rs`
  (`validate`, `answer`, `DecisionRefusal`, preview cut) and `rules.rs` (`AllowRule::for_call`,
  `matches`, `describe`) until T015–T017 pass
- [x] T019 [US3] Fixture and unit tests: `plan-approval` classifies `PlanApproval { plan }` with
  the whole plan; `ApprovePlan` answers allow with the input unchanged (equals the recorded
  answer); `KeepPlanning` with no note gives "The user wants to keep planning."; Tool choices on a
  plan approval are `WrongKind`; no `AllowRule` is offered for a plan approval
  (`core/tests/structured_permission.rs`, `decision.rs`)
- [x] T020 [US4] Fixture and unit tests: `question-multi` classifies `Question` with header,
  text, every option's label and description (absent → empty) and `multi_select` (absent → false);
  `Answer` gives `updatedInput` = input + `"answers": {<question text>: <labels joined by ", " or
  the free answer>}` equal to the recorded answer; `DeclineQuestion` gives "The user declined to
  answer."; `validate` returns `Incomplete` for an answer count ≠ question count, a label not among
  the options, two labels on a single-choice question, and neither a label nor a non-blank `other`
- [x] T021 [US3] [US4] Implement the `PlanApproval` and `Question` branches of `classify` and their
  `validate`/`answer` arms in `request.rs` and `decision.rs` until T019–T020 pass
- [x] T022 [P] [US2] Unit tests in `core/src/session.rs` and `core/src/structured/permission/mod.rs`:
  `PermissionMode::{Default, AcceptEdits, Plan, Bypass}` `cli_name()` = `default`, `acceptEdits`,
  `plan`, `bypassPermissions`; `from_cli` round-trips them and returns `None` for `auto`,
  `dontAsk`, `manual`; `is_default()`; `launch_args(mode, prompt_tool)` always has
  `--allow-dangerously-skip-permissions`, adds `--permission-mode <cli_name>` only when not
  Default, and `--permission-prompt-tool mcp__micold__permission_prompt` only when `prompt_tool`
- [x] T023 [US2] Implement `PermissionMode` in `core/src/session.rs` (serde, `Default`) and
  `launch_args` in `core/src/structured/permission/mod.rs` (T022)

---

## Phase 4: Daemon — requests held and decided (M2)

- [x] T024 [P] [US1] Tests in `core/tests/mcp_jsonrpc.rs`, `core/tests/mcp_tools_catalog.rs` and
  `core/tests/mcp_binding_plan.rs`: `route(…, ListScope)` lists `permission_prompt` for a
  structured caller only and a terminal caller's `tools/call` of it fails `unknown_tool`;
  `binding::plan(…, timeout_ms)` writes `PROMPT_TOOL_TIMEOUT_MS = 2_147_483_647` when given it,
  and a terminal session's binding stays byte-for-byte as before (`TOOL_TIMEOUT_MS`) (FR-001,
  FR-018, R3)
- [x] T025 [US1] Implement T024 in `core/src/mcp/jsonrpc.rs` (`ListScope`), `core/src/mcp/tools.rs`
  (`PROMPT_TOOL` with the contract's schema) and `core/src/mcp/binding.rs` (`timeout_ms` argument);
  update every existing caller of `plan` to pass `TOOL_TIMEOUT_MS`
- [x] T026 [P] [US1] Serde tests in `core/tests/protocol_roundtrip.rs` for
  `DaemonMsg::{PermissionRequested, PermissionResolved, PermissionInputText}` and
  `ClientMsg::{PermissionDecision, PermissionInput}` per [contracts/wire.md](contracts/wire.md);
  then add them to `core/src/protocol/messages.rs` and bump `PROTOCOL_VERSION` by one in
  `core/src/protocol/version.rs`
- [x] T027 [US1] Extend the stand-in `claude` in `daemon/tests/support/structured.rs` (R13): script
  commands `prompt <file>` (reads the tool server URL and bearer from the `--mcp-config` file its
  argv names, POSTs the file's `tools/call` over `std::net::TcpStream`, HTTP/1.1,
  `Connection: close`, appends the reply's `content[0].text` to `prompt-record`) and
  `prompt-async <file>` (the same on a thread, so several requests are held while it keeps reading
  stdin); a helper that turns `DaemonState::set_permission_prompts(true)` on for the tests
- [x] T028 [US1] Change the superseded FR-006a assertion in `daemon/tests/structured_session.rs`
  (plan *SC-007*): with the switch on, the structured argv carries
  `--allow-dangerously-skip-permissions` and `--permission-prompt-tool
  mcp__micold__permission_prompt` and no `--permission-mode` (Default); with it off, still
  `--allow-dangerously-skip-permissions` and no `--permission-prompt-tool`; the terminal argv assertion is untouched
- [x] T029 [US1] Integration tests in `daemon/tests/structured_permission.rs` (new) against the
  stand-in: a call becomes `PermissionRequested` carrying tool name, input preview, tool use id and
  kind, sent to two attached clients; the stand-in gets no reply while pending across 2 s and a
  client reconnect (FR-003, SC-001); Allow, Deny with and without a message and AlwaysAllow each
  reach the stand-in as the T015 answer and every client gets `PermissionResolved`; after
  AlwaysAllow an identical second call is answered allow with no `PermissionRequested`; a client
  connecting while a request is pending gets it exactly once, including one connecting during the
  open (FR-005); a request raised while the session's mode is Bypass is shown and held like any
  other, never answered by the daemon (edge case *Bypass mode*); `PermissionInput` returns the whole input text for a > 64 KiB Write
- [x] T030 [US1] Integration tests in `daemon/tests/structured_permission.rs`: two clients decide the
  same request concurrently, 50 iterations — exactly one decision reaches the stand-in, both see
  one resolution, the loser gets `OperationError` (SC-002); refusals with `OperationError` and the
  stand-in unchanged for an answered id, an unknown id, another session's id (session B named for
  A's request, `OtherSession`), a wrong-kind choice and an incomplete answer (FR-012); a malformed
  call is answered deny at once, no `PermissionRequested`, and the log names the tool but not the
  input (FR-004); a terminal session's credential cannot list or call the tool (FR-001); three requests of one session
  pending at once (`prompt-async`) are decided in another order than they arrived, each answer
  reaching its own call (edge case *Several requests at once*); two structured sessions with
  requests pending at once stay independent
- [x] T031 [US3] [US4] Integration tests in `daemon/tests/structured_permission.rs`: a plan
  approval request carries the whole plan and `KeepPlanning` with a note reaches the stand-in as
  deny with the note; a question request with two questions, one multi-select, is answered and the
  stand-in records the `answers` map; `DeclineQuestion` reaches it as deny
- [x] T032 [US1] Implement `daemon/src/permission.rs` (new): `PromptRegistry { pending, rules }`
  with `open` (rule match → `Opened::Answered`, else `Opened::Pending { id, receiver, broadcast }`),
  `decide` (validate, remove, `Decided { resolved, last_of_session, mode_change }`) and `replay`;
  `serve_prompt` (the `tools/call` handler: `parse_call`, `classify`, open under the state lock,
  await the receiver or the connection's `hangup` token) — no deadline (R4)
- [x] T033 [US1] Wire it in: `daemon/src/mcp/server.rs` routes by caller kind and sends
  `permission_prompt` to `permission::serve_prompt`; `daemon/src/state.rs` holds the registry
  inside the state lock, replays pending requests in the handshake under the same lock as the
  client's registration, and adds the never-persisted `set_permission_prompts(on)` switch (off by
  default, the pattern of `set_event_stream_bounds`); `daemon/src/server.rs` dispatches
  `PermissionDecision` and `PermissionInput`; `daemon/src/structured.rs` launches with
  `launch_args(Default, bound && switch)` and writes the structured binding with
  `PROMPT_TOOL_TIMEOUT_MS`; T028–T031 pass
- [x] T034 [US1] Describe the prompt tool, the switch, and the `Permission*` messages in
  `docs/daemon.md` (*The second loopback listener* section)

---

## Phase 5: Daemon — waiting state and withdrawal (M3)

- [ ] T035 [P] [US5] Integration tests in `daemon/tests/structured_permission.rs`: the session's
  activity is `AwaitingInput` (*Blocked: needs permission*) while a request is pending and
  `Working` after a decision; with two requests pending it changes only on 0 → 1 and 1 → 0; a
  stand-in that also fires its own `Notification` hook makes the needs-permission notification
  count once; two sessions waiting are marked separately (FR-009, R8)
- [ ] T036 [US5] Implement T035: on a session's first pending request `PromptRegistry::open` reports
  it and `daemon/src/state.rs` calls `note_activity(session, ActivityEvent::Hook(HookKind::Notification))`;
  a decision that leaves none applies `Hook(HookKind::PreToolUse)`; a withdrawal applies nothing
- [ ] T037 [US6] Integration tests in `daemon/tests/structured_permission.rs`: a pending request
  is withdrawn (`PermissionResolved { Withdrawn { reason } }` to every client within 1 s, the
  stand-in answered deny "The turn ended before you answered." while it still waits) when the
  stream carries a `result` (`TurnEnded`), on `TurnEndedWithoutResult`, when the process is killed
  (`ProcessEnded`), on stop (`SessionStopped`) and on remove (`SessionRemoved`); the agent hanging
  up withdraws with `AgentHungUp` and answers nothing; a decision after withdrawal is refused
  `NotPending`; nothing is pending after a daemon restart; always-allow rules are gone after the
  process ends (FR-011, SC-003)
- [ ] T038 [US6] Implement T037: `PromptRegistry::withdraw_session(session, reason)` and
  `withdraw(id, AgentHungUp)` in `daemon/src/permission.rs`; `daemon/src/structured.rs`'s reader
  calls the withdraw hook on `Result` and `TurnEndedWithoutResult`; `daemon/src/state.rs` withdraws
  on process end, stop and remove beside `confirmations_session_gone`, and drops the session's rules
  at process end

---

## Phase 6: Daemon — mode persisted, created and launched (M4)

- [ ] T039 [P] [US2] Tests in `core/tests/session_store.rs` and `core/tests/protocol_roundtrip.rs`:
  a record written before this feature loads as `Default`; each mode round-trips;
  `#[serde(default, skip_serializing_if = "PermissionMode::is_default")]` leaves a Default record's
  JSON unchanged; `SessionSummary.permission_mode` and `ClientMsg::SessionCreate.permission_mode`
  default to `Default` when absent (FR-013, FR-015)
- [ ] T040 [US2] Implement T039: `Session.permission_mode` in `core/src/session.rs`,
  `StoredSession.permission_mode` in `core/src/store.rs`, the two wire fields in
  `core/src/protocol/messages.rs`, `PROTOCOL_VERSION` +1
- [ ] T041 [P] [US2] Integration tests in `daemon/tests/structured_permission_mode.rs` (new): each of
  the four modes on `SessionCreate` is in the summary and in the stand-in's argv
  (`--permission-mode` only when not Default), and again after a restart and after a daemon
  restart and resume (SC-005); a terminal `SessionCreate` with a non-default mode is refused
  `OperationError` and writes no record; a terminal session's argv and binding file are unchanged
  (FR-018)
- [ ] T042 [US2] Implement T041: `Catalog::set_permission_mode(id, mode)` (refuses a terminal session,
  `ModeRefusal::NotStructured`) and create-with-mode in `daemon/src/catalog.rs`; the
  `SessionCreate` handler in `daemon/src/state.rs`; `daemon/src/structured.rs` launches with the
  session's persisted mode

---

## Phase 7: Daemon — mode change while running, following Claude, plan approval (M5)

- [ ] T043 [US2] Extend the stand-in in `daemon/tests/support/structured.rs`: a script step that
  awaits a `control_request` on stdin and writes a scripted `control_response` line (success with
  `response.mode`, error with `error` and `error_code`, or none), and a step that writes a
  `system`/`status` line with a given `permissionMode`
- [ ] T044 [P] [US2] Integration tests in `daemon/tests/structured_permission_mode.rs`: on a running
  session `SetPermissionMode` writes the contract's control line to stdin; `success` → persisted,
  broadcast in the summary, `OperationOk`; `error` → unchanged, `OperationError` with the CLI's
  text; no answer → unchanged and "Claude did not answer the mode change." after
  `CONTROL_ANSWER_BOUND` (10 s); a pending request is untouched by the change; the process ending
  first resolves the wait with "Claude stopped before it answered."; on a session not running the
  mode is persisted at once and used at the next launch; on a terminal session refused; after a successful change a
  `prompt` call is still raised and answered normally (the next request reaches the daemon whatever
  the mode; that the CLI applies the mode is the `mode-change` fixture's and Part B 5's evidence)
  (FR-017, SC-005)
- [ ] T045 [US2] Integration tests in `daemon/tests/structured_permission_mode.rs`: a `status`
  (or `init`) line whose `permissionMode` differs from the persisted mode changes the summary and
  the record; an unknown mode (`dontAsk`) leaves them unchanged and is logged once (FR-016)
- [ ] T046 [P] [US3] Integration tests in `daemon/tests/structured_permission.rs`: `ApprovePlan
  { accept_edits: true }` sends `set_permission_mode` `acceptEdits` and answers allow only after its
  `success`, and the summary shows Accept edits; `accept_edits: false` → `default`, whatever mode
  preceded Plan; a refused mode change still answers allow and the deciding client is told the mode
  stayed as reported; `KeepPlanning` sends no control request and the mode stays Plan
- [ ] T047 [US2] Implement T044–T045: `ClientMsg::SetPermissionMode` in
  `core/src/protocol/messages.rs` (`PROTOCOL_VERSION` +1); `ControlWaiters` in
  `daemon/src/structured.rs` (`write_control` through the stdin writer's mutex, the reader resolves
  the waiter of a matching `ControlResponse`, a process end resolves every waiter); the
  `SetPermissionMode` handler and following reported modes in `daemon/src/state.rs`; dispatch in
  `daemon/src/server.rs`
- [ ] T048 [US3] Implement T046: `decide` returns `mode_change` for `ApprovePlan` and
  `daemon/src/state.rs` sets the mode through the control request before sending the answer

---

## Phase 8: Client — render-free prompt state (M6)

- [ ] T049 [P] [US1] Reducer tests in `client/tests/features_permission_prompts.rs` (new):
  `PermissionRequested` adds a `Shown` under its session, a second one for a held id is ignored;
  `PermissionResolved` removes it and records the resolution by tool use id (*Allowed*, *Denied*,
  *Withdrawn*); `choice(id, Allow | AlwaysAllow | Deny)` builds the `PromptChoice` with the typed
  deny message (blank → `None`); `sending` is set on submit and cleared on `OperationError`; a
  session's state is dropped with the session; two sessions are isolated; `placement` gives
  `UnderCard(EntryKey::Tool(id))` when the transcript holds the card, else `AtEnd`; the input
  preview's cut flag is `input_bytes > input_preview.len()`
- [ ] T050 [US3] [US4] Reducer tests in `client/tests/features_permission_prompts.rs`: the plan
  text is held whole; Approve, Approve and accept edits and Keep planning with a note build their
  choices; `can_submit` is false until every question has a chosen option or a chosen, non-blank
  Other, a single-choice question holds one option, a multi-select several; `choice` is `None`
  while not submittable; Decline builds `DeclineQuestion` (FR-007, FR-008)
- [ ] T051 [US5] Reducer test in `client/tests/features_permission_prompts.rs`: `waiting(session)`
  is true while any request of that session is pending and false after the last resolves
- [ ] T052 [US1] Implement `client/src/features/permission_prompts.rs` (data-model *Client state*) and
  register it in `client/src/features/mod.rs`; add `State.permission_prompts` to `client/src/app.rs`;
  T049–T051 pass
- [ ] T053 [US1] Wire `client/src/shell/daemon_sync.rs`: `PermissionRequested`/`PermissionResolved`/
  `PermissionInputText` into the feature; `PermissionDecision`, `PermissionInput` and
  `SetPermissionMode` sent with their `req` and their `OperationOk`/`OperationError` replies routed
  back; a reducer test per route in `client/tests/features_permission_prompts.rs` first
- [ ] T054 [US2] Tests in `client/tests/structured_start_menu.rs`: the structured create flow
  carries a chosen mode into `SessionCreate.permission_mode` (none chosen → Default); choosing
  Bypass enters a confirmation step and only its confirm sends the create; cancelling returns to
  the choice; a terminal create sends Default (FR-014); and in
  `client/tests/features_permission_prompts.rs`, a mode change of an existing session to Bypass
  (the mode control, FR-017) goes through a confirmation state before `SetPermissionMode` is sent
- [ ] T055 [US2] Implement T054 in `client/src/features/session.rs` and the mode-change confirmation
  state in `client/src/features/permission_prompts.rs` (state only; no draw code)

---

## Phase 9: Window — requests in the transcript (M7; needs #939 M5–M6 on `main`, I-1)

- [ ] T056 [P] [US1] Add showcase entries and builder-API checks for `ChoiceList` and `PromptCard`
  so `client/tests/showcase_completeness.rs` and `client/tests/material_builder_api.rs` fail until
  both components exist
- [ ] T057 [P] [US1] Geometry gate `client/tests/gates/prompt_card_fits.rs`, registered in
  `client/tests/layout_snapshot.rs`: the action row stays inside the card at the narrowest pane
  width, for the tool, plan and question variants
- [ ] T058 [US4] Implement `client/src/ui/material/choice_list.rs` (`ChoiceList`: single or multiple
  choice rows, each label + description, an optional Other text row; builder API, theming, keyboard
  focus) and register it in `client/src/ui/material/mod.rs` and `client/src/showcase/`
- [ ] T059 [US1] Implement `client/src/ui/material/prompt_card.rs` (`PromptCard`: bordered block with
  a title, a body element and an action row; builder API) and register it likewise; T056–T057 pass
- [ ] T060 [US1] Draw the request block in `client/src/ui/transcript/permission.rs` (new): tool name,
  the input in the tool's familiar form where #939's tool card has one, else structured text; a
  bounded preview with **Show all** (sends `PermissionInput`); **Allow**, **Deny** with an optional
  message field, **Always allow** with its rule sentence; the card's *Allowed* / *Denied* /
  *Withdrawn* label; requests whose card is not held drawn after the last entry
- [ ] T061 [US1] Let the tool card of `client/src/ui/transcript/entries.rs` take the request block as
  an optional trailing element (I-1), fed from `placement`
- [ ] T062 [US3] Draw the plan view in `client/src/ui/transcript/permission.rs`: the whole plan with
  **Approve and accept edits**, **Approve**, **Keep planning** and its note field
- [ ] T063 [US4] Draw the question view in `client/src/ui/transcript/permission.rs`: each question's
  header and text, a `ChoiceList` per question, **Submit** enabled by `can_submit`, **Decline**

---

## Phase 10: Window — composer side, switch on, user guide (M8; needs #941 on `main`, I-2)

- [ ] T064 [P] [US2] Test in `client/tests/structured_start_menu.rs` (or the composer's test file
  from #941): the mode `Select` beside the composer offers the four modes, shows the session's
  summary mode, sends `SetPermissionMode`, and Bypass goes through the confirmation first
- [ ] T065 [US2] Draw the mode `Select` in #941's composer row and in the structured create flow,
  the Bypass confirmation dialog `client/src/ui/confirm_bypass.rs` (new, built like
  `ui/confirm_delete.rs`, naming the effect "Claude will run every tool without asking …"), and the
  label *Bypass: Claude runs every tool without asking* while the mode is Bypass
- [ ] T066 [US5] Show *Waiting for you* in #941's composer status slot while `waiting(session)`
- [ ] T067 [US1] Turn permission prompts on: remove `DaemonState::set_permission_prompts` and its
  test helper, launch with `--permission-prompt-tool` whenever the session is bound
  (`daemon/src/state.rs`, `daemon/src/structured.rs`, `daemon/tests/support/structured.rs`), and
  make T028's argv assertion the unconditional one in `daemon/tests/structured_session.rs`
- [ ] T068 [US1] User guide `docs/user-guide/worktrees-and-sessions.md`, *Structured sessions*:
  permission prompts, Always allow and its scope (this session only, never written to Claude's
  settings), plan approval, questions, the four modes and Bypass's risk; remove "permission
  prompts" from the not-yet list (FR-021)
- [ ] T069 Run quickstart Part B (steps 1–11) with the `visual-pass` skill and save the evidence in
  `specs/942-structured-permission-prompts/visual-pass/`

---

## Phase 11: Polish (close unit)

- [ ] T070 Run quickstart Part A and record the results in
  `specs/942-structured-permission-prompts/quickstart.md`

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phase 3 (M1); then Phases 4 → 5 → 6 → 7 → 8 → 9 → 10, each a milestone on
  `main` before the next starts. Phase 9 also waits for #939 M5–M6 (I-1), Phase 10 for #941 (I-2).
- Within a phase, each test task precedes the implementation it names (Constitution I).
- Stories: US1 and US2 (P1) carry the core and daemon paths; US3 and US4 reuse US1's registry and
  US2's mode change; US5 and US6 extend the registry; nothing depends on a later phase.

## Parallel Opportunities

- Phase 1: T001 and T002. Phase 2: T005, T007, T009, T011, T013 (disjoint files).
- Phase 3: T015, T016, T017 and T022 (separate files); T019 and T020 follow T015 and T016 in their
  files.
- Phase 4: T024 and T026; T029–T031 after T027 share one file and are written in sequence.
- Phase 5: T035 beside T036's files; Phase 7: T044 and T046 (different files).
- Phase 8: none — T049–T051 and T054 share `client/tests/features_permission_prompts.rs`.

## Implementation Strategy

MVP is M1 + M2: a structured session's tool request reaches the daemon, is held, and is answered
by any client (the stand-in tests); the switch keeps it off for users until M8. Each later
milestone adds one layer and is observable through its own tests; the feature becomes visible to
users in M8, with its user guide.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). M1–M6 depend on
nothing beyond `main`; **M7 needs #939 M5–M6 merged (I-1); M8 needs #941 merged (I-2)**. Until M8
the daemon-only switch (R14) keeps `--permission-prompt-tool` off, so no user sees a request no
window can answer. T070 (Polish, no code) is left to the close unit (milestones rule 4).

### M1 — Core: request kinds, answers, rules and modes against recorded fixtures 🎯 MVP

- **Tasks**: T001–T023
- **Deliverable**: `micold_core` parses every recorded CLI prompt call, classifies it as tool
  request, plan approval or question, builds the exact answer the CLI acted on, derives
  always-allow rules, encodes mode-change control lines and builds the launch arguments per mode;
  the parser reads the CLI's mode and control replies; the capture script and fixtures are in the
  tree
- **Satisfies**: FR-002, FR-004, FR-013 (core), FR-020, SC-004; US1 scenarios 2–4, US3 scenarios
  1–3, US4 scenarios 1–4 at the model
- **Verify**: `cargo test -p micold-core --test structured_permission --test structured_parse
  --test structured_transcript --test protocol_roundtrip`;
  `cargo test -p micold-core --lib -- structured::permission structured::control session::`;
  `bash scripts/tests/capture-permission-prompt.test.sh`
- **Depends on**: —
- **Tier**: full

### M2 — Daemon holds and decides prompt requests (switch off)

- **Tasks**: T024–T034
- **Deliverable**: with the switch on (tests), a structured session's prompt-tool call becomes a
  pending request shown to every client, replayed to a late client, held without a deadline and
  answered with the first decision of any kind; always-allow skips identical later calls;
  late, foreign and malformed decisions are refused
- **Satisfies**: US1 scenarios 1–6; US3 scenario 3, US4 scenarios 2–3 at the daemon; FR-001–FR-005,
  FR-010 (except plan-approval mode), FR-012, FR-018, FR-019; SC-001, SC-002, SC-007
- **Verify**: `cargo test -p micold-daemon --test structured_permission --test structured_session`;
  `cargo test -p micold-core --test mcp_jsonrpc --test mcp_binding_plan --test mcp_tools_catalog`
- **Depends on**: M1
- **Tier**: full

### M3 — Daemon: waiting state and withdrawal

- **Tasks**: T035–T038
- **Deliverable**: a pending request marks its session *Blocked: needs permission* and fires the
  needs-permission notification once; a request is withdrawn from every client within 1 s when its
  turn ends, its process ends, the session stops or is removed, or the agent hangs up
- **Satisfies**: US5 scenarios 1–2, US6 scenarios 1–2; FR-009 (daemon), FR-011; SC-003
- **Verify**: `cargo test -p micold-daemon --test structured_permission`
- **Depends on**: M2
- **Tier**: full

### M4 — Daemon: permission mode persisted, created and launched

- **Tasks**: T039–T042
- **Deliverable**: a structured session created in any of the four modes is launched in it, shows
  it in its summary and keeps it across restart and daemon restart; old records load as Default; a
  terminal session cannot be given a mode
- **Satisfies**: US2 scenarios 1–3; FR-013, FR-015, FR-018; SC-005 (launch, restart, daemon restart)
- **Verify**: `cargo test -p micold-daemon --test structured_permission_mode`;
  `cargo test -p micold-core --test session_store --test protocol_roundtrip`
- **Depends on**: M3
- **Tier**: full

### M5 — Daemon: mode change while running, following Claude, plan approval sets the mode

- **Tasks**: T043–T048
- **Deliverable**: `SetPermissionMode` changes a running session's mode through a stream-json
  control request (refusals and silence leave it unchanged with a reason); the CLI's reported mode
  changes are followed; approving a plan sets Accept edits or Default before Claude continues
- **Satisfies**: US2 scenario 5, US3 scenario 2; FR-010 (plan approval), FR-016, FR-017; SC-005
  (change then next request)
- **Verify**: `cargo test -p micold-daemon --test structured_permission_mode --test structured_permission`
- **Depends on**: M4
- **Tier**: full

### M6 — Client: render-free prompt state and create-flow mode

- **Tasks**: T049–T055
- **Deliverable**: the window's state holds each session's pending requests, drafts and
  resolutions, decides `can_submit`, builds every choice, reports waiting and placement, and sends
  decisions and mode changes; the structured create flow carries a mode with a Bypass
  confirmation step (no draw code yet)
- **Satisfies**: FR-006–FR-009 and FR-014 at the state level; US1, US3, US4, US5 client logic
- **Verify**: `cargo test -p micold-client --test features_permission_prompts --test structured_start_menu`
- **Depends on**: M5
- **Tier**: full

### M7 — Window: requests, plans and questions in the transcript

- **Tasks**: T056–T063
- **Deliverable**: the shared `ChoiceList` and `PromptCard` are in the showcase; the transcript
  draws a request block under its tool card (or at the end), the plan view and the question view;
  still unreachable for users until M8 turns the switch on
- **Satisfies**: FR-006, FR-007, FR-008 (drawn); Constitution VIII
- **Verify**: `cargo test -p micold-client --test showcase_completeness --test material_builder_api
  --test layout_snapshot`; the showcase entries for `ChoiceList` and `PromptCard`
- **Depends on**: M6; #939 M5–M6 on `main` (I-1)
- **Tier**: full

### M8 — Window: mode control, waiting state, prompts on, user guide

- **Tasks**: T064–T069
- **Deliverable**: a user of a structured session is asked for permission in the window, sees and
  changes the mode (Bypass after confirmation), sees *Waiting for you*, and the user guide
  describes it; quickstart Part B passes
- **Satisfies**: US1–US6 end to end; FR-009 (composer), FR-014, FR-017 (window), FR-021; SC-006
- **Verify**: quickstart Part B (steps 1–11) recorded in `visual-pass/`;
  `cargo test -p micold-daemon --test structured_session`; `scripts/check-user-guide-updated.sh`
- **Depends on**: M7; #941 on `main` (I-2)
- **Tier**: full
