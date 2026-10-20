# Data model: Review comments on a pull request's diff (#1149)

Extends 946's data model (`specs/946-review-replies/data-model.md`). Only what changes is listed.

## Core: `review::comment`

### `PrAnchor` (new)

| Field | Type | Rule |
|---|---|---|
| `number` | `u64` | > 0 |
| `head_branch` | `String` | non-empty, no control characters |
| `head_commit` | `String` | 40 or 64 lowercase hex digits; the commit of the diff shown when written |

### `Scope` (new)

`WorkingTree | PullRequest(u64)`. `ReviewComment::scope()` is `PullRequest(pr.number)` when `pr`
is set, else `WorkingTree`.

### `ReviewComment` (extends 946's)

New field `pr: Option<PrAnchor>`. Like `quote` and `text` it never changes after creation. `range`
stays "last known place", now in the PR head commit's version for a PR comment (new side) or the
base version (old side).

### `Draft` (extends)

New field `pr: Option<PrAnchor>`, copied to the comment by `add`.

### `EntryReview` operations (changed)

| Operation | Change |
|---|---|
| `add(draft, id, created)` | stores `draft.pr` |
| `begin_send(entry, scope, pr: Option<&PrContext>, outdated)` | snapshots Drafts of `scope` only; `NothingPending` when that scope has none; `Busy` while any send is open; prompt from `build` (working tree) or `build_pr` |
| `clear_sent(scope)` | removes Sent and Reported comments of `scope` |
| `discard_pending(scope)` | removes Drafts of `scope` not inside the open send |
| `report`, `mark`, `reanchor`, `set_text`, `delete`, `finish_send`, `abort_send` | unchanged |

State transitions are unchanged (`Draft -> Sent -> Reported`).

## Core: `review::prompt` (changed)

`PrContext { number: u64, title: String, head_branch: String, state: BranchPrState, current_head: String,
checked_out: Option<String>, unreadable: Vec<String> }`: `state` is the existing `pull_request::BranchPrState` (Open, Draft, Merged, Closed), reused, no new type (it gains `Serialize, Deserialize` in `pull_request.rs`, lower-case variant names on the wire);
`checked_out` is the entry's branch (`None` for detached or unknown); `unreadable` the recorded
commits that are not in the worktree. `build_pr(entry, &PrContext, comments, outdated) -> String`,
contracts/batch-format.md. `build` is unchanged for the working tree.

## Core: `review::anchor` / `review::diff` (changed)

`trait Lines { fn line(&self, number: u32) -> Option<&str>; fn len(&self) -> usize; }` implemented
by `SideLines`. `locate(quote, last, lines: &impl Lines) -> Located` is the 946 function, made
generic. `HunkLines`: the lines of one side that a `FileDiff::Text`'s hunks number (`line(n)` is
`None` outside them; `len()` is the highest number shown). `HunkLines::new(diff: &FileDiff, side:
Side)`.

## Core: `review::store`

`REVIEW_FILE_VERSION` 3, written only when a comment has `pr`; otherwise 2. Reads 1, 2, 3. Stored comment gains `"pr": null | { "number": 7,
"head_branch": "feat/x", "head_commit": "<hex>" }`. Versions 1 and 2 load with `pr` null.

## Wire (`protocol::messages`, `PROTOCOL_VERSION` 57)

- `ReviewComment` carries `pr` (postcard shape changes).
- `ReviewEditOp::Add { .., pr: Option<PrAnchor> }`, `ClearSent { scope: Scope }`,
  `DiscardPending { scope: Scope }`.
- `ClientMsg::ReviewSend { .., pr: Option<PrSend> }`; `PrSend { number, title, head_branch,
  state: BranchPrState, current_head }`.

## Client: `features::changes` (render-free)

| Item | Change |
|---|---|
| `PullRequestView.head: String` | the listed head when the diff read started (R3) |
| `ViewScope` | `view_scope(view) -> Scope`: `PullRequest(shown.number)` or `WorkingTree` |
| `PanelRow.scope`, `panel_rows` | rows carry their scope; groups in order working tree, then pull requests by number |
| `PanelGroup { scope, title: Option<String>, listed: bool }` | heading data: the PR number and title when listed, "not in the list" flag when not |
| `Effect::ReviewSend { .., pr: Option<PrSend> }` | `Some` while a listed pull request is shown |
| `can_pick` | true for a text file of a ready pull request diff or of the working tree |
| `send_action`, `tidy_actions`, `pending_counts`, `file_comments`, `status_counts`, `review_toolbar_label` | count and show only the shown scope (the panel's counts and rows show all scopes) |
| `judge_anchors` | PR comments: located in `HunkLines` of their file's patch, every file with comments |
