---
feature: 582-attach-provider-worktrees-sessions
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 13
planned_at: 8947038b
updated_at: 8947038b
suite_baseline: unknown
---

# Test List: Attach provider worktrees and sessions

Written after the fact (T044) from the tests as they stand. Criterion ids: `US<n>-AS<m>` is
acceptance scenario m of user story n in `spec.md`; `FR-nnn` is a functional requirement. The
`test` column names `file::fn`, files under `crates/<crate>/tests/` (the `daemon_sync.rs` rows U85-U91 are unit tests inside `crates/micold-client/src/shell/`) (the crate is named in each
section heading), and each name was checked with `grep -n`. Every state is `DONE` because the
tests exist and passed at the last gate; the red-phase evidence for each is in `cycle-log.md`
(T009/T012 have no recorded red, noted there).

Acceptance runner: the profile's `acceptance` command is the sandbox-real-runtime suite and does
not host this feature. The outer behaviors below are integration tests over composed modules
(daemon `Catalog` / `DaemonState` plus the MCP tool server, and the client reducer driven with
daemon messages), not end-to-end GUI tests. The GUI side is covered by the manual pass in
`visual/results-close.md`.

## Outer loop: acceptance behaviors

| id  | behavior                                                                                      | traces      | kind    | state | test                                                                                         |
| --- | --------------------------------------------------------------------------------------------- | ----------- | ------- | ----- | -------------------------------------------------------------------------------------------- |
| A1  | With no records, the project's unrecorded provider worktrees are reported as attachable        | US1-AS1     | example | DONE  | `micold-core/tests/attach_discovery.rs::agent_owned_unrecorded_worktrees_are_listed`         |
| A2  | An attached worktree survives a restart and shows with the agent filter off, untouched         | US1-AS2     | example | DONE  | `micold-daemon/tests/attach_apply.rs::attached_worktrees_survive_a_restart_and_show_with_the_filter_off` |
| A3  | Attaching again reports already attached and writes no duplicate                               | US1-AS3     | example | DONE  | `micold-daemon/tests/attach_apply.rs::attaching_again_reports_already_attached`              |
| A4  | This project's 5 sessions are listed as resumable and the sibling's 2 are not                  | US2-AS1     | example | DONE  | `micold-core/tests/attach_discovery.rs::resumable::this_projects_five_are_listed_and_the_siblings_two_never` |
| A5  | Resuming a listed session whose worktree is unattached attaches the worktree first, then starts | US2-AS2     | example | DONE  | `micold-daemon/tests/attach_apply.rs::resuming_a_session_in_an_unattached_worktree_attaches_the_worktree_first` |
| A6  | A session already in the catalog is not listed again                                           | US2-AS3     | example | DONE  | `micold-core/tests/attach_discovery.rs::resumable::a_session_already_in_the_catalog_is_not_listed` |
| A7  | A missing store gives no sessions and a note the user can see                                  | US2-AS4     | example | DONE  | `micold-core/tests/attach_discovery.rs::resumable::a_missing_store_gives_no_sessions_and_a_note` |
| A8  | `attach_worktree` by name attaches and `list_worktrees` returns it not hidden                  | US3-AS1     | example | DONE  | `micold-daemon/tests/attach_mcp.rs::attaching_by_dir_name_lists_the_worktree_as_not_hidden`  |
| A9  | `attach_worktree` on a path outside the project fails with a reason and changes nothing        | US3-AS2     | example | DONE  | `micold-daemon/tests/attach_mcp.rs::a_path_outside_the_project_and_another_projects_worktree_are_not_found_and_change_nothing` |
| A10 | `attach_worktree` on an attached worktree reports already attached, no duplicate               | US3-AS3     | example | DONE  | `micold-daemon/tests/attach_mcp.rs::attaching_again_reports_already_attached_and_creates_no_duplicate` |
| A11 | A Default caller is refused `attach_worktree` and nothing is attached                          | US3-AS4, FR-015 | example | DONE | `micold-daemon/tests/attach_mcp.rs::a_default_caller_is_refused_and_nothing_is_attached`     |
| A12 | Opening a project with no records and something found offers attach-all in one action          | US4-AS1     | example | DONE  | `micold-client/tests/attach_offer.rs::a_project_with_no_records_and_something_found_is_offered` |
| A13 | A project with provenance records never gets the offer and nothing is attached unasked         | US4-AS2     | example | DONE  | `micold-client/tests/attach_offer.rs::a_project_with_provenance_records_never_gets_the_offer` |

