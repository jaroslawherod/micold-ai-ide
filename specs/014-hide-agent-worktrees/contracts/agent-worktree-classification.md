# Contract: Agent Worktree Classification

Governs the pure core API that decides whether a discovered worktree belongs to the user or to an
AI assistant, and the visible-set accessor every worktree surface reads from. This is the
normative statement of FR-005/FR-006/FR-007 (research R1, R2, R3).

## `WorktreeOwner` and `Worktree` methods (src/worktree.rs)

```rust
/// Who created a worktree — the user, or an AI assistant for its own sub-task (FR-001).
/// Enum, not a bool, so a future third owner is an added variant rather than a refactor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeOwner {
    /// Created by the user (through the app or by hand). Always listed.
    User,
    /// Created by an AI assistant. Hidden unless the reveal control is on (FR-002).
    Agent,
}

impl Worktree {
    /// Classify this worktree from its names alone (FR-005). Derived, never stored.
    ///
    /// PRECONDITION: `self` came from `reconcile()`, which already guarantees the worktree
    /// lives directly under the project's `.claude/worktrees/` root — the location half of
    /// FR-005. Do not call on a `Worktree` obtained by any other route.
    pub fn owner(&self) -> WorktreeOwner;

    /// `true` iff [`Self::owner`] is [`WorktreeOwner::Agent`].
    pub fn is_agent_owned(&self) -> bool;
}
```

### Normative rule

A worktree is `Agent` **iff at least one** of the following holds:

1. `dir_name` equals `"agent-"` followed by an *agent id*.
2. `branch` is `Some(b)` and `b` equals `"worktree-agent-"` followed by an *agent id*.

Otherwise it is `User`.

An **agent id** is a string that is:

- at least **16** characters long, and
- composed **entirely** of ASCII hex digits (`0-9`, `a-f`, `A-F`).

Prefixes are matched case-sensitively; the id accepts either hex case.

Both conditions are normative in the spec itself (FR-005/FR-006, fixed by the 2026-07-23
clarification) — they are not an implementation liberty, and neither may be relaxed without a spec
change.

### Invariants

1. **Pure**: no I/O, no `fs`, no git, no clock, no global state. Same inputs → same output, always.
2. **Total**: defined for every `Worktree`, including `branch: None` and every `WorktreeStatus`.
3. **Health-blind**: `Valid`, `Missing`, and `Invalid` classify identically (FR-007).
4. **Non-mutating**: takes `&self`; nothing on disk is read or written (FR-008, SC-005).
5. **Stateless**: no caching, no memoization, no persisted field (FR-009).

### Truth table (normative test corpus — US1, US2, FR-006)

| `dir_name` | `branch` | Owner | Why |
|---|---|---|---|
| `agent-a885b42dc521fbda1` | `Some("worktree-agent-a885b42dc521fbda1")` | `Agent` | Both identifiers match (the real-world case) |
| `agent-abf6a58b16c3c9e6f` | `None` | `Agent` | Orphan/detached — dir alone suffices |
| `unrelated-dir` | `Some("worktree-agent-ae474105b29fbeb68")` | `Agent` | Branch alone suffices |
| `agent-a885b42dc521fbda1` | `Some("feat/real-work")` | `Agent` | Either identifier suffices (mismatch edge case) |
| `agent-foo` | `Some("agent/foo")` | `User` | Too short **and** `o` is not hex — the FR-006 case |
| `agent-face` | `None` | `User` | All hex but only 4 < 16 |
| `agent-deadbeefdeadbeef-parser` | `None` | `User` | Long enough, but the tail is not hex |
| `agent-deadbeefdeadbeef` | `None` | `Agent` | Exactly 16 hex digits — the boundary, inclusive |
| `agent-deadbeefdeadbee` | `None` | `User` | 15 hex digits — one below the boundary |
| `feat-1234-agent-runner` | `Some("feat/1234-agent-runner")` | `User` | Reserved word in the middle, no prefix match |
| `agent-` | `None` | `User` | Empty id |
| `agent-A885B42DC521FBDA1` | `None` | `Agent` | Uppercase hex accepted |
| `AGENT-a885b42dc521fbda1` | `None` | `User` | Prefix is case-sensitive |
| `worktree-agent-a885b42dc521fbda1` | `None` | `User` | Branch prefix in the **dir** position does not match |

The boundary rows (16 vs 15) and the case rows are required, not illustrative: they are what pins
the rule against a later "simplification" back to bare prefix matching.

## `State` visible-set API (src/app.rs)

```rust
impl State {
    /// The worktrees currently shown to the user: all of them while the reveal control is on,
    /// only user-owned ones while it is off (FR-002/FR-003). The single source every worktree
    /// surface reads from — see the consumer list below.
    pub fn visible_worktrees(&self) -> impl Iterator<Item = &Worktree>;
}
```

### Invariants

1. **Superset ordering preserved**: yields worktrees in `self.worktrees` order (already sorted by
   `dir_name` in `reconcile()`), so hiding never reorders the list.
2. **Identity, when revealed**: with `show_agent_worktrees == true`, yields exactly
   `self.worktrees` — the reveal control adds nothing and removes nothing else.
3. **Non-destructive**: `self.worktrees` continues to hold every discovered worktree regardless of
   the toggle; hiding is a view concern only.

### Required consumers

`worktree_tree()`, `available_tag_filters()`, and the sidebar's empty-state hint MUST read from
`visible_worktrees()`. `filtered_worktree_tree()` and `sidebar_entries()` inherit it transitively
and are not changed.

### Required non-consumers

`set_worktrees()`'s pruning and `sessions_in_worktree()` MUST continue to read `self.worktrees`
directly: they reason about *existence*, not visibility, and a hidden worktree still exists. In
particular a rename override for a hidden worktree MUST NOT be pruned.

## Out of scope for this contract

Discovery is unchanged: `parse_worktrees()`, `classify()`, `reconcile()`, and
`main.rs::discover_worktrees()` gain no parameter and no filtering step (research R9). Any design
that drops agent worktrees before they reach `State::worktrees` violates this contract, because it
makes the reveal control unimplementable.
