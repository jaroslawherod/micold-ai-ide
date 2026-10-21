# Contract: client ↔ daemon wire additions (#941)

All in `crates/micold-core/src/protocol/messages.rs`, JSON control frames (an `ImageAttachment`'s bytes
as base64), in **one** `PROTOCOL_VERSION` bump
(M1) with its `crates/micold-core/tests/schema_hash.rs` pin. Types from
[../data-model.md](../data-model.md). `SessionInterrupt` is unchanged and still refused for a
structured primary (#938 FR-019; FR-017).

## Changed

### `ClientMsg::SessionUserMessage`

```rust
SessionUserMessage { req: u64, session: SessionId, text: String, images: Vec<ImageAttachment> }
```

| Answer | When |
|---|---|
| `OperationOk { req, result: Ack }` | delivered (written to the CLI) **or** queued; the window learns which from the `StructuredInput` that follows |
| `OperationError { kind: Refused, message }` | `Blank`, `NotRunning`, `NotStructured` (sentences of `UserMessageRefusal`), or an `AttachmentRefusal` (over the limits) |
| `OperationError { kind: IoFailed, message }` | the write failed; nothing delivered, nothing queued (the window restores the draft, FR-005) |

`TurnInProgress` is no longer answered: a busy session queues (FR-018). The text and image bytes
are never logged.

### `StructuredEvent` (inside `StructuredEvents`)

Appended variants `UserSent { text: String, attachments: Vec<String> }` and `InterruptAsked`.

## New

### `DaemonMsg::StructuredInput`

```rust
StructuredInput {
    session: SessionId,
    phase: TurnPhase,          // NotRunning | Idle | Busy { turn: TurnId, interrupt: InterruptAsk }
    hold: bool,
    queue: Vec<QueuedSummary>, // id, text, attachments: Vec<AttachmentMeta>; no image bytes
}
```

- Sent to **every** client whenever `phase`, `hold` or `queue` of a structured session changes, in
  the order the daemon made the changes (under its state lock), and to a client for every
  structured session the daemon holds input state for, right after its handshake (FR-009).
- A change made by a daemon lock section (a send, a queue edit, a release, an interrupt) is sent by
  that section. A turn the stdout reader ended is sent by the input driver, one channel hop after
  the reader, with the delivery of the next queued message if any — well within SC-002's second.

### `ClientMsg::TurnInterrupt`

```rust
TurnInterrupt { session: SessionId, turn: TurnId }
```

No answer. Effects (research R4): the named open turn with no ask → `control_request` `interrupt`
written, `InterruptAsked` note, `hold` set when the queue is not empty, `StructuredInput` with
`Asked`; already `Asked` → nothing; `Overdue` → written again, `Asked`; a turn not open → nothing,
except `hold` when it names `TurnState.last_ended`. A terminal session or unknown session → nothing.

### Queue edits

| Message | Answer | Effect |
|---|---|---|
| `QueueEdit { req, session, entry: QueueEntryId, text: String, keep: Vec<u32> }` | `Ack` / `Refused` "That message was already sent or removed" / `Refused` `Blank` when the result would be blank | text replaced, only the attachments at `keep` stay, position kept |
| `QueueRemove { req, session, entry }` | `Ack` / `Refused` (gone) | entry dropped, never delivered |
| `QueueTake { req, session, entry }` | `OperationOk { result: Taken { text, images } }` / `Refused` (gone) | entry leaves the queue; its text and images go to this client only |
| `QueueRelease { req, session }` | `Ack` (also when nothing was held) | `hold` cleared once; head delivered at once if idle |

Each change is followed by one `StructuredInput` to every client.

## Unchanged and relied on

- `SessionSummary.activity` drives the sidebar and overview (FR-011); the daemon now feeds it from
  the turn for structured sessions (research R5).
- `StructuredEvents` delivery, history replay and saving (#938).
- #942's `Permission*` messages; #942's waiting state wins over working in the sidebar.