## Inner loop: unit behaviors

### `crates/micold-core/src/attach.rs` (tests in `micold-core/tests/attach_discovery.rs`)

| id  | behavior                                                                                  | traces        | kind    | state | test                                                                       |
| --- | ----------------------------------------------------------------------------------------- | ------------- | ------- | ----- | -------------------------------------------------------------------------- |
| U1  | A recorded worktree is not offered                                                        | FR-001, FR-004| example | DONE  | `attach_discovery.rs::a_recorded_worktree_is_not_listed`                   |
| U2  | A prunable or missing worktree is offered as unavailable, not attachable                  | Edge case     | example | DONE  | `attach_discovery.rs::a_prunable_or_missing_worktree_is_unavailable_missing` |
| U3  | Only worktrees directly under the provider directory qualify                              | FR-001, FR-005| example | DONE  | `attach_discovery.rs::only_worktrees_directly_under_the_provider_directory_qualify` |
| U4  | An unattached provider worktree stays hidden (agent-worktree hiding unchanged)            | FR-013        | example | DONE  | `attach_discovery.rs::an_unattached_provider_worktree_stays_hidden`        |
| U5  | Session count and provider of an offered worktree come from the catalog sessions there    | FR-001        | example | DONE  | `attach_discovery.rs::session_count_and_provider_come_from_the_catalog_sessions_at_that_worktree` |
| U6  | A root session targets Default and is resumable                                           | FR-006, FR-008| example | DONE  | `attach_discovery.rs::resumable::a_root_session_targets_default_and_is_resumable` |
| U7  | A session at an unattached worktree is flagged as needing the worktree attached           | FR-008        | example | DONE  | `attach_discovery.rs::resumable::a_session_at_an_unattached_worktree_needs_the_worktree_attached` |
| U8  | A deleted worktree's sessions are unresumable and never mapped elsewhere                  | FR-008, Edge case | example | DONE | `attach_discovery.rs::resumable::a_deleted_worktrees_sessions_are_unresumable_and_never_mapped_elsewhere` |
| U9  | A corrupt entry is skipped and noted while the rest list                                  | FR-009        | example | DONE  | `attach_discovery.rs::resumable::a_corrupt_entry_is_skipped_and_noted_while_the_rest_list` |
| U10 | A provider with no config directory is a note, not an error                               | FR-014        | example | DONE  | `attach_discovery.rs::resumable::a_provider_with_no_config_dir_is_a_note_not_an_error` |
| U11 | A Windows path in a different case is the same project                                    | FR-007        | example | DONE  | `attach_discovery.rs::resumable::a_windows_path_in_a_different_case_is_the_same_project` |
| U12 | Thousands of sessions list newest first and are bounded to the page                       | FR-006        | example | DONE  | `attach_discovery.rs::resumable::thousands_list_newest_first_and_bounded_to_the_page` |
| U13 | Discovery leaves the provider store byte-for-byte untouched                               | FR-010        | example | DONE  | `attach_discovery.rs::resumable::discovery_leaves_the_provider_store_untouched` |

### `crates/micold-core/src/provider.rs` `store_dirs` / session listing (`micold-core/tests/provider_store_dirs.rs`)

