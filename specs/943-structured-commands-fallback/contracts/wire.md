# Contract: client ↔ daemon additions (#943)

Every new field is `#[serde(default)]`. `PROTOCOL_VERSION` is bumped by one in each milestone that
changes the wire (M1: the `StructuredEvent` additions; M4; M6), each merging to `main` on its own.
A concurrent bump from #941/#942/#944 is a rebase conflict on `PROTOCOL_VERSION` and the
`schema_hash.rs` pin, resolved by taking the higher number + 1 (#941 I-6).

## Events (M1)

`StructuredEvent` gains `CompactBoundary`, `ConversationReset`, `CommandsReported`, `ModelChanged`,
`ModelRefused`, `KindSwitched` and the fields of [data-model.md](../data-model.md) *Events*. They travel in the
existing `StructuredEvents` frames and handshake replay; the command list and the session model
reach a window only this way (no separate message).

## Changed (M4)

| Message | Change |
|---|---|
| `SessionSummary` | `+ model: ModelChoice` (FR-013, FR-017) |

## New, client → daemon

```text
SetSessionModel   { req: u64, session: SessionId, model: ModelChoice }            (M4)
SessionSwitchKind { req: u64, session: SessionId, to: SessionKind,
                    command: Option<String>, confirmed: bool }                    (M6)
```

### `SetSessionModel` (FR-014–FR-017)

- As #942's `SetPermissionMode`: for a running, **idle** session it is answered after the CLI's
  answer (≤ the 10 s control bound): `OperationOk` with a `ModelChanged` event in the session's
  events, or `OperationError` with the CLI's text and the summary's `model` back at the previous
  choice (FR-016). For a **busy** session it is answered `OperationOk` once the choice is persisted
  and broadcast; the CLI's answer comes at the turn's end as a `ModelChanged` or a `ModelRefused`
  event, which every window shows (FR-015, FR-016). Not running: `OperationOk`, persisted only.
- `OperationError` for: unknown session, a terminal-kind session ("Model switching is for
  structured sessions; the choice is used at the next structured launch" — the choice **is**
  persisted, FR-017), an invalid `ModelName`.

### `SessionSwitchKind` (FR-019–FR-025)

- `OperationOk` when the switch completed (the process of the new kind started or failed
  to resume, US5 scenario 7), or when the session already has kind `to`, or when a switch to `to`
  is already running.
- `OperationError { kind, message }` with `kind` one of (`ErrorKind` gains `NeedsConfirmation`):

| Refusal (`kind`) | When | Window |
|---|---|---|
| `NotClaude` (`Refused`) | provider is not Claude Code (FR-019) | not offered; shown if sent anyway |
| `NeedsConfirmation` (`NeedsConfirmation`, message = the reason) | not `confirmed` and a turn is in progress, a #942 request is pending, or the terminal CLI's activity is `Working` (FR-021) | shows the confirmation dialog, resends with `confirmed: true` |
| `SwitchInProgress` (`Refused`) | a switch to the other kind is running (FR-025) | shows the text |
| `BadCommand` (`InvalidInput`) | `command` is set and `to` is `Structured`, or it is not a terminal-only command | — |

- The new kind reaches every window in the next catalog snapshot (`SessionSummary.kind`).

## Changed behaviour (M3)

`SessionUserMessage` whose text `classify_draft` reads as `Model(_)` or `TerminalOnly(_)` →
`OperationError` ("/model is handled by the app" / "/<name> needs the terminal"), nothing written,
nothing queued (FR-010, FR-018, SC-006). While a switch runs, a `SessionUserMessage` is queued on
hold by #941's `SessionInput` (FR-025; M6).
