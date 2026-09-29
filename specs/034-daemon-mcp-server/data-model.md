# Data Model: The session service exposes an MCP server to the AI sessions it runs

**Feature**: 034-daemon-mcp-server | **Plan**: [plan.md](./plan.md)

Everything here is runtime state of the service except the two settings (TM6), which persist with
the other settings. Nothing is added to the catalog file or to any session record.

## TM1 — ToolServer (daemon, `crates/micold-daemon/src/mcp/server.rs`)

| Field | Type | Notes |
|---|---|---|
| `addr` | `SocketAddr` | `127.0.0.1:<ephemeral>`, bound once at service start (FR-001, FR-007) |
| `bindings` | `Arc<Mutex<Credentials>>` | TM2 |
| `config_dir` | `PathBuf` | `<data_dir>/mcp`, owner-only (R7) |
| `confirmations` | `Arc<PendingConfirmations>` | TM5 |

Lives for the service's lifetime; set on `DaemonState` like the hook receiver (`OnceLock`). Absent
when binding fails: sessions then start unbound and the log says why (FR-005).

## TM2 — Credentials (daemon)

`HashMap<Credential, SessionId>` plus the reverse `HashMap<SessionId, Credential>`.

- `Credential(String)`: 32 hex chars from `Uuid::new_v4()`; compared as a whole string; never logged.
- **Issue**: `credential_for(session)` is idempotent while the session exists (a respawn reuses it).
- **Revoke**: on `delete_session` and when the session leaves the catalog. Service exit drops the map.
- **Invariant**: one credential ↔ one session (FR-006). A lookup miss is `401` with an empty body.

## TM3 — Binding (core seam, `crates/micold-core/src/provider.rs`)

A new provider method returns how a CLI takes a tool server, so no code outside `provider.rs`
matches on which CLI a session runs (feature 026's rule).

```text
enum ToolServerSupport {
    McpConfigArg,          // claude: --mcp-config <file> --allowedTools mcp__micold
    AdditionalMcpConfig,   // copilot: --additional-mcp-config @<file> --allow-tool micold
    Unsupported { reason: &'static str },   // pi: "Pi has no MCP support"
}
```

A second provider method, `input_readiness() -> InputReadiness { HookSessionStart,
ExtensionEvent(&'static str), OutputSettled }`, says how a fresh session shows it is ready for its
first prompt (FR-017, research R12): Claude `HookSessionStart`, Pi `ExtensionEvent("session_start")`
(falling back to `OutputSettled` when the component is declined), Copilot `OutputSettled`. The live
session carries a `ready: bool` set by that signal; it never touches the activity FSM.

`BindingPlan { args: Vec<OsString>, config_file: PathBuf }` is what the spawn path appends, built
from `ToolServerSupport`, the endpoint URL and the credential. `BindingOutcome = Bound(BindingPlan)
| Skipped(SkipReason)` where `SkipReason ∈ { Disabled, Unsupported(reason), NameTaken(path),
ServerUnavailable, WriteFailed(io error text) }`, each logged once per spawn (FR-005).

## TM4 — Caller and scope (core, `crates/micold-core/src/mcp/`)

`Caller { session: SessionId, project: PathBuf, location: SessionLocation, provider: AiCli }`,
resolved per request from the credential and the catalog. `location == Default` drives FR-015a.
Every target is resolved **inside `project` only**; anything else is `NotFound` (FR-010).

`WorktreeRef` = the worktree's directory name, or the literal `default`. `SessionRef` = the
session's UUID string.

## TM5 — PendingConfirmation (daemon, `crates/micold-daemon/src/mcp/confirm.rs`)

| Field | Type | Notes |
|---|---|---|
| `id` | `ConfirmationId(u64)` | monotonic per service run |
| `caller` | `SessionId` + its display label | named in the prompt |
| `operation` | `ConfirmedOp` | `DeleteWorktree{…}`, `DeleteSession`, `StopSession`, `InterruptSession`, `SendInput` |
| `target` | `ConfirmTarget` | `Worktree{project, dir_name, display}` or `Session{id, label}` |
| `deadline` | `Instant` | created + 60 s |
| `answer` | `oneshot::Sender<ConfirmOutcome>` | taken by the first resolver |

`ConfirmOutcome = Allowed | Declined | TimedOut | NoWindow | TargetGone | Abandoned`.

State transitions (one-way; the first transition wins, later ones are no-ops):

```text
created ──(no client connected)──▶ NoWindow        → "needs confirmation"
created ──broadcast──▶ pending ──answer allow──▶ Allowed    → perform the operation
                              ├─answer deny───▶ Declined   → "refused by policy"
                              ├─60 s──────────▶ TimedOut   → "needs confirmation"
                              ├─target/caller deleted, caller stopped ─▶ TargetGone → "not found"
                              └─agent's HTTP connection closed ─▶ Abandoned  → nothing performed, no reply
every exit from pending broadcasts ConfirmationWithdrawn{id}
```

The prompt text never contains the text of a `send_session_input` (FR-018 spirit; the prompt names
the operation and target only).

## TM6 — Settings (core `Settings`, persisted)

| Field | Type | Default | Read | Req |
|---|---|---|---|---|
| `tool_server_enabled` | `bool` | `true` | at spawn | FR-004 |
| `cross_session_access` | `CrossSessionAccess { Auto, ConfirmEachSend, Off }` | `Auto` | per request | FR-016 |

Both use `#[serde(default = …)]` in `StoredSettings`, so an older settings file loads with the
defaults.

## TM7 — Operation request and outcome (core, `crates/micold-core/src/mcp/`)

- `Operation`: one variant per row of the spec's *Operations* table, with typed, validated inputs
  (e.g. `ReadSessionOutput { target: SessionRef, lines: LineCount }` where `LineCount` is 1..=2000
  after clamping; `SendSessionInput { target, text: NonEmptyText }`).
- `OpError { category: ErrorCategory, message: String }`, `ErrorCategory ∈ { NotFound,
  InvalidInput, Conflict, RefusedByPolicy, NeedsConfirmation, ServiceError }` (FR-013), serialised
  as snake_case strings.
- `PolicyDecision` from the pure `policy::decide(&Caller, &Operation, &TargetFacts,
  CrossSessionAccess) -> Proceed | Confirm(ConfirmedOp) | Refuse(OpError)`, which encodes FR-014,
  FR-015, FR-015a and FR-016 in one table-tested function.
- Audit line (FR-018): `tracing::info!(target: "micold::mcp", caller = %id, op = name, target =
  %ref, outcome = category)`. The fields are fixed; no input text is ever a field.

## Relationships

```text
DaemonState ─1──1─ ToolServer ─1──*─ Credential ─1──1─ Session (catalog)
Session ─*──1─ Project ;  Caller = (Session, Project)
ToolServer ─1──*─ PendingConfirmation ─*──1─ Caller, ─*──1─ Target (Session | Worktree)
Settings.tool_server_enabled ─gates─▶ Binding (at spawn)
Settings.cross_session_access ─gates─▶ read_session_output / send_session_input (per request)
```
