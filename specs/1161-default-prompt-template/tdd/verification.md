---
feature: 1161-default-prompt-template
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 8fda5012e
behaviors: 13 # O1-O8, I1-I5 in tdd/test-list.md
proven: 0
likely: 0
test_after: 13 # no cycle log, no red recorded for any behavior (fail closed); see Test-first evidence
no_test: 0
high_smells: 0
criteria_total: 18 # US1 5 + US2 4 + US3 2 + US4 7 acceptance scenarios; FR-001..FR-014 mapped below
criteria_covered: 18
mutation_score: unmeasured # no mutation tool; 2 deliberate mutants run, both caught
mutants_survived: 0 # of the 2 run; the rest unmeasured
suite: not re-run by this audit; caller-reported passes only (features_worktree_form 36, mcp_default_prompt 11, one client shell test, one daemon lib test); full gate not recorded
independent: false # this audit continues a session that also wrote T026-T029; files were re-read cold
---

# TDD Verification: Default prompt template (#1161), M1-M3 and close tests T026-T029

**Verdict: FAIL.** No behavior has a recorded red: there is no `tdd/cycle-log.md`, the M1-M3
commits add tests and source together, and the close tests (T026-T029) were written against
existing code. By the rubric that is `TEST_AFTER` for every behavior. Everything else is sound: no
HIGH smell, no weakened existing test, every scenario has a test through the real entry point, and
two deliberate mutants were caught. The FAIL is a process verdict that the history cannot be
repaired to lift; see "Next step".

Independence: the close tests (T026-T029) were written in the same session lineage as this audit.
Every file cited below was re-read, but treat the smell pass on those tests as not independent.

## Test-first evidence

`tdd/test-list.md` was written at close and says so: test-first order for M1-M3 is unrecoverable.
The rubric reserves `LIKELY` for behaviors whose cycle log records a red; there is no cycle log, so
`LIKELY` is not available and the fail-closed class applies.

| Behavior | Class | Evidence |
| --- | --- | --- |
| I1-I3 renderer (`prompt_template.rs`) | TEST_AFTER | `30da479a0` (wip handover) adds test and source in one commit; no red recorded |
| I4 shared fixture | TEST_AFTER | same commit family; the fixture is `pub mod shared_fixture` in source, used by core, form and daemon tests |
| I5 `name_words` | TEST_AFTER | `d21e87594` adds the test after `04bd61f26` shipped the code |
| O1, O2, O3 (dialog) | TEST_AFTER | `2be5f7a52`, `4b7e78d0f` add form tests and source together; reducer-driven O3 tests added after the fact in `13f52232c` |
| O4 stored template reaches form | TEST_AFTER | `13f52232c` (T026) written against existing code; no red run. Proof is by mutation of the neighbouring reducer call, not of this path (unmeasured) |
| O5 validation | TEST_AFTER | `c31963164` adds tests and validator in one commit |
| O6 MCP | TEST_AFTER | `04bd61f26` adds `mcp_default_prompt.rs` and `ops.rs`/`mcp/tools.rs` together |
| O7 5 s bound | TEST_AFTER | `13f52232c` (T027) pins the constant against existing code; the slow-lookup test predates it |
| O8 GitHub off | TEST_AFTER | `13f52232c` (T028) against existing code; mutant proof below (caught) |

Mitigation, not a reclassification: for O3 and O8 a deliberate mutant now shows the tests fail when
the behavior is removed. That is evidence of strength, not of order.

## Existing tests changed by the feature

Unchanged conclusion from the previous audit for `30da479a0..169ef801b` (protocol-version
re-pin 64 to 65, support extraction from `mcp_create_session.rs`, layout snapshot regeneration,
struct-literal updates; none a weakening). New since: `13f52232c` splits two two-scenario tests in
`crates/micold-daemon/tests/mcp_default_prompt.rs` into four, and renames
`a_failed_or_slow_lookup_renders_title_and_body_empty` to `a_failed_lookup_...` plus
`a_slow_lookup_...`, and `a_blank_prompt_falls_through_and_a_bad_template_is_no_prompt` to
`a_blank_prompt_falls_through_to_the_template` plus `a_bad_template_is_no_prompt`. Every assertion
is carried over unchanged (diff read line by line); no test was skipped, filtered out, or had a
threshold lowered. The renames move names out of any `--exact` filter that used the old names; no
such filter exists in `mise.toml` tasks or scripts that this audit grepped for in the test list.

