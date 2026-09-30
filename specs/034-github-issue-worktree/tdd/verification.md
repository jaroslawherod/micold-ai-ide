---
feature: 034-github-issue-worktree
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset present)
verified_at: b463fc03 # short SHA audited (main after PRs #460 #468 #472 #478 #495 #496 #500)
behaviors: 135
proven: 113
likely: 6
test_after: 12
no_test: 0
not_applicable: 4
high_smells: 2
criteria_total: 23
criteria_covered: 23
mutation_score: unmeasured # no mutation tool in the profile; 15 deliberate mutants, 15 caught (sample, not a score)
mutants_survived: 0
suite: 3991 passed, 0 failed, 9 ignored, 366 binaries, 179s (twice; the second run after every mutant was restored)
---

# TDD Verification: Create a Worktree from a GitHub Issue

**Verdict: FAIL.** Two `HIGH` smells (A4's preview comparison can never fail; T095 asserts nothing
on a host without `gh`) and twelve behaviors the cycle log itself records as written after their
code. **No behavior is unbuilt**: all 23 spec scenarios reach a shell-level test, all 15 deliberate
mutants were caught, and the suite is green. Every finding is test strength or process.

Independence: this audit was not run by the session that wrote the tests. The smell pass was
delegated to four fresh-context subagents; every `HIGH` and every finding quoted here was re-opened
by the auditor, and the unvetted subagent items are marked as such under "What was not audited".

## Test-first evidence

History shape: the feature landed as 37 commits, one per cycle, each adding test and source
together (no test-only commit exists). The PRs were rebase-merged, so ordering inside a commit is
not recoverable from history; `PROVEN` therefore rests on the cycle log's recorded red plus the
absence of a source-before-test commit. The log and the history agree everywhere: no log entry
claims a red the history contradicts, and no commit precedes its cycle's test.

| Class | Count | Behaviors |
| --- | --- | --- |
| `PROVEN` | 113 | Every row with a red command and decisive output: cycles 1-12 (U1-U112 except below), 15-20, 22-25, 27, plus A1-A12, A15, A16 |
| `TEST_AFTER` | 12 | A13, A14, A17 (passed at first run, cycle 22); A18-A23 (cycle 26, "red: none observed -- **test-after**"); U88 (cycle 25); U105 (cycle 13); U70 core half (cycle 14) |
| `LIKELY` | 6 | U13, U16, U23, U40, U65, U75: passed on the stub, no red of their own, then pinned by a recorded deliberate mutant (U75 by `ISSUE_BUDGET_MS=0`). A rejection or budget test cannot fail against a stub that also rejects, so this is mostly unavoidable, but the rubric has no class for it |
| `NOT_APPLICABLE` | 4 | U1, U2, U4 (passed before the change: they characterize the moved runner), U5 |

