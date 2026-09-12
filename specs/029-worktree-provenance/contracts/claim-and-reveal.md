# Contract: The Claim Action and the Hidden Count

Governs the claim action on a revealed row and the count on 014's reveal control. Normative
statement of FR-014, FR-015, FR-015a, FR-020–FR-025b (research R6, R9).

## 1. The claim

### Protocol (`micold-core/src/protocol/messages.rs`)

```rust
// ClientMsg — modelled on WorktreeRename, which is already "durable catalog state, no git involved"
/// Record a worktree the app did not create as the user's own (029 FR-020).
WorktreeClaim {
    /// Correlation id.
    req: u64,
    /// Project path.
    project: PathBuf,
    /// Worktree directory name.
    dir_name: String,
},
```

Answered with `OperationResult::Ack` followed by `broadcast_catalog()`, or with
`OperationError { kind: ErrorKind::IoFailed }` if the record cannot be persisted. There is no
`InvalidInput` case: unlike a rename there is no user-supplied string to validate.

### Invariants

1. **Same record, same lifetime (FR-021).** A claim writes the record §2 of
   [`provenance-store.md`](./provenance-store.md) defines, with the same durability, the same removal
   on delete and forget, and the same read-only relationship to disk.
2. **Nothing on disk (FR-003, US5 scenario 4).** No git call, no `fs` call, no checkout. A test
   snapshots the worktree directory and its branch before and after and requires them unchanged.
3. **Honoured for reserved-convention names (FR-023).** The claim handler does not consult
   `matches_reserved_convention`. The veto binds the automatic backfill only.
4. **Idempotent.** Claiming an already-recorded worktree acks and changes nothing.
5. **No inverse (FR-024).** There is no `WorktreeUnclaim`, no un-claim menu entry, and no way to hide
   a worktree the app created. A worktree leaves the user-owned set only by deletion.
6. **Broadcast, so a second window agrees.** The `CatalogChanged` push is what drops the `agent` chip
   in another open window — the same mechanism that already carries a rename between windows.

### Client reducer (`micold-client/src/features/worktree.rs`)

```rust
pub enum Msg {
    // …
    /// Claim a revealed worktree as the user's own (029 FR-020), by `dir_name`.
    ClaimRequested(String),
}
```

Shape B, like `RenameConfirmed`: the reducer applies the record optimistically to
`state.workspace.worktree_provenance` and closes the row menu; `main.rs` matches the same variant a
second time to send `WorktreeClaim`.

**Invariants**

1. **Immediate (FR-022).** The row is user-owned in the same frame: it loses the `agent` chip, and it
   is still listed after the reveal control is switched off. Waiting for the daemon's ack would make
   the row vanish under the user's cursor the moment they toggled reveal off.
2. **Durable (US5 scenario 2).** The optimistic write is confirmed by the next catalog push; a
   persistence failure surfaces as the existing error notification.
3. **Sole mutation.** It touches `worktree_provenance` and `menu_open` and nothing else — not
   `show_agent_worktrees`, not `filters`, not `worktrees`.

### Row placement (`micold-client/src/ui/sidebar.rs`)

The claim entry is added to the worktree row's existing right-click menu, offered **only** when the
row's classification is `Agent`, which — since a hidden worktree produces no row — means only while
the reveal control is on (FR-020).

**`row_actions_cluster()` MUST remain unchanged**, as 014's contract requires. 014 forbids the hover
cluster being special-cased for an agent row, and FR-015 restates it: a revealed row offers exactly
the same actions as a user-created one, none disabled, hidden, reordered, or given an extra
confirmation, *plus* the claim. Adding the claim to the hover cluster instead of the menu would
change the cluster for one class of row, which is the thing both requirements forbid.

## 2. The hidden count

### Widget (`micold-client/src/ui/material/toggle_chip.rs`)

```rust
impl<M> ToggleChip<M> {
    /// A trailing count, rendered as `· N` after the label. Zero renders nothing (029 FR-025a).
    pub fn count(mut self, count: usize) -> Self;
}
```

**Invariants**

1. **Opt-in.** Chips that do not call `.count()` are pixel-identical to today. This feature must not
   restyle the tag filter chips, exactly as 014's contract required of `ToggleChip`'s introduction.
2. **Zero is absence, not `· 0`** (FR-025a): a project with nothing hidden shows 014's plain control,
   which is what keeps SC-003 honest.
3. **Themed and chainable.** The count takes its colour from the same `Roles` as the label, and
   `.count()` returns `Self` so the chain still terminates in `.into()` (Principle VIII).
4. **No second widget.** Building a bespoke labelled chip beside the reveal chip would undo the
   promotion 014 performed in this very file (research R9).

### Value (`micold-client/src/features/worktree.rs`)

```rust
/// How many of the active project's worktrees the reveal control is currently withholding
/// (029 FR-025). Zero while the control is on.
pub fn hidden_worktree_count(&self) -> usize;
```

**Invariants**

1. **Defined against the visible set**, as `worktrees.len() - visible_worktrees().count()`, so
   FR-025b — "switching the control on reveals exactly that many rows" — holds by construction rather
   than by two filters agreeing. This is 014's own single-source reasoning.
2. **Per project, per refresh (FR-025a).** It reads the active project's list, which
   `set_worktrees()` replaces on every refresh, so it tracks FR-012 without its own invalidation.
3. **Zero while revealed.** With the control on nothing is withheld, so the chip shows no count while
   active — the count answers "what am I not seeing", and while revealed the answer is nothing.
4. **Subject to tag filters (FR-025b).** The count is of the set the *control* governs, before tag
   filters narrow the tree; a tag filter can therefore leave fewer rows on screen than the count
   named, exactly as it can with the control off.
5. **Testable without a renderer.** It lives in the feature slice beside `has_visible_worktrees()`,
   which 014 put there for this reason (Principle I).

### Placement

Unchanged from 014: the reveal chip is rendered by `view` **above** `filter_bar()` and outside its
early return, so it stays reachable in a project whose only worktrees are hidden — now the common
case rather than the corner one, since that is what a project full of assistant session worktrees
looks like (SC-010).

## 3. Terminology

FR-015a: **no user-visible string is renamed by this feature.** 014's table stands unchanged.

| Surface | Required text |
|---|---|
| Reveal control label | `Show agent worktrees` (plus `· N` when N > 0) |
| Row badge | `agent` |
| User-guide section | "Agent worktrees" |
| Claim menu entry | `Claim as mine` |

"Assistant-owned" remains internal spec prose and MUST NOT reach the UI. The documentation (FR-019)
carries the wider meaning the word now has — "not created by this app" — while the word itself does
not change.