Checkboxes: T001-T029 are `[x]`. T025-T029 match the list. No behavioral task is unticked with its
behavior `DONE`.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (process) | No cycle log and no recorded red for any of the 13 behaviors; M1-M3 history is wip handover commits with tests and source together, T026-T029 were written against existing code. Test-first is unprovable and cannot be recovered. Decides the verdict. | `tdd/test-list.md` header; `30da479a0`, `2be5f7a52`, `c31963164`, `04bd61f26`, `13f52232c` |
| 2 | MED | The 5 s bound is pinned as a constant (`assert_eq!(PROMPT_LOOKUP_BOUND, 5 s)`), which a hand-edited literal in the test and constant would satisfy together, and it does not show the default is used: `prompt_lookup_bound: Mutex::new(PROMPT_LOOKUP_BOUND)` at `state.rs:1001` is unpinned, and the slow-lookup test overrides the bound to 200 ms. Changing line 1001 to another duration would pass. Prediction, not measured. Should assert the effective bound of a fresh `DaemonState`. | `crates/micold-daemon/src/state.rs:8153-8161`, `:1001`; `mcp_default_prompt.rs:287` |
| 3 | MED | Mutation unmeasured for most behaviors. Only 2 of the deliberate mutants prepared were run. Not run: blank-explicit-prompt check in `mcp/tools.rs`, `first_prompt_edited` guard, `PROMPT_LOOKUP_BOUND` 5 s to 50 s, `name_words` to empty, the `DaemonSettings` to form path behind O4. | see Mutation results |
| 4 | MED | Layout snapshot regenerated and accepted in the same commit (self-approving risk), mitigated by screenshots `visual/c3.png`, `c5.png`, `c11.png`. Still open from the previous audit. | `e3f4fceeb`, `crates/micold-client/tests/fixtures/layout_snapshot.txt` |
| 5 | MED | Eager test with substring assertions: one test drives listing arrival, pick and description arrival, asserting `contains`/`ends_with` on a pipe-joined string with a `{picked}` message rather than `assert_eq!` on the full value. A wrong separator or a duplicated segment would pass. | `crates/micold-client/tests/features_worktree_form.rs:657-718` (asserts at ~688-691, 713) |
| 6 | LOW | `Slow` lookup sleeps a real 2 s on an abandoned thread (`std::thread::sleep`); `settle()` polls with a 25 ms sleep up to 10 s. Test is bounded but not clock-free. Previous findings 7 (eager) is resolved by the split; this remainder is LOW. | `mcp_default_prompt.rs:190-194`, `:131-137` |
| 7 | LOW | Same behavior pinned at two levels: "a CLI chosen after none refreshes the field" in form-method form and reducer form. Acceptable double-loop; note only. | `features_worktree_form.rs:581` and `:720` |
| 8 | LOW | Test list `traces` for I4 names `shared_fixture`, which is the fixture module (`crates/micold-core/src/prompt_template.rs:188`), not a test; the tests are `renders_the_shared_fixture` (`prompt_template.rs:15`) and `the_dialog_renders_what_the_shared_fixture_says` (`features_worktree_form.rs:571`). O5 and O6 rows cite modules/files rather than test names. | `tdd/test-list.md` |
| 9 | LOW | FR-002's on-screen placeholder list has no automated test (glue; quickstart §B). US2 s1 literal example not asserted verbatim. | `prompt_template.rs:92`, `:23-32` |

Resolved since the previous audit: former findings 2 (reducer calls not driven through `Msg`), 3
(stored template to form), 4 (5 s bound unpinned, partly: see 2), 5 (GitHub-off gate untested), 7
(eager two-scenario tests).

