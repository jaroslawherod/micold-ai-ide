# Contract: sidebar grouping and row text (core)

Module `micold_core::sidebar_groups` (new, `crates/micold-core/src/sidebar_groups.rs`) and one
function added to `micold_core::overview_text`. Pure: no I/O, no clock, no logging (FR-023). Every
item below is a unit test in `micold-core` (FR-020), written red first.

## G1. Group of a card state

`StateGroup::of(state)`: `NeedsAction` and `NeedsAttention` → `NeedsAction`; `Working` →
`Working`; `Idle` → `Idle`; `Done` → `Done`. Total over `CardState::ALL`; no state maps to two
groups (FR-001, Principle II: a location is never in two groups).

## G2. Order

`StateGroup::ALL == [NeedsAction, Working, Idle, Done]`, and `Ord` agrees with it (FR-001, D2).

## G3. Partition

`group_by_state(items)`:

- returns the groups that have at least one item, in G2 order (FR-002);
- each group's items are the input items of that group in input order (FR-003, US1.2, US1.5);
- every input item appears in exactly one group (SC-001);
- a Needs attention item and a Needs action item in one group keep their input order relative to
  each other (US1.2);
- empty input → empty output.

## G4. Header text

`header_text(group, count)` = `"{label} \u{b7} {count}"`: "Needs action · 2", "Working · 1",
"Idle · 3", "Done · 4" (FR-002).

## G5. Open rule

`group_open(choice: Option<bool>, group, holds_current: bool) -> bool`:

| choice | group | holds_current | open |
|---|---|---|---|
| `Some(b)` | any | any | `b` |
| `None` | `Done` | false | false |
| `None` | `Done` | true | true |
| `None` | other | any | true |

(FR-005, US1.3, Edge Cases *Current session in a collapsed group*.) `StateGroup::starts_open()` is
false only for `Done`.

## G6. Run group state

`run_group_state(runs: impl IntoIterator<Item = RunCard>) -> CardState`: the worst state over the
runs, where `RunCard::Worktree(s)` counts as `s`, `NoWorktree { failed: true }` as
`NeedsAttention`, `NoWorktree { failed: false }` as `Idle`; `Idle` for no runs (Edge Cases *Run
groups*).

## T1. Second line (`overview_text::sidebar_second_line`)

`sidebar_second_line(row: RowKind, title: &str, branch: Option<&str>, sessions: (u32, u32)) ->
String`, `RowKind = ProjectRoot | Worktree`:

| Case | Output |
|---|---|
| `ProjectRoot`, (0, 0) | "project root · no sessions" |
| `ProjectRoot`, (2, 1) | "project root · 2 sessions, 1 running" |
| `Worktree`, title "CI red on main", branch "fix/1117_ci-red", (1, 1) | "fix/1117 · 1 session running" (US3.1) |
| `Worktree`, title = branch "fix/1117_ci-red", (1, 0) | "1 session" (US3.2, 1120 FR-007) |
| `Worktree`, title "x", branch "main", (0, 0) | "main · no sessions" (Edge Cases: branch as it is) |
| `Worktree`, no branch, (0, 0) | "No sessions" |

Built from `branch_line` and `meta_sessions` (feature 1120 contracts C6, C7; `sessions_line` became `meta_sessions` in BUG-1298 FR-023, so the phrase is the Overview meta line's, lower-case after " · " and capitalised alone); no copy of either.
