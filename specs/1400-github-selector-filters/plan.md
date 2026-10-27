# Implementation Plan: Filter the GitHub issue selector by search query, label and state

**Branch**: `claude/project-thread-5zc5qk` | **Date**: 2026-10-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1400-github-selector-filters/spec.md`

## Summary

Give the GitHub source of the New worktree form's issue selector the two modes the Jira source has
(1185, 1186). **Basic**: a **Filters** button opens a panel with include labels, exclude labels
(the repository's labels, as chips) and state (open / closed / all), and (US6, last milestone)
assignee, author and milestone. **Advanced**: a query input runs a GitHub issue search query on
Enter or **Run**, after local checks refuse scope qualifiers (`repo:`, `org:`, `user:`, `owner:`),
unclosed quotes, unbalanced parentheses and texts over 1,000 characters. Any non-default filter is
sent as one scoped search text — `repo:<o>/<n> is:issue <basic terms | (condition)> <sort or
sort:updated-desc>` — through GraphQL `search(type: ISSUE_ADVANCED)`, paged to the existing
1,000-issue cap; results outside the repository and pull requests are dropped; closed issues carry a
"Closed" status chip. GitHub's refusal shows its own messages; other failures show 034's notices
and Retry. The search text stays in both modes and narrows the filtered list locally, with 034's
search beyond the cap carrying the filter. The default filter sends exactly what the selector sends
today. The mode, the basic filters and the last run query are remembered per project in an
owner-only file under the app's data directory, and deleted when the project is forgotten.

Design: [research.md](./research.md) (R1–R16), [data-model.md](./data-model.md),
[contracts/](./contracts/), [quickstart.md](./quickstart.md).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain, `mise`).

**Primary Dependencies**: existing only — `iced` (GUI), `micold-material` (shared widgets),
`serde`/`serde_json`, the `gh` CLI the selector already runs (`GhCli`). No new crate.

**Storage**: one JSON file per project, `<data_dir>/github/<project id>.selector.json`,
owner-only (`micold_core::owner_only::write`), read at form open, written one field at a time
(research R11).

**Testing**: `cargo test` via `mise run test-core` / `mise run gate`; core tests in
`crates/micold-core/tests/`, client reducer, shell and source-scan tests in
`crates/micold-client/tests/`, geometry gates in `crates/micold-client/tests/gates/`, the recorded
quickstart §B pass against `examples/fake_gh.rs`.

**Target Platform**: Linux, macOS, Windows desktop.

**Project Type**: desktop app (Cargo workspace: `micold-core` render-free core, `micold-client` iced
GUI, `micold-material` shared widgets; `micold-daemon` untouched).

**Performance Goals**: one filtered listing is 1–10 `gh` runs (100 issues each, cap 1,000), each
within the existing 10 s budget; local checks are linear in at most 1,000 chars; no per-keystroke
request (query runs on Enter/Run; people search debounced 300 ms).

**Constraints**: the default filter's requests are byte-for-byte 034's (FR-009); no request while
the GitHub source is not showing or GitHub issues are off; no filter value or query in a log,
notification or session history (FR-024); the tallest dialog keeps its footer reachable.

**Scale/Scope**: one core module (~500 lines with its rules), one client state module, one shell
module, one view module, five `IssueSource` methods with default bodies, one shared helper module
each in core (`query_text`) and shell (`remembered`), a dev-only fake `gh`, a user-guide section.

No NEEDS CLARIFICATION remains. D1 (US6 in scope) is provisional pending the user (ledger); the
plan keeps US6 separable (§US6 separability).

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design (below).*

| # | Principle | Status | How |
|---|---|---|---|
| I | Test-First | PASS | Every rule is in `micold_core::github_filter` or the render-free `features::worktree_form::github_filter` and starts from a failing test (§Test strategy). Only `ui/github_filter.rs` and the hook lines in `ui/worktree_form.rs` use the GUI exception, covered by geometry gates and quickstart §B. |
| II | Multi-Session | PASS | No session state. The selector state lives in one window's open form; the remembered file is per project, read only at open, so two windows never change each other's open form (spec Edge "Two windows"; a two-`App` test). Nothing goes to a session's history (FR-024). |
| III | Worktree Integration | PASS (N/A) | A pick fills the form exactly as today (FR-017); no file or VCS step is added. |
| IV | Local-First | PASS | The remembered state is a local owner-only file in the per-user data directory. Requests go only to the user's own signed-in `gh`, already opted in by the GitHub issues switch (1102); the picker's statement names the filter (FR-023). Offline: 034's notice and Retry; everything else works. |
| V | Rust + iced | PASS | `LabelName`, `Login`, `MilestoneTitle`, `UserQuery` exist only through validation; `ScopedSearch` only from a `GithubFilter`, and the filtered `IssueSource` methods take only `&ScopedSearch`, so a refused or unscoped query cannot be sent. `BasicFilters`' setters keep include and exclude disjoint. `IssueList::{Refused, Rejected}` make "error and results at once" unrepresentable. |
| VI | Cross-Platform | PASS | No OS branch: the file path is `paths::data_subdir("github")`, written by `owner_only` (which abstracts Unix modes and Windows ACLs); `gh` is located as 034 does on each platform. CI runs all three. |
| VII | Documentation | PASS | `docs/user-guide/worktrees-and-sessions.md` gains "Filtering GitHub issues" after "Searching beyond the 1,000 loaded issues" (basic filters, advanced mode, scope, sort and refusal rules, errors, remembered per project) in the same changes (FR-027). |
| VIII | Reusable UI | PASS | Reuses `MenuSelect` (panel), `Typeahead` (label and US6 fields), `ToggleChip` (switch, state, removable label chips via `trailing_icon`), `TextField` (`.on_submit`), `Button`, `Text`, `StageProgress`, and `TypeaheadRow::status` for the Closed chip. The Basic/Advanced switch and the panel frame are extracted from the Jira code and shared, not copied (research R12, R13). No new widget. |

### Re-check after Phase 1 design

Unchanged. The contracts add no widget, no session state, no remote service and no OS branch. The
new `IssueSource` methods have default bodies, so no other implementor changes. Complexity
Tracking stays empty.

## Project Structure

### Documentation (this feature)

```text
specs/1400-github-selector-filters/
├── spec.md
├── plan.md                 # this file
├── research.md             # R1–R16
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── search-rules.md         # cleaning, refusals, sort split, scoped text, values, answers
│   ├── source-and-storage.md   # IssueSource additions, requests, privacy, shell, file
│   └── selector-ui.md          # layout, panel, list states, keyboard, geometry
├── checklists/
└── tasks.md                # /speckit-tasks (next unit)
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── github_filter.rs        # NEW: LabelName, IssueState, BasicFilters, UserQuery, QueryRefusal,
│                           #   GithubFilter, ScopedSearch, FilterFailure, SEARCH_* queries,
│                           #   parse_search_page, load_filtered, LabelChoices, load_label_choices,
│                           #   RememberedSelector, SelectorChange, NOT_ACCEPTED
│                           #   (US6: Login, Assignee, Author, MilestoneTitle, Milestone, Person,
│                           #   MilestoneChoices, their terms and parsers)
├── query_text.rs           # NEW: clean, cut_message (moved from jira_query.rs)
├── jira_query.rs           # uses query_text; QueryMode doc made provider-neutral
├── github.rs               # Issue.closed; IssueSource::{search_page, search_filtered, labels_page,
│                           #   (US6) milestones_page, assignable_users} with default bodies;
│                           #   GhCli + FakeIssueSource impls; node reader keeps closed issues for
│                           #   filtered pages. LIST_QUERY, SEARCH_QUERY, list_args, search_args untouched.
├── store.rs                # + github_selector_dir, github_selector_path, github_selector_file_name
└── lib.rs                  # + pub mod github_filter; pub mod query_text;
crates/micold-core/tests/
├── github_filter.rs        # NEW
└── github_gh_cli.rs        # extended

