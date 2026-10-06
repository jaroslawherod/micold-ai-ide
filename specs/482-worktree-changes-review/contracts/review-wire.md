# Contract: review comments on the client–service wire

Protocol change: `PROTOCOL_VERSION` 29 → 30 (`crates/micold-core/src/protocol/version.rs`); the
schema hash follows from the new types. All new types derive `Serialize, Deserialize` and live in
`crates/micold-core/src/protocol/messages.rs` (wire) and `crates/micold-core/src/review/` (domain).

## Client → service

```rust
ClientMsg::ReviewEdit {
    req: u64,
    project: PathBuf,
    worktree_dir: String,               // "" = Default, as SessionCreate's existing wire form
    edit: ReviewEditOp,
}

enum ReviewEditOp {
    Add { path: String, side: Side, start: u32, end: u32, quote: Vec<String>, text: String },
    SetText { id: CommentId, text: String },
    Delete { id: CommentId },
    ClearSent,
    DiscardPending,
}

ClientMsg::ReviewSend {
    req: u64,
    project: PathBuf,
    worktree_dir: String,
    outdated: Vec<CommentId>,           // the client's R14 judgement, used only for wording
}

ClientMsg::SettingsSet { …, diff_layout: Option<DiffLayout> }   // new optional field (R12)
```

## Service → client

```rust
DaemonMsg::ReviewChanged {
    project: PathBuf,
    worktree_dir: String,
    comments: Vec<ReviewComment>,       // every comment of the entry, pending and sent
    sending: bool,                      // a send is in progress for this entry
}

OperationResult::ReviewSent { session: SessionId, started: bool }   // new variant
```

`ReviewChanged` is pushed to every client attached to `project`: once per entry with comments
right after `Attached`, and after every change to that entry (edit, send start, send end, worktree
removal — the last as an empty list).

## Rules

| ID | Rule | Spec |
|---|---|---|
| W1 | `ReviewEdit::Add` is refused `InvalidInput` when `path` is absolute or contains `\`, `start == 0`, `start > end`, `quote.len() != end - start + 1`, or `text.trim()` is empty. Text is stored trimmed. | FR-011, FR-012 |
| W2 | A `worktree_dir` that is neither `""` (Default) nor a worktree of `project` in the catalog is refused `NotFound`; nothing is stored. | FR-021 |
| W3 | `SetText`/`Delete` on a comment of another entry, or an unknown id, is `NotFound`; on a comment inside an open send, `Busy`; on a sent comment, `Refused` (sent comments are only cleared). | FR-011, R6 |
| W4 | `ClearSent` removes sent comments only; `DiscardPending` removes pending comments not inside an open send. The client asks for confirmation before sending `DiscardPending`. | FR-019 |
| W5 | Every accepted edit is persisted before `OperationOk` is sent; a failed write answers `IoFailed` and leaves memory unchanged (catalog C3). | FR-020 |
| W6 | `ReviewSend` with an open send for the entry is `Busy`; with no pending comment, `InvalidInput`. Otherwise the service snapshots the pending ids, builds the prompt, sets `sending`, pushes `ReviewChanged`, then delivers (R4, R5). | FR-014, FR-018 |
| W7 | Target: the entry's running session with the latest `last_active`; with none, a new session of `default_ai_cli` in that entry, started, then typed once ready. Never another entry's session; never a resumed ended session. | FR-016, FR-021, US3 s4 |
| W8 | Delivered (`write_input` `Ok`): snapshot ids become `Sent { at }`, file written, `sending` cleared, `ReviewChanged` pushed, `OperationOk(ReviewSent)`. | FR-017 |
| W9 | Not delivered — start failed, not ready within `first_prompt_bound`, trust question pending, no bracketed paste, write error, session ended: nothing marked, `sending` cleared, `ReviewChanged` pushed, `OperationError { kind: Refused or Internal, message }` naming why. A session the send started stays (it is a normal session the user can use or close). | FR-017, US3 s2, Edge "Session ends during sending" |
| W10 | `sending` is never persisted; a service restart during a send leaves the comments pending. | FR-017 |
| W11 | Deleting a worktree (`ops::delete_worktree`, success) removes its comments from memory and file and pushes an empty `ReviewChanged`. A project removed from the catalog deletes its `reviews/` file. | FR-020, US4 s5 |
| W12 | The prompt text is never logged (034 FR-018 rule); logs name the entry, the comment count and the outcome. | Principle IV |
