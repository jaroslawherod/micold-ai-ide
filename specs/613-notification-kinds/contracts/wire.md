# Contract: wire protocol additions

**Feature**: 613 | Types in `crates/micold-core/src/protocol/messages.rs`. One bump, `PROTOCOL_VERSION`
29 → 30, in the first milestone that ships any of these; a later milestone that changes the wire
again takes the next number (039 R10). No `#[serde(default)]` on wire types.

```rust
// micold_core::attention
#[serde(rename_all = "snake_case")]
pub enum NotificationKind { NeedsPermission, SessionError, LongTaskFinished, TurnFinished }

// DaemonMsg — changed
AttentionGranted { session: SessionId, seq: u64, kind: NotificationKind },
// DaemonMsg — new
SessionErrorNotice { project: PathBuf, session: SessionId },

// DaemonSettings — new field
pub notification_kinds: NotificationKinds,
// ClientMsg::SettingsSet — new field (`None` leaves it unchanged)
notification_kinds: Option<NotificationKinds>,
```

| # | Rule |
|---|---|
| W5.1 | `AttentionGranted.kind` is the kind the service decided when it noted event `seq` (contract [classification.md](./classification.md) C10). It is never `SessionError`. |
| W5.2 | `SessionErrorNotice` is sent to exactly one connection per error ending, under C13–C14, and never answered. It is not an operation (no `req`). |
| W5.3 | `notification_kinds` is stored as `Settings::notification_kinds`, written by `persist_service_settings` and pushed with `SettingsChanged` to every window, as `desktop_notifications` is (039 W4.1). |
| W5.4 | `SettingsSet.notification_kinds` replaces all four values at once; the client sends its whole draft value. |
| W5.5 | `AttentionClaim`, `WindowView`, `SessionReveal`/`RevealSession`, `attention_seq` and `unread` are unchanged (039 W1–W3). |
