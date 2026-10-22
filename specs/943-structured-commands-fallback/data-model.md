# Data Model: Structured Session Commands (#943)

Names are Rust items; **new** marks an item that does not exist on `main`. Research references:
[research.md](research.md).

## Core: `micold_core::structured`

### Events (`event.rs`, `parse.rs`)

| Item | Change | From |
|---|---|---|
| `StructuredEvent::Init` | `+ slash_commands: Vec<String>`, `+ skills: Vec<String>`, `+ terminal_slash_commands: Vec<String>` (absent → empty) | R1 |
| `StructuredEvent::Assistant` | `+ model: Option<String>` (`message.model`; `"<synthetic>"` for a local answer), `+ error: Option<String>` (top-level `error`) | R4, R9 |
| `StructuredEvent::User` | `+ is_synthetic: bool` (`isSynthetic`) | R5 |
| `StructuredEvent::CompactBoundary` **new** | `{ trigger: Option<String>, pre_tokens: Option<u64>, post_tokens: Option<u64>, raw }` from `system/compact_boundary` | R5 |
| `StructuredEvent::ConversationReset` **new** | `{ trigger: Option<String>, raw }` from `type: conversation_reset` | R5 |
| `StructuredEvent::CommandsReported` **new**, daemon note | `{ commands: Vec<ReportedCommand>, models: Vec<ReportedModel> }`; never produced by `parse_line` | R1 |
| `StructuredEvent::ModelChanged` **new**, daemon note | `{ model: String }` (the resolved name the CLI answered) | R7 |
| `StructuredEvent::ModelRefused` **new**, daemon note | `{ model: String, message: String }` (a deferred change the CLI refused) | R7 |
| `StructuredEvent::KindSwitched` **new**, daemon note | `{ to: SessionKind }` | R8 |

Each new variant is wire-visible (`StructuredEvent` is in `StructuredEvents` frames): one
`PROTOCOL_VERSION` bump in M1, `schema_hash.rs` re-pinned. `ReportedCommand { name, description:
Option<String>, argument_hint: Option<String>, aliases: Vec<String>, builtin: bool }`;
`ReportedModel { value, display_name: Option<String>, description: Option<String> }`. Both are
built only by `commands::parse_initialize(&ControlOutcome)`, which drops entries with a blank or
non-string `name`/`value` and cuts descriptions to 512 bytes.

### Commands (`commands/` **new**)

