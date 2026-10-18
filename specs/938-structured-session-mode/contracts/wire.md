# Contract: client ↔ daemon additions (protocol 52)

`PROTOCOL_VERSION` 51 → 52. Every new field is `#[serde(default)]`. Control messages stay JSON.

## Changed

| Message | Change |
|---|---|
| `ClientMsg::SessionCreate` | `+ kind: SessionKind` (default `Terminal`). `Structured` with a non-Claude provider → `OperationError` with the refusal sentence, no record written (FR-003). |
| `SessionSummary` | `+ kind: SessionKind` (FR-020). |

## New, client → daemon

```text
SessionUserMessage { req: u64, session: SessionId, text: String }
```

Reply: `OperationOk` when written to the CLI; `OperationError { message }` for `Blank`,
`NotRunning`, `TurnInProgress`, `NotStructured`. The text is never logged.

## New, daemon → client

```text
StructuredEvents { session: SessionId, omitted: u64, events: Vec<SequencedEvent>, live: bool }
```

- Sent to a client that views the session's `Primary` (`SetViewedTerminals`). First the history:
  one or more `live: false` messages (the first carries `omitted`; a client starts a fresh log on
  the first `live: false` after a `live: true` or on first view), then each new event as
  `live: true` with one event. Seqs strictly increase; no seq is sent twice to one view (FR-017).
- A message's encoded size stays under `MAX_FRAME_LENGTH / 2`; an event that alone exceeds it is
  sent with its text replaced by a size notice (logged by the daemon).

```text
SessionRequestRefused { session: SessionId, request: RefusedRequest, reason: String }
```

Sent to the requesting client only, for `SessionInput`, `SessionResize`, `ScrollbackRequest` or
`SessionInterrupt` addressed to a structured session's primary (FR-019). Requests to its shell
instances (`process: Some(Shell(_))`) are served as today.

## Unchanged

Lifecycle, `SessionExited`, `CatalogChanged`, `SessionStart`/`Stop`/`Kill`/`Restart` behave the
same for both kinds (FR-009).