crates/micold-client/src/
├── features/worktree_form/github_filter.rs  # NEW: GithubSelector, Msg, reducer, active filter,
│                                            #   statement text, empty/refusal/rejection texts
├── features/worktree_form/jira_filters.rs   # ChoiceLoad::{start, answer} pub(super)
├── features/worktree_form.rs  # + field github_selector; Msg::GithubFilter; IssueList::{Refused,
│                              #   Rejected}; hooks in source_changed, issue_provider_changed,
│                              #   reset_issues, issue_notice, issue_search_request, issues_loaded
├── features/mod.rs            # + Outcome::GithubSelectorRemembered
├── app.rs                     # partition GithubSelectorRemembered beside JiraQueryRemembered
├── shell/github_filter.rs     # NEW: filtered load/search starters, choice starters, remember
├── shell/remembered.rs        # NEW: owner-only read-modify-write helper (from shell/jira.rs)
├── shell/jira.rs              # read/remember/remove_query call shell::remembered
├── shell/issues.rs            # on_form_opened reads the selector file; start_issue_load /
│                              #   start_issue_search branch on a non-default filter; gh locating
│                              #   factored out for reuse
├── shell/daemon_sync.rs       # on_project_forget_confirmed deletes the selector file
├── shell/mod.rs, main.rs      # module decls; route FormMsg::GithubFilter to shell::github_filter
├── ui/github_filter.rs        # NEW: filters_control, panel rows, query row, list-state texts
├── ui/jira_query.rs           # mode_switch(mode, on_change, r) generalised
├── ui/worktree_form.rs        # hook lines in issue_picker; panel frame extracted from
│                              #   jira_filters_control; issue_row Closed chip
└── ui/mod.rs                  # + mod github_filter;
crates/micold-client/examples/fake_gh.rs     # NEW, dev-only (research R16)
crates/micold-client/tests/
├── github_filter_state.rs     # NEW
├── github_selector_shell.rs   # NEW
├── issues_are_requested_only_on_named_events.rs   # extended
└── gates/{github_filters_fit.rs (NEW), panel_placement.rs, issue_rows_show_all_text.rs}
docs/user-guide/worktrees-and-sessions.md    # + "Filtering GitHub issues"
```

**Structure Decision**: the workspace's existing layout, as 1185 and 1186: rules in
`micold-core`, form state and transitions in the client's render-free `features/`, I/O in
`shell/`, drawing in `ui/`. Each new concern is a new file beside its Jira sibling; existing files
get hook lines, and the pieces both providers need (text cleaning, the remembered-file helper, the
mode switch, the panel frame, `ChoiceLoad`) are shared rather than copied.

## Requirement → design map

| Req | Design | Where |
|---|---|---|
| FR-001 | `mode_switch` shown while the GitHub source shows; search kept in both modes | [selector-ui §1](./contracts/selector-ui.md) |
| FR-002 | Switch transitions: no run on switch; reload basic only when `listed_for == Advanced`; draft kept | [data-model §10](./data-model.md); research R8 |
| FR-003 | `BasicFilters` (labels, state; US6 fields); Filters panel rows | data-model §2; selector-ui §2 |
| FR-004 | `ScopedSearch::listing` basic terms ANDed | [search-rules §4](./contracts/search-rules.md) |
| FR-005 | `load_label_choices` (≤1,000, sorted, partial) on first label-field focus; US6 milestones, people | [source-and-storage §1, §3](./contracts/source-and-storage.md); research R10 |
| FR-006 | `LabelName`/`Login`/`MilestoneTitle` parse; quoted `label:"…"`; `not_offered` count | search-rules §5; research R6 |
| FR-007 | `BasicFilters::described`; Clear filters; hidden in advanced | data-model §2; selector-ui §1 |
| FR-008 | Basic change bumps `issue_request_seq` → stale results dropped | data-model §10 |
| FR-009 | `GithubFilter::is_default` → 034's `load_listing` / `search_args`, untouched | research R1, R2; source-and-storage §2 |
| FR-010 | Run only from `on_submit`/Run; `DraftChanged` never runs | data-model §10; selector-ui §1 |
| FR-011 | `ScopedSearch` advanced text, `ISSUE_ADVANCED`, `sort:updated-desc` default | search-rules §4; research R3, R4 |
| FR-012 | `UserQuery::parse` → `QueryRefusal`; `IssueList::Refused`; nothing sent | search-rules §1–§2; research R5 |
| FR-013 | Source-shown event in advanced: run `last_run` (`ran`), else prompt | data-model §7, §10 |
| FR-014 | `load_filtered` cap 1,000; cap caption; Closed chip | search-rules §6; data-model §8; selector-ui §3 |
| FR-015 | `parse_search_page` drops other repositories and pull requests | search-rules §6 |
| FR-016 | Local rematch unchanged; `ScopedSearch::beyond` + `search_filtered` when incomplete | search-rules §4; source-and-storage §1 |
| FR-017 | Pick path unchanged (`apply_pick`) | §Project Structure (no change) |
| FR-018 | `FilterFailure::Rejected` from errors at `search`; messages cleaned, cut, ordered; `NOT_ACCEPTED` | search-rules §6; research R7 |
| FR-019 | `FilterFailure::Load` → `IssueList::Failed` + Retry same filter | data-model §10 |
| FR-020 | Empty texts per mode, Clear filters in basic | selector-ui §3 |
| FR-021 | `RememberedSelector` file, one-field writes, read at open | data-model §7; source-and-storage §5; research R11 |
| FR-022 | Kept on switch-off/Jira; deleted in `on_project_forget_confirmed` | source-and-storage §5 |
| FR-023 | Request table; statement text with filter | source-and-storage §2 |
| FR-024 | Redacted `Debug`; source scan; notification without value | source-and-storage §2; research R15 |
| FR-025 | No control, no request with GitHub issues off | source-and-storage §3 |
| FR-026 | Tab order, Enter/Escape, `takes_the_keyboard` chips | selector-ui §4 |
| FR-027 | User-guide section | §Project Structure |
| SC-001–SC-007 | The rows above; quickstart §A tests and §B B1–B18 (B17 SC-002 by hand with a real `gh`) | [quickstart.md](./quickstart.md) |

## Test strategy by layer

| Layer | Tests (Red first) | Requirements |
|---|---|---|
| Core unit (`crates/micold-core/tests/github_filter.rs`, new) | Every row of search-rules §1–§6; `BasicFilters` setters (disjoint, dedupe, `described`, `chosen_count`, `is_default`); `RememberedSelector` round trip, lenient read, `with_change`; `load_filtered` with `FakeIssueSource`: paging, cap, `complete`, out-of-scope drop before the cap, first error aborts, `Rejected` passes; `load_label_choices` order, dedupe, `not_offered`, `partial`; `Debug` redaction. `jira_query.rs` stays green after R12. | FR-003–FR-006, FR-011, FR-012, FR-014, FR-015, FR-018, FR-021, FR-024; SC-001, SC-003, SC-004 |
| Core adapter (`github_gh_cli.rs`, extended) | Each new method's argument vector (`-f` only, `ISSUE_ADVANCED`, the `q`); `answer_of` keeps `Rejected` on exit 1, classifies a stderr-only failure; `github_privacy.rs` and `github_search.rs` unchanged and green. | FR-009, FR-011, FR-019, FR-023 |
| Client reducer (`crates/micold-client/tests/github_filter_state.rs`, new) | Each data-model §10 transition through `App` messages with `FakeIssueSource` (asserting `asked_searches()`): US1 AS1–6, US2 AS1–5, US3 AS1–6, US4 AS1–5, US5 AS1–3; stale and out-of-order results; rapid changes; Jira and back; GitHub off; two `App`s on one project; typed number with a filter; remembered value not in the repository; switch never runs; outcomes only at switch, change, run (never a draft). | FR-001–FR-002, FR-007–FR-008, FR-010, FR-013, FR-016–FR-020, FR-022, FR-025; SC-005, SC-006 |
| Client shell (`crates/micold-client/tests/github_selector_shell.rs`, new; `main_tests.rs` for `on_form_opened` and forget) | File read at open; one-field read-modify-write; unreadable → default; forget deletes; write-failure notification has no value; `shell/jira.rs` behaviour unchanged after the helper extraction (`jira_connection_shell.rs` green). | FR-021, FR-022, FR-024 |
| Source scans (`issues_are_requested_only_on_named_events.rs`, extended) | Filtered and choice starters called only from source-and-storage §3 events; no log/notify in the new files takes a value. | FR-005, FR-023–FR-025 |
| Geometry gates | `gates/github_filters_fit.rs` (new); `panel_placement.rs` GitHub scenarios per mode; `issue_rows_show_all_text.rs` closed-row arm. | FR-007, FR-014, FR-026 layout; SC-007 |
| Quickstart §B visual pass | B1–B18 against `fake_gh` (B17 with a real `gh`). | FR-001, FR-003, FR-007, FR-026, FR-027; Principles VI, VIII |

## Delivery order

Milestones for the tasks unit to size (task-size gate); tasks.md §Milestones is the one that holds.

1. **M1 Core rules** — `query_text` extraction; `github_filter` basic values (labels, state),
   `BasicFilters`, `UserQuery`/`QueryRefusal`, `GithubFilter`, `ScopedSearch`, `FilterFailure`,
   `parse_search_page`, `load_filtered`, `LabelChoices`, `RememberedSelector`; `Issue.closed`;
   `IssueSource` additions (search, labels) + `GhCli` + fake; store paths. Core tests only. R3/R7
   fixture probe when a signed-in `gh` is available.
2. **M2 Basic filters in the form (US1, US2 basic, US5)** — `GithubSelector` basic half, reducer,
   label choices, Filters panel, chips, state, active line, Clear filters, Closed chip, filtered
   search beyond, statement; remembered file (`shell::remembered`), read at open, forget deletes;
   `fake_gh`; gates; user-guide basic-filter part; quickstart §B B1–B7, B12–B15.
3. **M3 Advanced mode (US3, US4, US2 advanced)** — `mode_switch` generalised; query row, Run,
   refusal and rejection states, FR-013 open-in-advanced; remembered mode and query; gates;
   user-guide advanced part; §B B8–B11, B17.
4. **M4 Assignee, author, milestone (US6, P3; provisional D1)** — see below.

## US6 separability

D1 (US6 in scope) awaits the user. US6 is built only in M4 and nothing before it names it:

- **Core**: `Login`, `Assignee`, `Author`, `MilestoneTitle`, `Milestone`, `Person`,
  `MilestoneChoices`, their terms in `ScopedSearch::listing` (appended after the label terms),
  their `described` parts, and `IssueSource::{milestones_page, assignable_users}` are all added in
  M4. `BasicFilters` gains three `Option` fields with `#[serde(default)]`, so files written by M2–M3
  read unchanged and `is_default`/`chosen_count` extend by one clause each.
