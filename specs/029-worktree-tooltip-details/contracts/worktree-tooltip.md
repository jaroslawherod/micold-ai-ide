# Contract: the worktree row tooltip

**Feature**: 029 · **Spec**: [spec.md](../spec.md) · **Plan**: [plan.md](../plan.md)

The rules a test may assert without reading the implementation. Every rule is a statement about the
`String` returned by `features::sidebar::worktree_tooltip`, read as `.lines()`.

## §1 Signature

```rust
pub fn worktree_tooltip(
    project_root: Option<&Path>,
    worktree: &Worktree,
    display_name: &str,
) -> String
```

**§1.1** Pure and total: no I/O, no clock, no global state; defined for every combination of its
inputs, including `project_root: None`, `branch: None`, and every `WorktreeStatus` (FR-012).

**§1.2** Deterministic: the same inputs produce the same string, byte for byte.

**§1.3** It replaces `worktree_location_label`. `DEFAULT_LOCATION_LABEL` is untouched and keeps its
own call site (FR-011).

## §2 Lines and their order

Lines appear in exactly this order; each is `"<Label>: <value>"`.

| # | Label | Present when | Value |
|---|-------|--------------|-------|
| 1 | `Name` | always | `display_name`, verbatim (FR-001, FR-002) |
| 2 | `Branch` | `worktree.branch.is_some()` | the branch name, verbatim (FR-004) |
| 3 | `Folder` | `worktree.dir_name != display_name` | `dir_name`, verbatim (FR-005) |
| 4 | `Location` | `project_root.is_some()` | see §3 (FR-003) |
| 5 | `Status` | `worktree.status.label().is_some()` | that word (FR-006) |

**§2.1** No line is ever empty and no label ever appears twice.

**§2.2** A line that is not present is **absent**, not blank and not a placeholder — the string has
no empty line and no `"Branch: "` with nothing after it (FR-004).

**§2.3** The first line is always `Name: …` (FR-001): the fact a shortened row could not show is
the fact the tooltip leads with.

## §3 The `Location` value

**§3.1** It is exactly what `worktree_location_label` produced before this feature: `path` relative
to `project_root` when the worktree is under it, the absolute path otherwise (FR-003). This
behaviour is not redesigned here, and the tests that pinned it continue to hold against the new
shape.

**§3.2** When `worktree.included` is `true`, the value gains the suffix `" (outside this app)"`
(FR-007, research R6). The phrase matches the row's own chip, and the path it follows is absolute
by construction (feature 016 BUG-002).

**§3.3** When `project_root` is `None` the line is absent and every other line still renders
(research R5) — the tooltip never disappears because the root is unknown.

## §4 The status word

**§4.1** `WorktreeStatus::label()` is the only source: `Missing → Some("missing")`,
`Invalid → Some("invalid")`, `Valid → None` (research R3).

**§4.2** The row's status chip reads the same method, so chip and tooltip cannot disagree about a
row (FR-006).

**§4.3** A `Valid` worktree's tooltip contains no `Status:` line at all — normal reads as normal.

## §5 Rendering

**§5.1** `ui/sidebar.rs` attaches the returned string via `TreeItem::row_tooltip` for **every**
worktree row, unconditionally (research R5). It makes no decision about content.

**§5.2** The shared `material::Tooltip` bounds its panel at the component's own `MAX_WIDTH` (320dp)
and wraps at `Wrapping::WordOrGlyph`, so a long path breaks rather than widening the panel
(FR-009, research R2).

**§5.3** Position and trigger are unchanged: hover to show, unhover to dismiss, opening below the
row, snapped inside the window by the rendering stack (FR-010, SC-005).

**§5.4** The row keeps ellipsizing its own label. This feature adds a way to read the whole name; it
does not stop the row from shortening it.

## §6 What this contract does not cover

- The "Default" entry's tooltip (FR-011) — unchanged, and asserted by its existing test.
- Nested session rows — no tooltip is added.
- Any change to how `display_name` is derived, or to the rename feature that overrides it.
