# Contract: branch queries over the client ↔ daemon protocol

Added when feature 016 was merged onto the post-daemon `main` (feature 010, T055). Git runs in the
daemon now, so the two-phase design in `branch-conflict.md` spans a process boundary: the *pure*
classification still lives in `micold-core` and is called daemon-side, while the client asks for it
by RPC.

Extends `specs/010-daemon-session-persistence/contracts/messages.md`.

---

## 1. Wire types

`BranchSituation`, `BranchCandidate`, `BranchOrigin`, `BlockReason`, `CreateMode`, and
`CreateStage` gain `Serialize`/`Deserialize` in `micold-core::worktree`. They are ordinary protocol
payloads.

No new persisted state — these cross the wire and are discarded with the form.

> ⚠️ **Known gap: these types are outside the `SCHEMA_HASH` guard.** `build.rs` hashes only
> `protocol/{messages,grid,envelope}.rs`. `messages.rs` *names* these types, so adding or removing
> a field there changes the hash — but adding a variant to `BranchSituation` or `CreateMode` in
> `worktree.rs` does **not**. A mismatched client and daemon would then pass the handshake and fail
> to deserialize at runtime, which is exactly what the guard exists to prevent.
>
> This is pre-existing in kind (`session.rs` contributes `SessionId`/`SessionLabel` on the same
> terms), and widening the hash to whole non-protocol modules would make unrelated edits — a
> comment, a private helper — force a spurious version mismatch. Fixing it properly means hashing
> the *type definitions* rather than the files, which changes `schema_hash`'s signature and its
> guard test. Left for a protocol-level change; recorded here so the next variant added to these
> enums is a deliberate decision rather than a surprise.

---

## 2. `ClientMsg::BranchPreflight` → `OperationResult::BranchPreflight`

```
BranchPreflight { req, project, branch, dir_name }
  → OperationOk  { req, result: BranchPreflight { situation } }
  → OperationError { req, kind, message, detail }
```

- **Read-only.** The daemon runs `worktree::preflight` and mutates nothing (SC-007). A client may
  issue it freely without side effects.
- Runs on the blocking pool: it shells out to `worktree list --porcelain` and, when the branch is
  not local, `for-each-ref`. Neither contacts a remote (FR-020).
- `dir_name` is required so the daemon can perform the directory-clash check that outranks every
  branch case (`branch-conflict.md` §1, rule 1).
- Errors: `GitFailed` when git itself fails, `Internal` if the blocking task panics,
  `NotFound`/`Refused` for a non-repo project (shared `reject_non_repo` path).

**Client obligation.** The reply decides what happens next, per `branch-picker.md` §5:

| Situation | Client action |
|---|---|
| `Free` | Send `WorktreeCreate` with `NewBranch` — no prompt (FR-025) |
| any, and the branch came from the picker | `WorktreeForm::mode_for(situation, preferred_remote)`; `Some(mode)` ⇒ create, `None` ⇒ prompt |
| otherwise | Raise the resolution prompt (FR-001) |

## 3. `ClientMsg::BranchList` → `OperationResult::BranchList`

```
BranchList { req, project }
  → OperationOk { req, result: BranchList { candidates } }
```

- **Read-only**, blocking pool, no network (FR-020).
- `candidates` arrives already ordered and annotated by `worktree::branch_candidates` — the client
  renders it as-is and does not re-sort.
- Sent when the form's source switches to `Existing`, not on every keystroke.

## 4. `ClientMsg::WorktreeCreate` gains `mode: CreateMode`

The daemon passes it straight to `create_worktree`, which **re-runs pre-flight and re-verifies the
mode before mutating** (FR-009). A stale answer therefore fails at the daemon, not at the client —
the client's earlier `BranchPreflight` is an optimisation for the prompt, never the authority.

`mode` defaults to `NewBranch`, so a client that never sends one behaves exactly as before.

---

## 4a. `DaemonMsg::OperationProgress` (FR-024)

```
OperationProgress { req, stage: CreateStage }
```

Pushed between a `WorktreeCreate` request and its terminal reply, once per stage **transition**
(a stage emits several internal lines; only changes are worth a frame).

- **Only the stage travels.** The wording is the client's: `CreateStage::label(&CreateMode)` is a
  pure function of the stage and the mode the client asked for, so a reuse reads "Checking out
  existing branch" and an overwrite reads "Replacing branch and creating worktree". Keeping the
  text client-side means the daemon has no presentation concerns and no locale to guess.
- **Lossy by design.** A client that misses a push simply shows the next stage; the terminal
  `OperationOk`/`OperationError` is what closes the operation. The push must therefore never be
  load-bearing for correctness.
- Sent on the client's own ordered frame channel (`frame_sender`), so it cannot block the runtime
  or touch the state lock; a departed client's sends are dropped.
- The client **peeks** its pending op rather than removing it — the operation is still running.
- Before the first push lands (the RPC is sent but git has not started) the form shows the generic
  "Creating worktree…". That window is real and is not an error state.

---

## 5. Error mapping (daemon → user)

| `CreateError` | `ErrorKind` | Message |
|---|---|---|
| `DuplicateDir` | `AlreadyExists` | unchanged |
| `BranchInUse { branch, reason }` | `Busy` | names the branch **and** its holder — a worktree, or the project checkout (FR-021, SC-006) |
| `SituationChanged` | `Refused` | the branch changed while the user was deciding; nothing was done (FR-009) |
| `RolledBack(stderr)` | `GitFailed` | git's stderr verbatim, unchanged |

`CreateError::DuplicateBranch` no longer exists, so the daemon's former `AlreadyExists` mapping for
it is gone. A `NewBranch` create against a taken name is now `Refused` (a stale answer), **not**
`AlreadyExists` — the existing branch is a decision for the client to resolve first.

---

## 6. Client-side failure surface

Both queries back the open create form, so their failures are written to `worktree_error` — the
form's own error line — **not** `notify_error`. The notification surface renders inside `base`,
which the modal wraps behind its scrim, so a banner raised while the form is open is dimmed out of
view. For `BranchList` staying silent is worse still: the empty picker would then claim the
repository has no branches.

---

## 7. Test obligations

**`crates/micold-daemon/tests/mutation_semantics.rs`** — real git, real RPCs:

1. `WorktreeCreate` with `ReuseLocal` puts the worktree on the existing branch (verified by
   `rev-parse --abbrev-ref HEAD` in the new worktree).
2. `WorktreeCreate` on a branch held by the project checkout ⇒ `Busy`, message naming the branch
   and its holder.
3. `BranchPreflight` on an existing branch ⇒ `LocalAvailable`, and no worktree directory appears.
4. `BranchList` returns the free branch as available and the project checkout's branch as blocked.
5. `NewBranch` against a taken name ⇒ `Refused`, no leftover directory, no catalog entry.
6. A `WorktreeCreate` streams `PreflightCheck` then `CreatingWorktree` before its terminal reply,
   each reported once (FR-024).

**`crates/micold-core/tests/protocol_roundtrip.rs`** — `WorktreeCreate` with each mode
(`TrackRemote` carries a payload), both new request variants, and `OperationProgress` survive the
wire.

**`crates/micold-client/tests/app_state.rs`** — the stage label is worded for the mode in flight,
mode-independent stages read the same everywhere, there is no label before the first push, and a
retry never inherits the previous attempt's wording.
