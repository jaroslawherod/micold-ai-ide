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

# TDD cycle log — #488 M5

M5 is characterization (T027/T028): the behaviours already hold since M1, so the tests were green on first run (11 passed in `codex_opencode_start`). T029: **no code change**. Red evidence (2026-10-09), by breaking the behaviour on purpose and reverting:

| Behaviour (task) | Break applied | Failing test (right reason) |
|---|---|---|
| Codex in an untrusted folder gets no first prompt (T028, FR-012) | `Codex::folder_trust` → `NeverAsks` | `a_first_prompt_is_never_typed_into_a_trust_question` (prompt delivered when it should be refused) |
| Unsupported tool-server reason is logged, session starts (T028, FR-011) | log text `no tool server:` renamed | `an_unsupported_tool_server_binding_is_logged_and_the_session_starts` |
| Badge stays `Unknown` through output and silence (T027, FR-009) | not broken: `Unknown` is the projection's default, so no cheap break exists; the test also asserts Codex/OpenCode `activity_source` is `None` and Claude Code's is not | `the_badge_stays_unknown_through_output_and_silence` (only its `activity_source` assertions can fail; AS2's busy/idle is pinned by `activity_pipeline::hooks_drive_the_projected_activity_signal`) |

# TDD cycle log — #488 M6

Red run (2026-10-09) of `cargo test -p micold-core --test sandbox_credentials` before any code: compile errors only (no `AiCliProvider::sandbox_auth_file`, no `codex_sign_in`, `ai_cli_auth` still an `Option`) — the red is the missing API, not a wrong assertion. Green after T032: `sandbox_credentials` 26 passed, `ai_cli_provider_seam` green.

| Behaviour (task) | Test |
|---|---|
| Each provider names its own sign-in file; copilot/pi `None` (T031) | `each_provider_names_its_own_sign_in_file` |
| Codex `$CODEX_HOME` under home followed, outside not (pure path cases) | `codex_sign_in_follows_a_codex_home_under_the_home_only` |
| `conventional()` fills `ai_cli_auth` from `AiCli::ALL`, claude first | `the_conventional_layout_lists_every_providers_sign_in_claude_first` |
| One toggle mounts every listed file at its own path | `the_sign_in_share_mounts_every_listed_file_at_its_own_path` |
| Absent files are pruned one by one, the present one stays | client `only_the_sign_in_files_this_host_has_are_shared` |

Existing tests that assumed one sign-in file were updated mechanically (`each_opt_in_adds_exactly_one_mount`, the two `home_*_to_create` tests, the two client prune tests). Finding: the sign-in mount is writable (rule N-4), not read-only as research R5 assumed; Codex/OpenCode follow Claude Code, the guide says so.
