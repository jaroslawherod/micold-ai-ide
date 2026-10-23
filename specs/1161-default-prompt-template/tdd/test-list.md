# Test list: Default prompt template (#1161)

Written at close, after M1-M3 shipped. Test-first order for M1-M3 cannot be recovered (no cycle
log; handover commits added tests and source together), so every behavior is `LIKELY` at best, not
`PROVEN`. Behaviors added at close (T026-T029) were written against existing code and proved by
mutation, not by a red run.

## Outer (acceptance)

| ID | Behavior | Criterion | Test |
| --- | --- | --- | --- |
| O1 | Dialog pre-fills the first prompt from the template when a CLI and an issue are chosen | US1 s1-s2 | `features_worktree_form.rs` `default_prompt_template`, reducer module |
| O2 | An edited prompt is what the session receives; a cleared field stays empty | US1 s3-s4 | `an_edited_prompt_is_what_the_session_receives`, `a_cleared_field_stays_empty` |
| O3 | Type, name and issue changes keep typed text and refresh an untouched field | US1 s5 | `typed_text_survives_...`, reducer-driven tests (T026) |
| O4 | A stored template reaches the opened dialog | FR-001 | `main_tests.rs` `a_stored_default_prompt_template_reaches_the_opened_form` (T026) |
| O5 | Settings rejects an unknown placeholder; a bad stored template renders empty | US3 | `features_settings.rs` `default_prompt_template` modules |
| O6 | MCP `create_session` / `create_worktree` type the rendered template | US4 s1-s5, s7 | `mcp_default_prompt.rs` |
| O7 | Title/body lookup is bounded at 5 s and falls back to empty | US4 s6 | `a_failed_lookup_...`, `a_slow_lookup_...`, `prompt_lookup_bound_tests` (T027) |
| O8 | GitHub issues off: no lookup, title/body empty | FR-014 | `github_issues_off_looks_nothing_up` (T028) |

## Inner (unit)

| ID | Behavior | Criterion | Test |
| --- | --- | --- | --- |
| I1 | Six placeholders render; missing value renders empty | US2 s1-s2 | `prompt_template.rs` |
| I2 | Doubled braces are literal; inserted body is not re-expanded | US2 s3-s4 | `prompt_template.rs` |
| I3 | A blank result is `None` | FR-009 | `blank_result_is_none` |
| I4 | One shared fixture across core, form and daemon | FR-011 | `shared_fixture` |
| I5 | `name_words` splits a branch name | US4 | `naming::name_words_tests` |
