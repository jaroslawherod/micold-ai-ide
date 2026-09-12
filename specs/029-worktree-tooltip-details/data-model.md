# Data Model: Worktree tooltip shows the full name and its details

**Feature**: [spec.md](./spec.md) · **Plan**: [plan.md](./plan.md)

## Nothing is stored

This feature adds no persisted field, no catalog entry, no message, and no application state. The
tooltip is derived on render from data the sidebar already holds, which is what FR-012 requires and
what keeps the hover path free of I/O. `store.rs`, the protocol, and `State` are untouched.

What follows is therefore a description of **inputs and derivation**, not of storage.

## Inputs

Every input already exists and is already in memory when a worktree row renders.

| Input | Source | Notes |
|-------|--------|-------|
| `display_name` | `State::worktree_display_name(dir_name)` (`features/worktree.rs:127`) | The user's rename if there is one, else `naming::display_name(dir_name)`. The row renders the same value, which is what FR-002 pins. |
| `worktree.dir_name` | `Worktree` (`micold-core/src/worktree.rs:116`) | Identity. For an included worktree it is a folder name that may have been disambiguated by `reconcile`. |
| `worktree.branch` | `Worktree` (`:120`) | `Option<String>` — `None` for an orphan or detached worktree. |
| `worktree.path` | `Worktree` (`:118`) | Absolute. |
| `worktree.status` | `Worktree` (`:122`) | `Valid` \| `Missing` \| `Invalid`. |
| `worktree.included` | `Worktree` (`:128`) | `true` when the worktree lives outside `.claude/worktrees/`. |
| `project_root` | `State`, threaded into the sidebar render | `Option<&Path>`; `None` when no project is active. |

## Derived value

A single `String` of newline-separated `Label: value` lines, built by
`features::sidebar::worktree_tooltip`. The full rules — order, omissions, wording — are the
contract: [contracts/worktree-tooltip.md](./contracts/worktree-tooltip.md).

```text
Name: <display_name>                                     always
Branch: <branch>                                         iff branch.is_some()
Folder: <dir_name>                                       iff dir_name != display_name
Location: <path> [ (outside this app)]                   iff project_root.is_some()
Status: <status word>                                    iff status != Valid
```

## One new core method

`WorktreeStatus::label(&self) -> Option<&'static str>` (`micold-core/src/worktree.rs`), the single
source for the status word shared by the row's chip and the tooltip's `Status:` line (research R3).

| Variant | `label()` |
|---------|-----------|
| `Valid` | `None` |
| `Missing` | `Some("missing")` |
| `Invalid` | `Some("invalid")` |

`None` for `Valid` is the point: it is what lets both call sites express "a healthy worktree says
nothing about its health" without either of them re-testing the variant.

## Unchanged by this feature

- **`DEFAULT_LOCATION_LABEL`** (`features/sidebar.rs:390`) — the "Default" entry's fixed tooltip
  keeps its exact current value and its call site (FR-011).
- **Session rows** — nested session rows gain no tooltip.
- **`Tag`, `TagFilter`, `WorktreeNode`** — no new tag, no new node field. `shown_for_current_session`
  and the `included` chip stay exactly as they are; the tooltip reads the same facts, it does not
  restate the chips.
