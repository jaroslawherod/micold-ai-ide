# Quickstart: Validating "Filter the GitHub issue selector by search query, label and state"

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**§A** is the automated suite (what CI runs). **§B** is the recorded manual pass for the render
glue no test reaches (Principle I's GUI exception), runnable headless with the `visual-pass` skill.

## Prerequisites

```bash
mise trust    # once per fresh worktree (cloud container without mise: scripts/gate.sh)
cargo build -p micold-client --example fake_gh   # §B only
```

§B drives the dev-only fake `gh` `crates/micold-client/examples/fake_gh.rs` (research R16): put a
directory holding it as `gh` first on the env-include `PATH` (Settings → Environment include
script printing `PATH=<dir>:$PATH`), open a project whose default remote is
`https://github.com/acme/app`. It logs each request's `q` to stderr. `MICOLD_FAKE_GH=reject` makes
the next filtered search answer a GraphQL error at `search` with two messages. Steps marked
**real gh** need a signed-in `gh` and a real repository (SC-002's github.com comparison).

## §A - Automated

```bash
mise run test-core    # github_filter rules, answers, load, remembered format
mise run gate         # fmt, clippy, workspace tests, script tests
```

| Layer | Files |
|---|---|
| Core unit | `crates/micold-core/tests/github_filter.rs` (new): contracts/search-rules.md §1–§6; `RememberedSelector` round trip and lenient read; `load_filtered` cap, paging, out-of-scope drop, first error aborts. Existing `github_search.rs`, `github_load.rs`, `github_privacy.rs`, `jira_query.rs` stay green unchanged (FR-009, R12). |
| Core adapter | `crates/micold-core/tests/github_gh_cli.rs` (extended): the argument vectors of each new request; `answer_of` keeps a `Rejected` on exit 1. |
| Client reducer | `crates/micold-client/tests/github_filter_state.rs` (new): every transition of data-model §10; US1–US5 acceptance scenarios through `App` messages with `FakeIssueSource`; Edge rows (stale results, rapid changes, Jira and back, GitHub off, two windows, typed number with a filter). |
| Client shell | `crates/micold-client/tests/github_selector_shell.rs` (new): file read at open, one-field writes, forget deletes, write-failure text has no value. |
| Source scans | `issues_are_requested_only_on_named_events.rs` (extended): requests only from contracts/source-and-storage.md §3 events; no log/notify takes a value. |
| Geometry | `gates/github_filters_fit.rs` (new), `gates/panel_placement.rs`, `gates/issue_rows_show_all_text.rs` (extended). |

## §B - Recorded manual pass

| # | Step | Expect | Covers |
|---|---|---|---|
| B1 | Open the New worktree form, Issue source, GitHub. | Today's list; no active line; the switch shows Basic; the statement is 034's. | FR-009, SC-007 |
| B2 | Filters → Exclude labels: pick `in-progress`, then `blocked`. | Chips appear; button "Filters · 2"; list reloads with neither label; active line "Excluding labels in-progress, blocked"; statement names the filter; fake logs `-label:"in-progress" -label:"blocked"`. | US1 AS1–2, AS6, FR-023 |
| B3 | Type `crash`. | The filtered list narrows locally (no request unless the list is incomplete). | US1 AS4, FR-016 |
| B4 | Include `bug`, then try to include `blocked`. | Every row has `bug`; `blocked` moves from Exclude to Include. | US1 AS3, Edge "Same label" |
| B5 | Close and reopen the form; restart the app and reopen. | Same filters applied, active line shown, one listing. | US2 AS1, SC-005 |
| B6 | State → Closed, then All. | Closed rows carry the "Closed" chip; All shows both. Pick a closed issue: ticket, name and type fill. | US5, FR-017 |
| B7 | Clear filters. | Today's list; no active line. Reopen: default. | US1 AS5, US2 AS5 |
| B8 | Advanced; type `-label:in-progress sort:created-asc`, press Enter. | Filters button and active line replaced by Query + Run; list in created order; fake logs `repo:acme/app is:issue (-label:in-progress) sort:created-asc`. | US3 AS1–3 |
| B9 | Run `repo:other/x`, then `(a`, then `label:"x`. | Each shows its refusal text; fake logs nothing. | US4 AS5, SC-004 |
| B10 | `MICOLD_FAKE_GH=reject`, run `lable:bug`. | Both of GitHub's messages, in order, query kept in the input; fix and run: list returns. | US4 AS1–2 |
| B11 | Type a new draft without running; switch to Basic; reopen the form. | Basic filters' list as before; reopened in Basic; switch to Advanced shows the last run query, not the draft. | US3 AS6, US2 AS4 |
| B12 | Switch the source to Jira and back. | Mode and filters kept, search text cleared, list reloaded. | Edge "Jira and back" |
| B13 | Keyboard only: Tab through switch, Clear, search, Filters, panel fields and chips; Enter adds a label; Escape closes. | Every control reachable and operable. | FR-026 |
| B14 | Settings: turn GitHub issues off; open the form. | No filter control, no request. | FR-025 |
| B15 | Forget the project; check `<data_dir>/github/`. | Its `.selector.json` is gone. | FR-022 |
| B16 | **US6** Assignee: Me, No assignee, a person; Author: Me, typed login; Milestone: `v2.4`, No milestone. | Each list holds only matching issues; fake logs each term. | US6 AS1–4 |
| B17 | **real gh** Advanced `-label:in-progress state:open` on a real repository; compare with github.com's issue search for the same query in that repository. | Same set of issues. | SC-002 |
| B18 | Light and dark theme; Linux, macOS, Windows when available. | Same layout and behaviour. | Principle VI |
