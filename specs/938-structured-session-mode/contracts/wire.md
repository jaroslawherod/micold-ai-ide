# Contract: client ↔ daemon additions (protocol 52 and on)

`PROTOCOL_VERSION` is bumped by one in each milestone that changes the wire, because each
milestone merges to `main` on its own: M2 (`kind` fields, `StructuredEvents`; 51 → 52 if `main` is
still at 51), M3 (`SessionUserMessage`), M6 (`SessionRequestRefused`). Every new field is `#[serde(default)]`. Control messages stay JSON.

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
  one or more `live: false` messages (the first carries `omitted`, the rest `0`), then each new
  event as `live: true` with one event. Seqs strictly increase within a log; no seq is sent twice
  to one log (FR-017).
- A viewer that falls behind the live channel is sent a fresh, whole snapshot, never a gap (R6).
  So a client starts a fresh log on every `live: false` message except one that continues the
  history in progress: the previous message was `live: false` and this one's first `seq` is one
  more than the last it holds. An empty `live: false` message (an empty history) is a fresh, empty
  log.
- A message's encoded size stays under `MAX_FRAME_LENGTH / 2`; an event that alone exceeds it is
  sent alone as `OversizedOnWire { original_bytes }` under its own `seq` (logged by the daemon).

```text
SessionRequestRefused { session: SessionId, request: RefusedRequest, reason: String }
```

Sent to the requesting client only, for `SessionInput`, `SessionResize`, `ScrollbackRequest` or
`SessionInterrupt` addressed to a structured session's primary (FR-019). Requests to its shell
instances (`process: Some(Shell(_))`) are served as today.

## Unchanged

Lifecycle, `SessionExited`, `CatalogChanged`, `SessionStart`/`Stop`/`Kill`/`Restart` behave the
same for both kinds (FR-009).
