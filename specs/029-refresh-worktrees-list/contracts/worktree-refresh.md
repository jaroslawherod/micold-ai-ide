# Contract: `WorktreeRefresh` — the user-initiated re-read

Owns the rule that a re-read of a project's worktree listing is something a **user can ask for**
(FR-003), and the rule that a listing produced this way is indistinguishable from one produced by
any other trigger (FR-004).

---

## 1. The request

```rust
/// Re-read a project's worktrees from git + the filesystem, on the user's explicit request
/// (feature 029, FR-003). **Mutates nothing** — neither the repository nor the app's own
/// settings. Idempotent, and safe to send at any time a project is open.
WorktreeRefresh {
    /// Correlation id.
    req: u64,
    /// Project path.
    project: PathBuf,
},
```

Added to `ClientMsg` in `crates/micold-core/src/protocol/messages.rs`, beside the other
worktree RPCs (`WorktreeInclude`, `WorktreeExclude`, `WorktreeDelete`, `WorktreeRename`).

**Correlated despite mutating nothing.** The read-only RPCs on this protocol are correlated too
(`BranchPreflight`, `BranchList`, `RepoRootQuery`) for the reason `AiCliAvailabilityRequest`'s doc
gives — "a request-shaped message with no id would be the odd one out on this protocol" — and here
there is a second reason that is load-bearing: the reply is the **only** thing that ends the
button's busy state (FR-007). An uncorrelated request could not.

## 2. The reply

```rust
DaemonMsg::OperationOk { req, result: OperationResult::Ack }
```

**No new `OperationResult` variant.** `Ack` (`messages.rs:941`) is exactly right: the operation
"simply succeeded", and the only fact the client needs from the reply is that the re-read finished.

**The refreshed listing is not in the reply.** It arrives independently, as the `CatalogChanged`
broadcast that `refresh_worktrees_and_broadcast` already sends, and reaches the sidebar through
`catalog_sync::reconcile_catalog` → `State::set_worktrees` — the same path a create, a delete, an
include and an attach all use.

> This is FR-004's implementation and not merely its consequence. "A user must not be able to tell
> which trigger produced the listing" is guaranteed by there being **one** path, not by two paths
> being carefully kept in agreement. Carrying the listing in the reply would create the second one,
> and every future change to reconciliation would then have to remember it exists.

## 3. Failure

```rust
DaemonMsg::OperationError { req, kind: ErrorKind::IoFailed, message, detail }
```

Raised only if the project is unknown or its repo path cannot be resolved. Note that
`DaemonState::refresh_worktrees` (`state.rs:799`) **cannot fail**: a non-git or unknown project
takes its `_ =>` arm and *clears* the cache entry, which is the correct answer to "what worktrees
does this non-repository have" rather than an error.

So the daemon arm's shape is:

```
ClientMsg::WorktreeRefresh { req, project } => {
    refresh_worktrees_and_broadcast(state, project).await;
    send_ack(state, id, req);
}
```

— two lines, matching `ProjectAdd`'s success arm (`server.rs:1398-1402`). The client's
`OperationError` handling exists for protocol completeness and for a future in which discovery can
fail, not because this arm can currently reach it.

**On the client side, a failure MUST NOT clear the listing** (FR-008). The error path writes
`refreshing = false` and raises a notice; it does not call `set_worktrees`.

## 4. Ordering

The ack is sent **after** the broadcast, and this order is load-bearing in one direction only:

- A client that receives `Ack` has, by then, already received (or has queued before it) the
  `CatalogChanged` carrying the refreshed listing — the daemon writes both to the same connection
  in order.
- So a client may safely treat `Ack` as "the list you now hold is the refreshed one", which is what
  lets the completion notice say so honestly.

The reverse order would let the button return to idle while the old list was still on screen — a
one-frame lie, but the kind that is reported as a bug.

## 5. Broadcast, not per-client

`refresh_worktrees_and_broadcast` pushes to **every** connected client, unlike the attach path's
`refresh_worktrees_and_send`, which pushes to one.

Correct here for the reason the attach comment gives in reverse: attach is per-client and exclusive,
so a broadcast would reach clients that are not in that project. A refresh is neither. Git's answer
about a repository is the same answer for everyone looking at it, and a second window showing the
same project must not keep a listing this one has just learned is stale. This is also what FR-011
means by "MUST NOT disturb worktree listings belonging to other projects": other *projects* are
untouched (the refresh names one project and the snapshot overlays only its entry), while other
*windows on the same project* are correctly updated.

## 6. Protocol version

`PROTOCOL_VERSION` 9 → 10, in the same edit that adds the variant.

This feature has exactly one wire-visible change, so the rule features 026 and 027 record — one bump
per feature, because `build.rs` regenerates `SCHEMA_HASH` over `messages.rs` and
`tests/schema_hash.rs` fails on a second move — is satisfied without needing to batch anything.

Doc line to add, in the established style:

> Bumped 9 → 10 for feature 029's `ClientMsg::WorktreeRefresh`: the worktree listing's re-read
> becomes something a user can ask for, where it was previously only a consequence of attach,
> create, delete, include, exclude or project-add.

## 7. What this contract does not add

- **No `DaemonMsg`.** The catalog push already exists.
- **No `OperationResult` variant.** `Ack` carries the whole meaning.
- **No progress frames.** `OperationProgress` exists for the multi-stage worktree create; a single
  `git worktree list` has no stages to report.
- **No daemon-side timeout.** The bounded wait is the client's (research R5) — the daemon has no way
  to know how long a client is willing to wait, and every other RPC on this protocol leaves that to
  the caller.
