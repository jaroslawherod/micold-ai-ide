# Research: Structured Session Prompt Input (#941)

Every decision below names what was chosen, why, and what was rejected. Probes **P-1…P-6** were run
on 2026-10-21 against Claude Code 2.1.306 (`claude -p --input-format stream-json --output-format
stream-json --verbose --include-partial-messages`, model haiku) from a scratch directory; their
lines are recorded as fixtures in M1 (R13).

## Probes

| ID | What was sent | What the CLI did |
|---|---|---|
| P-1 | A user message, no `--replay-user-messages` | Nothing on stdout repeats the message (as every #938 fixture shows): the transcript has no user entry unless the daemon records one |
| P-2 | Same, with `--replay-user-messages` | `{"type":"user",…,"isReplay":true}` when the CLI *starts* that message's turn; an image block is replayed with its whole base64 `data` |
| P-3 | A second user message while the first turn streamed | Accepted silently and run as its own turn after the first one's `result` (the CLI has its own queue) |
| P-4 | `{"type":"control_request","request_id":"micold-1","request":{"subtype":"interrupt"}}` mid-turn | `control_response` `success` (`response: {"still_queued":[]}`) at once; then the partial `assistant` message, a `user` line with text `[Request interrupted by user]` (no `isReplay`), and a `result` with `subtype: "error_during_execution"`, `is_error: true`, `terminal_reason: "aborted_streaming"`; the process kept running and took the next message |
| P-5 | A user message whose content is `[{"type":"text",…},{"type":"image","source":{"type":"base64","media_type":"image/png","data":…}}]` | Answered about the picture; the turn ended `success` |
| P-6 | (P-3 again) the message sent during the interrupted turn | Ran after the interrupted turn ended, although the interrupt answered `still_queued: []` |

P-3 and P-6 decide R2: a message written to the CLI during a turn is run by the CLI whatever the
daemon's queue says, so nothing may be written to stdin while a turn is open except control
requests.

## R1. Where the turn, the queue and the hold live

- **Decision**: the per-process turn stays where #938 put it, `TurnState` inside
  `StructuredSession` (reset by every spawn, ended by the stdout reader on a `result` and at
  end-of-file, so `TurnEndedWithoutResult` keeps its place after the last line). The queue, the
  hold and the turn counter are per **session**, not per process: a new
  `micold_core::structured::input::SessionInput` kept in `DaemonInner.structured_inputs:
  HashMap<SessionId, SessionInput>` beside `event_streams`, so they outlive a process end and a
  restart and go with the session's removal. Every decision (deliver now, queue, which entry next,
  hold, release) is a pure method of `SessionInput` taking the turn's state as an argument; the
  daemon calls it under its state lock and does the writes off the lock (#938 R7's split).
- **One lock order for the turn**: the reader ends the turn on `TurnState`'s own mutex, outside the
  state lock; `TurnState` also records `last_ended: Option<TurnId>` there. Every daemon path that
  decides on the turn (a send, the driver, a release, an interrupt, a queue edit) reads `TurnState`
  *under the state lock*, as `accept_user_message` does today, and opens a turn in the same lock
  section that read it idle. So no decision acts on a turn that ended or opened behind its back.
- **Rejected**: moving `TurnState` into `DaemonInner` — the reader would have to take the state
  lock to end a turn and to push the end-of-file note, and `teardown` could return before the note
  was pushed, so the final events save (#938 R14) would miss it. **Rejected**: the queue inside
  `StructuredSession` — it would die with the process (FR-021, US4 AS4). **Rejected**: a queue per
  window — clarification Q3.

## R2. Delivery: one message per turn, the daemon decides

- **Decision**: `SessionUserMessage` no longer refuses `TurnInProgress`. `SessionInput::submit(draft,
  turn, running)` returns `Deliver` (idle, and the queue is empty or on hold — Q5) or `Queued
  { entry }` (busy, or idle behind a queue that is not on hold, so a send never overtakes the
  queue at the turn boundary, edge case *Sending at the turn boundary*, FR-032); `Blank`,
  `NotRunning` and `NotStructured` stay refusals. When the stdout reader sees a `result`, it ends the
  turn as today and calls the spawn's new `on_turn_end` callback, which sends `TurnClosed { session,
  by: Result | Exit }` on a channel to one daemon task (the *input driver*, like the plan-usage
  feed of #944). The driver, under the lock, calls `SessionInput::pump(turn)`: it pops the head only
  when `TurnState` is `Idle` **now** and the queue is not on hold, and opens the turn in the same lock
  section, then writes off the lock. Every other lock section that changes the queue, the hold or
  the turn (a send, a release, an edit, a take) ends with the same `pump`, so after any of them the
  state is never idle with a queue that is neither empty nor held; the driver's wake-up only closes
  the gap between the reader ending a turn and the next lock section. A send that arrives in that
  gap finds the turn idle and the queue waiting, is queued behind it, and the `pump` of the same
  section delivers the head (edge case *Sending at the turn boundary*); a failed write puts the entry back at the head with the reason (edge case *Refusal or
  delivery failure*) and holds the queue.
- **A newer turn already open**: if a send opened turn t2 before the driver handled t1's end,
  `pump` sees `InTurn` and does nothing; the queue waits for t2's `result` (FR-019: one message per
  turn; P-3: nothing written during a turn).
- **Two windows at once** (FR-032): both `SessionUserMessage`s reach `submit` under the state lock in
  arrival order; the first opens the turn, the second sees `InTurn` and is queued.
- **Rejected**: writing queued messages to the CLI and letting its own queue order them (P-3) —
  the daemon could then neither edit, remove nor hold an entry once written, nor show one queue to
  every window. **Rejected**: merging queued messages into one — spec *Assumptions*.

## R3. The user's message in the transcript and the history

- **Decision**: a new daemon note `StructuredEvent::UserSent { text, attachments: Vec<String> }`
  (attachment file names only) is pushed to the session's event stream when the message is written
  to stdin, before any line of its turn can arrive. #939's builder turns it into
  `EntryKind::UserMessage { text, attachments }`; the input history (FR-025) is the
  `UserMessage` entries of the transcript, oldest first, so it includes messages from other windows
  and messages restored from the saved events, and never a refused one (never written, never
  noted). `text_len` counts the text, so the history's byte bound (#938 R5) holds.
- **Rejected**: `--replay-user-messages` (P-2) — every image's base64 would sit in the event
  history, on the wire to every window and in the saved events file (a 4 MB screenshot per message),
  and the 938 argv assertion would change. **Rejected**: the client adding its own entry — a second
  window would not see it (FR-003).

## R4. Interrupt

- **Decision**: a new `ClientMsg::TurnInterrupt { session, turn: TurnId }` naming the turn the
  window saw busy. `SessionInterrupt` keeps its terminal meaning and #938's refusal for a structured
  primary (FR-017; 938's tests unchanged). Under the lock the daemon checks the named turn is the
  open one: if so and no interrupt is pending, the turn's interrupt becomes `Asked`, the queue goes on
  hold (Q2), and off the lock it writes `control::encode_interrupt(request_id)` (P-4's line) and
  pushes a daemon note `StructuredEvent::InterruptAsked`. A second interrupt for the same turn while
  `Asked` does nothing (FR-016); one while `Overdue` writes the request again and returns to `Asked`
  (FR-015). An interrupt naming a turn that already ended changes nothing, except that it still puts
  the queue on hold when it names `TurnState.last_ended` (read under the state lock, so it is
  current even before the driver woke) (edge case *Interrupt racing the turn's
  end*) — it never interrupts a later turn the queue started meanwhile.
- **Overdue** (FR-015): the daemon arms a 10 s timer per ask; when it fires and the same turn is
  still `Asked`, the turn's interrupt becomes `Overdue` and every window is told. The window shows
  the notice and the existing **Stop session** action; nothing is stopped by the daemon.
- **Transcript** (FR-014): a `Result` closes the turn with `EntryKind::Interrupted` in place of the
  footer when the turn carries an `InterruptAsked` note or the CLI's `[Request interrupted by
  user]` user text (P-4); that text is absorbed, not shown as a user message nor counted in the
  history. A turn the CLI ended on its own after an ask still closes with its own footer when
  neither marker is in it.
- **Withdrawal of #942 requests** (US3 AS6): already covered by #942 R4 — the interrupted turn's
  `result` withdraws its pending requests.
- **Rejected**: a signal to the process (SIGINT) — ends `-p` mode, FR-013 needs the same process.
  **Rejected**: reusing `SessionInterrupt` without a turn — a Stop pressed as the turn ended would
  interrupt the next queued turn. **Rejected**: client-side 10 s timers — a window that starts
  viewing late, or a remote daemon's clock, would disagree.

## R5. Idle/busy for every window and the sidebar

- **Decision**: a new `DaemonMsg::StructuredInput { session, phase, hold, queue }` (contracts/wire.md)
  is broadcast to every client whenever any of it changes (by the lock section that changed it; a
  turn the reader ended is broadcast by the driver one channel hop later, well inside SC-002), and sent to a client for every structured
  session at handshake, as #942's requests are replayed (FR-009, FR-021). `phase` is `NotRunning`,
  `Idle` or `Busy { turn, interrupt: None | Asked | Overdue }`. The sidebar and the overview keep
  reading `SessionSummary.activity`: the daemon feeds `ActivityEvent::TurnBegan` when it opens a turn
  and `TurnEnded` when it ends, mapped like `UserPromptSubmit` and `Stop`; for a structured session
  the CLI's own `UserPromptSubmit` and `Stop` hooks are ignored so the turn decides (spec
  *Assumptions*), while `Notification` (#942's waiting state) still applies and wins (FR-011).
- **Rejected**: a `turn` field on `SessionSummary` — every catalog broadcast would carry the queue's
  changes and a queue edit would re-send every summary. **Rejected**: deriving busy in the window
  from the event stream — a window that starts viewing mid-turn (FR-009) must replay history first,
  and the stream does not know about holds or interrupt asks.

## R6. The queue's hold

- **Decision**: `SessionInput.hold: bool`, set when the process ends or is stopped, when a process
  starts with a non-empty queue (restart, resume, daemon restart — Q1), and when the user interrupts
  (Q2); cleared only by `ClientMsg::QueueRelease { req, session }` from any window, under the lock,
  so two releases release once (FR-022). Release while idle delivers the head at once; release while
  busy waits for the turn's result. A `result` never sets the hold. An empty queue's hold is cleared
  with its last entry, so an empty queue never shows **Send queued**.
- **Changing Q1/Q2 later** (ledger Decisions 3–4 are pending the user): auto-resume after a restart
  is one line in `SessionInput::on_process_start`; "deliver next at once after an interrupt" is one
  line in `on_interrupt`; moving the queue into one composer is a client-only change (QueueTake of
  every entry). None touches the wire.

## R7. Queue entries and their edits

- **Decision**: `QueuedMessage { id: QueueEntryId, text, images: Vec<ImageAttachment>, queued_by:
  u64 }` (the daemon's `ClientId`, for logs); `QueueEntryId(u64)` from the session's own counter
  (`SessionInput.next_entry`), unique within the session for the daemon run. The broadcast carries
  `QueuedSummary { id, text, attachments: Vec<AttachmentMeta> }` — names and sizes, never image
  bytes, so a queue edit stays a small frame. Edits name the entry: `QueueEdit { req, session, entry,
  text, keep: Vec<u32> }` (the new text and which of its attachments stay, by index), `QueueRemove
  { req, session, entry }`, `QueueTake { req, session, entry }` (answered `OperationResult::Taken
  { text, images }` with the bytes, to that window only; refused while the requester says its draft
  is not empty is the window's rule, FR-020). An edit, remove or take of an entry that was delivered
  or removed meanwhile is refused "That message was already sent or removed" and changes nothing.
  Adding attachments to a queued entry is done by moving it back into the composer (FR-020's move
  back); in-place edits change text and drop attachments.
- **Rejected**: indexes instead of ids — two windows editing while the head is delivered would hit
  the wrong entry. **Rejected**: image bytes in every broadcast — 16 MiB frames per keystroke of an
  edit.

## R8. Attachments

- **Decision**: `micold_core::structured::attachment`: `ImageMedia::{Png, Jpeg, Gif, Webp}` sniffed
  from the file's first bytes (magic numbers, not the extension), `MAX_IMAGE_BYTES = 3_750_000` (the
  Messages API's 5 MB limit on the base64 form), `MAX_IMAGES_PER_MESSAGE = 20`, and
  `MAX_MESSAGE_IMAGE_BYTES = 9 MiB` so a message with its images stays under `MAX_FRAME_LENGTH`
  (16 MiB) with room for the text: control frames are JSON (`protocol::codec`), where an
  `ImageAttachment`'s bytes travel as base64 (4/3 of their size, 12 MiB); postcard (the saved queue)
  keeps them as plain bytes. (Corrected in M1: the plan had assumed postcard control frames and
  12 MiB.) `AttachmentRefusal::{Unreadable, TooLarge, TooMany,
  MessageTooLarge}` carry the file name and a sentence (FR-029). `encode_user_message(text, images)`
  writes P-5's content list: the text block first when non-blank, then each image in order, base64
  by the `base64` crate already in the workspace. Classification of a drop
  (`classify_drop(paths, sniff)`) splits images from paths in drop order; paths go through feature
  487's `path_insert::plan_insertion` exactly as a terminal drop (FR-026, FR-028). The client reads
  the file (the files are on the window's machine, the daemon may be remote or sandboxed).
- **Preview** (FR-027): iced's `image` feature, added to the workspace `iced` features; the window
  decodes a thumbnail from the bytes it already holds. **Rejected**: no preview — FR-027;
  **rejected**: decoding in core with the `image` crate — core stays render-free.
- **Rejected**: sending paths and letting the CLI read images — Claude Code reads a path as text
  unless told to, and a sandboxed or remote daemon does not see the window's files.

## R9. Persistence of the queue

- **Decision**: under **Save terminal history**, the session's queue (entries with their images and
  the hold, which is always `true` on restore) is saved as `<session>.queue` beside `<session>.events`
  in the same `HistoryStore` directory: postcard with a format version. Each queue change takes a snapshot under the lock with an
  increasing generation and hands it to one writer (the history saver's thread); the writer drops a
  snapshot older than one it already wrote, so two quick changes never land out of order. The file is removed when the queue empties and with the session's
  other files on delete or forget. Restored with the event stream (`event_stream_loading`), on hold.
- **Accepted**: a crash between writing the head to the CLI and saving the shorter queue restores
  that message once more, on hold and visible, so the user removes it rather than it going out twice
  unseen. The queue has no length cap (the spec sets none); a read refuses a file over
  `MAX_QUEUE_FILE_BYTES` (1 GiB) as damaged, as an events file over its bound is.
  With saving off it lives only in `DaemonInner` (Q4).
- **Rejected**: the queue inside the `.events` file — it changes independently of the events and
  would rewrite a 64 MiB history for a queue edit. **Rejected**: the catalog file — image bytes in
  the catalog.

## R10. The composer in the window

- **Decision**: render-free `features/composer.rs` holds per session: the draft (text, caret line
  and column, attachments with their bytes), the history position and the draft saved on stepping
  in, the pending send (`req` → the draft sent, restored on `OperationError` or a lost connection —
  FR-005, SC-005), the notice line, the queue entry being edited, and the last `StructuredInput` the
  daemon sent. `composer_key(press, caret, lines, composing, phase) -> KeyIntent` decides Enter
  (send; while an IME composition is open it completes it, edge case), Shift+Enter (new line),
  Up on the first line / Down on the last (history), Escape (interrupt when busy, else nothing).
  The text editor's `Content` lives in the binary (`app::Editor`, feature 686's pattern), one per
  session, so the caret survives pane and session switches (FR-007).
- **Shared components** (Principle VIII): `TextArea` gains `.key_binding(…)` and `.max_height(…)`
  (grow, then scroll inside, FR-008); new `AttachmentChip` (thumbnail, name, remove), `QueueRow`
  (position, one-line text, attachment count, Edit / Remove / Move to composer), `BusyIndicator`
  (indeterminate circular indicator plus a text label, never colour alone, FR-010), each with a
  builder, showcase entry and theme snapshot. The composer itself is `ui/composer.rs` glue.
- **Seams**: a status-row slot that #942 fills with its mode `Select` and *Waiting for you*, and a
  completion slot above the text area for #943; both empty here.
- **10,000-line draft** (FR-008, SC-007): iced's `text_editor` is a rope-backed editor; the draft's
  text is read from `Content` only on send, history stepping and line queries, never per frame.
  Measured by a release-build test that applies 10,000-line inserts and edits through
  `Editor::perform` and times each (SC-007's 100 ms), plus §B 8.
- **Rejected**: iced `text_input` (single line); a terminal-like raw key handler (loses IME,
  selection and clipboard).

## R11. Drops on a structured pane

- **Decision**: `DropTarget` gains `Composer(SessionId)` for a pane showing a structured primary;
  `shell/drops.rs` hands such a pane's files to `features::composer` (images read off the UI thread
  with `tokio::fs`, classified, attached or refused with a notice; paths inserted at the caret) and
  focuses the composer. A terminal pane's drop is untouched (FR-026, SC-008).

## R12. Cross-platform

- Enter, Shift+Enter, Up, Down and Escape carry no platform modifier; nothing differs by OS. Paths
  are formatted by `path_insert` per platform as feature 487 does. The control line and the image
  blocks are the same on every OS. The stand-in tests run on all three CI platforms.

## R13. Tests against a stand-in, with recorded lines

- **Decision**: the stand-in `claude` of `crates/micold-daemon/tests/support/structured.rs` gains
  `on-message <file>` (for each later stdin user line, replay the file: one turn per message) and
  `on-interrupt <file>` / `ignore-interrupt` (on a stdin `control_request` interrupt, replay the
  file, or do nothing so the ask goes overdue). The files are recorded fixtures:
  `interrupt-turn.jsonl` (P-4) and `image-turn.jsonl` (P-5) captured with
  `scripts/capture-stream-json.sh`, extended with an `--interrupt-after-first-delta` mode.
- **Rejected**: the real CLI in tests — network and credentials (FR-019 of #938's test rule).
