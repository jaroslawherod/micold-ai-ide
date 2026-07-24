# Data Model: Hide Agent Worktrees

Phase 1 for [plan.md](./plan.md). Entities are drawn from the spec's Key Entities and mapped onto
the existing types in the render-free core. **No persisted schema changes** — nothing here is
written to the store.

## Entity: Worktree ownership

Maps the spec's *Ownership classification* onto a new enum plus two methods on the existing
`Worktree`.

### `WorktreeOwner` (new — `src/worktree.rs`)

| Field / Variant | Meaning |
|---|---|
| `User` | Created by the user through the app (or by hand); always listed |
| `Agent` | Created by an AI assistant for its own sub-task; hidden unless revealed |

- Derives `Debug, Clone, Copy, PartialEq, Eq` — matching `WorktreeStatus`, the enum it sits
  alongside.
- An enum rather than a `bool` (Principle V): a future third owner is an added variant, not a
  boolean-blindness refactor of every call site.

### `Worktree` (existing — `src/worktree.rs`, unchanged fields)

No field is added. Ownership is derived:

| Method | Returns | Notes |
|---|---|---|
| `owner(&self)` | `WorktreeOwner` | Derived from `dir_name` and `branch` |
| `is_agent_owned(&self)` | `bool` | `matches!(self.owner(), WorktreeOwner::Agent)` |

**Derivation rule** (see [contracts/agent-worktree-classification.md](./contracts/agent-worktree-classification.md)
for the normative statement):

```text
Agent  ⟺  dir_name  = "agent-"          ++ H
      ∨   branch    = "worktree-agent-" ++ H          (branch present)
where H = a string of ≥ 16 characters, every one an ASCII hex digit
User   ⟺  otherwise
```

**Location precondition**: FR-005's "directly under the project's managed worktrees directory"
half is already enforced upstream by `reconcile()`, which only emits entries whose parent is
`worktrees_root` — for git-registered records and for orphan on-disk directories alike. The methods
above therefore decide only the naming half, and must not be called on a `Worktree` obtained by any
other route.

### Validation rules

| Rule | Source | Behavior |
|---|---|---|
| Whole remainder must be hex | FR-006 | `agent-deadbeef-cafe-refactor` → `User` (tail not hex) |
| Minimum identifier length | FR-006 | `agent-face` → `User` (4 < 16) |
| Either identifier suffices | FR-005, "Name/branch mismatch" edge case | dir matches, branch does not → `Agent` |
| Missing branch is not disqualifying | FR-007, "Detached worktree" edge case | `branch: None` + matching dir → `Agent` |
| Health state is irrelevant | FR-007 | `Valid` / `Missing` / `Invalid` classify identically |
| Classification is stateless | FR-009, spec Key Entities | Recomputed per call; nothing cached or persisted |

### State transitions

None. Ownership is a pure function of a worktree's names, so it cannot transition without the
worktree itself being renamed — which the app never does to an agent worktree (FR-008).

## Entity: Reveal control

Maps the spec's *Reveal control*.

### `State.show_agent_worktrees` (new — `src/app.rs`)

| Property | Value |
|---|---|
| Type | `bool` |
| Default | `false` (hidden) |
| Lifetime | Transient — in-memory, never written to the store (FR-010a) |
| Scope | The current project in the current run: reset to `false` on every project switch (FR-010e) — like `default_expanded`, **not** like the sticky `sidebar_filters` |
| Mutated by | `Message::ShowAgentWorktreesToggled` (toggle) and `restore_after_activation()` (reset to `false`) |

### `Message::ShowAgentWorktreesToggled` (new — `src/app.rs`)

Unit variant. Reducer flips `show_agent_worktrees` and does **nothing else** — in particular it
must not touch `sidebar_filters`, `expanded`, or any overlay (FR-010d).

### State transitions

```text
        ShowAgentWorktreesToggled
Hidden  ─────────────────────────▶  Revealed
   ▲                                    │
   └────────────────────────────────────┘
        ShowAgentWorktreesToggled

app start / restart ─▶ Hidden   (always, regardless of prior run — FR-010a)
project switch      ─▶ Hidden   (always, in the incoming project — FR-010e;
                                 switching back does not restore it either)
```

## Entity: Visible worktree set

The derived collection every worktree surface reads from. Not a stored field — an iterator, so it
cannot go stale when the toggle flips (research R3).

### `State::visible_worktrees(&self)` (new — `src/app.rs`)

```text
show_agent_worktrees == true   →  all of self.worktrees
show_agent_worktrees == false  →  self.worktrees where !is_agent_owned()
```

### Consumers (all rebased onto it)

| Consumer | Location | Requirement |
|---|---|---|
| `worktree_tree()` | `src/app.rs:1468` | FR-002 — rows |
| `filtered_worktree_tree()` | `src/app.rs:1490` | FR-003 — via `worktree_tree()`, tag filters unchanged |
| `sidebar_entries()` | `src/app.rs:1503` | FR-003 — via `filtered_worktree_tree()`; the `Default` entry stays exempt |
| `available_tag_filters()` | `src/app.rs:1529` | FR-003 — no chips from hidden worktrees (research R7) |
| Empty-state hint | `src/ui/sidebar.rs:102` | FR-003 — "No worktrees yet" vs "No worktrees match the filter" (research R7) |

### Explicit non-consumers

| Kept on the full `self.worktrees` | Why |
|---|---|
| `set_worktrees()` pruning (`src/app.rs:1279`) | Expansion, hover, menu, delete-target and rename overrides are pruned against *existence*, not visibility — a hidden worktree still exists, and its rename override must survive |
| `sessions_in_worktree()` (`src/app.rs:1318`) | Operates on session records by `dir_name`; visibility is irrelevant to which sessions must be terminated |
| Session records | FR-011 — no pruning, no dedicated handling (research R8) |

## Entity: Agent badge

Maps FR-010b onto the existing tag strip.

### `Tag::Agent` (new variant — `src/naming.rs`)

| Property | Value |
|---|---|
| Produced by | `State::worktree_tags()`, appended when `worktree.is_agent_owned()` |
| Not produced by | `parse_tags(dir_name)` — it needs the branch too, so it is added at the same layer as `Tag::Status` |
| Rendered as | Existing shared `Tag` chip primitive, label `"agent"` (never "assistant" — spec Terminology), accent `roles.on_surface_variant` |
| Filterable | **No** — there is no corresponding `TagFilter` variant (research R5) |

### Interaction with existing tag logic

| Site | Change |
|---|---|
| `tag_chip()` (`src/ui/sidebar.rs:317`) | One new `match` arm |
| `matches_filters()` (`src/app.rs:204`) | None — matches on `TagFilter`, which gains no variant |
| `available_tag_filters()` (`src/app.rs:1529`) | Ignores `Tag::Agent` (it already only reads `Type`/`Issue`); its `Untyped` detection now runs over the visible set only |

A revealed agent worktree therefore carries exactly one chip — `agent` — plus a `missing`/`invalid`
status chip if unhealthy.

Because its machine name carries no conventional type, a **revealed** agent worktree does count as
untyped, so the panel may offer an `Untyped` filter chip that matches it. That is correct and
required: FR-010d says tag filters apply to revealed entries exactly as they apply to user-created
ones. While **hidden** it contributes no chip at all, which is the R7 defect this rebasing fixes.
