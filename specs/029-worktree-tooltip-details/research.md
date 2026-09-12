# Research: Worktree tooltip shows the full name and its details

**Feature**: [spec.md](./spec.md) · **Plan**: [plan.md](./plan.md) · **Date**: 2026-09-03

Six questions the plan could not answer by assertion. Each is recorded as a decision, the reason
it was taken, and what was rejected — because on a feature this small, the *shape* of the answer is
the whole design.

---

## R1 — Every line is labelled, including the name

**Decision**: the tooltip is a block of `Label: value` lines, name first:

```text
Name: Tooltip of worktree should show full name
Branch: feat/tooltip-of-worktree-should-show-full-name
Folder: feat-tooltip-of-worktree-should-show-full-name
Location: .claude/worktrees/feat-tooltip-of-worktree-should-show-full-name
```

**Rationale**: FR-008 requires each fact on its own labelled line, and an unlabelled first line
would be an exception the tests would have to encode as one. A uniform block is also what makes the
contract checkable by reading `.lines()` — every assertion in
[contracts/worktree-tooltip.md](./contracts/worktree-tooltip.md) is a statement about which lines
exist and in what order, which is only simple while *all* of them have the same shape.

**Alternatives considered**:

- *Material 3 rich-tooltip shape* — a subhead (the name, unlabelled) over supporting text. It is
  the prettier surface and it is what the M3 spec describes, but the subhead is meant for a
  *content* tooltip with a title, not a diagnostic list where the name is one fact among five. It
  would also put a special case in the middle of the builder for no testable gain.
- *One run-on string* (`name — branch — path`) — smallest change, and it fails FR-008 outright.
  It is also what the row already is: a single line that runs out of room.

---

## R2 — The tooltip is bounded by a component-owned ceiling, not an anatomy token

**Decision**: `ui/material/mod.rs` gains its own documented `MAX_WIDTH` constant (320dp, Material
3's rich-tooltip ceiling), applied to the tooltip's inner container, and the tooltip's text wraps
at `Wrapping::WordOrGlyph`.

**Rationale**: `Wrapping::Word` is iced's default and cannot break a long path — a worktree path
contains no spaces, so word wrapping would leave one unbreakable run and the ceiling would be
ignored. `WordOrGlyph` (`iced_core::text::Wrapping`, 0.14) falls back to glyph level exactly for
that case. The ceiling itself is a *component* figure, in the same class as
`ui/material/tab.rs`'s `LABEL_MAX_WIDTH` — a number the component owns and documents — not a
figure from feature 018's §7, which states no tooltip row.

**Alternatives considered**:

- *Add `tokens::anatomy::tooltip::MAX_WIDTH`* — rejected. `anatomy::ALL` is defined as "the figures
  §7 states", `micold-core/tests/tokens_anatomy.rs` asserts each constant against that contract,
  and `micold-client/tests/anatomy_call_sites.rs` walks the same array. Adding a figure §7 does not
  contain would mean editing feature 018's contract from inside feature 029 — cross-feature churn
  for a number one component uses once.
- *Pre-wrap the text in the render-free builder* — rejected. Wrapping depends on the rendered glyph
  widths and the font; a pure function guessing at them would be wrong at every font size and would
  put a layout decision in a module that cannot measure anything.
- *Leave it unbounded* — rejected by SC-005. The rendering stack's own
  `snap_within_viewport` (default `true` in `iced_widget::tooltip`) keeps the tooltip *inside* the
  window, but a tooltip wider than the window would be snapped and still clipped. Snapping bounds
  the position; only `max_width` bounds the size.

---

## R3 — The status word has one source, in the core

**Decision**: add `WorktreeStatus::label(&self) -> Option<&'static str>` to
`micold-core/src/worktree.rs` (`Missing` → `"missing"`, `Invalid` → `"invalid"`, `Valid` → `None`).
The status chip in `ui/sidebar.rs:332-339` and the tooltip's `Status:` line both read it.

**Rationale**: the chip's word and the tooltip's word describe the same row, and a user seeing them
disagree would rightly stop trusting both. The chip's current `match` already spells `Valid` as
`""` — an empty chip that only never renders because the caller happens to check the status again
first. `Option` makes "healthy has no word" a fact the type states (Principle V) instead of an
invariant two call sites have to remember.

**Alternatives considered**:

- *Duplicate the wording in the tooltip builder* — rejected: two literals, one meaning, and no test
  that could notice them drifting.
- *Put the word in `features/sidebar.rs`* — rejected: the chip lives in `ui/`, which is downstream
  of `features/` but also of `micold-core`; the core is the only place both can read from without
  one render module importing another's private vocabulary.

---

## R4 — The builder returns a `String`, not a structured block

**Decision**: `worktree_tooltip(project_root: Option<&Path>, worktree: &Worktree, display_name:
&str) -> String`, replacing `worktree_location_label`. `ui/sidebar.rs` passes the result straight to
`TreeItem::row_tooltip`.

**Rationale**: keeps the render layer at one call and no branch — the condition the Principle I GUI
exception is scoped to. A `Vec<(label, value)>` would move the join (and therefore the `: `, the
line order and the omissions) into `ui/`, where no test can reach it. Assertions on a `String` are
not weaker here: every rule in the contract is an assertion about `.lines()`.

**Alternatives considered**:

- *A `WorktreeTooltip` struct with a `Display` impl* — equivalent in testability, one more type for
  no additional caller. Rejected as ceremony; if a second surface ever needs the fields separately,
  the struct is a refactor away and the tests already pin the rendering.
- *Keep `worktree_location_label` and add a second function* — rejected: two functions, both
  attached to the same row, is how the chip and the tooltip would come to disagree (see R3).

---

## R5 — The tooltip is attached unconditionally; the location line is what is conditional

**Decision**: `ui/sidebar.rs` attaches the row tooltip for every worktree row. When the project
root is unknown, `worktree_tooltip` omits the `Location:` line and keeps the rest.

**Rationale**: today the whole tooltip is inside `if let Some(root) = project_root`
(`ui/sidebar.rs:562-565`), so a row with no known root has no tooltip at all — which under FR-001
would mean no way to read a shortened name. Moving the conditional inside the builder makes the
function total over its inputs and moves the one remaining branch out of `ui/`.

**Alternatives considered**:

- *Keep the outer `if let`* — rejected: it leaves a decision in the render layer and a hole in
  FR-001 for a state the type system says is reachable.

---

## R6 — "Outside this app" is stated on the `Location:` line

**Decision**: for an `included` worktree the location value gains a parenthetical:
`Location: /abs/path (outside this app)`. The phrase matches the row's existing chip
(`ui/sidebar.rs:544`) and the core's own wording for the same condition
(`micold-core/src/worktree.rs:482`, `branch_candidates.rs:189`).

**Rationale**: FR-007's fact *is* a fact about location — an included worktree is one that lives
somewhere else — so it belongs to that line rather than to an invented label. Feature 016's
BUG-002 already established that these rows show an absolute path for exactly this reason; the
parenthetical says out loud what the absolute path implies.

**Alternatives considered**:

- *A separate labelled line* — needed a label, and every candidate (`Note:`, `Placement:`,
  `Included:`) either says nothing or leaks the field name into the UI.
- *Fold it into `Status:`* — rejected: a worktree can be both outside this app *and* missing, and
  one line cannot carry two independent facts without becoming the run-on string R1 rejected.
