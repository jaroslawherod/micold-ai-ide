# Research: MCP self stop and own-worktree delete (#1602)

Each decision: what, why, rejected alternatives. Paths checked against the tree.

## R1. Policy: self-targeted requests become `Confirm`

**Decision**: in `micold-core/src/mcp/policy.rs::decide`, delete the three self-refusal arms
(own worktree, `StopSession` self, `DeleteSession` self). The existing arms then return
`PolicyDecision::Confirm`. A pure helper `is_self_targeted(&Caller, &Operation) -> bool` (caller's session id
equals the target, or the target worktree is the caller's `SessionLocation::Worktree`) feeds the prompt and
the deferral. The Default guard arm stays first, so `delete_worktree` from Default is still refused
(FR-007) while `stop_session` on itself from Default is allowed (FR-015a).

**Rationale**: one pure, table-tested function is how 034 holds all policy (034 plan summary). The
live-session conflict needs no change: the caller is a live session of W, so `stop_sessions: false` already
fails with `live_sessions_conflict` (FR-002, scenario 6).

**Rejected**: a new `ConfirmedOp::SelfDelete` variant (duplicates the three existing variants and their
wire form); checking self-targeting in the daemon only (splits policy across crates).

## R2. Result first, then the action

**Problem**: `ops::delete_worktree` kills the session's PTYs. The agent's tool call is an HTTP request the
daemon answers (`mcp/server.rs::handle_connection`, one response then close). Killing the agent before it
reads the reply loses the answer.

**Decision**: `tools::call` returns the JSON result plus `Option<Deferred>` (enum of `DeleteWorktree`,
`StopSession`, `DeleteSession`, built only after `ask_user` returned `Ok`). `handle_connection` writes the
reply, shuts down its write side, waits for the peer to close (`peer_closed`, already in `server.rs`) for at
most 2 s (`DELIVERY_GRACE`), then spawns `Deferred::run`. It runs whether or not the write or the wait
succeeded, because the user already confirmed (spec Edge Cases, "answer lost on stop"). The 2 s bound gives
SC-001's 10 s with margin.

**Plumbing**: `until_answered` (server.rs) returns `Option<Value>` and its closure yields a `Value`. It becomes
generic over the closure's output; the `Route::CallTool` arm yields `(Value, Option<Deferred>)` out of the spawned
task, and the `Route::Prompt` arm keeps `Value`. A hangup after the call is past confirmation still yields the
`Deferred` (the call finishes, per `until_answered`'s contract).

**Rationale**: proof the client read the reply is its own close, with no fixed sleep and no new channel. A
panic in the call task yields no `Deferred`, so nothing destructive runs after a failure.

**Rejected**: run the action inline then reply (the answer is lost, the exact refusal reason in 034);
fixed `sleep` (flaky on slow hosts, wastes time on fast ones); a client acknowledgement message (MCP has
none); spawning the action from inside `call` with a delay (loses the ordering guarantee).

## R3. Reading the loss summary

**Decision**: add `Git::worktree_loss(&self, dir: &Path) -> io::Result<WorktreeLoss>`, implemented in
`GitCli` (`micold-core/src/git.rs`) and `FakeGit`. `GitCli` runs two local commands in `dir`:
`git status --porcelain=v1 -z --untracked-files=all` for changed and untracked files, and
`git log --format=%h%x00%s` over `@{upstream}..HEAD` when an upstream exists, else `HEAD --not --remotes`
(commits on no remote-tracking branch, spec Assumptions). Pure parsers in new `worktree/loss.rs` turn
output into `LossSummary` with the 50-entry bound plus total counts. The daemon calls it on a blocking
thread right before `ask_user`, only for `delete_worktree`. A git error yields `unreadable` and the user
may still accept (spec Edge Cases).

**Rationale**: reuses the `Git` trait seam, so core tests use `FakeGit` and one real-repo test covers
`GitCli`. Local refs only (Principle IV).

**Rejected**: reuse `runs::summary::read_uncommitted_files` (needs a base branch and a review base,
returns no commits and skips the no-base case); `git2` crate (new dependency for two commands); a sidebar
cache (stale, and the spec says the state at prompt time).

## R4. Carrying it to the dialog

**Decision**: `DaemonMsg::ConfirmationRequested` gains `loss: Option<LossSummary>` (only worktree deletes)
and `self_targeted: bool`. `LossSummary { files, files_more, commits, commits_more, unreadable }`. The
dialog `ui/confirm_agent_request.rs` shows the lists when non-empty, the count line when `*_more > 0`, an
"could not read" line when `unreadable`, and a line saying the asking session ends when `self_targeted`.
One protocol bump 73 to 74; fields are not optional on the wire beyond `Option`, the version check gates old
peers as every earlier bump did. If another feature takes 74 first, take the next number.

**Rejected**: a separate request/reply for the summary (another round trip and a window with no data on
replay; `replay` must show the same data to a late window); stuffing text into `target_label` (loses
structure, cannot be tested for the 50 bound).

## R5. Reporting a failed deferred step

**Decision** (ledger C2): `Deferred::run` logs `tracing::error!` (lands in the recent-errors ring, as
`ops::delete_worktree` already does) and broadcasts new `DaemonMsg::DeferredActionFailed { project, subject,
reason }` to every window; the client calls `app.core.notify_error` (exists, used in `daemon_sync.rs`).
`subject` is the worktree display name, or the session label for story 4. A removal that leaves
`leftovers` or fails the branch delete is reported the same way with that reason, because the user
cannot see it otherwise. The session stays stopped.

**Rejected**: reuse `OperationError` (carries a request id no window owns); `SessionErrorNotice` (names a
session whose record may be gone, one window, attention semantics); log only (the user would not know the
worktree survived).

## R6. Windows, macOS, open directory handles

**Decision**: reuse `ops::delete_worktree`, which kills PTYs then retries the directory removal with
backoff of 100 ms doubling five times (about 3 s). `Deferred::run` calls it with `stop_sessions: true`
only; it archives the sessions and kills their PTYs internally (ops.rs), exactly the sidebar's order. No separate archive step.

**Residual risk**: a shell that outlives the kill by more than 3 s on Windows leaves the directory
(spec says it must be gone on all three). Mitigation in tasks: a daemon test that holds a handle in W and
asserts removal on the Windows CI job; if it fails, extend the retry bound in `ops.rs` (shared with the
sidebar, so one change). Any residue surfaces as the R5 notice, never silently.

## R7. `stop_session` and `delete_session` extracted

**Decision**: move the bodies after confirmation in `daemon/src/mcp/tools.rs::stop_session` and
`::delete_session` into `perform_stop_session` and `perform_delete_session`. The direct path calls them
inline; `Deferred::run` calls the same functions, so the sidebar-equivalent behavior is written once (the
repo's duplication budget, CLAUDE.md).

## R8. Docs and 034 amendments

**Decision**: update `docs/user-guide/agent-tools.md`, the descriptions at
`micold-core/src/mcp/tools.rs` (the `delete_worktree` text "\"default\" and the calling session's own
worktree cannot be deleted" and the `stop_session` text "A session cannot stop ..."), the `delete_session`
text, and 034's `spec.md` FR-014 and FR-015 and `contracts/mcp-tools.md` (FR-014a). Interrupt, send and
read self-refusals stay documented.
