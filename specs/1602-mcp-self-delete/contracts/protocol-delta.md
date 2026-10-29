# Contract: client and service protocol delta

Base: `PROTOCOL_VERSION = 73`. This feature bumps once to 74 (take the next free number if another feature
lands first) and updates the changelog comment in `protocol/version.rs` and the pin in
`crates/micold-core/tests/schema_hash.rs`.

```text
DaemonMsg::ConfirmationRequested { ...existing...,
    loss: Option<LossSummary>,     // Some for DeleteWorktree only
    self_targeted: bool }

DaemonMsg::DeferredActionFailed { project: PathBuf, subject: String, reason: String }

LossSummary { files: Vec<String>, files_more: u32,
              commits: Vec<LossCommit>, commits_more: u32, unreadable: bool }
LossCommit  { short_hash: String, subject: String }
```

- `replay` to a late window carries the same `loss` and `self_targeted` as the live broadcast.
- `DeferredActionFailed` goes to every connected window, is not replayed, and is never answered.
- No `ClientMsg` change.
