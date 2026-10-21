# Research: Structured Session Permission Prompts (#942)

Each decision: what was chosen, why, and what was rejected. **P-n** marks a probe run on
2026-10-21 against the real CLI (Claude Code `2.1.306`, the dev container) with a stub MCP server
that logged every call and answered as scripted; the probe scripts are not part of the change, the
recorded fixtures of R12 are their committed form.

## R1. Transport: the prompt tool on the session service's tool server

- **Decision**: the daemon serves a tool `permission_prompt` on the existing tool server (feature
  034, `crates/micold-daemon/src/mcp/`), and every structured launch names it with
  `--permission-prompt-tool mcp__micold__permission_prompt` (the binding's server name is
  `micold_core::mcp::jsonrpc::SERVER_NAME`). The caller is the session that holds the presented
  bearer credential (`mcp::credentials::Credentials::session_for`), so no session can raise or
  answer another's request (FR-001) without new authentication.
- **Observed (P-1)**: the CLI calls the tool with `{"tool_name", "input", "tool_use_id"}` and
  `_meta["claudecode/toolUseId"]`, sends `Accept: application/json, text/event-stream`, and reads
  the answer from `content[0].text` as JSON: `{"behavior":"allow","updatedInput":{…}}` or
  `{"behavior":"deny","message":"…"}`. A deny message reaches Claude as an `is_error` tool result
  carrying the text (P-2). The prompt tool is **not** in the model's tool list (`init.tools` omits
  `mcp__micold__permission_prompt` when it is the prompt tool), so the model cannot call it itself.
- **Rejected — the CLI's in-stream channel** (`--permission-prompt-tool stdio`): P-7 shows it works
  (`control_request` `can_use_tool` on stdout, with `permission_suggestions` and `blocked_path`)
  and it would avoid R3's timeout. Rejected because the issue's acceptance fixes the MCP tool (spec
  *Assumptions*) and the tool server already gives per-session identity, sandbox reachability
  (feature 034) and the hang-up signal (`server.rs` `peer_closed`). The control channel is still
  used, for mode changes (R6), where the spec requires it. Recorded so #943 or later can revisit
  it; nothing here would block a switch: the core model (R2–R5) does not depend on the transport.
- **Rejected — the hook receiver** (a `PreToolUse` hook answering `permissionDecision`): a hook's
  reply has a short CLI-side timeout and no first-answer-wins across windows; the spec reuses the
  hook receiver only for activity (R8).

## R2. Request model and classification (FR-002, FR-004)

- **Decision**: core `micold_core::structured::permission::request::parse_call(&Value) ->
  Result<PromptCall, Malformed>`; `PromptCall { tool_name, input: Map, tool_use_id: Option<String> }`
  (`_meta["claudecode/toolUseId"]` as fallback for the id). `Malformed` when `tool_name` is
  missing/not a string or `input` is not a JSON object; the daemon answers deny with the reason at
  once and logs the tool name only (never the input).
- `classify(&PromptCall) -> RequestKind`: `ExitPlanMode` with a non-blank string `plan` →
  `PlanApproval { plan }` (P-3: input `{plan, planFilePath}`); `AskUserQuestion` whose `questions`
  is a non-empty array of objects each with a string `question`, a string `header`, a boolean (or
  absent → false) `multiSelect` and ≥ 2 options each with a string `label` (description may be
  absent → empty) → `Question { questions }` (P-4); everything else, the two above included when
  their shape does not fit, → `Tool`. Never dropped (spec edge cases).
- **Rejected**: classifying in the daemon — classification is decision logic, so it lives in core
  under unit and fixture tests (Constitution I).

## R3. Holding the call: the CLI's MCP tool timeout (FR-003)