- **Client**: three panel rows, their `FilterField`s and `ChoiceLoad`s, the people debounce, and
  their reducer arms are added in M4 in `github_filter.rs` sections of their own.
- **Tests/docs**: US6 rows (search-rules §4 last two basic rows, §5 `Login` table; quickstart B16;
  the user-guide paragraph) land with M4.

If the user leaves US6 out: drop M4, and a spec unit moves US6, FR-003's assignee/author/milestone
clause, FR-005's people/milestone choices and FR-006's login rule to Out of Scope, and removes the
**US6**-marked rows from these design files. M1–M3 code and tests stay as they are.

## Risks

| Risk | Mitigation |
|---|---|
| `ISSUE_ADVANCED` refused or behaving differently from github.com (not probed: no signed-in `gh` in the container) | One constant (R3); M1 fixture probe when possible; §B B17 compares with github.com by hand. Fallback recorded as a ledger deviation. |
| The GraphQL error shape of a refused search differs from R7's assumption | R7's rule keys on `path: ["search"]`, not the `type`; probe fixtures replace hand-written ones when recorded. |
| GitHub's search index lags edits by seconds | Spec Assumption: the lag is GitHub's; documented in the user guide. |
| A filtered 1,000-issue list is ~1.9× slower than the default list (bodies in pages, R9) | Default list keeps its two passes; filtered lists are usually small. |
| Tallest dialog grows in advanced mode | The query row takes the active line's slot (R14); `panel_placement.rs` scenario per mode; fallback: Run as an icon button inside the query row's width. |
| Shared-helper extraction (R11, R12, R13) changes Jira behaviour | Each extraction lands with the Jira tests (`jira_query.rs`, `jira_connection_shell.rs`, `jira_filters_picker.rs`, `jira_filters_fit.rs`) green before GitHub code uses it. |

## Complexity Tracking

None: the Constitution Check has no violation to justify.
