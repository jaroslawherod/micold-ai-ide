# Contract: the selector in advanced mode

**Feature**: [spec.md](../spec.md) | Research [R8](../research.md#r8--the-selector-a-mode-switch-of-togglechips-and-the-shared-typeahead-given-an-on_submit), [R9](../research.md#r9--state-messages-and-requests-in-the-client) | Data model [§6–§7](../data-model.md#6-jiraquery-client--the-selectors-advanced-state-one-field-of-worktreeform)

## 1. Layout (`crates/micold-client/src/ui/jira_query.rs`, new)

Shown only while `form.jira_site().is_some()` and `form.source == BranchSource::Issue` (FR-001;
GitHub source and integration off: nothing, SC-005).

```
[ GitHub ][ Jira ]                     ← provider_switch (533), unchanged
[ Basic ][ Advanced ]                  ← mode_switch: two ToggleChips, active = jira_query.mode
Reads issues of project ABC matching your query from https://…  Sends …   ← notice (FR-015)
┌ Query ─────────────────────────────┐ [ Run ]
│ assignee = currentUser() AND …   ✕ │
└────────────────────────────────────┘
  ABC-42  [In Progress]  Fix login …   ← rows: the basic row form (FR-006)
Showing the first 1,000 issues this query returns.   ← only when !complete
```

- `mode_switch(form, r)`: `ToggleChip::new("Basic", JiraQuery(ModeChanged(Basic)), r)` and
  `"Advanced"`, `.active(..)`; it heads the issue picker's column (one `if` at the top of
  `ui::worktree_form::issue_picker`), not a dialog field of its own: the form is the tallest dialog,
  and a field's gap pushed it over the app bar (M3, decision D10).
- `picker(col, form, board, r)`: called from the top of `ui::worktree_form::issue_picker`, after the
  switch, when the mode is advanced (one early `return`), so the basic body below is untouched.
- Query field: `material::Typeahead::new(&draft, rows, DraftChanged, r).label("Query")
  .placeholder(PROMPT).open(list_open).highlighted(highlight).empty_message(EMPTY)
  .on_submit(Run).on_focus(Focused).on_move(HighlightMoved).on_dismiss(Dismissed)
  .on_pick(RowPicked)`, in a `row!` with `Button::text("Run", r).on_press(Run)`.
- Rows: built by the same row builder the basic picker uses (`issue_rows_with`'s per-issue part,
  extracted so both call it), from `Loaded.listing.issues` in order, with empty match spans.
- In place of the rows: `Idle` with empty `last_run` → prompt caption; `Refused` → its text,
  `r.error` tint; `Running` → `StageProgress("Running the query on <host>…")`; `Failed(Rejected)` →
  one error-tinted line per message, or the plain text; `Failed(Load)` → `error.message(source)` and
  **Retry** (`JiraQuery(Retry)`), as basic mode's failure; `Loaded` empty → `No issue matches this
  query.` muted.
- A failed run's lines scroll in a viewport one caption line high (`FAILURE_MAX_HEIGHT`, 20 px),
  and **Retry** sits in the field's row after **Run**: the form is the tallest dialog, and a second
  line or a row of its own pushed it over the app bar (M4, decision D11).

## 2. The shared `Typeahead` gains `.on_submit` (`crates/micold-client/src/ui/material/typeahead.rs`)

`pub fn on_submit(mut self, message: M) -> Self` — handed to the inner `TextField::on_submit`.
Enter with a highlighted, enabled row is claimed by the list and picks (unchanged); Enter with no
highlighted row reaches the field and publishes `message`. Without `.on_submit` the Typeahead
behaves exactly as before (every existing call site). Documented in the component's module header
(Principle VII, VIII).

## 3. Keyboard

Enter in the field runs (no row highlighted) or picks (row highlighted); Up/Down move; Escape
closes the list; Tab leaves the field, then reaches **Run**. Typing never runs (FR-004).

## 4. The notice (FR-015)

`jira_query::notice(site)`, used by `WorktreeForm::jira_notice` while the mode is advanced:
`Reads issues of project {KEY} matching your query from {origin}. Sends the project key, your query
and your credentials, and, with this dialog closed, the keys of the tickets your worktrees are
linked to, to show their status.`

## 5. Pick (FR-009)

`RowPicked(index)` resolves the `index`-th issue of `Loaded` and applies the same body as
`worktree_form::issue_picked` (ticket, name, type from labels and issue type, picked issue,
default prompt); that body is factored into `WorktreeForm::apply_pick(&Issue, mapping)` so both
paths call it. `WorktreeForm::issue_number_at` and `refresh_default_prompt`'s lookup read the shown
list (`shown_issues()`: advanced results in advanced mode, `issues.held()` otherwise).
