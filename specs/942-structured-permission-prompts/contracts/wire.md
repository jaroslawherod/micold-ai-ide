# Contract: client ↔ daemon additions

Every new field is `#[serde(default)]`; control messages stay JSON. `PROTOCOL_VERSION` is bumped by
one in each milestone that changes the wire (M1: the `StructuredEvent` additions, 57 → 58; M2, M4,
M5), each merging to `main` on its own.

## Changed (M4)

| Message | Change |
|---|---|
| `SessionSummary` | `+ permission_mode: PermissionMode` (FR-015) |
| `ClientMsg::SessionCreate` | `+ permission_mode: PermissionMode` (default `Default`). Non-default with `kind: Terminal` → `OperationError`, no record (FR-018). |

## New, daemon → client (M2)

```text
PermissionRequested { request: PermissionRequest }
PermissionResolved  { session: SessionId, id: PromptId, tool_use_id: Option<String>, resolution: Resolution }
```

- Sent to **every** connected client. A client completing its handshake while requests are pending
  is sent one `PermissionRequested` per pending request, under the same lock as its registration,
  so a request opened meanwhile reaches it exactly once (the `ConfirmationRequested` rule, FR-005).
- Every request that is sent is later resolved exactly once by `PermissionResolved`, unless the
  daemon stops first (then the connection ends; nothing is restored, FR-011).
- `PermissionInput { req, session, id }` → `PermissionInputText { req, text }` with the whole
  input JSON, or `OperationError` when the request is no longer pending (R10).

## New, client → daemon

```text
PermissionDecision { req: u64, session: SessionId, id: PromptId, choice: PromptChoice }   (M2)
SetPermissionMode  { req: u64, session: SessionId, mode: PermissionMode }                   (M5)
```

| Reply | When |
|---|---|
| `OperationOk { req }` | decision returned to the CLI / mode applied (running: the CLI answered `success`; not running: persisted) |
| `OperationError { req, message }` | a `DecisionRefusal` (FR-012); mode: not structured, CLI's `error` text, no answer within 10 s (FR-017) |

The deny message, the note and answers are never logged.

## Unchanged

`ConfirmationRequested`/`ConfirmationAnswer` (feature 034) and every terminal-session message.
`StructuredEvents` carries the new `Status` and `ControlResponse` events as any other event.
