# Quickstart: validating the Structured Session Prompt Input (#941)

## Part A — automated

Prerequisite: a checkout of this branch; `mise trust` once.

| Command | Proves |
|---|---|
| `mise run test-core` | `structured::{input, attachment, turn, control, user_message}`, the transcript's `UserSent` / `Interrupted` handling, `SavedQueue` round trips, and the recorded fixtures (`structured_parse` over `interrupt-turn.jsonl`, `image-turn.jsonl`) |
| `cargo test -p micold-daemon --test structured_input` | US1, US3, US4 against the stand-in (one turn per message, interrupt, overdue, queue, hold, two clients, restart, daemon restart with and without saving) |
| `cargo test -p micold-client --test features_composer` | drafts, keys, history, pending sends and their restore, queue edit state, drop classification |
| `cargo test -p micold-client --test layout_snapshot` | the composer and the new shared components fit their gates |
| `cargo test -p micold-client --release --test composer_large_draft -- --ignored` | SC-007: 10,000-line draft edits each under 100 ms |
| `mise run gate` (cloud container: `scripts/gate.sh`) | everything, including every terminal and drop-on-terminal test unchanged (SC-008) |

## Part B — manual / visual pass (GUI exception)

Prerequisites: `mise run app` (or the dev build) with a signed-in Claude Code; one structured
session (**Claude Code (structured)** from the start-session chevron); a second window
(**New window**); a PNG screenshot, a JPEG and a text file on disk. Run in light and dark theme.

1. **Send** (US1): type `hello`, press Enter. The composer empties, **You** shows `hello`, the pane
   shows **Working** with the indicator and a **Stop** control; the sidebar row shows working.
   When the reply ends: **Ready**, Stop gone, sidebar idle — within a second in both windows.
2. **Multi-line and paste** (US2 AS1–AS3): Shift+Enter twice, then Enter: one message with the
   breaks. Paste 200 lines of code: the composer grows to its limit, then scrolls; the transcript
   stays visible above it.
3. **History** (US2 AS4–AS6): Up on the first line recalls the last message, again the one before;
   Down past the newest restores the draft. In a three-line draft with the caret on line 2, Up and
   Down move the caret only. Recall, edit, send: the history still holds the original.
4. **Switch away and back** (FR-007): type a draft, switch session and back: text, caret and chips
   are there.
5. **Interrupt** (US3): ask for a 2,000-word story; press Escape in the composer (then, next turn,
   the Stop control). The turn ends with **You interrupted this turn**; Ready; send again works.
   Tiled with the other window: both show Ready.
6. **Queue** (US4): while busy, send two messages: two rows above the composer, numbered; nothing
   goes out. Edit the second (text changes in place), remove nothing yet. When the turn ends the
   first goes out alone, then the second. While busy, queue one and remove it: never delivered.
   With an empty draft, **Move to composer** brings a queued message back; with a draft it is not
   offered. The second window shows the same rows in the same order throughout.
7. **Hold** (FR-022): queue two messages, interrupt: **Queue on hold** with **Send queued**; send a
   new message: it goes at once, the queue stays held; press Send queued in the other window: the
   first queued message goes. Queue one more, stop the session, start it: still on hold.
8. **Large draft** (FR-008): paste a 10,000-line file; typing stays smooth; send; the reply
   arrives.
9. **Attachments** (US5): drop the PNG and the text file together: one chip with name and preview,
   the text file's path at the caret, quoted as a terminal drop would be. Send: the transcript's
   **You** shows the attachment name; Claude describes the picture. Drop a 10 MB image: not
   attached, the composer names it and says it is too large. Drop only an image and send: the
   picture alone is delivered.
10. **Not running and offline** (FR-006): stop the session: the composer says it is not running and
    Send is unavailable; the draft stays. Quit the background service: the composer says the
    service is unreachable; restart it: send works, the draft was kept.
11. **Terminal unchanged** (FR-001, SC-008): a terminal Claude Code session has no composer; Ctrl-C
    and a file drop behave as before.
12. **Keyboard and contrast**: Tab reaches Send, Stop, Send queued, each queue row's actions and
    each chip's remove; busy, idle, hold and refusal states read without colour (greyscale
    screenshot).

Record the results (pass/fail, platform, theme, screenshots) in the ledger's visual pass entry.