| id  | behavior                                                                              | traces        | kind    | state | test                                                                   |
| --- | ------------------------------------------------------------------------------------- | ------------- | ------- | ----- | ---------------------------------------------------------------------- |
| U14 | Claude returns the root and the git-listed worktrees                                  | FR-006        | example | DONE  | `provider_store_dirs.rs::claude_returns_the_root_and_the_git_listed_worktrees` |
| U15 | Claude finds the store dir of a worktree git no longer lists                          | FR-006, Edge case | example | DONE | `provider_store_dirs.rs::claude_finds_the_store_dir_of_a_worktree_git_no_longer_lists` |
| U16 | Claude never returns a sibling project's directory                                    | FR-007        | example | DONE  | `provider_store_dirs.rs::claude_never_returns_a_sibling_projects_directory` |
| U17 | Claude rejects a matching name whose transcript cwd is elsewhere                      | FR-007        | example | DONE  | `provider_store_dirs.rs::claude_rejects_a_matching_name_whose_transcript_cwd_is_elsewhere` |
| U18 | Claude rejects a transcript cwd in a prefix lookalike of the managed directory        | FR-007        | example | DONE  | `provider_store_dirs.rs::claude_rejects_a_transcript_cwd_in_a_prefix_lookalike_of_the_managed_directory` |
| U19 | `is_within` is strictly below, on a path boundary                                     | FR-007        | example | DONE  | `provider_store_dirs.rs::is_within_is_strictly_below_on_a_path_boundary` |
| U20 | A directory with a corrupt first line is skipped and reported                         | FR-009        | example | DONE  | `provider_store_dirs.rs::claude_skips_a_directory_with_a_corrupt_first_line_and_says_so` |
| U21 | A Windows-style path matches case-insensitively                                       | FR-007        | example | DONE  | `provider_store_dirs.rs::claude_matches_a_windows_style_path_case_insensitively` |
| U22 | Copilot and Pi return their known locations only                                      | FR-014        | example | DONE  | `provider_store_dirs.rs::copilot_and_pi_return_the_known_locations_only` |
| U23 | Copilot lists this project's sessions from its index                                  | FR-006        | example | DONE  | `provider_store_dirs.rs::copilot_lists_this_projects_sessions_from_its_index` |
| U24 | Pi lists this project's sessions from its directory                                   | FR-006        | example | DONE  | `provider_store_dirs.rs::pi_lists_this_projects_sessions_from_its_directory` |
| U25 | A provider with no readable store lists nothing, with no error                        | FR-014        | example | DONE  | `provider_store_dirs.rs::a_provider_with_no_readable_store_lists_nothing` |

### `crates/micold-daemon/src/catalog.rs` attach and resume (`micold-daemon/tests/attach_apply.rs`)

| id  | behavior                                                                              | traces        | kind    | state | test                                                                   |
| --- | ------------------------------------------------------------------------------------- | ------------- | ------- | ----- | ---------------------------------------------------------------------- |
| U26 | A batch writes one record per dir name                                                | FR-002        | example | DONE  | `attach_apply.rs::a_batch_writes_one_record_per_dir_name`              |
| U27 | A recorded dir name is already attached and nothing is written                        | FR-004        | example | DONE  | `attach_apply.rs::a_recorded_dir_name_is_already_attached_and_nothing_is_written` |
| U28 | A target absent from the live cache is refused and unrecorded                         | FR-005        | example | DONE  | `attach_apply.rs::a_target_absent_from_the_live_cache_is_refused_and_unrecorded` |
| U29 | A worktree outside the managed directory is refused and unrecorded                    | FR-005        | example | DONE  | `attach_apply.rs::a_worktree_outside_the_managed_directory_is_refused_and_unrecorded` |
| U30 | A missing worktree is refused as unavailable                                          | Edge case     | example | DONE  | `attach_apply.rs::a_missing_worktree_is_refused_as_unavailable`        |
| U31 | Two concurrent attaches of one worktree leave exactly one record                      | FR-016        | example | DONE  | `attach_apply.rs::two_concurrent_attaches_of_one_worktree_leave_exactly_one_record` |
| U32 | Attaching changes nothing in the worktree (branch, files, uncommitted changes)        | FR-003        | example | DONE  | `attach_apply.rs::attaching_changes_nothing_in_the_worktree`           |
| U33 | Sixteen worktrees attach in one action within ten seconds                             | FR-012, SC    | example | DONE  | `attach_apply.rs::sixteen_worktrees_attach_in_one_action_within_ten_seconds` |
| U34 | An unreadable project refuses every target and records nothing                        | FR-009        | example | DONE  | `attach_apply.rs::an_unreadable_project_refuses_every_target_and_records_nothing` |
| U35 | A failed persist rolls the records back so a retry attaches                           | FR-004        | example | DONE  | `attach_apply.rs::a_failed_persist_rolls_the_records_back_so_a_retry_attaches` |
| U36 | Discovery lists this project's sessions and attaching one adds an idle entry (no start) | FR-006, FR-008 | example | DONE | `attach_apply.rs::discovery_lists_this_projects_sessions_and_attaching_one_adds_an_idle_entry` |
| U37 | A session of a deleted worktree is refused, not resumed elsewhere                     | FR-008        | example | DONE  | `attach_apply.rs::a_session_of_a_deleted_worktree_is_refused_not_resumed_elsewhere` |
| U38 | A second resume of a Starting session is refused as already running                   | FR-016        | example | DONE  | `attach_apply.rs::a_second_resume_of_a_starting_session_is_refused_as_already_running` |
| U39 | A second resume of a Running session is refused as already running                    | FR-016        | example | DONE  | `attach_apply.rs::a_second_resume_of_a_running_session_is_refused_as_already_running` |
| U40 | A second resume of a Restarting session is refused as already running                 | FR-016        | example | DONE  | `attach_apply.rs::a_second_resume_of_a_restarting_session_is_refused_as_already_running` |
| U41 | A second resume of an Idle session is one entry, already attached                     | FR-016        | example | DONE  | `attach_apply.rs::a_second_resume_of_an_idle_session_is_one_entry_already_attached` |
| U42 | A second resume of a Failed session is one entry, already attached                    | FR-016        | example | DONE  | `attach_apply.rs::a_second_resume_of_a_failed_session_is_one_entry_already_attached` |
| U43 | The already-running refusal tells the user why                                        | FR-016        | example | DONE  | `attach_apply.rs::the_already_running_refusal_tells_the_user_why`      |
| U44 | A sandboxed daemon reports the store is not readable                                  | FR-009        | example | DONE  | `attach_apply.rs::a_sandboxed_daemon_reports_the_store_is_not_readable` |
| U45 | The resume launch carries the right argument and session id for each provider         | FR-008, FR-014 | example | DONE | `attach_apply.rs::the_resume_launch_carries_the_right_argument_and_session_id_for_each_provider` |

