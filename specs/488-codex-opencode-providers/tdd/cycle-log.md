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

# TDD cycle log — #488 M1/M3 retrospective red (T037) and T041 pins

Core behaviours only; the client and daemon rows belong to their crates. Each break was applied by hand to one line, the single test run, the line restored (2026-10-10). Command prefix: `cargo test -p micold-core --test`.

| Behaviour | Break applied | Command (after the prefix) and failing line |
|---|---|---|
| Provider surface | `CodexProvider::command` returns `"codexx"` | `codex_provider identity_is_codex` — `left: "codexx" right: "codex"` |
| Store round trip | `StoredAiCli::Codex => AiCli::Pi` in `store.rs` | `store_roundtrip every_provider_survives` — `left: [.., Pi, Pi, OpenCode] right: [.., Pi, Codex, OpenCode]` |
| Protocol 39 | `PROTOCOL_VERSION` = 38 | `schema_hash` — `the_wire_changes_for_this_feature_cost_exactly_one_version_bump`, `left: 38 right: 39` |
| Chooser lists | `AiCli::ALL` lists `Pi` where `Codex` was | `ai_cli_registry` — `iterating_the_variants_is_deterministic_and_complete` |
| Seam additions | `ClaudeProvider::identity` returns `Minted` | `ai_cli_provider the_app_picks` — `left: Minted right: AppAssigned` |
| Codex candidates | cwd filter replaced by `true` | `codex_provider a_rollout_for_another` — assertion that no candidate is offered fails |
| Naming | `"user_message"` matched as `"user_messagex"` | `codex_provider the_session_is_named` — `left: None right: Some("fix the flaky test")` |
| Bounded prefix | `PREFIX_BYTES` 64 KiB → 64 MiB | `codex_provider only_a_bounded` — `left: Some("too late") right: None` |
| Archive marker | `mark_archived` writes nothing | `codex_provider closing_marks` — `is_archived` assertion fails |
| Codex bind/resume | rows in the M3 table above (T018/T019 breaks) | not repeated |

T041 pins, each failing when its constant or filter is mutated (provider.rs lines, then restored):

| Pin | Mutation | Failing test |
|---|---|---|
| 2 s `CLOCK_ALLOWANCE`, Codex (1 s inside, 3 s outside) | `from_secs(0)` and `from_secs(10)` | `codex_provider resume::the_clock_allowance_is_two_seconds` (both) |
| 2 s `CLOCK_ALLOWANCE`, OpenCode | `from_secs(0)` and `from_secs(10)` | `opencode_provider resume::the_clock_allowance_is_two_seconds` (both) |
| `RECENT_DAYS` = 3 | 4 and 2 | `codex_provider resume::a_fourth_day_directory_is_not_offered` (both) |
| Fallback skips `<…` context item | drop `!starts_with('<')` | `resume::the_fallback_name_skips_inserted_context_items` |
| Fallback skips `# AGENTS.md` item | drop that filter | same test |

Note: in the first mutation run (dropped `<` filter) `the_seam_answers_for_activity_tools_readiness_and_trust` also failed once; it passes unmutated and in every other run, and was not reproduced.
