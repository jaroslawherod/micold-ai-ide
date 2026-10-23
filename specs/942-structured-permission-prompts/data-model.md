# Data Model: Structured Session Permission Prompts (#942)

Types are named as they will be written; **NEW** marks a type or module this feature adds, and
everything else exists on `main`. Decisions behind them: [research.md](research.md).

## PermissionMode (core `session.rs`, persisted, on the wire) — NEW

| Variant | `cli_name()` | Launch adds |
|---|---|---|
| `Default` (default) | `default` | nothing (feature 938 FR-006a; FR-013) |
| `AcceptEdits` | `acceptEdits` | `--permission-mode acceptEdits` |
| `Plan` | `plan` | `--permission-mode plan` |
| `Bypass` | `bypassPermissions` | `--permission-mode bypassPermissions` |

- `from_cli(&str) -> Option<PermissionMode>`; `is_default()`.
- `Session.permission_mode`: mutable (unlike `kind`), set by `Catalog::set_permission_mode(id,
  mode)`, which refuses a terminal session (`ModeRefusal::NotStructured`).
- `StoredSession.permission_mode`: `#[serde(default, skip_serializing_if =
  "PermissionMode::is_default")]` — every record written before this feature loads as `Default`
  (FR-013); no `schema_version` bump.
- `SessionSummary.permission_mode`, `ClientMsg::SessionCreate.permission_mode`: `#[serde(default)]`.
  A `SessionCreate` with a non-default mode and `kind: Terminal` is refused (FR-018).