### MCP surface (`micold-core/src/mcp/tools.rs`, `micold-daemon/src/mcp/tools.rs`)

| id  | behavior                                                                              | traces        | kind    | state | test                                                                   |
| --- | ------------------------------------------------------------------------------------- | ------------- | ------- | ----- | ---------------------------------------------------------------------- |
| U46 | Attaching by absolute path and by branch resolves the same worktree                   | FR-005        | example | DONE  | `micold-daemon/tests/attach_mcp.rs::attaching_by_absolute_path_and_by_branch_resolves_the_same_worktree` |
| U47 | An ambiguous branch and an unavailable worktree are invalid input                     | FR-005        | example | DONE  | `micold-daemon/tests/attach_mcp.rs::default_an_ambiguous_branch_and_an_unavailable_worktree_are_invalid_input` |
| U48 | `list_resumable_sessions` pages newest first, honours limit and offset, skips catalog sessions | FR-011 | example | DONE | `micold-daemon/tests/attach_mcp.rs::resumable_sessions_page_newest_first_with_limit_and_offset_and_skip_catalog_sessions` |
| U49 | Policy: Default is refused `attach_worktree` yet may list resumable sessions          | FR-015, FR-011 | example | DONE | `micold-core/tests/mcp_policy.rs::a_default_caller_is_refused_attach_worktree_and_may_list_resumable_sessions` |
| U50 | Catalog: `attach_worktree` takes one reference, is audited, not destructive           | FR-005        | example | DONE  | `micold-core/tests/mcp_tools_catalog.rs::attach_worktree_takes_one_reference_and_is_audited_but_not_destructive` |
| U51 | Catalog: `list_resumable_sessions` takes a limit and an offset                        | FR-011        | example | DONE  | `micold-core/tests/mcp_tools_catalog.rs::list_resumable_sessions_pages_with_a_limit_and_an_offset` |

### Client (`crates/micold-client/src/features/attach.rs`, `shell/daemon_sync.rs`)

Dialog reducer, in `micold-client/tests/attach_dialog.rs`.

