# Contract: run groups on the wire (protocol 36)

Additions to `micold_core::protocol::messages`. `PROTOCOL_VERSION` 35 → 36; the handshake refuses a
mismatched pair, so no field needs a legacy default.

## Client → daemon

```text
ClientMsg::RunGroupCreate {
    req: u64,
    project: PathBuf,
    naming: WorktreeNaming,      // type, optional ticket, name
    prompt: String,
    base_branch: String,
    providers: Vec<AiCli>,       // one per run, in run order; 2..=MAX_RUNS
}
ClientMsg::RunGroupPick { req: u64, project: PathBuf, group: GroupId, run: u8 }
ClientMsg::RunGroupDismiss { req: u64, project: PathBuf, group: GroupId }
```

## Daemon → client

```text
DaemonMsg::RunGroupsChanged { project: PathBuf, groups: Vec<RunGroup> }
OperationResult::RunGroupCreated { group: GroupId }
OperationResult::RunPicked { group: GroupId, run: u8, integration: Integration }
```

`RunGroupsChanged` carries the project's whole group list (at most a handful of groups of at most 8
runs) and is idempotent, like `CatalogSnapshot`. It is pushed on: every accepted create, every run
status change, a pick, a dismiss, a worktree delete that emptied or shortened a group, and once per
connection right after `Attached`.

## W1 — `RunGroupCreate`

Refused, creating nothing and pushing nothing:

| Condition | `ErrorKind` | Message names |
|---|---|---|
| project unknown or not a git repository | `InvalidInput` | the project |
| `providers.len()` outside `MIN_RUNS..=MAX_RUNS` | `InvalidInput` | the allowed range |
| `naming` fails `naming::derive` | `InvalidInput` | the `NamingError` text |
| `prompt` empty or whitespace only | `InvalidInput` | the prompt |
| `base_branch` is not a local branch now | `NotFound` | the branch (Edge "Base branch missing") |

Accepted: the daemon resolves `base_commit` (`git rev-parse <base_branch>`), derives the N names
(R5), writes the group with every run `Creating`, answers `OperationOk(RunGroupCreated)`, pushes
`RunGroupsChanged`, and spawns one task per run. A per-run failure never fails the request (FR-005).

## W2 — per-run progress

Each run task pushes `RunGroupsChanged` on each of its transitions. A run whose worktree creation
fails records `Failed { step: Worktree, reason }` with `CreateError`'s own message and leaves no
branch or directory behind (`ops::create_worktree`'s rollback, FR-006). A run whose session starts
but whose prompt is undelivered records `PromptNotDelivered { reason }` from
`FirstPromptUndelivered::message` and keeps its worktree and session (FR-004, US1 s4).

The prompt text never appears in a log line or an error message (482 W12).

## W3 — `RunGroupPick`

Taken under the project's worktree gate (R9), in this order; each refusal changes nothing:

1. group unknown → `NotFound`.
2. `group.winner.is_some()` → `Refused`, "this group already has a winner" (FR-018, US4 s6).
3. any run of the group is `Creating` or `Starting` → `Busy`, naming that run (FR-017).
4. the named run has no branch (`Failed`) → `Refused`, with its reason (FR-017).
5. the run's worktree holds uncommitted changes (R8) → `Refused`, `Uncommitted { files }`, listing
   them (FR-012, US4 s9).
6. the merge pre-check reports conflicts → `Refused`, `Conflicts { files }` (FR-013, US4 s2).
7. the base branch's tip is no longer `observed old tip` → `Refused`, `BaseMoved`.
8. the base branch is checked out and git refuses the merge → `Refused`, `BaseBusy(git's message)`.
9. git cannot run the pre-check → `Refused`, `GitTooOld` or `Git(message)`.

Accepted: the integration of [integration.md](./integration.md) runs, `winner` and the run's `Picked`
status are written **after** the ref moved, `OperationOk(RunPicked { integration })` is answered and
`RunGroupsChanged` is pushed.

A run whose session is still working is **not** refused here: the confirmation FR-017 asks for is the
client's, before it sends this message.

## W4 — `RunGroupDismiss`

Removes only the group record; no worktree, branch or session is touched (FR-020). Unknown group →
`NotFound`. Answered `OperationOk(Ack)` with a `RunGroupsChanged` push.

## W5 — persistence and consistency

- Every accepted change is persisted before memory changes; a failed write is answered
  `OperationError` and the groups stay as they were (482 W5).
- `ops::delete_worktree` calls `runs::forget_worktree` on success, so a deleted run leaves its group
  and an emptied group disappears (FR-007, US2 s4).
- Forgetting a project removes its runs file (`remove_runs`).
- On the first read of a project's file, interrupted runs are marked and reconciled per research R10
  before anything is pushed.