| Item | Shape | Rules | FR |
|---|---|---|---|
| `CommandKind` | `Skill`, `Custom`, `BuiltIn`, `App` (`/model`) | kind rule R1 | FR-007 |
| `Command` | `name`, `kind`, `description: Option<String>`, `argument_hint: Option<String>`, `aliases`, `terminal_only: Option<&'static str>` (the reason) | | FR-007, FR-010 |
| `CommandList` | `Vec<Command>`, `source: ListSource` (`Session`, `Fallback`) | `CommandList::from_history(&[SequencedEvent])`: the latest `CommandsReported` and the latest `Init` the history holds, merged by R1 (a later launch's note replaces an earlier one, Edge Cases *Command list changes*); none → `fallback()` = `BUILT_INS` + `TERMINAL_ONLY` + `/model`, `source: Fallback` (FR-009, US2 scenario 8). Every `TERMINAL_ONLY` entry and `/model` are always present. Duplicate names of different kinds both kept (Edge Cases *Name clashes*) | FR-007, FR-009, FR-011 |
| `TERMINAL_ONLY` | `&[TerminalOnly { name, why }]`, `CHECKED_AGAINST: &str` | ≥ `config`, `mcp`, `agents`, `login`, `logout`; a command is terminal-only if its name or alias is listed or in `terminal_slash_commands` | FR-011, FR-012 |
| `BUILT_INS` | `&[&str]` known built-ins for the fallback list (`compact`, `clear`, `context`, `usage`, …) | | FR-009 |
| `DraftCommand` | `Send`, `Model(Option<String>)`, `TerminalOnly(String)` | `classify_draft(text, &CommandList)`: `/` + name (no leading space, name ends at whitespace or end) → by name/alias; `/model` with blank rest → `Model(None)`, with a name → `Model(Some)`; text not starting with `/`, `/` + space, `/` alone → `Send` | FR-001, FR-010, FR-018 |
| `CommandInvocation` | `name`, `args: String` | `parse_invocation(text)` — same name rule; args = rest as typed | FR-003 |

### Palette (`commands/palette.rs` **new**)

| Item | Rules | FR |
|---|---|---|
| `palette_query(draft, caret) -> Option<&str>` | `Some(name-so-far)` while the draft starts with `/`, has no whitespace before the caret, and the caret is within the first word; else `None` (closed) | FR-007, US2 scenarios 4, 7 |
| `filter(&CommandList, query) -> Vec<usize>` (indices into the list's commands) | R10 tiers; empty query → all, grouped Skill, Custom, BuiltIn, App, alphabetical | FR-008, SC-002 |
| `PaletteSelection` | `highlight: Option<usize>`; moves with `typeahead::move_highlight`; reset to 0 on each query change | FR-008a |
| `accept(name) -> (String, usize)` | the draft `/<name> ` and the caret after it | FR-008a |

### Model (`model.rs` **new**)

| Item | Rules | FR |
|---|---|---|
| `ModelChoice` | `Default` \| `Named(ModelName)`; serde: `Default` is skipped in the store | FR-017 |
| `ModelName` | non-blank, no whitespace, ≤ 128 bytes; `ModelName::parse(&str) -> Result<_, ModelRefusal>` | FR-014 |
| `ModelRefusal` | `Invalid`, `Refused { message, code }`, `NoAnswer` | FR-016 |
| `launch_args(&ModelChoice)` | `Default` → `[]`; `Named(n)` → `["--model", n]` | FR-017 |
| `MODEL_SHORTCUTS` | `sonnet`, `opus`, `haiku` | FR-014 |
| `session_model(&[SequencedEvent]) -> Option<String>` | the latest of `Init.model` and `ModelChanged.model` | FR-013 |

### Control (`control.rs`)

`+ encode_initialize(id)`, `+ encode_set_model(id, &ModelChoice)` (`Default` → `"model":"default"`),
`+ parse_set_model(&ControlOutcome) -> Result<String, ModelRefusal>` (R7). Daemon side: **new**
`StructuredSession::send_control(request_id, line) -> Result<ControlOutcome, ControlFailed>`, the
write-and-wait taken out of `set_permission_mode` (M3). `ControlFailed` **new**: `NoAnswer`, `Stopped`,
`Unsent`; `ModeChangeFailed` is then built from it plus `ControlOutcome::Error`.

### Transcript (`transcript/`)

| Item | Change | FR |
|---|---|---|
| `EntryKind::UserMessage` | `+ command: Option<CommandInvocation>` (from `parse_invocation` of the `UserSent` text; #941) | FR-003 |
| `EntryKind::CommandOutput` **new** | `{ text }`: a synthetic assistant text after a command user turn | FR-006 |
| `EntryKind::Compacted` **new** | `{ trigger: Option<String>, summary: Option<String> }`: a `CompactBoundary` plus the `is_synthetic` user text after it | FR-004 |
| `EntryKind::Cleared` **new** | from `ConversationReset` | FR-005 |
| `EntryKind::ModelChanged` **new** | `{ model }` from the note; `ModelRefused` → `{ refused: Some(message) }` | FR-015, FR-016 |
| `EntryKind::TerminalPeriod` **new** | from a `KindSwitched { to: Terminal }`; closed by the next `KindSwitched { to: Structured }`; one entry per period | FR-023 |
| `EntryKind::AssistantText` | `+ auth_failed: bool` (`error == "authentication_failed"`) | FR-022a |
| `Entry` | `+ era: u32`, bumped after each `Cleared` | FR-005 |

## Core: session and store

| Item | Change | FR |
|---|---|---|
| `Session.model` | `ModelChoice`, persisted; set only through `Catalog::set_model` | FR-017 |
| `Session::resume_handle()` (M4) | `Uuid`: the id in M4; `conversation` or the id from M5 on | FR-020 |
| `Session.conversation` (**provisional, M5**) | `Option<Uuid>` | FR-005a |
| `Session.kind` | now mutable through `Catalog::set_kind` only; doc comment updated (replaces 938 FR-002) | FR-019 |
| `StoredSession` | `+ model` (serde default, skip when `Default`), `+ conversation` (serde default, skip when `None`); no `schema_version` bump | FR-017, FR-005a |
| `LaunchSpec` (M4) | `+ model: ModelChoice`, `+ resume_handle: Uuid` (used instead of `session_id` for `LaunchMode::Resume`; filled from `Session::resume_handle()`) | FR-017, FR-020 |

## Wire (`protocol/messages.rs`)

See [contracts/wire.md](contracts/wire.md): `SessionSummary.model`, `SetSessionModel`,
`SessionSwitchKind`, `ErrorKind::NeedsConfirmation`.

## Daemon state

| Item | Shape | Rules | FR |
|---|---|---|---|
| `PendingModel` | per session `Option<ModelChoice>` under the state lock | replaced by each pick; written by the model task when idle or at the turn's end | FR-015, Edge Cases |
| `switching` | per session `Option<SessionKind>` (target) under the state lock | set for the whole switch; second request R8 step 3 | FR-025 |

## Client (render-free)

| Item | Where | Holds | FR |
|---|---|---|---|
| `PaletteState` | `features/command_palette.rs` **new**, per composer | `open: bool`, `query`, `rows: Vec<usize>`, `highlight` | FR-007–FR-008a |
| `ModelPickerState` | same module | `open`, typed text, items from `CommandsReported.models` or `MODEL_SHORTCUTS` | FR-014 |
| `SwitchOffer` | `features/session.rs` | `{ session, to, command: Option<String>, reason }`; confirmation step when the session is busy, has a pending request, or is `Working` | FR-010, FR-021, FR-022a |
| `palette_key(state, key) -> PaletteAction` | `features/command_palette.rs` | Up/Down move, Tab/Enter accept, Escape close; `None` when closed so #941's `composer_key` runs | FR-008a |
