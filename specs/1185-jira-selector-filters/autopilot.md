# Autopilot ledger — #1185 jira-selector-filters

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1185 (issue #1185: Filter the Jira issue selector by reporter, label, component and fix version); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1185
- **Worktree branch**: claude/project-thread-xwkotj
- **Started**: 2026-10-24
- **Phase**: implement
- **Next step**: M2 rebased onto main (3b7ae99f) and pushed; orchestrator opens the M2 PR (title and body in the scratchpad: m2-pr-title.md, m2-pr-body.md), waits on CI, merges; then M3 (T031–T039) — first resolve the D14 row-width follow-up and jaro's open question (one Filters button with a panel?)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1344 | Design (spec, plan, tasks) | merged (rebase) | 4e204fd4b00deed3feaa8a30d519b5ad7b03cc48 |
| #1381 | M1 component filter + mechanism | merged (rebase, squashed) | 3b7ae99fdd8794db201a16b9732f0ce441d85cdc |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T022 | full | Component filter + filter mechanism (row, active line, Clear filters) | #1381 | done; rebased onto #1186 M3, refit (D12), gate passed at aa3b2c52; merged 3b7ae99f |
| M2 | T023–T030 | full | Fix-version filter, AND with component | — (pushed to claude/project-thread-xwkotj; orchestrator opens the PR) | done; rebased onto main 3b7ae99f (D13), Jira layouts re-recorded, rebase review CLEAN, gate passed at ea884360 |
| M3 | T031–T039 | full | Reporter filter (own user, typed people search, by account) | — | pending |
| M4 | T040–T045 | full | Label filter from loaded issues; all four in one row | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Overlap with #1186 (advanced JQL in the same selector, worked by another thread in parallel)? | This spec covers only the four basic filters (reporter, label, component, fix version); free-form/advanced JQL is listed out of scope and left to #1186. Both touch `search_jql`/`list_jql` in crates/micold-core/src/jira.rs and the Jira picker in worktree_form.rs: expect a merge-order rebase. | agent-resolved | spec.md#Out of Scope; orchestrator prompt |
| D2 | spec | Cloud environment: `gh` unauthenticated, `mise` missing | Skipped scripts/autopilot/branch-start.sh (needs gh); branch already at origin/main 0e5a1e9b. Gate is scripts/gate.sh. | agent-resolved | orchestrator prompt |
| D3 | clarify | Which people does the reporter filter offer? | Own user first, then project users with browse permission, searched by typed name | agent-resolved | issue #1185 body ("its … users"); spec.md#FR-014 |
| D4 | clarify | Where do label choices come from? | Labels on the not-Done issues of the unfiltered listing (≤1,000), alphabetical, no extra request; notice when the listing is capped | agent-resolved | issue #1185 body ("from the connected Jira project"); Jira `GET /rest/api/3/label` is site-wide only |
| D5 | clarify | Label choices once a filter is chosen? | Taken at the unfiltered load, kept for the dialog | agent-resolved | spec.md#FR-005, #FR-007 |
| D6 | plan | How do the filter clauses compose with #1186's JQL work? | Separate `micold_core::jira_filter` (`clauses`, `narrowed`); `list_jql`/`search_jql`/`list_open`/`search_open` unchanged; additive `JiraSource` methods with defaults | agent-resolved | research.md#R1, #R10 |
| D7 | plan | Filter controls in the 520 px, tallest dialog? | One row of four `Typeahead`s (Any row clears), one "Filtered by … / Clear filters" line while active; no new widget | agent-resolved | research.md#R8; SC-001 |
| D8 | tasks | US1 split? M1 over the ~800-line guide | Split US1 along AS3: M1 = mechanism + component, M2 = fix version; the mechanism has no smaller observable slice. T046 (quickstart, no code) left to the close unit | agent-resolved | tasks.md#Notes; milestones.md rule 3, 4 |
| D9 | M1 implement | The planned filter row (and its fallback, placeholders for labels) overflowed the 800 px panel-placement gate by 7 px with no filter chosen: a label adds no height to a `Typeahead` | The filter fields ride on the picker's main row, trailing (beside the issue search, the Retry or the loading line, FillPortion 2 : 1 per filter); the active line + **Clear filters** stays under it, and a capped list's caption joins the active line's paragraph while a filter is chosen. Every FR kept; contracts/filter-ui.md §2 layout deviated. M2–M4 add fields to that row and must re-check width and height (`gates/jira_filters_fit.rs`, `panel_placement`) | agent-resolved | ui/worktree_form.rs `issue_picker`; layout fixture `add-worktree-dialog-jira-filtered` 664 px |
| D10 | M1 implement | T011 says "through `App` messages over `FakeJiraSource`", but `tests/` reaches only the library `State`, not the binary's shell | Split: reducer behaviour in `tests/jira_filters_picker.rs` (State messages, answers fed as the shell's messages); requests over `FakeJiraSource` in `main_tests.rs::jira_filter_shell` (first focus asks once, clauses on load and search, Retry, switch off) | agent-resolved | crates/micold-client/src/main_tests.rs |
| D11 | M1 gate | `feature_registration_cost` counts each `features/*.rs` file as a feature, so `features/jira_filters.rs` failed it as an unregistered feature | The filter reducer is part of the New worktree form: it lives at `features/worktree_form/jira_filters.rs`, declared in `features/worktree_form.rs`; tasks.md, plan.md, data-model.md, contracts/filter-ui.md, quickstart.md paths updated | agent-resolved | gate-2 failure; review B M1 F2 |
| D12 | M1 rebase | After rebasing onto #1186's M3 (Basic/Advanced switch above the picker, 32 px), `panel_placement` failed: `jira-filtered` panel y=50 (15 px into the app bar), `jira-credentials-refused` y=62 (3 px). Not #1186's: origin/main 5a5bc7e1's fixture has `credentials-refused` at y=70 | Fit M1 beside the switch, #1186 untouched: (1) **Clear filters** trails the switch's row (+8 px: a button is 40, the chips 32); (2) the active line is text only and carries the values sent — the statement no longer appends "Filters sent: …" (a 4th line, +16 px); FR-014 amended in spec.md (clarification entry) to say so: the statement says the filters are sent, the active line names the values (review D12 r1 F1); (3) a filtered cap caption reads "Showing the N most recently updated." so "Filtered by … . Showing …" keeps one line; (4) a failed listing seats no filter field (a field is 56, Retry 40) and names the chosen filters beside **Retry** (FR-013: filters left in place to retry or clear). Result: filtered y=66, refused y=70. Plan's placeholder fallback rejected: a label adds no height (D9). contracts/filter-ui.md §1, §2, §4, tasks T011/T012, plan FR-014 row, user guide updated | agent-resolved | layout fixture re-recorded for the Jira states only (loading, credentials-refused, loaded, no-open-issues, filtered); `gates/jira_filters_fit.rs` |
| D13 | M2 implement | Start M2 before #1381 (M1) merges? | Yes: M2 was prepared on local branch wip-1185-m2 stacked on M1's head e265593f while #1381's CI ran; no branch-start.sh, rebased onto main after #1381 merged as 3b7ae99f, keeping M1's D12 layout (Clear filters on the switch's row, text-only active line naming the values, no field beside Retry) | orchestrator | orchestrator prompt |
| D14 | M2 implement | Two filter fields at the M1 shares (main 2 : 1 per field) left each ~110 px: the text input overflowed its share by 16.7 px (containment gate) and "Any component" was cut | The picker's main control and each filter field take equal shares (`PICKER_MAIN` = FillPortion(1)), ~150 px each, as M1's single field had; height unchanged (panel_placement green); the loading line wraps to two lines. M3/M4 cannot add fields to this row at these widths: they need a row of their own or a narrower control (follow-up kept) | agent-resolved | containment + jira_filters_fit red at 2:1:1; ui/worktree_form.rs `PICKER_MAIN` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A M1 | 1 | 41e5337948f133936bb4c5e99334bf44890ac799:4fdb308aa493ee31abef392f5d915366916fc5e1 | CHANGES: 5 fixed (re-search after a filtered reload, kept pick's prompt, stale choice index, Dismissed while not editing, Reporter `choose(Some)`), 5 declined |
| Review A M1 | 2 | 1d2dd0e37d2c4daaade2f729fad8b5aae7cba2b4:f9a90c3782fde89d27f8951426a6e9e1e801827c | CLEAN |
| Review B M1 | 1 | 1d2dd0e37d2c4daaade2f729fad8b5aae7cba2b4:f9a90c3782fde89d27f8951426a6e9e1e801827c | CHANGES: 1 MAJOR (no `tdd/cycle-log.md`: written, red by compile against origin/main and by mutation), 1 MINOR (module path: D11) |
| Review B M1 | 2 | dfd5e253b878efc359c1e9c76793dcd58968503c:edc2a5116bad0aa9239c823766a42908bc1ef694 | CLEAN (1 MINOR: stale module path in contracts/filter-ui.md and quickstart.md — fixed, prose only) |
| Review D12 (rebase refit) | 1 | 77331c1c96849947b4c6c4e6e3992c9564f9efbe:430fd19f9e0dd48c9db0ca2f20cfdd8281cab5a1 | CHANGES: 1 MAJOR (FR-014 weakened through the ledger alone: spec.md FR-014 amended with a clarification entry, prose only), 2 MINOR (stale test name: renamed, active line asserted; one-line check: declined) |
| Review D12 (rebase refit) | 2 | 29a873c6ed917df301be9eae8a12fc99b2ce6d61:aa3b2c523957f19728557c4fc7bf3a5cabe84df2 | CLEAN |
| Review A M2 | 1 | 90a525c42bb3339b4655d236bd0b295aa0c9be94:0cf22212a7325d538abc99d5a97d94b4eeb17c10 | CLEAN (3 MINOR: issue_picker doc displaced — fixed; `choices()` clones — declined; `released` not shown — declined) |
| Review B M2 | 1 | d6f29faa90cf9a1c34034faa4a8d785dce237360:e4de8a606ecec897976a5ce1e3a911a9181d5735 | CLEAN (2 MINOR: cramped main row — kept as follow-up, D13; compile red "by construction" in cycle-log — noted) |
| Visual pass M2 | 1 | e4de8a60 (CPU rasteriser renders of 8 states) | B3 PASS; FAIL: "This project has no versions" cut to "This project ha…" in the list — fixed (wrapped reason under a "No versions" line, gate `an_empty_choice_list_shows_its_whole_reason`); CONCERN: main row cramped (D13) |
| Review rebase M2 (sonnet) | 1 | 8a4b7ddeee15324fd610b34c498807a1d6734ac6:ea88436025f3b37a41cad12c65aff40c2c5ceffa | CLEAN (main's D12 layout kept; Fix version field, PICKER_MAIN, awaited_choices newest-request fix intact; fixture touches only jira states) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | `load_filtered` / `JiraCloud::list_filtered` duplicate `load` / `list_open`; route the old through the new | research.md#R10 keeps `load`, `list_jql`, `search_jql`, `list_open`, `search_open` unedited so #1186 (built in parallel on the same functions) rebases cleanly; fold them after both land (follow-up) |
| M1 | A r1 | `on_filter` calls `start_jira_load` directly, bypassing `start_issue_load` | The load still runs through the one `jira_request` gate (switch, connection, token); filters exist only for the Jira source, so the provider dispatch has nothing to choose; pinned by `filter_choices_and_filtered_loads_start_only_from_their_handlers` |
| M1 | A r1 | A filtered reload opens the issue list | Same as the first load and Retry: the answer shows the list it was asked for (FR-009); the filter list closed on the pick, so nothing is covered |
| M1 | A r1 | `rows()` clones / `rematch` clone; `seat` closure style | MINOR (taste, per-keystroke cost of a few dozen components) |
| M1 | A r1 | Highlight lands on inert rows | MINOR; same behaviour as the existing issue and session pickers' Typeahead |
| M1 | D12 r1 F3 | The one-line check compares the active line's height with "Clear filters"'s, which might be the 40 px button | `Overflow::natural_height` is the painted paragraph, not the node: the label is one 20 px line, two caption lines are 32 px, so a wrap fails it |
| M2 | A r1 | `choices()` returns an owned, cloned/mapped `ChoiceLoad<String>` on every call | MINOR (taste): lists of project scale; the same per-call clone M1's review A judged MINOR; one owned view keeps the two filters' different choice types out of the row code |
| M2 | A r1 | `VersionChoice::released` never reaches the UI | Deliberate: FR-003 asks only for the order (unreleased first); the spec's entity names the state, the UI contract shows the name alone; the flag is what `parse_versions` orders by |

## Follow-ups not done

- After #1185 and #1186 both merge: implement `JiraSource::load`/`JiraCloud::list_open`/`search_open` through their filtered forms with empty filters (review A M1, declined for R10).
- M2 (D14): with two fields the issue search shows ~83 px of its placeholder and a chosen value ~31 px ("Payments" scrolls; the active line names it whole). M3/M4 cannot add the Reporter and Label fields to this row: they need a row of their own (finding vertical room in the 800 px window, panel_placement) or narrower field chrome.
- M2–M4 add fields to the picker's main row (D9) and values to the one-line active line (D12: the filtered form is at y=66, 1 px of slack; a second line of "Filtered by …" pushes it over the app bar): re-check `gates/jira_filters_fit.rs` and `panel_placement`. The visual pass of M1 found the Component field ~153 px wide, so a chosen value scrolls in a 35 px text area beside the search icon and the clear action (the active line under it names it in full); the four-field row M4 builds has to give each field room or move the fields to a row of their own.
