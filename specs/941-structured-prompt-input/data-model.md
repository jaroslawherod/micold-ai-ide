# Data Model: Structured Session Prompt Input (#941)

Core types are render-free, in `micold-core` (`structured::{input, attachment, control,
user_message, turn}`, `structured::transcript`, `terminal_history::store`). Daemon state is in
`micold-daemon/src/state.rs`; window state in `micold-client/src/features/composer.rs`. Wire shapes:
[contracts/wire.md](contracts/wire.md).

## Core

### TurnState (changed, `structured/turn.rs`)

Per process, held by `StructuredSession` (research R1).

| Variant | Meaning |
|---|---|
| `Idle` | No turn open. A new process starts here. |
| `InTurn { turn: TurnId, interrupt: InterruptAsk, asks: u32 }` | A message was written; its `result` has not arrived. `asks` counts the interrupts written in this turn (the ask epoch; daemon-side, not on the wire). |

Beside the variant, `last_ended: Option<TurnId>`: set by the reader when it ends a turn, read under
the state lock by a late interrupt (R4).

`InterruptAsk`: `None` → `Asked` (an interrupt was written) → `Overdue` (10 s passed, FR-015);
`Overdue` → `Asked` on a new interrupt. Any → turn ends.

| Method | Rule |
|---|---|
| `accept(turn: TurnId, running) -> Result<(), UserMessageRefusal>` | `NotRunning` if not running; `TurnInProgress` if `InTurn` (now only an internal answer: `SessionInput::submit` queues on it); else opens `InTurn { turn, None }`. Blank checking moves to `SessionInput::submit` (a message with images and no text is not blank). |
| `ask_interrupt(turn) -> InterruptOutcome` | `Write` when `turn` is the open turn and its ask is `None` or `Overdue` (→ `Asked`); `AlreadyAsked` when `Asked`; `NotThisTurn` otherwise (FR-016). |
| `overdue(turn, ask_epoch) -> bool` | `Asked` of the same turn with `asks == ask_epoch` → `Overdue`, so the timer of an earlier ask never marks a later one. |
| `on_event(&Result)` → `Idle`, returning the ended turn and whether an interrupt was asked. |
| `on_exit()` | As today: `TurnEndedWithoutResult` when a turn was open. |

`TurnId(u64)`: per session, increasing across processes (from `SessionInput::next_turn`).

### SessionInput (new, `structured/input.rs`)

Per session, held by the daemon in `DaemonInner.structured_inputs` (R1). Never shared between
sessions (FR-031).

| Field | Type | Notes |
|---|---|---|
| `queue` | `VecDeque<QueuedMessage>` | Order is delivery order (FR-018, FR-019). |
| `hold` | `bool` | R6. Invariant: `hold` implies `!queue.is_empty()`. |
| `next_turn` | `TurnId` | Next turn's id. |
| `next_entry` | `u64` | Per-session counter behind `QueueEntryId` |

| Method | Returns / effect | Requirement |
|---|---|---|
| `submit(message, busy: bool, running: bool)` | `Deliver(TurnId)`, `Queued(QueueEntryId)` or a refusal, by the rule below | FR-003, FR-004, FR-018, FR-022, FR-032 |
| `pump(turn: &TurnState)` | `Some(head)` popped when `turn` is `Idle` now and `!hold`; else `None`. The caller opens the turn in the same lock section. Called at the end of every lock section that changes the queue, the hold or the turn, and by the driver. Never sets `hold`. | FR-019, FR-022, R2 |
| `on_interrupt(turn)` | sets `hold` if the queue is not empty | Q2 |
| `on_process_end()` / `on_process_start()` | set `hold` if the queue is not empty | FR-022, Q1 |
| `release()` | clears `hold` (`Released`), or `NothingHeld` when it was not held; the `pump` that follows delivers the head when idle, so two releases deliver once | FR-022 |
| `edit(id, text, keep)`, `remove(id)`, `take(id)` | `Err(QueueRefusal::Gone)` when `id` is not queued; `edit` keeps the position; `take`/`remove` clear `hold` when the queue empties | FR-020 |
| `put_back(message, reason)` | re-inserts a failed delivery at the head and sets `hold` | edge case *Refusal or delivery failure* |
| `snapshot()` | `Vec<QueuedSummary>`, `hold` | FR-018, FR-021 |

`submit` rule, in order: blank (no non-whitespace text, no image) → `Err(Blank)` (FR-004); not
running → `Err(NotRunning)` (FR-006; a not-running session's composer does not send, the queue is
fed only by a running session); busy → `Queued`; idle and the queue is non-empty and not on hold →
`Queued` (behind it; the `pump` that ends the same lock section delivers the head); otherwise (idle, queue empty or on hold) → `Deliver` with a new `TurnId`
(Q5).

### QueuedMessage / QueuedSummary / QueueEntryId