- Every structured launch also carries `--allow-dangerously-skip-permissions` (R6), whatever the
  mode, and `--permission-prompt-tool mcp__micold__permission_prompt` while the tool server bound
  the session (R14's switch, `DaemonState::set_permission_prompts`, kept it off until M8). Built by core
  `structured::permission::launch_args(mode, prompt_tool: bool) -> Vec<String>`.

## PromptCall (core `structured::permission::request`) — NEW

What the CLI sent (research R1, R2).

| Field | Type | From |
|---|---|---|
| `tool_name` | `String` | `arguments.tool_name` |
| `input` | `serde_json::Map<String, Value>` | `arguments.input` (must be an object) |
| `tool_use_id` | `Option<String>` | `arguments.tool_use_id`, else `_meta["claudecode/toolUseId"]` |

`parse_call(arguments, meta) -> Result<PromptCall, Malformed>`; `Malformed { reason: &'static str }`
(`"no tool name"`, `"the input is not an object"`) — answered deny at once, logged without the
input, never pending (FR-004).

## RequestKind (core) — NEW

| Variant | When (`classify`) | Fields |
|---|---|---|
| `PlanApproval` | `tool_name == "ExitPlanMode"` and `input.plan` is a non-blank string | `plan: String` |
| `Question` | `tool_name == "AskUserQuestion"` and `input.questions` has the question shape (R2) | `questions: Vec<Question>` |
| `Tool` | everything else, the two above included when their shape does not fit | — |

### Question

`{ header: String, text: String, options: Vec<QuestionOption>, multi_select: bool }`;
`QuestionOption { label: String, description: String }`. At least one question, at least two
options each; `description` may be empty.

## PermissionRequest (wire; core `protocol/messages.rs`) — NEW

What every client is shown.

| Field | Type | Note |
|---|---|---|
| `id` | `PromptId(u64)` | daemon-wide, never reused within a daemon run |
| `session` | `SessionId` | |
| `tool_use_id` | `Option<String>` | the transcript card it belongs to (#939 `EntryKey::Tool`) |
| `tool_name` | `String` | |
| `input_preview` | `String` | the input's JSON text, at most `PREVIEW_BYTES` (64 KiB), cut at a char boundary |
| `input_bytes` | `u64` | the whole input's JSON size; `> input_preview.len()` means cut |
| `kind` | `RequestKind` | plan text and questions are never cut |
| `always_allow` | `Option<String>` | the rule Always allow would add, as a sentence (`Bash: "npm test"`), `None` for plan approvals and questions |

## PromptChoice (wire, client → daemon) — NEW

| Variant | Valid for | Answer to the CLI (R5) | Side effect |
|---|---|---|---|
| `Allow` | `Tool` | allow, input unchanged | — |
| `AlwaysAllow` | `Tool` | allow, input unchanged, `updatedPermissions` (session) | adds the session's `AllowRule` |
| `Deny { message: Option<String> }` | `Tool` | deny, message or "The user denied this request." | — |
| `ApprovePlan { accept_edits: bool }` | `PlanApproval` | allow, input unchanged | mode → `AcceptEdits` / `Default` first (R7) |
| `KeepPlanning { note: Option<String> }` | `PlanApproval` | deny, note or "The user wants to keep planning." | mode stays `Plan` |
| `Answer { answers: Vec<QuestionAnswer> }` | `Question` | allow, input + `answers` map | — |
| `DeclineQuestion` | `Question` | deny, "The user declined to answer." | — |

`QuestionAnswer { labels: Vec<String>, other: Option<String> }`, one per question in order.
`validate(kind, choice) -> Result<(), DecisionRefusal>`: wrong kind → `WrongKind`; answers count ≠
questions, a label not among the question's options, two labels on a single-choice question, or
neither a label nor a non-blank `other` → `Incomplete` (FR-008).

## DecisionRefusal (core) — NEW

`NotPending` (answered, withdrawn or never existed), `OtherSession` (the id belongs to another
session than the one named), `WrongKind`, `Incomplete`, `NotStructured`. Each has a user sentence;
the daemon replies `OperationError { message }` and changes nothing (FR-012).

## Resolution (wire) — NEW

`Allowed`, `AllowedAlways`, `Denied`, `PlanApproved { mode: PermissionMode }`, `KeptPlanning`,
`Answered`, `Declined`, `Withdrawn { reason: WithdrawReason }`; `WithdrawReason { TurnEnded,
ProcessEnded, SessionStopped, SessionRemoved, AgentHungUp }`.

## AllowRule (core `structured::permission::rules`) — NEW

| Tool | Rule matches a later call with |
|---|---|
| `Bash` | the same `command` |
| `Edit`, `MultiEdit`, `Write` | the same `file_path` |
| `NotebookEdit` | the same `notebook_path` |
| `WebFetch` | the same URL host (lower-cased); an unparseable URL: the same URL |
| any other | the same tool name |

`AllowRule::for_call(&PromptCall)`, `matches(&PromptCall) -> bool`, `describe() -> String`.
A call whose required field is missing gets the tool-name rule only if the tool is not one of the
named ones (a `Bash` call without `command` gets no rule: Always allow is then not offered).

## PromptRegistry (daemon `permission.rs`, inside `DaemonState`'s lock) — NEW

| Item | Rule |
|---|---|
| `pending: BTreeMap<PromptId, Pending>` | `Pending { request: PermissionRequest, call: PromptCall, answer: oneshot::Sender<Value> }` |
| `rules: HashMap<SessionId, Vec<AllowRule>>` | dropped when the session's process ends (R9) |
| `open(session, call, kind) -> Opened` | `Opened::Answered(Value)` when a rule matches (no request), else `Opened::Pending { id, receiver, broadcast: DaemonMsg }`; first pending of the session → activity `Notification` (R8) |
| `decide(session, id, choice) -> Result<Decided, DecisionRefusal>` | removes the entry; `Decided { resolved: DaemonMsg, last_of_session: bool, plan: Option<PlanApproved> }`; for an `ApprovePlan`, `PlanApproved { mode, .. }` holds the allow answer until `DaemonState::finish_plan_approval` has set the mode (R7), which then broadcasts `PlanApproved { mode }` with the mode the session has |
| `withdraw_session(session, reason) -> Vec<DaemonMsg>` | removes every entry of the session; each waiting call answered deny "The turn ended before you answered." |
| `withdraw(id, AgentHungUp)` | the one entry; nothing is answered |
| `replay() -> Vec<DaemonMsg>` | one `PermissionRequested` per pending entry, oldest first, for a client's handshake |

State transitions of one request: `Pending → Allowed | AllowedAlways | Denied | PlanApproved |
KeptPlanning | Answered | Declined | Withdrawn`; terminal on the first transition, never back.
Not persisted; empty after a daemon restart (FR-011).

## Stream-json additions (core `structured::event`, `parse`)

| Variant / field | Source line | Note |
|---|---|---|
| `Init.permission_mode: Option<String>` | `system`/`init` `permissionMode` | FR-016 |
| `Status { permission_mode: Option<String>, raw }` — NEW | `system`/`status` | sent on a mode change (P-3, P-9) |
| `ControlResponse { request_id: String, outcome: ControlOutcome, raw }` — NEW | `control_response` | `ControlOutcome::Success(Value)` / `Error { message, code: Option<String> }` |

#939's transcript absorbs `Status` and `ControlResponse` (no entry); every line still makes one
event (feature 938 FR-014). `SavedEvent::Line` keeps them as raw lines, so a restored history parses
them again unchanged.

## ControlRequest (core `structured::control`) — NEW

`encode_set_mode(request_id, PermissionMode) -> String` (one line with `\n`); request ids
`micold-<n>`, per process. The daemon's `ControlWaiters` (in `StructuredSession`) maps a request id
to a `oneshot` resolved by the reader thread on the matching `ControlResponse`, bounded by
`CONTROL_ANSWER_BOUND = 10 s`; a process end resolves every waiter with "Claude stopped before it
answered.".

## Client state (`features/permission_prompts.rs`) — NEW, render-free

| Item | Rule |
|---|---|
| `sessions: HashMap<SessionId, SessionPrompts>` | per window; dropped with the session |
| `SessionPrompts.pending: BTreeMap<PromptId, Shown>` | `Shown { request, draft: Draft, sending: bool }`; a second `PermissionRequested` for a held id is ignored |
| `SessionPrompts.resolved: HashMap<String, Resolution>` | by tool use id: the card's *Allowed* / *Denied* / *Withdrawn* label, this window run only |
| `Draft` | `deny_message`, `note`, `answers: Vec<QuestionDraft { chosen: BTreeSet<usize>, other: String, other_chosen: bool }>` |
| `can_submit(id)` | every question has a chosen option or a chosen, non-blank Other (FR-008) |
| `choice(id, action) -> Option<PromptChoice>` | built from the draft; `None` when not submittable |
| `waiting(session) -> bool` | any pending: #941's composer shows "Waiting for you" (FR-009) |
| `placement(session, transcript) -> Vec<(Placement, PromptId)>` | `UnderCard(EntryKey::Tool(id))` when the transcript holds the card, else `AtEnd` |
