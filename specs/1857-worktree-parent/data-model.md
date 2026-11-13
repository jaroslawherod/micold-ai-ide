# Data Model: worktree parent

- **WorktreeId**: uuid, minted once per worktree incarnation. Stored `dir_name -> WorktreeId` (`worktree_ids`). Replaced on create under the same name; removed with the worktree.
- **ParentRef**: `Default` | `Worktree(WorktreeId)`.
- **ParentLink** (`worktree_parents`, keyed by child `WorktreeId`): `{ parent: ParentRef, removed: bool, last_name: String }`. `last_name` is the parent's `dir_name` at the last time it was live, used to label a removed parent. `Default` never gets `removed = true`.
- **ParentState** (derived): `Known` if `removed == false` and the target is live, else `Removed`.

State transitions: absent -> Link(known) at create; Link(known) -> Link(removed) on parent deletion or reconcile; Link removed with the child when the child is deleted. No other transition (FR-004).

Validation: explicit parent must be `default` or a live worktree of the same project (FR-002). A link never points at its own child's id (guaranteed: the parent exists before the child).

On-disk: two additive, defaulted, `skip_serializing_if = empty` maps in the project state; absent in files written before this feature, which therefore load as all-unparented (FR-003).