No HIGH smell from the catalogue: assertions are value-specific, doubles are the repo's
`FakeIssueSource`-style fakes, the daemon tests drive the real MCP tool server with a stand-in CLI,
and the reducer tests go through `State::update`. Hard Rule 7: no instruction-like text found in
the files read.

## Mutation results

No mutation tool in the profile. Deliberate mutants, sampled 2 behaviors (not exhaustive), run by
the caller and recorded here; this audit did not run builds or mutants.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| delete `form.refresh_default_prompt()` in `type_selected`, `crates/micold-client/src/features/worktree_form.rs:1221` | O3 | No | Caught by `type_ticket_and_name_changes_refresh_an_unedited_field` |
| `if false && !state.github_issues_enabled()` in `crates/micold-daemon/src/ops.rs:595` | O8 | No | Caught by `github_issues_off_looks_nothing_up` |
| blank explicit prompt check, `mcp/tools.rs` | O6 | unmeasured | not run |
| `first_prompt_edited` guard, `worktree_form.rs` | O2, O3 | unmeasured | not run |
| `PROMPT_LOOKUP_BOUND` 5 s to 50 s, `state.rs:663` | O7 | unmeasured | caught by the constant pin by construction; the `:1001` default path is not (finding 2) |
| `name_words` to empty, `naming.rs` | I5 | unmeasured | not run |
| `DaemonSettings` to form path, `shell/` | O4 | unmeasured | not run |

Both executed mutants were restored and the suite re-run green per the caller's facts; this audit
did not re-verify the restore (`git status` clean at `8fda5012e`).

## Traceability

Entry: the form reducer (`State::update` with `Msg`) for the dialog, the MCP tool server for MCP,
the shell `update` for settings arrival.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 s1-s2 | `features_worktree_form.rs:498,507`; reducer `:619-645` | Yes (reducer) |
| US1 s3-s4 | `a_cleared_field_stays_empty` `:518`, `an_empty_template_leaves_the_field_empty` `:527` | Form methods; reducer path covered for edit guard `:647` |
| US1 s5 | `:536`, reducer `:636`, `:647`, `:657` | Yes (reducer) |
| US2 s1-s4 | `prompt_template.rs:23-62` | Core API |
| US3 s1-s2 | `features_settings.rs` `default_prompt_template` modules; form `:459` | Draft validate, form |
| US4 s1-s5, s7 | `mcp_default_prompt.rs` | Yes (MCP server, real PTY) |
| US4 s6 | `:273`, `:~287`, `state.rs:8153` | Yes (bound overridden; default unpinned, finding 2) |
| FR-001 | core and daemon `settings_default_prompt_template.rs`; `main_tests.rs:3591` | Yes |
| FR-002 | `prompt_template.rs:92` pins the constant only | Manual (§B) |
| FR-011 | shared fixture in core, form, daemon | Yes |
| FR-013 | `features_worktree_form.rs:459`, `:592` | Yes |
| FR-014 GitHub off | `mcp_default_prompt.rs:305` | Yes |
| SC-003 | existing `mcp_create_session.rs`, `mcp_create_worktree.rs`, `an_empty_template_types_nothing` | Yes |

Untested criteria: FR-002's on-screen list (manual). Tests tracing to nothing: none.

## What was not audited

- The full workspace suite and `mise run gate`: no run, no wall time, no failure-versus-baseline
  separation. Only the four caller-reported results above.
- Mutants other than the two listed; no mutation tool, no score, no coverage.
- `features_settings.rs` modules, the daemon `settings_default_prompt_template.rs` and
  `github_issue_lookup.rs`: skimmed in the previous audit, not re-read in full here.
- The visual pass (quickstart §B) and the sandbox acceptance suite; Windows, macOS, remote daemons.
- Performance and flakiness over repeated runs; the real-clock `Slow` sleep was not timed.
- Independence: not independent for T026-T029 (see top).

## Next step

Test-first for M1-M3 cannot be recovered, so finding 1 cannot be cleared by work; it needs either an
explicit waiver recorded by the owner (which would move the verdict to PASS_WITH_GAPS with findings
2-5 open) or acceptance of the FAIL. Technical follow-ups: T030-T033 in `tasks.md`.
