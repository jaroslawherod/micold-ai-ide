# Implementation Plan: Filter the Jira issue selector by reporter, label, component and fix version

**Branch**: `claude/project-thread-xwkotj` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1185-jira-selector-filters/spec.md`

## Summary

Add four single-value filters (reporter, label, component, fix version) to the Jira source of the
create-worktree issue picker. A new render-free core module `micold_core::jira_filter` holds the
chosen values (`JiraFilters`), turns them into ANDed JQL clauses with every value a quoted, escaped
string sent whole, and narrows the existing `list_jql`/`search_jql` output without editing them, so
issue #1186 (advanced JQL, built in parallel on the same functions) meets this feature at one call
site. Choices come from Jira on first focus of a filter (components, versions, own user, project
users by typed name) or, for labels, from the unfiltered listing already loaded. A filter change
reloads the listing through the existing issue-load seq, which already discards stale results. The
dialog gains one row of four `Typeahead` fields and, while filters are active, one line naming them
with **Clear filters**. Design: [research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/), [quickstart.md](quickstart.md).

## Technical Context

**Language/Version**: Rust 2021 workspace, toolchain as the repo pins it.

**Primary Dependencies**: existing only: `iced` (UI), `ureq` 3 (`JiraCloud`, now also `GET`),
`serde_json`, `base64`. No new crate.

**Storage**: none. Filters and choices live in the open form only, never serialised (FR-006,
FR-016).

**Testing**: `cargo test` via `mise run test-core` (core) and `mise run gate` (cloud:
`scripts/gate.sh`); client integration tests in `crates/micold-client/tests/`; geometry gates in
`tests/gates/`; quickstart Part B visual pass against `examples/fake_jira.rs`.

**Target Platform**: Linux, macOS, Windows (iced client; the daemon is untouched).

**Project Type**: desktop app, Rust workspace (`micold-core` render-free logic, `micold-client`
GUI and shell).

**Performance Goals**: one Jira request per filter change (listing) and one per choice list; the
reporter search debounced 300 ms; every request on a blocking thread, never in `view`/`update`.

**Constraints**: 10 s budget per request (`REQUEST_TIMEOUT`); 1,000-issue cap and its notice as
today; dialog fixed at 520 px wide and the tallest dialog (one added row plus one conditional
line); no name or account in `Debug`/logs (FR-006).

**Scale/Scope**: up to 1,000 loaded issues; tens of components/versions; 50 people per search.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design: unchanged.*

- [x] **I. Test-First**: all decisions (clauses, escaping, parsing, label choices, reducer
  transitions, seq discarding) sit in `micold-core` or the render-free `features/worktree_form/jira_filters.rs`
  and start from failing tests; `ui/worktree_form.rs` only draws the reducer's state (glue
  exception, covered by geometry gates and quickstart Part B). PASS.
- [x] **II. Multi-Session**: filter state is per form (per window's `App`), never shared or
  persisted; a test runs two `App`s and changes one (edge case "Concurrency"). PASS.
- [x] **III. Worktree Integration**: picking an issue fills the form as before; no file/VCS
  operation added. PASS.
- [x] **IV. Local-First**: nothing stored; requests only to the user's own opted-in Jira site, with
  the statement (533 FR-005b) widened to name the filters (FR-014); without network the picker
  shows failures and manual creation works (SC-004). PASS.
- [x] **V. Rust + iced**: `JiraFilters` holds `Option` per field (at most one value is the only
  representable state); `AccountId` can only be built by parsing, so a reporter clause can never
  carry a display name; `ChoiceLoad<T>` makes "loaded and failed" unrepresentable. PASS.
- [x] **VI. Cross-Platform**: no platform code; same requests and widgets everywhere. PASS.
- [x] **VII. Documentation**: `docs/user-guide/worktrees-and-sessions.md` ("From a Jira issue":
  Filters paragraph, statement text) and `docs/user-guide/settings.md` (Jira: what is sent) in the
  same change. PASS.
- [x] **VIII. Reusable UI**: the four filters reuse `material::Typeahead`; the line reuses `Text`
  and `Button`; no new widget and no change to a shared component (research R8). PASS.

## Project Structure

### Documentation (this feature)

```text
specs/1185-jira-selector-filters/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── jira-filter-source.md   # clauses, trait additions, requests, parsing
│   └── filter-ui.md            # layout, states, behaviour, statement
└── tasks.md                    # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── jira_filter.rs        # NEW: JiraFilters, FilterKind, AccountId, JiraUser, VersionChoice,
│                         #      LabelChoices, jql_string, clauses, narrowed, parse_*, label_choices
├── jira.rs               # JiraSource + list_filtered/search_filtered/components/versions/myself/
│                         #   project_users (defaults); load_filtered; FakeJiraSource scripts
│                         #   (list_jql/search_jql untouched)
└── lib.rs                # pub mod jira_filter