- **Observed**: the CLI ends a pending MCP tool call by its own timers, and the call then fails as
  the tool's error result while Claude carries on without the tool:
  - P-5: with `MCP_TOOL_TIMEOUT=60000` (the dev container's value) the call failed at 60 s, "tool
    \"permission_prompt\" timed out after 60s", whether the reply was plain JSON or an SSE stream
    with keep-alive comments; P-6: MCP `notifications/progress` on the call's `progressToken` did
    not extend it;
  - P-12: with the variable unset, a plain JSON reply failed at 60 s ("The operation timed out.")
    and an SSE reply at 300 s ("sent no response or progress for 300s … set a per-server
    \"timeout\" (ms) to allow longer silent runs for just this server");
  - P-13: with the binding's per-server `"timeout"` set to `2147483647`, a plain JSON reply after
    330 s was read and acted on, with `MCP_TOOL_TIMEOUT` unset and with it at `60000` (the
    per-server value wins).
- **Decision**: a structured session's binding file gives the `micold` server
  `"timeout": PROMPT_TOOL_TIMEOUT_MS = 2_147_483_647` (the largest Node timer, ≈ 24.8 days; a
  larger value overflows to an immediate timeout in Node) instead of `TOOL_TIMEOUT_MS` (120 s) that
  `micold_core::mcp::binding::plan` writes today. `plan` takes the timeout as an argument; the
  daemon passes the structured value only for structured sessions, so terminal sessions' binding is
  byte-for-byte unchanged (FR-018). It affects only the `micold` server's own tools: the
  destructive-operation confirmation keeps its own 60 s bound in the daemon (feature 034), so no
  other tool of ours waits longer than before. The reply stays a plain JSON response, as every
  other tool's.
- The daemon never times a request out (FR-003): `PromptRegistry` (R4) has no deadline, unlike
  `mcp::confirm::CONFIRM_TIMEOUT`.
- **Rejected**: setting `MCP_TOOL_TIMEOUT` in the CLI's environment (P-6 shows it works) — it would
  also lift the timeout of the user's own MCP servers' tools; an SSE reply with keep-alives or
  progress notifications (P-5, P-6, P-12: not enough on their own); answering deny before the CLI's
  timer (violates FR-003).

## R4. Pending requests in the daemon (FR-003, FR-005, FR-010–FR-012)

- **Decision**: `crates/micold-daemon/src/permission.rs` (NEW): `PromptRegistry`, bookkeeping alone,
  kept inside `DaemonState`'s lock beside the connected clients exactly as `mcp::confirm::Registry`
  is, so opening a request and broadcasting it, and a new client's registration with its replay of
  every pending request, are each one atomic step (no request missed, none shown twice — FR-005).
  Each entry: `PromptId(u64)` (daemon-wide counter), session, `PromptCall`, `RequestKind`, and the
  `oneshot::Sender<Answer>` the waiting tool call holds. `decide(session, id, choice)` validates the
  choice (R5) and removes the entry: the first decision wins, a second one finds nothing and is
  refused `NotPending` (FR-010, FR-012).
- The wait: the `tools/call` handler opens the entry and awaits the receiver or the connection's
  `hangup` token (`mcp/server.rs` already cancels it when the agent hangs up). A hang-up withdraws
  the entry; nothing is answered to a gone peer.
- **Withdrawal (FR-011, SC-003)**: every pending request of a session is withdrawn — the CLI
  answered deny ("The turn ended before you answered."), every client sent
  `PermissionResolved { Withdrawn }` — when the session's stream carries a `Result` (the turn
  ended: P-8 shows an interrupt ends the turn with a `result` while the MCP call stays open, so
  this covers #941's interrupt without depending on it) or a `TurnEndedWithoutResult`, when its
  process ends, and when it is stopped or removed (the paths that call
  `confirmations_session_gone` today). Nothing is persisted (spec *Assumptions*).
- **Rejected**: requests as events in the session's event history — they would be saved to disk
  with the history (feature 938 R14) and restored as if pending; a pending request is state, not
  CLI output. **Rejected**: sending requests only to clients viewing the session — the confirm
  path's broadcast-to-every-window with replay on handshake already gives FR-005's guarantee, and
  a window must know of a request to mark it even when the session is not in view.

## R5. Decisions and answers (FR-010)

- **Decision**: wire enum `PromptChoice` (core, `protocol/messages.rs`): `Allow`, `AlwaysAllow`,
  `Deny { message: Option<String> }`, `ApprovePlan { accept_edits: bool }`,
  `KeepPlanning { note: Option<String> }`, `Answer { answers: Vec<QuestionAnswer> }`,
  `DeclineQuestion`. Core `permission::decision::validate(kind, choice)` admits `Allow`,
  `AlwaysAllow`, `Deny` for `Tool`; `ApprovePlan`, `KeepPlanning` for `PlanApproval`; `Answer`,
  `DeclineQuestion` for `Question`; an `Answer` must have one entry per question, each with ≥ 1
  label from that question's options (exactly one when not multi-select) or a non-blank `other`
  (FR-008 "submittable only when every question is answered", enforced by the daemon as well).
- `permission::decision::answer(call, kind, choice) -> Value` is the JSON the tool returns in
  `content[0].text`:
  - allow: `{"behavior":"allow","updatedInput": <input unchanged>}` (P-1);
  - deny: `{"behavior":"deny","message": <message, or "The user denied this request.">}`;
  - question: allow with `updatedInput` = the input plus `"answers": {<question text>: <labels
    joined by ", ", or the free answer>}` — P-4: the CLI turned exactly this into the tool result
    `"Which color?"="red", "Which fruits?"="apple, pear"`; decline: deny "The user declined to
    answer.";
  - plan approval: allow with the input unchanged (P-3: the CLI answered "User has approved your
    plan" and left plan mode for `default`); keep planning: deny with the note, or "The user wants
    to keep planning.".
- **Rejected**: letting a client send a raw `updatedInput` — a client could then run a different
  tool input than the one shown; the daemon only ever returns the CLI's own input, extended with
  the answers for a question.

## R6. Permission mode (FR-013–FR-017)

- **Decision**: core `PermissionMode { Default, AcceptEdits, Plan, Bypass }` in `session.rs`,
  `cli_name()` = `default`, `acceptEdits`, `plan`, `bypassPermissions` (P-9: `set_permission_mode`
  accepted `default` and `acceptEdits`; the CLI's `--help` lists `acceptEdits`, `plan`,
  `bypassPermissions` among its choices); `from_cli(&str) -> Option<Self>`. Another mode the CLI
  reports (its `--help` also lists `auto`, `dontAsk`, `manual`, which the app never sets) leaves
  the persisted mode unchanged and is logged once (FR-013's closed set).
- **Launch** (FR-013, FR-015): `--permission-mode <cli_name>` only when the mode is not Default
  (Default adds nothing, feature 938 FR-006a), and `--allow-dangerously-skip-permissions` on every
  structured launch. P-9: without it, `set_permission_mode` to `bypassPermissions` is refused with
  `error_code: "bypass_not_launched"`; P-10: with it, the switch succeeds. The flag only makes Bypass
  *reachable*; the mode stays what `--permission-mode` says, and only the daemon's control request,
  sent after the user's FR-014 confirmation, can select it (Claude cannot: its only mode change is
  `ExitPlanMode`, which the user approves, R7).
- **Change while running** (FR-017): stdin control request
  `{"type":"control_request","request_id":"<id>","request":{"subtype":"set_permission_mode","mode":"<cli_name>"}}`,
  written through the same stdin mutex as user messages (`StructuredSession::write_user_message`'s
  writer). The answer is a stdout `control_response` with `subtype` `success` (`response.mode`) or
  `error` (`error`, `error_code`) for that `request_id` (P-9). The daemon waits for it up to
  `CONTROL_ANSWER_BOUND` (10 s); on success it persists the mode and broadcasts the catalog; on an
  error or no answer it changes nothing and the client is told the CLI's `error` text, or "Claude
  did not answer the mode change.". A session not running: persisted at once, used at next launch.
- **Following Claude** (FR-016): the CLI reports its mode in `system`/`init` (`permissionMode`) and
  in `system`/`status` lines whenever it changes (P-3, P-9). The daemon applies the reported mode
  when it differs from the persisted one.
- **Parser**: two new `StructuredEvent` variants, `Status { permission_mode: Option<String>, raw }`
  for `system`/`status` and `ControlResponse { request_id, outcome: Result<Value, ControlError>,
  raw }` for `control_response`, plus `Init.permission_mode`. They stay in the event history (every
  line is an event, feature 938 FR-014); #939's transcript absorbs both (no entry) and #938's log
  line names them. **Rejected**: intercepting control responses before the history — breaks the
  line-count invariant 938's tests assert; **rejected**: reading `raw` of `Unrecognised` in the
  daemon — leaves the lines as raw rows in the transcript.
- **Shared with #941**: #941's interrupt is a control request too (`subtype: interrupt`, P-8 answers
  `success`). `structured::control` (encode a request, correlate a response) is built here; if #941
  lands first with its own, this feature uses that one instead (integration point I-3 in plan.md).

## R7. Plan approval sets the mode (FR-010, US3)

- **Decision**: on `ApprovePlan { accept_edits }` the daemon first sends `set_permission_mode`
  (`acceptEdits` or `default`) and waits for its answer (R6), then answers the tool allow; on
  `KeepPlanning` it answers deny and sends nothing. P-3 shows the CLI leaves plan mode for `default`
  on an allowed `ExitPlanMode` by itself; setting the mode before the answer makes the next tool
  request run in the chosen mode whatever the CLI does on its own. If the mode change is refused,
  the plan is still approved (Claude was told yes) and the client is told the mode stayed as
  reported.
- **Rejected**: `updatedPermissions: [{type: setMode}]` in the answer — R9 shows the MCP path
  ignores `updatedPermissions`.

## R8. Activity: "waiting for you" through the existing path (FR-009, US5)

- **Decision**: when a session's pending count goes 0 → 1, the daemon calls
  `DaemonState::note_activity(session, ActivityEvent::Hook(HookKind::Notification))` — the call the
  hook receiver makes for a `Notification` hook (`hooks.rs`), so `AwaitingInput`, *Blocked: needs
  permission* (`location_state.rs`), the attention mark and the needs-permission notification
  (feature 613, `TurnChange::AskedUser`) follow unchanged. When it goes 1 → 0 by a decision, it
  applies `Hook(HookKind::PreToolUse)` (back to `Working`: the turn continues); by a withdrawal at
  turn end it applies nothing (the turn's own `Stop` hook / process end decides). A CLI that also
  fires its own `Notification` hook leaves the signal `AwaitingInput` once: `note_activity` counts
  only a change (`began_waiting`), so the session is never shown waiting twice.
- **Rejected**: a new `ActivitySignal` variant — the spec asks for exactly the terminal session's
  activity, and a new variant would need every reader of the signal changed.

## R9. Always allow (FR-010; clarify item 4 — the rule when the CLI sends no suggestions)

- **Observed (P-11)**: the MCP prompt tool receives **no** rule suggestions (only `tool_name`,
  `input`, `tool_use_id`), and an answer carrying
  `updatedPermissions: [{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"<command>"}],"behavior":"allow","destination":"session"}]`
  is ignored: the identical second call was asked again. (The `stdio` channel does send
  suggestions, P-7, with `destination: "localSettings"`.)
- **Decision**: the daemon keeps this session's always-allow rules itself and answers a matching
  later request allow at once, without opening a pending request (US1 scenario 4). Core
  `permission::rules::AllowRule::for_call(&PromptCall) -> AllowRule` and `AllowRule::matches(&call)`:
  - `Bash`: the exact `command` string;
  - `Edit`, `MultiEdit`, `Write`: the exact `file_path`; `NotebookEdit`: the exact `notebook_path`;
  - `WebFetch`: the URL's host (`url::Url` is not a dependency: the host is the text between
    `://` and the next `/`, `?` or `#`, lower-cased; an unparseable URL gives an exact-URL rule);
  - every other tool (MCP tools, unknown tools): the tool name, any input.
  Never for a plan approval or a question (Always allow is offered for tool requests only).
- Scope: rules live in the session's live entry and are dropped when its process ends; never
  written to a Claude settings file (FR-010). The answer still carries `updatedPermissions` with
  destination `session` (so a CLI that honours it skips the call itself; the fixture records that
  this version does not).
- **Rejected**: the CLI's suggestion format — not sent on this path; **rejected**: matching CLI
  rule syntax (`Bash(git *)`) — the daemon would re-implement and drift from the CLI's matcher; an
  exact-input rule is narrower than the CLI's own and never allows more than the user saw.

## R10. Wire (FR-005, FR-010, FR-012, FR-015, FR-017)

- **Decision**: see [contracts/wire.md](contracts/wire.md). `PermissionRequested` and
  `PermissionResolved` to every connected client (replay on handshake under the state lock, as
  `ConfirmationRequested`); `PermissionDecision` and `SetPermissionMode` answered `OperationOk` /
  `OperationError { message }`; `SessionSummary.permission_mode`, `SessionCreate.permission_mode`
  with `#[serde(default)]`. `PROTOCOL_VERSION` +1 in each milestone that changes the wire.
- A request's `input` travels as JSON text with a cap: `PREVIEW_BYTES = 64 KiB` of it on the wire
  plus `input_bytes`; the whole input is fetched with `PermissionInput { req, session, request }`
  only when the user opens it (spec edge case "a very large input"; frames stay far below
  `MAX_FRAME_LENGTH`). A plan's text and a question's options are never cut.

## R11. Window integration (FR-006–FR-009, FR-014, FR-017)

- **Decision**: render-free `crates/micold-client/src/features/permission_prompts.rs` holds per
  session the pending requests (by `PromptId`), the resolutions seen this run (shown on the card as
  *Allowed*, *Denied*, *Withdrawn*), and each request's draft (deny message, question choices,
  free answer). It decides `can_submit`, builds the `PromptChoice`, and forgets a session's state
  with the session. Draw code in `ui/transcript/permission.rs` (GUI exception) places a request
  under its tool card (`EntryKey::Tool(tool_use_id)` of #939's transcript) or, when the card is not
  held, at the end of the transcript (spec edge case).
- New shared components (Principle VIII): `ui/material/choice_list.rs` (`ChoiceList` — single or
  multiple choice rows, each label + description, with an optional "Other" text row; builder API)
  — no radio-group primitive exists today (`checkbox.rs`, `toggle_chip.rs` are single toggles);
  `ui/material/prompt_card.rs` (`PromptCard` — a bordered block with a title, a body element and an
  action row) used by all three request views. Buttons, text fields, dialogs and the select reuse
  `button.rs`, `text_field.rs`, `dialog.rs`, `select.rs`.
- Mode control: a `Select` of the four modes beside #941's composer, and in the creation flow of the
  start menu's "Claude Code (structured)" entry. Choosing Bypass opens a confirmation dialog built
  like `ui/confirm_delete.rs` naming its effect ("Claude will run every tool without asking …").
- **Rejected**: a new transcript `EntryKind` for requests (the seam #939's contract suggested) —
  requests are not events and are not in the transcript's history, so an overlay keyed by tool use
  id keeps #939's model untouched and its guarantees (G1–G5) unaffected.

## R12. Fixtures (FR-020, SC-004)

- **Decision**: `crates/micold-core/tests/fixtures/permission-prompt/`, one JSON file per case:
  `tool-request.json`, `always-allow.json`, `plan-approval.json`, `question-multi.json`,
  `mode-change.json`, each holding the recorded `tools/call` arguments (or control lines), the
  answer the stub returned, and the CLI's resulting tool-result text from the same run; scrubbed
  like the stream-json fixtures (paths → `<CWD>`, `<HOME>`, session uuid → `<SESSION_ID>`) with a
  README naming the CLI version and argv. Recorded by `scripts/capture-permission-prompt.sh`, which
  runs a small stub MCP server (`scripts/capture/prompt-stub.py`) and the existing
  `scrub_stream_json` of `scripts/capture-stream-json.sh` (sourced, not copied). Core tests: every
  fixture's call parses and classifies as its kind, and `answer()` for the fixture's choice equals
  the recorded answer (SC-004).
- The matching stream-json stdout of each run goes beside the 938 fixtures
  (`tests/fixtures/stream-json/prompt-*.jsonl`) for the parser's `Status` / `ControlResponse` /
  `Init.permission_mode` tests.

## R13. Integration tests without a real CLI (FR-019)

- **Decision**: the stand-in `claude` of `crates/micold-daemon/tests/support/structured.rs` gains
  a script command `prompt <file>`: it reads the tool server URL and bearer from the `--mcp-config`
  file its argv names, POSTs the file's JSON-RPC `tools/call` over a plain `std::net::TcpStream`
  (HTTP/1.1, `Connection: close`), and appends the reply's `content[0].text` to `prompt-record`;
  `prompt-async <file>` does the same on a thread, so a script can hold several requests at once
  and keep reading stdin. Control requests on stdin are answered by a scripted line
  (`await-stdin` then a `control_response` line), so mode changes are testable.
- **Rejected**: a real CLI in CI (FR-019); a mock tool server — the test must cover the real
  `tools/call` path, credential included.

## R14. Feature switch until the window can answer

- **Decision**: until the milestone that ships the window side (#939's view and #941's composer on
  `main`), a structured session must not raise requests nobody can answer. `DaemonState` gains
  a daemon-only, never persisted `permission_prompts: bool`, off by default and set by
  `DaemonState::set_permission_prompts(on)` (the pattern of `set_event_stream_bounds`; not a user
  setting, so `micold_core::settings::DaemonConfig` is untouched); the launch adds
  `--permission-prompt-tool` only while it is on, and the daemon tests turn it on. The window milestone flips the default and removes the
  switch. Off, a structured session behaves exactly as feature 938 ships it.
- **Rejected**: landing the launch flag with the daemon milestone — a request would hold the turn
  with no window able to answer it.

## Probe log

| Probe | What | Result |
|---|---|---|
| P-1 | Bash request, answer allow | call shape as R1; command ran |
| P-2 | answer deny with a message | Claude got the message as an error tool result and quoted it |
| P-3 | `--permission-mode plan`, `ExitPlanMode` | input `{plan, planFilePath}`; allow → `status` `permissionMode: default`; next Write asked |
| P-4 | `AskUserQuestion` 2 questions, one multi-select | shape as R2; answers as R5 accepted verbatim |
| P-5 | answer after 330 s with `MCP_TOOL_TIMEOUT=60000`, JSON and SSE keep-alive | timed out at 60 s either way |
| P-6 | SSE + progress notifications, 100 s, variable 60 s; JSON 100 s, variable 600 s | first timed out at 60 s; second answered |
| P-7 | `--permission-prompt-tool stdio` | `control_request` `can_use_tool` with `permission_suggestions` |
| P-8 | `interrupt` control request while a call is pending | `success`; the turn ended with a `result`; the MCP call stayed open |
| P-9 | `set_permission_mode` `acceptEdits`, `default`, `bypassPermissions` | success, success + `status` lines; bypass refused `bypass_not_launched` |
| P-10 | as P-9 bypass, launched with `--allow-dangerously-skip-permissions` | success |
| P-11 | always-allow with `updatedPermissions` (session), same command twice | second call asked again |
| P-12 | answer after 330 s, variable unset, JSON and SSE keep-alive | JSON failed at 60 s, SSE at 300 s (idle) |
| P-13 | binding `"timeout": 2147483647`, answer after 330 s, variable unset and 60 s | answered and acted on in both |
