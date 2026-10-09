# TDD cycle log — #488 M3

M3's tests and code were written in one pass, not red-first. Red evidence below is retrospective: each behaviour's implementation was broken on purpose, the test run, and the break reverted (2026-10-09).

| Behaviour (task) | Break applied | Failing test (right reason) |
|---|---|---|
| Bound session resumes its own conversation, unbound starts fresh (T018/T019) | `launch_args_in` never returns `resume <id>` | `codex_provider::resume::a_bound_session_resumes_its_conversation_and_an_unbound_one_starts_fresh`, `…a_binding_whose_conversation_is_gone_can_be_replaced` |
| Bind only when exactly one candidate (T018, FR-006) | `sole_candidate` takes the first of several | `codex_provider::resume::only_exactly_one_candidate_is_ever_bound` |
| A shell session in the same folder must not block binding (review A F1, T021) | peer filter without the AI-CLI mode check | `codex_resume::a_shell_session_beside_it_does_not_hold_the_binding_back` ("timed out waiting for the binding") |

Not broken retrospectively (covered by the same passes, green): seam additions for claude/copilot/pi/fake (T016), name from first turn, bounded prefix, archive marker, daemon bind + restart resume, two same-folder sessions never cross-resume (T020).

# TDD cycle log — #488 M4

Written test-first this time. Red run (2026-10-09) of `cargo test --test opencode_provider` before any OpenCode code: 8 passed (M1 tests), 7 failed — `resume::a_bound_session_resumes_…` (no `--session` args), `candidates_are_this_folders_…`, `the_nested_time_shape_is_read_too`, `a_conversation_another_session_is_bound_to_…`, `two_unclaimed_conversations_are_never_guessed_between` (all: `new_conversations` returned nothing), `a_session_is_named_from_its_first_typed_turn` (no name), `a_cli_that_fails_hangs_prints_junk_or_is_missing_…` (control case found nothing). Each failed on the missing behaviour, not on a compile error. Green after the provider code: 15 passed.

| Behaviour (task) | Test |
|---|---|
| Bound session resumes `--session <id>`, unbound/other/fresh start without args (T023/T024) | `resume::a_bound_session_resumes_its_own_conversation_and_an_unbound_one_starts_fresh` |
| Candidates by directory, created after spawn, not bound elsewhere; both time shapes | `candidates_are_…`, `the_nested_time_shape_is_read_too`, `a_conversation_another_session_is_bound_to_…` |
| Never guess between two (FR-006) | `two_unclaimed_conversations_are_never_guessed_between` |
| Bad JSON, wrong shape, exit≠0, 2 s timeout, no CLI ⇒ nothing (FR-007) | `a_cli_that_fails_hangs_prints_junk_or_is_missing_yields_nothing` |
| Name from first typed turn via `export`; none when unbound/unreadable (FR-008) | `a_session_is_named_from_its_first_typed_turn`, `an_unbound_session_or_an_unreadable_export_has_no_name` |
| Daemon: bind, restart resumes, no cross-resume, archive marker (T025, FR-010) | `codex_resume.rs` OpenCode cases (the Codex fixture is shared) |

The daemon cases were added with the code; one race in the first draft (the stub recorded its conversation before the second session started, so session 1 was legitimately bound) was a test flaw, fixed by making the stub record a second after its start.