crates/micold-client/
├── src/shell/jira_http.rs     # one private request(method, path, query, body); the six methods
├── src/shell/issues.rs        # Jira load/search pass the form's filters; choice-request starters
├── src/shell/jira_filters.rs  # NEW: on_filter_* handlers (focus → request, people debounce, retry)
├── src/features/worktree_form/jira_filters.rs # NEW: FilterState, ChoiceLoad, FilterMsg, reducer, active line,
│                                #      list rows per kind, statement suffix
├── src/features/worktree_form.rs # pub mod jira_filters; WorktreeForm.jira_filters; FormMsg::JiraFilter; reset points;
│                                 #   jira_search_request(); empty/cap texts with filters; notice
├── src/ui/worktree_form.rs    # filter row + active line in the Jira issue section
├── src/main.rs / app dispatch # route FormMsg::JiraFilter to shell::jira_filters
├── examples/fake_jira.rs      # components, versions, myself, user search, clause-aware search
└── tests/                     # see quickstart Test layers
docs/user-guide/{worktrees-and-sessions,settings}.md
```

**Structure Decision**: logic in `micold-core` (testable with `test-core`), form state and
transitions in the client's render-free `features/`, IO in `shell/`, drawing in `ui/`, matching
features 533 and 1101. The new filter code sits in its own files (`jira_filter.rs`,
`features/worktree_form/jira_filters.rs`, `shell/jira_filters.rs`) so the files #1186 also edits
(`jira.rs`, `jira_http.rs`, `features/worktree_form.rs`, `ui/worktree_form.rs`) get only small,
additive hunks.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001, FR-002 | `FilterState` / `JiraFilters` (one `Option` per kind, Any row, `ClearedAll`); [filter-ui §2](contracts/filter-ui.md) |
| FR-003 | `parse_components`, `parse_versions` (archived out, unreleased first); `JiraCloud::components/versions` ([source §3-4](contracts/jira-filter-source.md)) |
| FR-004 | `parse_myself`, `parse_users`, `JiraUser` equality by account; `project_users` debounced search ([research R3](research.md)) |
| FR-005 | `label_choices` taken at the first unfiltered load (`LabelSource`); partial notice row ([research R5](research.md)) |
| FR-006 | lazy `ChoiceLoad` on focus ([research R7](research.md)); redacted `Debug` of `AccountId`/`JiraUser` ([research R9](research.md)) |
| FR-007 | `narrowed(list_jql, f)` in `list_filtered`; `load_filtered` keeps the cap; cap caption text |
| FR-008 | reducer bumps `issue_request_seq` on change ([research R6](research.md)) |
| FR-009 | local rematch unchanged; `search_filtered` with `narrowed(search_jql, f)` |
| FR-010 | `jql_string`, `JiraFilters` setters refuse control chars, parsers skip them, reporter by `AccountId` ([research R2](research.md)) |
| FR-011 | empty filtered listing text + Clear filters ([filter-ui §3](contracts/filter-ui.md)) |
| FR-012 | per-kind `ChoiceLoad::Failed` + `ChoicesRetry(kind)`; nothing else blocked |
| FR-013 | failures render as today; `FilterState` untouched by a listing failure |
| FR-014 | `jira_notice` text + the active line naming the values sent (ledger D12); request table ([source §3](contracts/jira-filter-source.md)) |
| FR-015 | typeahead focus stops, Any row, Clear filters button, active line |
| FR-016 | `FilterState` reset points ([data-model §7](data-model.md)) |
| FR-017 | pick path unchanged |
| SC-001 | one row, focus + pick ×2; quickstart B3 |
| SC-002, SC-003 | core unit: 16 combinations, ≥20-value corpus; client: fake answers per clause set |
| SC-004 | client: each failure cause on each filter, then search, pick, create |
| SC-005 | client: scripted rapid changes, out-of-order answers, only the last shown |

## Test strategy (layer per requirement)

- **Core unit** (`mise run test-core`): `jira_filter_clauses.rs` (clause order, escaping corpus of
  ≥20 values incl. `"`, `\`, edge spaces, >100 chars, non-Latin; all 16 combinations; `narrowed`
  with and without ` ORDER BY `; search text still cut while a filter value is not);
  `jira_filter_choices.rs` (each parser: shape errors, skipped entries, order, dedup; own user
  removed from people; `label_choices` sort/dedup/partial; `Debug` redaction; `load_filtered` cap
  and paging over the fake). Existing `jira_jql.rs` proves `search_jql` unchanged.
  FR-003, 004, 005, 006, 007, 009, 010; SC-002, SC-003.
- **Client adapter** (`jira_http_adapter.rs`, local listener): each new request's method, path,
  query encoding of typed text, headers, body JQL with clauses, redirect and status
  classification. FR-014.
- **Client integration** (`jira_filters_picker.rs`, `FakeJiraSource`, through `App` messages):
  US1–US3 acceptance scenarios, empty choices, failure + Retry per filter (SC-004), key typed with
  filters, rapid changes and out-of-order results (SC-005), source switch reset (FR-016), two
  dialogs (Principle II), statement text (FR-014). Extended `jira_is_requested_only_on_named_events`
  (no choice request until focus; one per first focus) and `jira_token_never_leaks` (no reporter
  name/account in `{:?}` of form, messages, or captured logs).
- **Geometry gates**: `gates/jira_filters_fit.rs` (four fields fit the row; lists anchor under
  their fields; the active line and Clear filters fit one line) and a `panel_placement.rs`
  scenario with the Jira source, filters active and the cap caption shown (footer reachable).
- **Quickstart Part B** (visual pass): look, keyboard path, narrow-list tooltips, quoted names.

## Risks

- **Dialog height**: one row plus one line in the tallest dialog. The Jira section has no details
  block, which is the room used. If `panel_placement` fails, the fallback (decided now) is to
  drop the field labels to placeholders (dense fields) before anything else; recorded as a
  deviation in the ledger if taken.
- **#1186 rebase**: both features edit `JiraCloud`'s list/search bodies and the Jira section of
  the form. Kept small by R1/R10 (no edits to `list_jql`, `search_jql`, `list_open`,
  `search_open`); whoever merges second composes `narrowed(...)` with #1186's JQL at that one
  call site. T008's private `request` helper also moves the existing POST and the status
  bisection in `jira_http.rs`: a known conflict point with #1186, resolved by whoever merges
  second.
- **`/user/viewissue/search` on sites with restricted user visibility** may answer users without
  `displayName` or none at all: such users are skipped and the own user stays offered (edge case).

## Complexity Tracking

No violations.
