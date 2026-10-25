# Contract: Jira filters in the create-worktree dialog

Reducer: `crates/micold-client/src/features/worktree_form/jira_filters.rs` (render-free). Shell:
`crates/micold-client/src/shell/issues.rs` (+ `jira_filters` helpers). View:
`crates/micold-client/src/ui/worktree_form.rs`.

## 1. When filters show
- Shown only while `form.jira_site().is_some()` and `form.source == BranchSource::Issue`, for every
  `IssueList` state but `NotRequested` (filters stay during a load or after a failure, FR-013).
  *As built (ledger D12, D15):* after a failed listing the **Filters** button trails **Retry**,
  the chosen filters named between them, with **Clear filters** above.
- Never shown for GitHub; never shown with Jira off or unconnected (no request either, 533 FR-020).
- Leaving Jira, leaving the Issue source, a Jira availability change, or opening the form resets
  `FilterState` (FR-016).

## 2. Layout
```
[Reporter ▾] [Label ▾] [Component ▾] [Fix version ▾]      ← one row, equal widths
Filtered by component Payments, fix version 2.4   [Clear filters]   ← only while any is chosen
[Issue search typeahead …]
Showing the 1,000 most recently updated open issues matching the filters.   ← cap caption
```
*As built (ledger D9, D12):* beside feature 1186's Basic / Advanced switch, the dialog has no line
to spare, so the layout is
```
[Basic] [Advanced]                                         [Clear filters]   ← while any is chosen
Reads open issues of project KEY from ORIGIN. Sends …       ← statement, unchanged by filters
[Issue search typeahead …]          [Component ▾] …        ← fields trail the main row
Filtered by component Payments. Showing the 1,000 most recently updated.   ← one line
```
*As built from M3 (ledger D15, awaiting jaro's confirmation):* the fields left the main row for a
panel, and the search has the row's width less one button:
```
[Basic] [Advanced]                                         [Clear filters]   ← while any is chosen
Reads open issues of project KEY from ORIGIN. Sends …       ← statement
[Issue search typeahead ……………………………]  [Filters · 2 ▾]     ← button: count while any is chosen
Filtered by component Payments. Showing the 1,000 most recently updated.   ← one line
                                ┌───────────────────────┐
                                │ [Reporter ▾]          │  ← panel, 360 px, floats under the
                                │ [Component ▾]         │    button, trailing edges aligned;
                                │ [Fix version ▾]       │    one field per filter, stacked
                                └───────────────────────┘
```
- The button (`PanelToggled`) opens and closes the panel; Escape or a press outside closes it
  (`PanelDismissed`), and closing it closes any open field list (text back to the chosen value).
  While it is open, Tab and Shift+Tab cycle through the Filters button and the panel's fields
  (the panel keeps the key); Escape closes it and puts the keyboard back on the button, from which
  Tab goes on through the dialog. The panel stays open across picks and the reloads they
  start; a reload answered while it is open does not open the issue list under it. Opening it asks
  Jira nothing: each field still asks for its choices on its own first focus.
- Feature 1186: the switch to Advanced closes the panel and its lists; the reducer refuses every
  filter message but a close while advanced mode shows.
- Field label = filter name; field text = chosen value's label (reporter: "You (name)" for the own
  user, else the display name), else the typed text, else empty with the label resting.
- List rows: row 0 "Any <filter>"; then ranked choices; each row truncates and has the full text as
  tooltip. Reporter: own user first, then the typed search's people.
- Choice states in place of rows (via the list's empty message or a single disabled row):
  loading "Loading components…"; failed: the cause (`IssueLoadError::message`) and a **Retry** row
  that sends `ChoicesRetry(kind)`; empty "This project has no components" / "no versions" /
  "No labels on the loaded issues"; reporter with no match "No project user matches; you are still
  offered"; labels partial: a trailing disabled row "Only labels of the 1,000 most recent issues";
  labels while the unfiltered load runs/failed: "Labels load with the issues" / "Labels need the
  issue list: retry it above".

## 3. Behaviour
| Event | Effect |
|---|---|
| focus a request-backed field, `NotRequested` | start its choice request (reporter: `myself`) |
| type in reporter field | debounce 300 ms, then `project_users(text)`; newer text discards older result |
| pick a value / Any / Clear filters | `chosen` updated; if changed: new `issue_request_seq`, `IssueList::Loading`, load with filters |
| type in issue search | unchanged rule; search beyond carries `chosen` |
| filtered listing empty | "No open issue matches the filters." + **Clear filters** (FR-011) |
| no search match with filters | typeahead empty message "No open issue matches the search and filters." |
| listing/search failure | as today (533 FR-018); filter row and line stay |
| pick an issue | unchanged (FR-017) |

## 4. Statement (FR-014)
`WorktreeForm::jira_notice` becomes: "Reads open issues of project KEY from ORIGIN. Sends the
project key, your search text, the filters you choose, the name you type to find a reporter and
your credentials, and, with this dialog closed, the keys of the tickets your worktrees are linked
to, to show their status." While filters are chosen, followed by "Filters sent: <same list as the
active line>." *As built (ledger D12):* no suffix; the statement says it sends "the filters you
choose" and the active line under the issue search names the values sent, so a fourth line of
statement does not push the tallest dialog over the app bar. *As built from M3 (ledger D15):* "Reads
open issues of project KEY from ORIGIN. Sends the project key, your search text, the filters you
choose, a name typed to find a reporter and your credentials, and, with this dialog closed, linked
tickets' keys, to show their status." — the §4 wording one line too long for the tallest dialog
(`panel_placement` red by 8 px), so "the name you type" became "a name typed" and feature 1101's
clause "the keys of the tickets your worktrees are linked to" became "linked tickets' keys".