| id  | behavior                                                                              | traces        | kind    | state | test                                                                   |
| --- | ------------------------------------------------------------------------------------- | ------------- | ------- | ----- | ---------------------------------------------------------------------- |
| U52 | Opening with no project does nothing                                                  | FR-001        | example | DONE  | `attach_dialog.rs::opening_without_a_project_does_nothing`             |
| U53 | Opening waits for the report, then lists it                                           | US1-AS1       | example | DONE  | `attach_dialog.rs::opening_waits_for_the_report_then_lists_it`         |
| U54 | A report for another project is dropped                                               | FR-007        | example | DONE  | `attach_dialog.rs::a_report_for_another_project_is_dropped`            |
| U55 | Checkbox selection toggles and ignores unavailable rows                               | FR-001        | example | DONE  | `attach_dialog.rs::checkbox_selection_toggles_and_ignores_unavailable_rows` |
| U56 | Attach-selected sends only the ticked rows                                            | FR-002        | example | DONE  | `attach_dialog.rs::attach_selected_sends_only_the_ticked_rows`         |
| U57 | Attach-selected with nothing ticked sends nothing                                     | FR-002        | example | DONE  | `attach_dialog.rs::attach_selected_with_nothing_ticked_sends_nothing`  |
| U58 | Attach-all sends every attachable row and not the missing one                         | FR-012        | example | DONE  | `attach_dialog.rs::attach_all_sends_every_attachable_row_and_not_the_missing_one` |
| U59 | A second press while applying changes nothing                                         | FR-016        | example | DONE  | `attach_dialog.rs::a_second_press_while_applying_changes_nothing`      |
| U60 | An applied batch closes the dialog and says what happened                             | US1-AS2       | example | DONE  | `attach_dialog.rs::an_applied_batch_closes_the_dialog_and_says_what_happened` |
| U61 | Already attached is said, not silent                                                  | FR-004        | example | DONE  | `attach_dialog.rs::already_attached_is_said_not_silent`                |
| U62 | A refusal is reported as an error                                                     | FR-005        | example | DONE  | `attach_dialog.rs::a_refusal_is_reported_as_an_error`                  |
| U63 | A failed apply keeps the dialog and shows why                                         | FR-009        | example | DONE  | `attach_dialog.rs::a_failed_apply_keeps_the_dialog_and_shows_why`      |
| U64 | Cancel is ignored while an apply is outstanding                                       | FR-016        | example | DONE  | `attach_dialog.rs::cancel_is_ignored_while_an_apply_is_outstanding`    |
| U65 | A stale answer with no apply outstanding is dropped                                   | FR-016        | example | DONE  | `attach_dialog.rs::a_stale_answer_with_no_apply_outstanding_is_dropped` |
| U66 | Cancel closes the dialog                                                              | US1-AS2       | example | DONE  | `attach_dialog.rs::cancel_closes_the_dialog`                           |
| U67 | The report's sessions are kept for the dialog to draw                                 | FR-006        | example | DONE  | `attach_dialog.rs::the_report_s_sessions_are_kept_for_the_dialog_to_draw` |
| U68 | Resume sends one session target                                                       | FR-008        | example | DONE  | `attach_dialog.rs::resume_sends_one_session_target`                    |
| U69 | An unresumable session offers no resume and says why                                  | FR-008        | example | DONE  | `attach_dialog.rs::an_unresumable_session_offers_no_resume_and_says_why` |
| U70 | A second resume while one is applying changes nothing                                 | FR-016        | example | DONE  | `attach_dialog.rs::a_second_resume_while_one_is_applying_changes_nothing` |
| U71 | A resumed session closes the dialog and a refusal names its reason                    | FR-008, FR-016 | example | DONE | `attach_dialog.rs::a_resumed_session_closes_the_dialog_and_a_refusal_names_its_reason` |
| U72 | Notes are a footer when nothing was found and for real problems                       | US2-AS4       | example | DONE  | `attach_dialog.rs::notes_are_a_footer_when_nothing_was_found_and_for_real_problems` |
| U73 | Opening then cancelling leaves nothing behind (isolation guard)                       | FR-013        | example | DONE  | `micold-client/tests/features_attach.rs::opening_then_cancelling_leaves_nothing_behind` |

Offer banner reducer, in `micold-client/tests/attach_offer.rs`.

