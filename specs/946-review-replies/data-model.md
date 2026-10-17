# Data model: Review Comments the Agent Answers (#946)

Extends 482's data model (`specs/482-worktree-changes-review/data-model.md`). Only what changes is
listed.

## Core: `review::comment`

### `CommentState` (replaces 482's)

| Variant | Fields | Meaning |
|---|---|---|
| `Draft` | — | Written, not sent (482's `Pending`). Editable and deletable. |
| `Sent` | `at: u64` | Delivered in a batch at `at`. |
| `Reported` | `sent_at: u64`, `report: Report` | Applied or Answered. |

`status()` → `Status::{Draft, Sent, Applied, Answered}` (what the UI and the panel counts use).

### `Report`

| Field | Type | Rule |
|---|---|---|
| `outcome` | `Outcome::{Applied, Answered}` | |
| `reply` | `String` | `\r\n`/`\r` → `\n`; trimmed; non-empty when `Answered`; ≤ 20,000 chars |
| `first_at` | `u64` | Time of the first report; kept by later reports (FR-005) |
| `last_at` | `u64` | Time of the latest report |
| `by` | `Reporter::{Agent { session: Uuid }, User}` | Who reported last |

### `CommentRef`

`C-` + first 8 lowercase hex digits of `CommentId`'s UUID. `ReviewComment::reference()`. Parsing
accepts the prefix `C-` or `c-`, upper or lower case hex, and optional surrounding whitespace.

### `ReviewComment` (extends 482's)

Unchanged fields: `id`, `path`, `side`, `range`, `quote`, `text`, `state`, `created`. `range` now
means *last known place* (research R3); `quote` and `text` never change after creation (FR-023).

### `EntryReview` operations (new or changed)

| Operation | Allowed from | Result | Refusals (`ReviewError`) |
|---|---|---|---|
| `add(draft, id, created)` | — | Draft | `Invalid`; `RefTaken` when `id`'s reference is in use |
| `begin_send(entry, outdated)` | Drafts exist | snapshot of Drafts | `Busy`, `NothingPending` |
| `finish_send(snapshot, at)` | | snapshot's comments → `Sent { at }` | |
| `report(ref, outcome, reply, by, at)` | `Sent`, `Reported` | `Reported`, `first_at` kept | `NotFound`, `Draft`, `InSend`, `Invalid` (blank answered reply, over limit) |
| `mark(id, outcome, note, at)` | `Sent` | `Reported { by: User }` | `NotFound`, `Refused` (not Sent), `InSend`, `Invalid` |
| `reanchor(moves)` | any state | ranges where `from` matches | none: non-matching moves are skipped; returns how many applied |
| `clear_sent()` | | removes Sent and Reported | |
| `set_text`, `delete`, `discard_pending` | Draft only | as 482 | as 482 |

State transitions: `Draft → Sent → Reported ⟲ Reported`. Nothing returns to Draft (FR-001).

### `Move`

`{ id: CommentId, from: LineRange, to: LineRange }`; `to.len() == from.len()`.

## Core: `review::anchor` (new)

`locate(quote, last, lines) -> Located` with `Located::{Same(LineRange), Moved(LineRange), Gone,
Ambiguous}` and `Located::shown(&self, last, len) -> LineRange` (research R2). `is_outdated(status)`:
true only for Gone/Ambiguous on a Draft or Sent comment (FR-021, FR-024).

## Core: `review::prompt`

`build(entry: EntryKind, comments: &[ReviewComment], outdated: &[CommentId]) -> String`, same
signature, new output (contracts/batch-format.md). `REPORT_TOOL: &str = "report_review_comment"`
is shared with the MCP catalog so the preamble cannot name a tool that does not exist.

## Core: `review::reply` (new, small)

`display(reply: &str) -> String`: keeps `\n`; replaces every other C0 control character with its
Unicode control picture (U+2400 + code, so ESC shows as `␛`) and DEL with `␡`, so nothing is
rendered as a control code. `shortened(reply, max_chars) -> (String, bool)` cuts at 4,000
characters on a char boundary for cards and rows and says whether it cut.

## Core: `review::store::ReviewFile`

`version: 2`. Reads 1 and 2. Stored state (JSON):

```json
{ "draft": null }
{ "sent": { "at": 1790000000 } }
{ "reported": { "sent_at": 1790000000, "outcome": "answered", "reply": "…",
                "first_at": 1790000100, "last_at": 1790000200,
                "by": { "agent": "6f1c…uuid" } } }
```

Version 1's `{ "pending": null }` loads as Draft. `by` for a user mark: `"user"`.

## Wire (`protocol::messages`, `PROTOCOL_VERSION` 49)

- `ReviewComment` carries the new `CommentState` (postcard shape changes).
- `ReviewEditOp::Reanchor { moves: Vec<Move> }` and `ReviewEditOp::Mark { id, outcome, note:
  String }`.
- `ReviewSend { outdated }` unchanged; the daemon now ignores ids of current-side comments (it
  judges them, research R3).
- `ReviewChanged` unchanged.

## MCP (`mcp::tools`)

`Operation::ReportReviewComment { comment: CommentRef, outcome: Outcome, reply: String }`;
`tool_name()` `"report_review_comment"`, `audit_target()` the reference.

## Client: `features::changes::OpenView` (render-free)

| Field | Replaces / new | Meaning |
|---|---|---|
| `anchors: BTreeMap<CommentId, Anchor>` | replaces `outdated` | Where each comment of the entry is drawn and whether it shows the outdated mark; kept until its file is read again |
| `panel: Panel { open: bool, focus: Option<CommentId>, offset: u32, viewport: u32 }` | new | The review panel |
| `reply_open: Option<CommentId>` | new | The full-reply dialog |
| `ComposerTarget::Answer(CommentId)` | new arm | Writing a manual Answered note |

`Anchor { shown: LineRange, outdated: bool }`. Derived rows: `panel_rows(state) ->
Vec<PanelRow>` (grouped by path, then line, side, created), `status_counts`, `Jump::{Shown {
path, line }, Hidden(Why)}` where `Why::{KindToggledOff, NotListed}`.
