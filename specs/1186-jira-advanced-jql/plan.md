# Implementation Plan: Advanced JQL in the Jira Issue Selector

**Branch**: `claude/project-thread-vf2ck6` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1186-jira-advanced-jql/spec.md`

## Summary

Add a **Basic / Advanced** switch to the Jira issue selector of the New worktree form. In advanced
mode the user types JQL; on Enter or **Run** the app checks it locally (quotes closed, parentheses
balanced outside strings, at most 2,000 characters after cleaning), wraps it as
`project = "<KEY>" AND (<condition>) <sort>` with the user's `ORDER BY` kept or `ORDER BY updated
DESC` added, and lists up to 1,000 returned issues of the connected project in the basic row form.
A Jira `400` shows Jira's own messages; other failures show basic mode's notices and Retry. The
mode and the last run query are remembered per project in an owner-only file beside the
connection's token, and removed with the connection.

The rules are a new pure core module `micold_core::jira_query` beside `jira.rs` (ledger D3: the
basic `list_jql`/`search_jql` are not touched); the request is one new `JiraSource` method; the
client adds one state module, one shell module and one view module, with a few hook lines in the
existing form files.

## Technical Context

**Language/Version**: Rust stable (workspace toolchain, `mise`).

**Primary Dependencies**: iced (GUI), `ureq` 3 (Jira HTTP, client crate only), `serde`/`serde_json`
(body and remembered file), `base64`. No new dependency.

**Storage**: one JSON file per project, `<data_dir>/jira/<project id>.query.json`, owner-only
(`micold_core::owner_only::write`), beside `<project id>.token` (research R7).

**Testing**: `cargo test` via `mise run test-core` / `mise run gate`; core integration tests in
`crates/micold-core/tests/`, client reducer and source-scan tests in `crates/micold-client/tests/`,
the loopback adapter test `jira_http_adapter.rs`, geometry gates in `crates/micold-client/tests/gates/`,
and the recorded quickstart §B pass.

**Target Platform**: Linux, macOS, Windows desktop.

**Project Type**: desktop app (Cargo workspace: `micold-core` render-free core, `micold-client`
iced GUI, `micold-daemon` — untouched by this feature).

**Performance Goals**: a query is one to ten `POST`s (100 issues each, cap 1,000), each with the
existing 10 s budget; local checks are linear in at most 2,000 chars. No per-keystroke work.

**Constraints**: nothing is sent before Enter/Run; no request while the Jira source is not
showing; the user query never reaches a log, notification or `settings.json` (FR-016).

**Scale/Scope**: one selector mode, ~1 core module (~300 lines with tests), ~3 client modules, a
`JiraSource` method, one `Typeahead` builder method, a user-guide section.

No NEEDS CLARIFICATION remains: the one deferred item (clarify Q5, the message cap) is research R4
(300 characters, 10 messages).

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design (below).*

| # | Principle | Status | How |
|---|---|---|---|
| I | Test-First | PASS | Every rule is in `micold_core::jira_query` or the render-free `features::worktree_form::jira_query` and is written Red first (§Test strategy). Only the view glue (`ui/jira_query.rs`, the two hook lines in `ui/worktree_form.rs`) uses the GUI exception, validated by quickstart §B and the geometry gates. |
| II | Multi-Session | PASS (N/A) | No session state: the selector state lives in the open form of one window and is dropped with it. Nothing is written to a session's history (FR-016). |
| III | Worktree Integration | PASS (N/A) | Creating the worktree is the existing form's path; a pick fills the same fields (FR-009). No git step. |
| IV | Local-First | PASS | The remembered state is a local owner-only file; the Jira request is the opt-in integration of 533 (switch + connection), sent only on Enter/Run or the FR-004a run, with the notice naming the query (FR-015). Offline: the request fails with basic mode's notice; everything else works. |
| V | Rust + iced | PASS | `UserQuery` (only from `parse`) and `ScopedQuery` (only from a `UserQuery`) make a refused or unscoped query unsendable: `JiraSource::query_page` takes `&ScopedQuery`. `QueryRun` makes "error and results at once" unrepresentable. |
| VI | Cross-Platform | PASS | No OS branch: the file path comes from `paths::data_subdir("jira")`, written with `owner_only` (which already abstracts Unix modes and Windows ACLs). CI runs all three. |
| VII | Documentation | PASS | `docs/user-guide/worktrees-and-sessions.md` gains "Listing Jira issues with JQL" (FR-017) in the same change; the `Typeahead` header documents `.on_submit`. |
| VIII | Reusable UI | PASS | Reuses `ToggleChip` (switch), `Typeahead` + picker rows (list), `Button`, `Text`, `StageProgress`. The one extension is a chainable `.on_submit(message)` on the shared `Typeahead`, builder form, no new widget. The basic row builder is factored so both modes call one function (no copy). |

### Re-check after Phase 1 design

Unchanged: the contracts add no widget, no session state, no remote dependency, no OS branch. The
`JiraSource` method has a default body, so no implementor is forced to change (V). Complexity
Tracking stays empty.

## Requirement → design map

| Req | Design | Where |
|---|---|---|
| FR-001 | `mode_switch` shown only with the Jira source showing | [selector-ui §1](./contracts/selector-ui.md#1-layout-cratesmicold-clientsrcuijira_queryrs-new) |
| FR-002 | Advanced `picker` replaces the basic body; `JiraQuery` never reads `issue_query` or filters | selector-ui §1; [data-model §6](./data-model.md#6-jiraquery-client--the-selectors-advanced-state-one-field-of-worktreeform) |
| FR-003 | Basic `IssueList`/`issue_query` untouched by advanced; `draft` kept while open; first switch to basic loads | data-model §6 transitions |
| FR-003a | GitHub→Jira: mode kept, `draft = last_run`, advanced re-runs | data-model §6 transitions |
| FR-004 | `Run` only from `on_submit` / Run button; `DraftChanged` never runs | selector-ui §2–§3 |
| FR-004a | Jira-source-shown event in advanced: run if `last_run` non-empty, else prompt; no basic load | data-model §6; [source-and-storage §3](./contracts/selector-source-and-storage.md#3-the-run-cratesmicold-clientsrcshelljira_queryrs-new) |
| FR-005 | `ScopedQuery::new` | [jql-rules §2–§3](./contracts/jql-rules.md#3-the-scoped-query-fr-005) |
| FR-006 | `jira_query::load` over `load_pages` (cap 1,000), rows in order, cap caption | jql-rules §6; selector-ui §1 |
| FR-007 | `UserQuery::parse` → `QueryRefusal`; `query_page` takes only `ScopedQuery` | jql-rules §1–§2; data-model §1–§3 |
| FR-008 | `keep_project` filter in the load | jql-rules §6 |
| FR-009 | `WorktreeForm::apply_pick` shared by both pick paths | selector-ui §5 |
| FR-010 | 400 → `Rejected(rejection_messages)`; 300 chars, 10 messages; plain text when none | jql-rules §4; source-and-storage §2 |
| FR-011 | `Load(IssueLoadError)` rendered as basic; `Retry` re-sends the same `ScopedQuery` | data-model §4, §6 |
| FR-012 | `Loaded` empty → `No issue matches this query.` | jql-rules §5 |
| FR-013 | `RememberedQuery` file; read at open; mode written at switch, query at run | data-model §5; source-and-storage §4 |
| FR-014 | File untouched by switch-off / GitHub; deleted in `shell::jira::apply` Remove | source-and-storage §4 |
| FR-015 | Same endpoint, headers, gate; `jira_query::notice` names the query | source-and-storage §2; selector-ui §4 |
| FR-016 | Query in no log, notification text or session; write-failure text omits it | source-and-storage §4–§5 |
| FR-017 | User-guide section | §Project Structure, docs |
| SC-001–SC-005 | Covered by the rows above and quickstart §A/§B | [quickstart.md](./quickstart.md) |

## Test strategy by layer

| Layer | Tests (Red first) | Requirements |
|---|---|---|
| Core unit (`crates/micold-core/tests/jira_query.rs`, new) | Every row of jql-rules §1–§4 tables; `ScopedQuery` texts; `load` with `FakeJiraSource::with_query`: paging, 1,000 cap and `complete`, other-project keys dropped, first error aborts, `Rejected` passes through; `RememberedQuery` round trip and lenient read. `jira_load.rs` stays green after `load_pages` turns generic. | FR-005–FR-008, FR-010, FR-013, SC-002, SC-003 |
| Client reducer (`crates/micold-client/tests/jira_query_state.rs`, new; render-free `features`) | Each data-model §6 transition: switch shown only with Jira; switch never runs; Run/Enter runs, typing does not; refused queries send nothing; stale `Ran` dropped; Retry same scoped query; FR-004a open-in-advanced run and prompt; FR-003 restore basic state; FR-003a GitHub round trip; pick fills as basic; outcomes `JiraQueryRemembered` at switch and run only (never for a draft). | FR-001–FR-004a, FR-009, FR-011–FR-014, US1–US3 |
| Client adapter (`crates/micold-client/tests/jira_http_adapter.rs`, extended, loopback server) | `query_page` body is `{jql: scoped, fields, maxResults: 100}` with the four headers only; 400 with messages → `Rejected`; 400 HTML → `Rejected([])`; 401/429/timeout classified as basic; status path unchanged by `BadRequest(body)`. | FR-005, FR-010, FR-011, FR-015 |
| Client shell (`jira_connection_shell.rs` extended; `main_tests.rs` for `on_form_opened`) | File read at open → `Restored`; write is read-modify-write of one field; Remove deletes the file; write failure notice has no query text. | FR-013, FR-014, FR-016 |
| Source scans (`issues_are_requested_only_on_named_events.rs`, `jira_is_requested_only_on_named_events.rs`) | `start_query` called only from the accepted-event paths; the source still asked for once, in the gate; no `tracing::`/`log` macro and no notification in `features/worktree_form/jira_query.rs`, `shell/jira_query.rs` or `ui/jira_query.rs` takes the draft, `last_run` or a `UserQuery`/`ScopedQuery`. | FR-004, FR-015, FR-016 |
| Shared component (`ui/material/typeahead.rs` unit tests) | `.on_submit` reaches the field; Enter with a highlighted row still picks; no `.on_submit` = old behaviour. | R8, Principle VIII |
| Geometry gates (`crates/micold-client/tests/gates/`) | Advanced rows show all text (arm of `issue_rows_show_all_text.rs`); the dialog with the switch keeps the panel placement the basic gate measures. | FR-006, SC-005 |
| Quickstart §B visual pass | B1–B13 (render glue, theme, platforms). | FR-001, FR-002, FR-015, Principle VI/VIII |

## Project Structure

### Documentation (this feature)

```text
specs/1186-jira-advanced-jql/
├── spec.md
├── plan.md              # this file
├── research.md          # R1–R9
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── jql-rules.md
│   ├── selector-source-and-storage.md
│   └── selector-ui.md
├── checklists/
└── tasks.md             # /speckit-tasks (next unit)
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── jira_query.rs              # NEW: UserQuery, QueryRefusal, ScopedQuery, QueryFailure,
│                              #      rejection_messages, load, RememberedQuery, texts
├── jira.rs                    # + JiraSource::query_page (default body); FakeJiraSource queries
├── github.rs                  # load_pages generic over its error type (signature only)
├── store.rs                   # + jira_query_path beside jira_token_path
└── lib.rs                     # + pub mod jira_query;
crates/micold-core/tests/
└── jira_query.rs              # NEW