| Type | Fields |
|---|---|
| `QueueEntryId(u64)` | unique within the session for the daemon run; restored entries get new ids |
| `QueuedMessage` | `id`, `text: String`, `images: Vec<ImageAttachment>`, `queued_by: u64` (the daemon's client id, for logs only) |
| `QueuedSummary` (wire) | `id`, `text` (whole; the window cuts it to one line), `attachments: Vec<AttachmentMeta>` |

### ImageAttachment / AttachmentMeta / ImageMedia (new, `structured/attachment.rs`)

| Type | Fields / values | Rules |
|---|---|---|
| `ImageMedia` | `Png`, `Jpeg`, `Gif`, `Webp` | `sniff(&[u8]) -> Option<ImageMedia>` by magic bytes; `mime()` gives `image/png` … |
| `ImageAttachment` | `name: String` (file name only), `media`, `bytes: Vec<u8>` | `bytes.len() <= MAX_IMAGE_BYTES` (3,750,000) |
| `AttachmentMeta` | `name`, `media`, `size: u64` | what windows other than the sender see |
| `AttachmentRefusal` | `Unreadable { name, why }`, `TooLarge { name, size }`, `TooMany { name }`, `MessageTooLarge { name }` | `Display` names the file and why (FR-029) |
| `DropPlan` | `images: Vec<PathBuf>`, `paths: Vec<PathBuf>` | `classify_drop(paths, sniff)` in drop order; folders and non-images → `paths` (FR-028) |

Limits: `MAX_IMAGES_PER_MESSAGE = 20`; `MAX_MESSAGE_IMAGE_BYTES = 9 MiB` (sum per message, so the
`SessionUserMessage` frame, JSON with the bytes in base64, fits `MAX_FRAME_LENGTH`). `admit(draft_images, candidate) ->
Result<(), AttachmentRefusal>` checks all three in that order.

### Encoded lines (changed / new)

- `encode_user_message(text, images) -> String`: one stream-json line, content `[text?, image…]`
  (text block omitted when blank, FR-027 AS6), base64 by the workspace `base64` crate.
- `control::encode_interrupt(request_id) -> String`: P-4's line.

### StructuredEvent (two new daemon notes)

| Variant | Pushed when | Transcript |
|---|---|---|
| `UserSent { text: String, attachments: Vec<String> }` | the message was written to stdin | `EntryKind::UserMessage { text, attachments }` (FR-003, FR-030) |
| `InterruptAsked` | an interrupt request was written | none; marks the open turn |

`text_len`: `UserSent` counts `text`; `InterruptAsked` 0. `EVENTS_FORMAT_VERSION` stays 1: the
variants are appended, so files written before decode unchanged; a file with them is refused by an
older daemon as damaged (938 FR-021c), which only a downgrade meets.

### Transcript (changed, `structured/transcript/`)

- `EntryKind::UserMessage { text, attachments: Vec<String> }` (was `{ text }`).
- `EntryKind::Interrupted` — closes a turn in place of `Footer` when the turn holds `InterruptAsked`
  or the CLI's `[Request interrupted by user]` text, which is absorbed (R4, FR-014).
- `Transcript::user_messages() -> impl Iterator<Item = &str>` — the history, oldest first (FR-025).

### SavedQueue (new, `terminal_history/store.rs`)

`QUEUE_FORMAT_VERSION = 1`; `{ entries: Vec<(text, Vec<ImageAttachment>)> }` in postcard;
`save_queue(id, entries)`, `load_queue(id) -> QueueLoad::{Queue, None, Damaged(reason)}`, removed
with the session's history files and when the queue empties (R9). Restored on hold.

## Daemon

| Where | What |
|---|---|
| `DaemonInner.structured_inputs` | `HashMap<SessionId, SessionInput>`; created with the event stream, removed with it (session delete/forget) |
| `StructuredSession` | `turn: Arc<Mutex<TurnState>>` (changed type), `on_turn_end` callback → `input_feed` |
| `input_feed` | `tokio::sync::mpsc` of `TurnClosed { session, turn, by: Result \| Exit, interrupted }` (named apart from `ActivityEvent::TurnEnded`); one driver task |
| interrupt timer | `tokio::time::sleep(INTERRUPT_OVERDUE = 10 s)` per ask; on fire `TurnState::overdue` |

Lifecycle of one queued message:

```text
submit (busy) ──► Queued ──edit──► Queued (same place)
                    │  └─remove──► gone (never delivered)
                    │  └─take────► gone (back in one window's composer)
          result & !hold
                    ▼
               Delivering ──write ok──► delivered (UserSent note, turn opens)
                    └─write failed──► head of queue again, hold
```

Hold:

```text
          process ends / starts with a queue / user interrupt
 not held ─────────────────────────────────────────────► held
    ▲                                                     │
    └────────── QueueRelease (once) / queue empties ──────┘
```

## Window (`features/composer.rs`)

| Type | Fields |
|---|---|
| `Composers` | `HashMap<SessionId, Composer>` (per window, FR-007) |
| `Composer` | `draft: Draft`, `history: Option<HistoryWalk>`, `pending: Option<(u64, Draft)>`, `notice: Option<Notice>`, `editing: Option<QueueEditDraft>`, `remote: Option<InputState>` |
| `Draft` | `text`, `caret: (line, column)`, `images: Vec<ImageAttachment>`; `is_sendable()` = non-whitespace text or an image (FR-004) |
| `HistoryWalk` | `index` into `Transcript::user_messages()`, `saved: Draft` (FR-024) |
| `InputState` | the last `StructuredInput`: `phase`, `hold`, `queue` |
| `KeyIntent` | `Send`, `NewLine`, `HistoryBack`, `HistoryForward`, `Interrupt`, `Edit` (let the editor have it) |

Derived for the view: `can_send` (connected, running, sendable, no pending send), `offers_stop`
(phase `Busy`), `offers_send_queued` (hold and running), `overdue` (interrupt `Overdue`),
`can_take(entry)` (draft empty, FR-020).
