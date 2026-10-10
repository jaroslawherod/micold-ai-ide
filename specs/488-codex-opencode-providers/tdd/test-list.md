# Test list — #488 Codex and OpenCode providers

Outer = observable through a public seam (acceptance scenario, spec.md). Inner = one pure derivation. Tests named are the ones that exist; daemon and client rows are in their crates.

## Outer

| Behaviour | Scenario | Test |
|---|---|---|
| A Codex or OpenCode session starts the right command and is remembered across a restart | US1-1..3 | `codex_opencode_start.rs`; core `store_roundtrip::every_provider_survives_a_save_and_load` |
| Default provider and MCP `create_session` accept both | US1-4, US1-5 | `provider_choice_surfaces.rs`, `features_settings.rs`, `mcp_tools_catalog.rs` |
| A missing CLI is shown unavailable with its command, and a start is refused | US2-1..3 | `missing_cli_is_reported_where_it_is_chosen.rs`, `directory_availability.rs`, `cli_reason.rs` |
| A bound session resumes its own conversation, an unbound one starts fresh, never another's | US3-1, US3-2 | `codex_resume.rs`, `codex_opencode_start.rs`, core `resume::*` |
| A session is named from its first typed turn | US3-3 | `the_session_is_named_from_its_first_typed_turn` (both providers) |
| No resume offered ⇒ fresh start without error | US3-4 | `a_cli_that_fails_hangs_prints_junk_or_is_missing_yields_nothing` |
| Activity stays `Unknown` without a source | US4-1 | `activity_pipeline.rs` (Codex/OpenCode cases) |
| Sandbox: sign-in files shared writable at their own paths, absent ones pruned, git credentials read-only | US5-1..3 | `sandbox_credentials.rs`, `sandbox_argv::only_the_ai_cli_sign_in_is_mounted_writable` |
| Wire carries both providers (protocol 39) | US1 | `schema_hash::the_wire_changes_for_this_feature_cost_exactly_one_version_bump` |

## Inner

| Behaviour | Test |
|---|---|
| Provider surface: command, name, id, empty fresh args, PATH/PATHEXT availability | `identity_is_codex`, `identity_is_opencode`, `a_fresh_start_passes_no_arguments`, `availability_follows_the_path_it_is_given`, `a_cmd_shim_or_exe_on_the_path_counts_when_pathext_lists_it` |
| Persisted names and `AiCli::ALL` order (chooser lists) | `ai_cli_registry::*` |
| Seam additions keep app-assigned ids for claude/copilot/pi/fake | `ai_cli_provider::minted_identity_seam::*` |
| Candidates: this directory, newer than spawn less 2 s, three newest day directories, not bound elsewhere | `resume::a_new_rollout_in_this_directory_is_a_candidate`, `…another_directory…`, `…older_than_the_spawn…`, `the_clock_allowance_is_two_seconds` (both), `a_fourth_day_directory_is_not_offered`, `candidates_are_this_folders_…` |
| Exactly one candidate is bound; a binding is written once; a stale one can be replaced | `only_exactly_one_candidate_is_ever_bound`, `a_binding_is_written_once_and_never_reassigned`, `a_binding_whose_conversation_is_gone_can_be_replaced` |
| Naming: typed turn first, `response_item` fallback skipping `<environment_context>` and `# AGENTS.md` | `the_session_is_named_from_its_first_typed_turn`, `a_response_item_turn_names_the_session_when_there_is_no_event`, `the_fallback_name_skips_inserted_context_items` |
| Only a bounded prefix is read | `only_a_bounded_prefix_is_read_for_the_name` |
| Close leaves a marker outside the CLI store | `closing_marks_the_session_archived_outside_the_cli_store`, `closing_a_session_leaves_a_marker_outside_the_cli_store` |
| Missing or unreadable store yields nothing | `a_missing_or_unreadable_store_yields_nothing` |
| Launch resolves the provider's own store | `terminal_backend::a_codex_launch_resumes_the_conversation_bound_in_its_store` |