The log is honest about every `TEST_AFTER`, and each was followed by a mutant check (cycles 13, 14,
16, 25, 26). This audit re-ran the equivalent mutants against the final tree (M4, M6, M8, M10): each
was caught at both the unit and the shell level, so the tests do bite today. The finding is about
discipline, not about a test that passes vacuously (A18-A23's individual weaknesses are in F4, F8).

Existing tests changed by the feature (Phase 2 check), each read against its diff:

| Test | Change | Judgment |
| --- | --- | --- |
| `issue_source_state.rs::a_pick_fills_ticket_and_name` (U55) | dropped `type_ == Some(Chore)` "the pick leaves the type alone"; now expects the type cleared | Requirement change (FR-014 supersedes), red seen in cycle 21, logged. Not a weakening |
| `main_tests.rs` A5/A6 | `TypeSelected(Fix)` moved after the pick | Same requirement change, logged. **A4 was not moved; see F1** |
| `issue_source_state.rs::a_search_does_not_start_while_creating` | renamed and inverted (U110) | Logged twice (cycles 16, 17) with the reason |
| `github_gh_cli.rs` `ListNotFound` stub | removed (cycle 16) and re-added in another form (cycle 17) | Logged; the behaviour is pinned by `a_typed_error_stands_at_any_exit_status` |
| `issue_source_state.rs` cap caption (U104) | text changed to the contract's wording | Logged as a stated test change |
| `main_tests.rs` A21, A22 | real `JsonFileSettingsStore` replaced by `FakeSettingsStore`, file read-back removed (commit `ccf815e4`) | Forced by the architecture gate, logged, but it **weakens what the two tests pin; see F4** |
| `protocol_auth.rs`, `schema_hash.rs` | version pin 15 to 16 (later 17) | Mandatory bump, logged |
| `icons.rs` `ALL.len()` 32 to 35 | three new glyphs | Legitimate |
| `settings_sections.rs` `DEFERRED` emptied | the deferred setting arrived, as the test demands | Legitimate |

No assertion was removed without a replacement, no test was skipped or renamed out of a filter, and
no threshold was lowered. No weakened existing test that the requirement did not force was found.

`tasks.md` against the test list: 91 tasks ticked, none unticked; every behavior is `DONE`. No
ticked task lacks a `DONE` behavior and no `DONE` behavior has an unticked task.

## Findings

Ordered by severity. "Kind" separates behavior that is unbuilt (none) from test-strength and
process items.

| # | Sev | Kind | Finding | Evidence |
| --- | --- | --- | --- | --- |
| F1 | HIGH | test strength | Vacuous equality in A4. `TypeSelected(Fix)` runs before `pick()`, and `pick()` sends `mapping: vec![]`, which clears the type (FR-014). Both previews are then `Err(type required)`, so `assert_eq!(as_issue.preview(), as_new.preview())` passes for any preview implementation. A4's "preview equals the new-branch preview" is unpinned at the shell level. `issue_source_state.rs::issue_source_previews_as_new` (U57) does pin it, so FR-012 is covered, but A4 claims US1-4 and FR-012 | `crates/micold-client/src/main_tests.rs:5428-5435`; compare the fixed A5 at `:5444` |
| F2 | HIGH | test strength | Conditional logic: `a_desktop_launch_finds_a_working_gh` returns after an `eprintln!` with zero assertions when `gh` is found nowhere, or only through a version manager, and is green. By design (T095) and CI fails closed (`skip_unless_ci` asserts `!on_ci()`), so the macOS/Windows/Linux CI legs do assert. But a local green says nothing about FR-026, and the test lacks `#[ignore]` so the skip is invisible in the count | `crates/micold-core/tests/github_locate_desktop_launch.rs:56-59, 69-78` |
| F3 | MED | process | Twelve behaviors `TEST_AFTER` and six without their own red (table above). Mitigated by mutants; A18-A23 is the largest block and was the only M5 outer loop | `cycle-log.md` cycles 13, 14, 22, 25, 26 |
| F4 | MED | test strength | A21 no longer pins restart persistence (US3-4, FR-020). After `ccf815e4` it serializes the saved `Settings` through `serde_json` and reads it back (serde's derive, not `JsonFileSettingsStore`), and `stored(&store) == expected` reads back what the fake was saved. The test-list row still says "read back from `settings.json` by a fresh store". A22's "file unchanged" became `saves().is_empty()` (no write attempted), and `stored(&store) == default_mapping()` at `:6119` is a tautology beside it; its blank and duplicate cases share one test and `saves` is checked only after the second. Persistence proper is pinned only in core (`settings_issue_mapping.rs`, U80) | `main_tests.rs:6061-6081, 6085-6120`; `test-list.md` rows A21, A22 |
| F5 | MED | test strength | Process-global `std::env::set_var` never restored, in test binaries that run tests in parallel: `GIT_CONFIG_GLOBAL` races with `git_cli_lists_remotes_and_none_is_not_an_error` (its comment says no other test depends on it, but that test spawns git); `GH_DEBUG=api` races with the other `GhCli` tests. Set it on the child `Command` or serialize | `crates/micold-core/tests/git_remotes.rs:125`; `crates/micold-core/tests/github_gh_cli.rs:160` |
| F6 | MED | test strength | Relative oracle: U42 asserts the `RemoteList` refusal equals whatever `BranchList` returns, never that it says "not a repository"; if both regressed to the same generic error it passes | `crates/micold-daemon/tests/remote_list.rs:191-210` |
| F7 | MED | test strength | Re-implemented expectation plus conditional: U40 rebuilds the expected name by re-implementing whitespace normalisation, and `if slugify(..).len() <= 50` skips the equality for long titles, so the bound alone is checked for them | `crates/micold-core/tests/naming_from_title.rs:101-104` |
| F8 | MED | test strength | Claims larger than assertions. A1: only `matches!(github, Available(_))`, not "three sources, issue one enabled" (`main_tests.rs:5327-5330`). A3: the Down/Enter half derives the expected ticket from the same `issue_number_at(highlight)` the pick uses (`:5371-5384`). A2: rows' titles and labels are not asserted, only numbers in the fake's order (`:5344-5353`). U85: "editing the mapping does not change the picked type" (`:5867-5871`) reads nothing after the save, so no implementation fails it (FR-014a is held by U84). `capped_rig`'s guard `!is_some_and(Loaded{complete})` also passes on Loading/Failed (`:5599-5608`). U53's `is_none_or(..)` admits an implementation that clears the highlight on every re-rank (`issue_source_state.rs:446-451`) |  |
| F9 | MED | test strength | U70 core half, `a_body_only_match_is_hidden`, pins the generic `typeahead::rank` with issues that simply do not match; "matched only in body on GitHub" cannot be represented, and "number, title or label" is exercised only by title. The reducer half (`issue_source_state.rs::an_unmatched_searched_issue_is_hidden`) is the real pin | `crates/micold-core/tests/github_search.rs:39-51` |
| F10 | MED | test strength | SC-006 (U82) checks only that no top-level key contains "issue" or "github" beyond the mapping, so a leaked `gh_token` key passes, and no issue is ever loaded into the state being saved | `crates/micold-core/tests/settings_issue_mapping.rs:613-623` |
| F11 | MED | test strength | U75's guard `every_frame_budget_measurement_is_release_only` matches tests containing `took < BUDGET_MS`; the issue test uses `ISSUE_BUDGET_MS`, so removing its `ignore` attribute fails no guard; and the 1,000-row corpus size and that "cra" matches any row are never asserted | `crates/micold-core/tests/typeahead_budget.rs:217-262, 293-327` |
| F12 | MED | test strength | U62 is a source-text scan of exact trimmed lines (`Some(chosen) => start_issue_load(app, chosen),`): a rename or rewrap breaks it and a caller split across lines evades it. The behaviour pin (opening the form issues no load; FR-003) is carried by `issue_opening_the_form_asks_for_remotes` only indirectly. The scanner also `return`s silently on unreadable dirs (`:83-85, 114-116`) | `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs:28-72, 129-162` |
| F13 | MED | test strength | Real-time sleeps: `process_run_bounded.rs:96` sleeps a fixed 4 s and bounds wall time at `bound + 1 s` (flaky under io stalls, the profile's known failure mode); `main_tests.rs:5623-5687` (U74, A10) wait out the real 300 ms debounce twice; `github_gh_cli.rs::a_hung_gh_is_timed_out` uses real 500 ms and 5 s | as cited |
| F14 | MED | test strength | Missing assertion messages, against the profile convention "every assertion carries a message": roughly 30 bare `assert_eq!`/`assert!` in `issue_source_state.rs` (U44-U56, U103, U104), `github_locate.rs`, `github_remote.rs`, `git_remotes.rs:96-111`, `github_load.rs`, `github_parse.rs`, `github_gh_cli.rs`, `naming_from_title.rs`, `github_search.rs:29`. Eager tests that fail without saying which behavior broke: U44, U46, U72, U83, U96, U9, U33, U94/U102, U95/U111 | as cited |
| F15 | MED | test strength | Redundant or source-grepping tests: `icons_font.rs:70-80` (U93) is covered by `every_icon_codepoint_has_a_glyph` and `icons.rs`; `material_builder_api.rs:124-133` (U65) greps source text for `pub fn disabled(mut self, disabled: bool)`; `showcase_completeness.rs:763-772` checks `COMPONENTS` metadata, not a rendered disabled chip | as cited |
| F16 | MED | test strength | `bypassed helper`: `main_tests.rs:5246-5270` hand-rolls `PathResolver` with a call counter while core ships `FakeEnvIncludeResolver::answering(..)` with `.calls()` and the same module already uses it at `:5911`. `remote_list.rs:28-109` copies `git`/`catalog_with_project`/`connect`/`ask` that ~17 other daemon test files also copy (repo precedent) | as cited |
| F17 | MED | test strength | Weak negatives and gaps: U34's `Other(ref s) if !s.is_empty()` (`github_classify.rs:121`) where the exact fallback `"gh exited with status 1"` is knowable; U31 tests the `gh auth login` text only with exit 4; `github_gh_cli.rs:174, 195, 247` switch on `cfg!(unix)` so `GH_DEBUG`/`GH_PAGER` (U102) are unverified on the Windows stub; U20's `!contains("chocolatey")` is case-sensitive (`github_locate.rs:132-137`); U29 does not assert `total_open == 1001` (`github_load.rs:59-66`); U66's fixture cannot tell "search first" from "lookup first" | as cited |
| F18 | LOW | process | Test-list bookkeeping: T095's test has no row; U43 still says "`PROTOCOL_VERSION` is 16" while `protocol_auth.rs` and `schema_hash.rs` now both pin 17 (two tests pin one number); `updated_at` still the planning SHA; `RemoteList` round-trips only with a non-empty `remotes` | `test-list.md:5-8, 150`; `protocol_auth.rs:176-179`; `protocol_roundtrip.rs` |
| F19 | LOW | style | Duplicated setup, magic literals (`1_234`, `999_999`, index `3`), unclear names (`not_signed_in`, `no_access`), a synthetic token-shaped literal in a URL fixture that secret scanners may flag (`github_remote.rs:54`, not a credential), `assert!(!on_ci())` helper that is named like a skip | as cited |

No secrets were found in tests or in `fixtures/gh/*`. No file contained text addressed to the
auditor. No test is `#[ignore]`d or skipped in the committed state, except U75 and its four
siblings, which are release-only by the repository's own BUG-003 convention and run in CI's
release job; this audit ran U75 in release and it passed (9 passed).

## Mutation results

No mutation tool is configured (`mutation: null`), so this is a **sample of 15 deliberate mutants**,
not a score. One change each, applied to a clean tree, the named tests run, the file restored with
`git checkout`, tree confirmed clean, then the full workspace suite re-run green (3991 passed).

| Mutant | Behavior | Caught | By |
| --- | --- | --- | --- |
| M1 `choose_remote` prefers `upstream` over `origin` | U14, U15 | Yes | `origin_wins_when_on_github`, `first_github_remote_otherwise` |
| M2 cap boundary `>=` to `>` | U29 | Yes | `the_cap_is_1000` |
| M3 slug bound `<=` to `<` | U36, U40 | Yes | `fits_and_the_50_boundary`, `slug_never_exceeds_50` |
| M4 label match made case-sensitive | U78, A15 | Yes | unit `case_and_no_match`, `validation`; shell `issue_label_matching_ignores_case`, `issue_an_invalid_mapping_is_not_saved` |
| M5 last mapping entry wins | U77, A12 | Yes | unit `mapping_order_wins`; shell `issue_the_first_mapping_entry_wins` and A20 |
| M6 a pick keeps the type when no label matches | FR-014, A13 | Yes | unit `the_pick_sets_or_clears_the_type`, `a_pick_fills_ticket_and_name`; shell A13 |
| M7 a pick keeps an existing ticket | FR-010a, A8 | **Unit: survived. Shell: caught** | `issue_source_state.rs` has no test that a pick replaces a typed ticket; A8 `issue_a_pick_replaces_ticket_and_name` catches it. U55 fills a blank form only |
| M8 duplicate label never detected | U79, A22 | Yes | unit, `features_settings`, shell A22 |
| M9 stale load result applies | U46, FR-007a | Yes | `only_the_awaited_result_applies` and two more; shell A7 |
| M10 `into_settings` drops the mapping | U86, A21 | Yes | `the_mapping_survives_other_saves`; shell A19, A20, A21 |
| M11 `GH_DEBUG` not removed | U102 | Yes | `gh_cli_runs_gh_as_specified` |
| M12 `gh` run in the working directory, not home | FR-022/R6 | Yes | `gh_cli_runs_gh_as_specified` |
| M13 search also on a complete list | U71, FR-005a | **Unit: caught. Shell: survived** | `search_only_when_incomplete`; A10 covers only the incomplete list |
| M14 env-include `PATH` after process `PATH` | U18 | Yes | `candidate_order` |
| M15 settings load drops the mapping (`persist.rs:266`) | FR-020 | Yes | six shell `issue_*` tests |

Survivors, triaged: M7 (unit layer) and M13 (shell layer) each survive at one layer only and are
caught at the other, so neither is a survivor inside a `DONE` behavior. They are layer gaps, not
missing behavior. Not sampled: U1-U10, U24-U35 beyond the above, U41-U43, the daemon arm, FR-005's
ranking, FR-025's caption text, the toggle chip.

## Traceability

Every spec scenario maps to a shell-level test through `update_inner` with a fake issue source and
fake settings store. The profile's `sandbox_real_*` acceptance runner cannot see this form, so
`main_tests.rs::issue_source` is the highest real entry point that runs without a display or
network; the rendered half is quickstart §B, run by `visual-pass` (evidence under `evidence/`).

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1-1, FR-001/002 | A1 (weak, F8), U44, U96 | Yes |
| US1-2, FR-004 | A2 (rows weak, F8), U17-U30 | Yes |
| US1-3, FR-005 | A3 (half weak, F8), U52 | Yes |
| US1-4, FR-009/010/012 | A4 (**preview half vacuous, F1**), U55, U57 | Yes, FR-012 via U57 only |
| US1-5, US1-6, FR-011/012 | A5, A6 | Yes |
| US1-7, FR-006/007a | A7, U46, U48, U51 | Yes |
| US1-8, FR-010a | A8 (M7 caught here only) | Yes |
| US1-9, FR-003/025 | A9, U62 (source scan, F12), U96 | Yes |
| US1-10, FR-005a | A10, U66-U74, U95 | Yes |
| US2-1 to US2-7, FR-013-015, FR-021 | A11-A17, U76-U78, U83-U85 | Yes |
| US3-1 to US3-3, US3-5, US3-6, FR-017-019, FR-021 | A18-A20, A22, A23, U79, U87-U92 | Yes |
| US3-4, FR-020 | A21 (**no longer a restart test, F4**), U80-U82 (core) | Partly: persistence is pinned in core, the shell half proves only that `Saved` hands the mapping to `save` |
| FR-007, FR-022, FR-026, SC-004 | U31-U35, U94, U17-U22, T095 (F2) | Unit and CI only; quickstart §B10/B11 by hand |
| FR-023, FR-024, SC-006 | U50, U59, U62, U82 (weak, F10) | Yes |

All 135 `traces`-named tests exist (checked mechanically by `fn` name); none is missing. Untested
criteria: none. Tests tracing to nothing: T095's `github_locate_desktop_launch.rs` has no list row
(F18); U97 and U98-U112 were added to the list with reasons.

## What was not audited

- Mutation was a 15-mutant sample by hand, not a tool run: there is no score. Areas without a
  mutant: the bounded runner's kill and drain (U1-U4, mutated in the cycle log only), the GraphQL
  parsers and arguments (U24-U27, U66-U68), `classify` (U31-U34), `RemoteList` in the daemon, the
  settings draft reducers (U87-U93), the ranking and highlight logic, and every UI caption.
- Coverage: no tool (`coverage: null`), so no branch data corroborates the traceability table.
- The `sandbox_real_*` acceptance suite and the release-mode daemon half were not run; the feature
  has none of its own.
- Rendered behavior (colour, layout, tooltips at 520 px) is quickstart §B, visual-pass evidence only;
  the layout snapshot fixtures were checked for regeneration history, not for correctness.
- macOS and Windows: `#[cfg(windows)]` and macOS arms of U17-U22 and T095 run only on CI legs this
  audit did not observe.
- The smell pass relied on four read-only subagents; the `HIGH` items and the cited A21/A22, U42,
  U40, U70, U85, `set_var` lines were re-opened by the auditor. The remaining MED/LOW items
  (F12-F17) are the subagents' reads, spot-checked, not each re-opened line by line.
- Performance: U75 and the 10 s bound (SC-004) are the only timed criteria; load behaviour against
  a live GitHub was not assessed. The full-suite wall time (179 s) is faster than the profile's 343 s
  baseline, taken at a different commit.
- The other feature numbered 034 (`034-daemon-mcp-server`) shares commit-message ids (`U72-U75`,
  `U82`) with this one; its commits were excluded and were not audited.
