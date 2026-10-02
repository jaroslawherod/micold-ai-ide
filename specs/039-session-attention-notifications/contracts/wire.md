# Contract: wire protocol additions

**Feature**: 039 | Types live in `crates/micold-core/src/protocol/messages.rs`. Each group bumps
`PROTOCOL_VERSION` once, in the milestone that ships it (research R10). No `#[serde(default)]`.

## W1 — View report, attention sequence, claim (version 21)

```rust
// SessionSummary
pub attention_seq: u64,

// ClientMsg
WindowView { focused: bool, in_view: Option<SessionId> },
AttentionClaim { session: SessionId, seq: u64 },

// DaemonMsg
AttentionGranted { session: SessionId, seq: u64 },
```

| # | Rule |
|---|---|
| W1.1 | The client sends `WindowView` once after every `Welcome`, and afterwards whenever the derived value differs from the last one sent. `in_view` is `Some` only with `focused: true`. |
| W1.2 | The service keeps one `WindowView` per connection and forgets it when the connection ends. |
| W1.3 | On a change of a session's activity into `AwaitingInput` from any other value, with no connection reporting it in view, the service adds one to `attention_seq`, persists, and the `SessionChanged` it already sends carries the new value. A repeated waiting signal changes nothing. |
| W1.4 | `AttentionClaim` is answered with `AttentionGranted` to the claimer only, when `seq` is greater than the highest sequence already granted for the session and not greater than its current `attention_seq`. Otherwise it is not answered. An unknown session is not answered. |
| W1.5 | Neither message is an operation: there is no `req`, no `OperationOk`, no `OperationError`. |
| W1.6 | `WindowView` and `AttentionClaim` need no project attachment: a window may view or claim nothing it could not already see in the catalog snapshot. |

## W2 — Unread (version 22)

```rust
// SessionSummary
pub unread: bool,
```

| # | Rule |
|---|---|
| W2.1 | The step of W1.3 also sets `unread`. |
| W2.2 | When a `WindowView` names a session whose `unread` is set, the service clears it, persists, and broadcasts `SessionChanged`. |
| W2.3 | Nothing else changes `unread`: not a change of activity, not a claim, not a grant, not the setting of W4. |

## W3 — Reveal (version 23)

```rust
// ClientMsg
SessionReveal { project: PathBuf, session: SessionId },

// DaemonMsg
RevealSession { project: PathBuf, session: SessionId },
```

| # | Rule |
|---|---|
| W3.1 | The service forwards `SessionReveal` as `RevealSession` to exactly one connection: the one attached to `project`; else the one that most recently reported `focused: true`; else the sender. |
| W3.2 | The service forwards without checking that the project or session exists. The receiving window decides (contract [desktop-notification.md](./desktop-notification.md) N5). |
| W3.3 | The service changes no session, no attachment and no stored state for a reveal. |

## W4 — Setting (version 24)

```rust
// DaemonSettings and ClientMsg::SettingsSet
pub desktop_notifications: bool,
```

| # | Rule |
|---|---|
| W4.1 | Stored as `Settings::desktop_notifications`, default `true`, written by `persist_service_settings` and pushed with `SettingsChanged`, as `tool_server_enabled` is. |
| W4.2 | While it is `false`, W1.4 grants nothing. W1.3 and W2 are unaffected. |
