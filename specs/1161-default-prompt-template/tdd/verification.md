---
feature: 1161-default-prompt-template
verdict: BLOCKED
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 169ef801b
behaviors: 0 # no tdd/test-list.md exists
proven: 0
likely: 0
test_after: 0 # not classifiable; see Test-first evidence
no_test: 0
high_smells: 0
criteria_total: 18 # US1 5 + US2 4 + US3 2 + US4 7 acceptance scenarios
criteria_covered: 18 # by reading only; no test was run
mutation_score: unmeasured # no mutation tool; 0 deliberate mutants executed
mutants_survived: unmeasured
suite: not run (disk full, link failed with a bus error; see Phase 0)
independent: true # auditor did not write these tests
---

# TDD Verification: Default prompt template (#1161), M1-M3

**Verdict: BLOCKED.** The feature has no `tdd/test-list.md` and no `tdd/cycle-log.md`, so
test-first discipline cannot be shown for any behavior, and the suite could not be run (the `/home`
disk was at 100% with 839 MB free and the daemon test link died with `ld terminated with signal 7
[Bus error]`). What follows is a static audit from cold reads of the tests, the sources and the git
history. It is not a pass.

## Scope of the diff

`git diff 7406e4cdc...HEAD` covers 1959 files because main merged unrelated features into the branch
after the design PR. Only the 1161 commits were audited (17, `30da479a0`..`169ef801b`, found with
`git log --grep 1161`). Tests read: `crates/micold-core/tests/prompt_template.rs`,
`crates/micold-core/tests/settings_default_prompt_template.rs`,
`crates/micold-core/tests/github_privacy.rs` (new test), `crates/micold-daemon/tests/mcp_default_prompt.rs`,
`crates/micold-daemon/tests/settings_default_prompt_template.rs` (listed in the commit, not re-read in full),
the `default_prompt_template` modules of `crates/micold-client/tests/features_worktree_form.rs:435` and
`crates/micold-client/tests/features_settings.rs:3276,3307`, and `naming::name_words_tests`.

## Test-first evidence

| Behavior | Class | Evidence |
| --- | --- | --- |
| all (no list) | not classifiable; treated as `TEST_AFTER` (fail closed) | No `cycle-log.md`, so no red output recorded for any cycle. |
| T002/T003 renderer, T004/T005 setting, T006/T007 daemon, T009/T010 form, T011/T012 draft | no ordering shown | `30da479a0` and `2be5f7a52` are `wip` handover commits that add tests and source together (for example `crates/micold-core/tests/prompt_template.rs` with `src/prompt_template.rs`). |
| T015-T017 validation (M2) | no ordering shown | `c31963164` adds `features_settings.rs` (+80) and the validator in one commit. |
| T020-T024 MCP (M3) | no ordering shown | `04bd61f26` adds `mcp_default_prompt.rs` (327 lines) and `ops.rs`/`mcp/tools.rs` in one commit. |

tasks.md says tests are "written first (Constitution I)" and must "fail for the right reason", but
nothing in the repository records that they did. Autopilot ledger entries record review verdicts,
not reds. Checkboxes: T001-T024 are all `[x]`; there is no test list to cross-check them against.

## Existing tests changed by the feature

No assertion was removed, loosened, skipped or renamed out of a filter, and no threshold or exclusion
was lowered in the 1161 commits. Changes to pre-existing test files:

- `crates/micold-core/tests/protocol_roundtrip.rs:942` `the_protocol_version_is_64` renamed and
  re-pinned to 65; `schema_hash.rs:326` constant `64` to `65`. Legitimate (wire change), not a weakening.
- `crates/micold-daemon/tests/mcp_create_session.rs`: `Env`, `trust_project`, `install_cli` (about
  70 lines) moved to `tests/support/stand_in_cli.rs`. Reuse, not weakening. Content equivalence was
  not byte-compared.
- `crates/micold-client/tests/fixtures/layout_snapshot.txt` regenerated in `e3f4fceeb` (Settings page
  grew 1164 to 1280 px; rows 14-16 replaced). Explained by the new field, with screenshots
  `visual/c3.png`, `c5.png`, `c11.png`, but regenerated and accepted in the same step (finding 6).
