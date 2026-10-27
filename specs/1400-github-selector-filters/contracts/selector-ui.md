# Contract: selector UI

`crates/micold-client/src/ui/github_filter.rs` (new) lays out what
`features::worktree_form::github_filter` decides; `ui/worktree_form.rs::issue_picker` gets hook
lines only. View glue under the GUI exception (Constitution I): its look is checked by the geometry
gates and quickstart §B. Research R8, R13, R14.

## 1. Layout (GitHub source showing, GitHub issues on)

```text
Basic mode                                        Advanced mode
[Basic|Advanced]   [Clear filters] [Filters · 2 v] [Basic|Advanced]
Reads issues of acme/app from GitHub, sending …   Reads issues of acme/app from GitHub, sending …
[Search issues ...............................]   [Query ........................] [Run]
Excluding labels in-progress, blocked             [Search issues .......................]
<rows | loading | notice+Retry | GitHub's messages | refusal | empty text>
```

- The switch is `ui::jira_query::mode_switch` generalised (research R12): `mode_switch(mode,
  on_change, r)`; Jira and GitHub both call it. Shown only while the GitHub source shows (FR-001);
  the Jira layout is unchanged.
- **Clear filters** trails the switch row in basic mode while `!basic.is_default()` (1185 D12;
  FR-007); hidden in advanced mode.
- The **Filters** button trails the row above the search (after **Clear filters**), not the search's
  row: beside the search it narrowed the issue list by 168 px and long titles no longer fit it (M2,
  deviation accepted). `material::MenuSelect`, width 160 px, label `filters_label(chosen_count)`
  ("Filters" / "Filters · 2"), panel 360 px; the panel frame is the function extracted from
  `jira_filters_control` (research R13).
- The active line: `BasicFilters::described()` under the search row, one line, ellipsized with the
  full text as tooltip; hidden when default and in advanced mode (FR-007; Edge "Default filter").
- Advanced: `material::TextField` "Query", placeholder "Type a GitHub search query, then press
  Enter or Run", `.on_submit(Run)`; **Run** `Button`. Shown in place of the Filters button and the
  active line (US3 AS1). Before any run on the project: the rows area shows the same prompt text
  (FR-013).

## 2. The Filters panel (FR-003, FR-005)

Rows, in order:

1. **Include labels** — `Typeahead` over `LabelChoices` minus labels already in `include`; a label
   held by `exclude` is offered too, and picking it moves it (spec Edge "Same label included and
   excluded"). Picking adds to `include` (removing from `exclude`), clears the field. Under it, each chosen label
   as `ToggleChip::new(name).active(true).trailing_icon(Icon::Close)`; press removes. A remembered
   label absent from loaded choices shows "<name> · not in this repository" (Edge).
2. **Exclude labels** — the same over `exclude`.
3. **State** — `ToggleChip`s "Open", "Closed", "All", one active.
4. **US6 Assignee** — `Typeahead`: "Me", "No assignee", then people matching the typed text;
   single value; first row "Any assignee" clears.
5. **US6 Author** — "Me", people, then "Use login <typed>" when the typed text is a valid `Login`;
   first row "Any author".
6. **US6 Milestone** — "No milestone", open milestones by title; first row "Any milestone".

Choice states stand in a field's list as rows (1185): "Loading labels…"; the failure notice of
034 FR-007 with **Retry** (that field only, Edge "Choices fail to load"); "This repository has no
labels" (field disabled); "Only the first 1,000 labels are offered"; "2 labels can't be offered
because their names hold a quote".

## 3. List area states

| State | Shown |
|---|---|
| `Loading` | "Loading issues from GitHub…" (034) |
| `Failed { e }` | `e.message(source)` + **Retry** (034 FR-007; FR-019) |
| `Rejected(m)` non-empty | "GitHub did not accept the search query:" then each message on its own line, error tint, in order (FR-018) |
| `Rejected([])` | "GitHub did not accept the search query." |
| `Refused(r)` | `r.message()` (FR-012), error tint |
| `Loaded`, empty, basic | "No issue matches these filters." + **Clear filters** (FR-020) |
| `Loaded`, empty, advanced | "No issue matches this query." (FR-020, US3 AS5) |
| `Loaded`, cap reached | 034's cap caption ("Showing the 1,000 most recently updated of N…"; with a sort qualifier "Showing the first 1,000 of N…") (FR-014) |
| No search match with a filter active | "No issue matches; a filter is active." / incomplete list: "No loaded issue matches. The issue may be outside the filter." (Edge "Typed issue number with a filter") |

Rows: `issue_row` unchanged except the closed status chip (data-model §8). On a closed issue the
Closed chip takes precedence over a status the project's status board holds for it (1101 FR-010):
FR-014 requires a closed issue to be marked closed, and a board status on a closed issue is
behind GitHub's state. The chip is then not stale. An open issue keeps today's order (board
status, else the listing's).

## 4. Keyboard (FR-026)

Tab order: switch chips → Clear filters → search → Filters button (basic) / Query → Run
(advanced). The Filters panel opens with Enter/Space on the button; Tab moves through its fields
and chips; Escape closes it (MenuSelect's behaviour). Enter in Query runs; Enter in a label field
with a highlighted row adds that label. Every chip is a `ToggleChip` with `takes_the_keyboard()`.

## 5. Geometry

- `tests/gates/github_filters_fit.rs` (new): the Filters button fits the row above the search; the panel
  stacks its rows under the button inside the window; a chip row with 6 labels wraps inside the
  panel; the active line and the query row each fit one line.
- `tests/gates/panel_placement.rs`: a GitHub scenario in each mode with the active line / query row
  and the cap caption shown — the footer stays reachable (the tallest dialog).
- `tests/gates/issue_rows_show_all_text.rs`: an arm with a closed issue row (status chip fits),
  and an arm with a closed issue the status board holds as "In Progress" (stale): the row shows
  "Closed", not stale (§3).