| id  | behavior                                                                              | traces        | kind    | state | test                                                                   |
| --- | ------------------------------------------------------------------------------------- | ------------- | ------- | ----- | ---------------------------------------------------------------------- |
| U74 | Nothing found means no offer                                                          | FR-012        | example | DONE  | `attach_offer.rs::nothing_found_means_no_offer`                        |
| U75 | No report yet means no offer                                                          | FR-012        | example | DONE  | `attach_offer.rs::no_report_yet_means_no_offer`                        |
| U76 | An unreadable project is not known to have no records, so no offer                    | FR-009, FR-012 | example | DONE | `attach_offer.rs::an_unreadable_project_is_not_known_to_have_no_records` |
| U77 | A report for another project is dropped                                               | FR-012        | example | DONE  | `attach_offer.rs::a_report_for_another_project_is_dropped`             |
| U78 | Listing attaches nothing                                                              | FR-012        | example | DONE  | `attach_offer.rs::listing_attaches_nothing`                            |
| U79 | Dismissing hides the offer for this project only                                      | FR-012        | example | DONE  | `attach_offer.rs::dismissing_hides_the_offer_for_this_project_only`    |
| U80 | A dismissed offer stays dismissed when the report is refreshed                        | FR-012        | example | DONE  | `attach_offer.rs::a_dismissed_offer_stays_dismissed_when_the_report_is_refreshed` |
| U81 | Dismissing leaves the attach dialog listing everything                                | FR-001        | example | DONE  | `attach_offer.rs::dismissing_leaves_the_attach_dialog_listing_everything` |
| U82 | Attach-all sends every attachable worktree once                                       | FR-012        | example | DONE  | `attach_offer.rs::attach_all_sends_every_attachable_worktree_once`     |
| U83 | The answer ends the offer and says what happened                                      | FR-012        | example | DONE  | `attach_offer.rs::the_answer_ends_the_offer_and_says_what_happened`    |
| U84 | A failed attach keeps the offer so it can be retried                                  | FR-012        | example | DONE  | `attach_offer.rs::a_failed_attach_keeps_the_offer_so_it_can_be_retried` |
| U85 | An answer with nothing in flight is ignored                                           | FR-016        | example | DONE  | `attach_offer.rs::an_answer_with_nothing_in_flight_is_ignored`         |
| U86 | A batch that all failed keeps the offer                                               | FR-012        | example | DONE  | `attach_offer.rs::a_batch_that_all_failed_keeps_the_offer`             |

Shell wiring, in `micold-client/src/shell/daemon_sync.rs` (`#[cfg(test)] mod`).

| id  | behavior                                                                              | traces        | kind    | state | test                                                                   |
| --- | ------------------------------------------------------------------------------------- | ------------- | ------- | ----- | ---------------------------------------------------------------------- |
| U87 | Opening a project asks for the offer and the report reaches it                        | FR-012        | example | DONE  | `daemon_sync.rs::opening_a_project_asks_for_the_offer_and_the_report_reaches_it` |
| U88 | The dialog's apply answer routes to the dialog                                        | FR-002        | example | DONE  | `daemon_sync.rs::the_dialog_apply_answer_routes_to_applied`            |
| U89 | The banner's apply answer routes to the offer                                         | FR-012        | example | DONE  | `daemon_sync.rs::the_banner_apply_answer_routes_to_offer_applied`      |
| U90 | A resume answered attached or already attached starts that session                    | FR-008        | example | DONE  | `daemon_sync.rs::a_resume_answered_attached_or_already_attached_starts_that_session` |
| U91 | A resume refused as already running starts nothing                                    | FR-016        | example | DONE  | `daemon_sync.rs::a_resume_refused_as_already_running_starts_nothing`   |

## Invariants and edge cases still to place

- `attach_discovery.rs:316` assertion for a provider with no config directory is open (T049); the
  test exists as U10 but its assertion is weaker than FR-014 wants.
- The two-attach race (U31) uses a test-owned `Mutex`, not two `AttachApply` messages through
  `DaemonState` (T046); concurrency through the real server path is not yet shown.
- The Windows path-case tests (U11, U21) run on the host path rules; a real Windows run is not
  part of this suite.

## Out of scope

- Real-GUI end-to-end of the dialog and banner: covered by the manual pass, `visual/results-close.md`.
- Protocol wire shape (`ClientMsg::AttachDiscover`, `AttachApply`): covered by
  `micold-core/tests/protocol_roundtrip.rs` and `schema_hash.rs` generic round-trip tests, not
  listed per behavior.
- Anything in the spec's "Out of Scope" section (`spec.md`, section Out of Scope).
- Property tests, mutation and coverage: the profile has no tool for them.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md`:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets`
- Acceptance: `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1`
- Mutation, coverage, property: none in the profile