- Literal updates for new struct fields (`body_text: String::new()`, `default_prompt_template: None`).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (process) | No test list and no cycle log: test-first is unprovable for every behavior, and the suite was not run. Blocks the verdict. | `specs/1161-default-prompt-template/tdd/` did not exist; commits above |
| 2 | MED | Dialog tests call `form.refresh_default_prompt()` themselves after assigning `form.ticket`/`type_`/`name` directly. The reducers that must call it (`type_selected`, `ticket_changed`, `name_changed`, source change, listing and description arrival, issue pick, branch pick, ten call sites) are not driven through `Msg` by any test that asserts `first_prompt`. Dropping a call would likely go unnoticed. This is inference from reading, not a measured survivor. Covers US1 s5 and "details arriving later re-render". | `features_worktree_form.rs:497-595`; call sites `worktree_form.rs:888,1165,1221,1245,1254,1309,1471,1526,1691,1743` |
| 3 | MED | The path that gets the stored template into the form (daemon settings to client state to `WorktreeForm.default_prompt_template`) has no test that reads a template from settings and sees the dialog pre-fill; `main_tests.rs` only passes `String::new()`. | `crates/micold-client/src/main_tests.rs:539,2654,3796` |
| 4 | MED | The 5 s lookup bound in the spec (US4 s6) is never exercised or pinned: tests set the bound to 200 ms. Changing `PROMPT_LOOKUP_BOUND` would pass. | `crates/micold-daemon/src/state.rs:659`; `mcp_default_prompt.rs:279,287` |
| 5 | MED | The `github_issues_enabled()` gate in `prompt_facts` (nothing leaves the machine with GitHub issues off) has no test with the switch off. | `crates/micold-daemon/src/ops.rs:600`; `mcp_default_prompt.rs` |
| 6 | MED | Layout snapshot regenerated and accepted in one commit (self-approving risk), mitigated by screenshots. | `e3f4fceeb` |
| 7 | MED | Eager tests: two sandboxes and two scenarios in one test, so one failure hides the other. `Slow` sleeps a real 2 s (`thread::sleep`) in a thread that the bound abandons. | `mcp_default_prompt.rs:274-296,315-327,190-194` |
| 8 | LOW | `settle()` polls with a 25 ms sleep up to 10 s; acceptable for a PTY but a fixed-poll pattern. | `mcp_default_prompt.rs:131-137` |
| 9 | LOW | US2 s1's literal example (`Fix Crash on save on fix/42_crash (fix)`) is not asserted; the six-placeholder and shared-fixture tests cover the same substitution. | `prompt_template.rs:15-32` |

No HIGH smell from the catalogue was found: assertions are value-specific (`assert_eq!` on full
rendered strings and on the stand-in CLI's recorded input), doubles are the repo's `FakeIssueSource`,
and the daemon tests drive the real MCP tool server. Hard Rule 7: no instruction-like text found in
the files read.

## Mutation results

Not measured. The profile has no mutation tool. Six deliberate mutants were prepared (explicit-prompt
blank check in `mcp/tools.rs`, the `first_prompt_edited` guard, the `refresh_default_prompt` call in
`type_selected`, `PROMPT_LOOKUP_BOUND` 5 s to 50 s, the `github_issues_enabled` gate, `name_words`
to empty), but none was run because the disk filled and every build or link failed. The tree was
verified clean after the attempt (`git status --short` empty). Findings 2, 4 and 5 are the places
where such mutants are expected to survive; that is a prediction.

## Traceability

Scenarios to tests (read, not run). "Entry" says whether the test goes through the feature's real
entry point: the reducer methods for the dialog (the repo tests the UI this way; layout is
quickstart §B), the MCP tool server for MCP.

| Criterion | Tests | Entry |
| --- | --- | --- |
| US1 s1-s4 | `features_worktree_form.rs` `a_chosen_cli_and_an_issue_fill...`, `an_edited_prompt_is_what_the_session_receives`, `a_cleared_field_stays_empty`, `an_empty_template_leaves...` | Form methods, not `Msg` (finding 2) |
| US1 s5 | `typed_text_survives_a_change_of_issue_type_or_name...` | Form methods |
| US2 s1-s4 | `prompt_template.rs` `renders_all_six_placeholders`, `missing_value_renders_empty`, `doubled_braces_are_literal`, `inserted_body_is_not_expanded_again`, `blank_result_is_none` | Core API |
| US3 s1 | `features_settings.rs:3307` module, `unknown_placeholder_is_an_error_with_the_contract_text` | Draft validate |
| US3 s2 | `features_settings.rs` stored-invalid test (~3363), `settings_default_prompt_template.rs`, form test `an_invalid_stored_template...` | Draft and form |
| US4 s1-s7 | `mcp_default_prompt.rs` (seven tests, one per scenario, plus blank-prompt and bad-template) | MCP tool server, real PTY with stand-in CLI |
| FR-011 | `shared_fixture` used by core, form and daemon tests | Yes |
| FR-001 | `settings_default_prompt_template.rs` (core and daemon) | Store and catalog |
| FR-002 (placeholder list in UI) | none; `placeholders_list_matches_the_message_order` pins the constant only | Glue, quickstart §B |
| SC-003 | existing `mcp_create_session.rs`, `mcp_create_worktree.rs` unchanged, plus `an_empty_template_types_nothing` | Yes (not run) |

Untested: FR-002's on-screen list (manual), the 5 s bound (finding 4), the GitHub-off gate (finding 5).
Tests tracing to nothing: none. There is no `traces` column to verify.

## What was not audited

- No test was run: the suite, the feature's tests and any mutant. The disk was full (839 MB free of
  858 GB), a link ended in a bus error, and reclaiming space in other worktrees is a shared action I
  did not take. Passing/failing status of every test above is unknown to this audit; the profile's
  baseline (5071 passed at `cdc473ab`) predates the feature.
- Coverage: the profile has none.
- `crates/micold-daemon/tests/settings_default_prompt_template.rs`, `github_issue_lookup.rs` and
  the `main_tests.rs` literals were skimmed through diffs, not read in full.
- The visual pass (quickstart §B) and the sandbox acceptance suite.
- Windows and macOS legs; remote daemons.
- Performance and flakiness over repeated runs.