crates/micold-client/src/
├── features/worktree_form/jira_query.rs     # NEW: JiraQuery state, Msg, reducers, notice
├── features/worktree_form.rs  # + field jira_query; Msg::JiraQuery; State::awaited_jira_query; hooks in source_changed,
│                              #   issue_provider_changed, reset_issues, jira_notice; apply_pick,
│                              #   shown_issues
├── features/mod.rs            # + pub mod jira_query; Outcome::JiraQueryRemembered
├── app.rs                     # route Outcome::JiraQueryRemembered (shell's, no-op here)
├── shell/jira_query.rs        # NEW: start_query, read/write remembered file
├── shell/issues.rs            # on_form_opened reads the file; mode switch to basic starts the load
├── shell/jira.rs              # Remove also deletes the query file
├── shell/jira_http.rs         # query_page; PostError::BadRequest(body)
├── shell/mod.rs, main.rs      # module decl; route FormMsg::JiraQuery(..) to shell::jira_query
├── ui/jira_query.rs           # NEW: mode_switch, picker
├── ui/worktree_form.rs        # one `if` for the switch, one early return in issue_picker;
│                              #   per-issue row builder factored out of issue_rows_with
├── ui/mod.rs                  # + mod jira_query;
└── ui/material/typeahead.rs   # + .on_submit(M)
crates/micold-client/examples/fake_jira.rs   # + `badjql` mode, logs jql
crates/micold-client/tests/
├── jira_query_state.rs        # NEW
├── jira_http_adapter.rs, jira_connection_shell.rs, issues_are_requested_only_on_named_events.rs,
│   jira_is_requested_only_on_named_events.rs, gates/issue_rows_show_all_text.rs   # extended
docs/user-guide/worktrees-and-sessions.md    # + "Listing Jira issues with JQL" after "From a Jira issue"
```

**Structure Decision**: the existing workspace layout; each new concern is a new file next to its
feature-533/1101 sibling, and existing files get hook lines only.

## Overlap with #1185 (basic field filters)

#1185 (`specs/1185-jira-selector-filters`, spec only on `origin/claude/project-thread-xwkotj`)
changes `search_jql`/`list_jql`, adds filter fields and reducers to the form, filter controls to
`issue_picker`, choice-loading methods to `JiraSource`, and names filter values in `jira_notice`.
This plan keeps D3 and stays off those lines where it can. Shared structures both features touch,
each with an additive edit:

| Shared structure | This feature's edit | Rebase cost |
|---|---|---|
| `JiraSource` trait + `FakeJiraSource` (`core/src/jira.rs`) | one method with a default body, one script field, two builder methods, appended at the end | adjacent-append conflict at most |
| `WorktreeForm` struct, `worktree_form::Msg` | one field `jira_query`, one variant `JiraQuery(..)` | one-line conflicts |
| `source_changed`, `issue_provider_changed`, `reset_issues` (#1185 FR-016 resets filters there) | one call each into `features::worktree_form::jira_query` | one-line conflicts |
| `WorktreeForm::jira_notice` (#1185 FR-014 adds filter values) | one early branch to `jira_query::notice` in advanced mode | small |
| `ui::worktree_form::issue_picker` (#1185 adds the filter row) | one early `return` at the top; the basic body is untouched | small |
| `ui::worktree_form::modal` | one `if` pushing `mode_switch` after `provider_switch` | small |
| `issue_rows_with` | per-issue row builder extracted (#1185 may not touch it) | possible |
| `shell/jira_http.rs` `PostError` | `BadRequest` carries the body | none unless #1185 edits `send` |

Settings and `settings.json` are not touched (the remembered state is a file of its own), and
`search_jql`/`list_jql` are unchanged. When #1185 lands first, FR-002/FR-003 cover its filters with
no extra code: advanced mode reads none of the form's basic fields.

## Delivery order

Milestones for the tasks unit to size (Task-size gate); [tasks.md §Milestones](./tasks.md#milestones) cuts them into M1–M5 and is the one that holds:

1. **M1 Core rules**: `jira_query.rs` parse/scope/messages/texts/remembered format, `load_pages`
   generic, `query_page` + fake, `jira_query::load`, `jira_query_path`. Core tests only.
2. **M2 Request and storage**: `JiraCloud::query_page` and the 400 body; `shell/jira_query.rs` file
   read/write; Remove deletes the file; `fake_jira` `badjql`. Adapter and shell tests.
3. **M3 Selector state and view**: `features::worktree_form::jira_query` reducers and hooks; `Typeahead::on_submit`;
   `ui/jira_query.rs`; shell wiring; scans and geometry gates; user guide; quickstart §B.

## Risks

| Risk | Mitigation |
|---|---|
| Jira's sort clause rules differ from R2 (e.g. `ORDER BY` inside a function argument) | Jira judges validity; a mis-split query is refused by Jira with its message (FR-010), never sent unscoped: the balance check covers the whole text. |
| A Jira 400 that is not a JQL error (bad field list) shows as a query error | `FIELDS` is fixed and tested; basic mode would fail the same way. |
| `serde_json` map order makes `errors` order differ from Jira's | Documented (R4); `errorMessages`, the common carrier, keep Jira's order. |
| Two windows writing the file at once | Atomic rename; read-modify-write per field; spec accepts last write wins. |
| #1185 merges first and moves the hook lines | Plan §Overlap lists every shared line; each is additive. |

## Complexity Tracking

None: the Constitution Check has no violation to justify.
