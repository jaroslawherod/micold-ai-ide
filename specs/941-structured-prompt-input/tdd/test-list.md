---
feature: 941-structured-prompt-input
loop: outside-in
profile: .specify/memory/tdd-profile.md
updated_by: T068 (finding 5 of tdd/verification.md)
---

# Test List: Structured prompt input

Derived from `spec.md` (US1-US5 acceptance scenarios, FR-001..FR-033, SC-001..SC-008) and the Traceability
table of `tdd/verification.md`. Every test below was confirmed to exist by `grep -n "fn <name>"`.

| id | behaviour | traces |
| --- | --- | --- |
| US1-AS1 | Send to an idle session delivers one line | `micold-daemon/tests/structured_input.rs::a_send_to_an_idle_session_is_one_line_equal_to_its_encoding` |
| US1-AS2 | Busy shown in pane and sidebar after delivery | `micold-daemon/tests/structured_input.rs::every_window_sees_busy_then_idle_within_a_second_of_the_result`; `micold-client/tests/gates/composer_fits.rs::busy_and_idle_say_so_and_only_busy_offers_stop` |
| US1-AS3 | Result returns every window to idle | `micold-daemon/tests/structured_input.rs::every_window_sees_busy_then_idle_within_a_second_of_the_result`; `micold-daemon/tests/structured_input.rs::the_sessions_activity_follows_the_turn` |
| US1-AS4 | Blank draft is not sendable | `micold-core/tests/structured_input.rs::a_blank_message_is_refused_and_the_turn_stays_idle`; `micold-client/tests/features_composer.rs::can_send_needs_a_sendable_draft_a_running_session_a_connection_and_no_pending_send` |
| US1-AS5 | Not-running session: composer says so | `micold-client/tests/features_composer.rs::can_send_needs_a_sendable_draft_a_running_session_a_connection_and_no_pending_send`; `micold-core/tests/structured_input.rs::a_session_that_is_not_running_refuses_after_the_blank_check` |
| US1-AS6 | Refused send keeps text and shows reason | `micold-client/tests/features_composer.rs::a_refused_send_comes_back_unchanged_with_the_reason`; `micold-client/tests/features_composer.rs::a_refused_send_keeps_what_was_typed_meanwhile_after_it`; `micold-client/tests/features_composer.rs::a_lost_connection_restores_every_pending_draft`; `micold-daemon/tests/structured_input.rs::a_failed_write_is_answered_io_failed_and_queues_nothing` |
| US1-AS7 | Terminal session has no composer | `micold-client/tests/gates/composer_fits.rs::a_terminal_session_has_no_composer`; `micold-client/tests/features_composer.rs::only_a_pane_showing_a_structured_primary_shows_the_composer`; `micold-client/tests/pane_drop_target.rs::a_pane_showing_a_structured_primary_is_the_composers_drop_target` |
| US2-AS1 | Enter sends, Shift+Enter breaks line | `micold-client/tests/features_composer.rs::enter_sends_and_shift_enter_breaks_the_line`; `micold-client/tests/features_composer.rs::enter_during_an_ime_composition_completes_it` |
| US2-AS2 | Pasted text with breaks/tabs/non-ASCII sent verbatim | `micold-daemon/tests/structured_input.rs::a_send_to_an_idle_session_is_one_line_equal_to_its_encoding` |
| US2-AS3 | Long draft scrolls inside composer | `micold-client/tests/gates/composer_fits.rs::a_long_draft_grows_the_composer_to_its_limit_and_the_transcript_stays` |
| US2-AS4 | History-back recalls newest first | `micold-client/tests/features_composer.rs::history_back_recalls_newest_first_down_to_the_oldest`; `micold-client/tests/features_composer.rs::up_then_enter_resends_the_previous_message` |
| US2-AS5 | History-forward restores the draft | `micold-client/tests/features_composer.rs::stepping_forward_past_the_newest_restores_the_draft_unchanged`; `micold-client/tests/features_composer.rs::editing_a_recalled_message_leaves_the_history_unchanged` |
| US2-AS6 | Up/Down move the caret away from first/last line | `micold-client/tests/features_composer.rs::up_steps_back_only_on_the_first_line_and_down_forward_only_on_the_last`; `micold-client/tests/features_composer.rs::a_session_without_messages_keeps_up_and_down_as_caret_moves` |
| US2-AS7 | Drafts kept per session | `micold-client/tests/features_composer.rs::drafts_pictures_and_carets_are_kept_per_session`; `micold-client/tests/features_composer.rs::closing_a_session_forgets_its_composer` |
| US3-AS1 | Stop control / Escape interrupts a busy turn | `micold-daemon/tests/structured_input.rs::an_interrupt_ends_the_turn_and_the_same_process_runs_on`; `micold-client/tests/features_composer.rs::escape_interrupts_only_a_busy_session`; `micold-client/tests/features_composer.rs::escape_on_a_focused_transcript_interrupts_only_a_busy_session` |
| US3-AS2 | Interrupted turn closes with a notice | `micold-client/tests/gates/composer_fits.rs::an_interrupted_turn_closes_with_its_notice_and_a_message_names_its_pictures` |
| US3-AS3 | No stop control when idle/not running | `micold-client/tests/features_composer.rs::stop_is_offered_only_while_busy_and_stop_session_only_while_overdue`; `micold-client/tests/gates/composer_fits.rs::busy_and_idle_say_so_and_only_busy_offers_stop` |
| US3-AS4 | Unanswered interrupt reported overdue after 10 s | `micold-daemon/tests/structured_input.rs::an_ignored_interrupt_goes_overdue_and_may_be_asked_again`; `micold-client/tests/gates/composer_fits.rs::the_queue_hold_overdue_notice_and_chips_fit_under_the_transcript` |
| US3-AS5 | Two windows interrupting interrupt once | `micold-daemon/tests/structured_input.rs::an_interrupt_that_names_no_open_turn_writes_nothing`; `micold-daemon/tests/structured_input.rs::a_late_interrupt_naming_the_last_turn_holds_the_queue` |
| US3-AS6 | Interrupt withdraws pending permission request | `micold-daemon/tests/structured_input.rs::an_interrupt_withdraws_the_turns_pending_permission_request` |
| US4-AS1 | Busy send is queued at the end | `micold-daemon/tests/structured_input.rs::a_busy_send_is_queued_and_each_result_delivers_one`; `micold-core/tests/structured_input.rs::a_message_sent_while_busy_is_queued_and_nothing_is_delivered` |
| US4-AS2 | A result delivers the first queued message | `micold-core/tests/structured_input.rs::one_turn_takes_one_queued_message_in_order`; `micold-daemon/tests/structured_input.rs::a_busy_send_is_queued_and_each_result_delivers_one` |
| US4-AS3 | Queued message removable, never delivered | `micold-core/tests/structured_input.rs::a_removed_entry_is_never_delivered`; `micold-core/tests/structured_input.rs::an_edit_replaces_the_text_keeps_chosen_pictures_and_the_position`; `micold-client/tests/features_composer.rs::a_queued_message_moves_back_only_into_an_empty_draft` |
| US4-AS4 | Process end/stop holds the queue | `micold-daemon/tests/structured_input.rs::exit_and_stop_hold_the_queue_until_one_release`; `micold-core/tests/structured_input.rs::a_process_end_and_a_restart_hold_the_queue`; `micold-daemon/tests/structured_input_saved.rs::a_saved_queue_survives_a_restart_on_hold` |
| US4-AS5 | Interrupt keeps the queue on hold | `micold-daemon/tests/structured_input.rs::an_interrupt_holds_the_queue_and_its_end_delivers_nothing`; `micold-core/tests/structured_input.rs::an_interrupt_holds_a_queue_and_nothing_goes_out_until_released` |
| US4-AS6 | Both windows see the queue | `micold-daemon/tests/structured_input.rs::queue_edits_name_their_entry`; `micold-daemon/tests/structured_input.rs::two_windows_sending_at_once_deliver_one_and_queue_the_other` |
| US5-AS1 | Dropped image attaches to the draft | `micold-client/tests/features_composer.rs::dropped_paths_are_inserted_at_the_caret_as_a_terminal_drop_plans_them`; `micold-core/src/structured/attachment.rs::a_drop_splits_into_pictures_and_paths_in_drop_order` |
| US5-AS2 | Send with pictures: one user line with images | `micold-daemon/tests/structured_input.rs::a_send_to_an_idle_session_is_one_line_equal_to_its_encoding` |
| US5-AS3 | Non-image file/folder drop inserted as path text | `micold-client/tests/features_composer.rs::dropped_paths_are_inserted_at_the_caret_as_a_terminal_drop_plans_them`; `micold-core/src/structured/attachment.rs::a_file_that_is_no_picture_is_unreadable` |
| US5-AS4 | Removing an attachment before send | `micold-client/tests/features_composer.rs::drafts_pictures_and_carets_are_kept_per_session`; `micold-client/src/ui/material/attachment_chip.rs::the_remove_control_is_a_labelled_keyboard_stop` |
| US5-AS5 | Oversize/unreadable image not attached, named | `micold-client/tests/features_composer.rs::a_too_large_picture_and_a_failed_read_are_not_attached_and_named`; `micold-core/src/structured/attachment.rs::every_refusal_names_the_file_and_says_why` |
| US5-AS6 | Attachments-only draft is sendable | `micold-client/tests/features_composer.rs::a_draft_of_pictures_alone_is_sendable`; `micold-core/tests/structured_input.rs::a_message_of_pictures_only_is_delivered` |
| US5-AS7 | Terminal pane drops unchanged | `micold-client/tests/pane_drop_target.rs::a_file_dropped_on_the_pane_view_is_reported_with_the_pane_under_the_pointer` |
| FR-001 | Composer below the transcript | `micold-client/tests/gates/composer_fits.rs::the_composer_sits_under_a_structured_transcript_in_both_schemes` |
| FR-002 | Enter sends, Shift+Enter new line | `micold-client/tests/features_composer.rs::enter_sends_and_shift_enter_breaks_the_line`; `micold-client/tests/features_composer.rs::no_composer_key_needs_a_platform_modifier` |
| FR-003 | One line per sent draft | `micold-daemon/tests/structured_input.rs::a_send_to_an_idle_session_is_one_line_equal_to_its_encoding` |
| FR-004 | Blank draft not sendable | `micold-core/tests/structured_input.rs::a_blank_message_is_refused_and_the_turn_stays_idle` |
| FR-005 | Refusal shown, text kept | `micold-client/tests/features_composer.rs::a_refused_send_comes_back_unchanged_with_the_reason` |
| FR-006 | Composer disabled when not running/no connection | `micold-client/tests/features_composer.rs::can_send_needs_a_sendable_draft_a_running_session_a_connection_and_no_pending_send` |
| FR-007 | Draft kept per window per session | `micold-client/tests/features_composer.rs::drafts_pictures_and_carets_are_kept_per_session` |
| FR-008 | Composer grows to a limit then scrolls | `micold-client/tests/gates/composer_fits.rs::a_long_draft_grows_the_composer_to_its_limit_and_the_transcript_stays` |
| FR-009 | Idle/busy same in every window | `micold-daemon/tests/structured_input.rs::every_window_sees_busy_then_idle_within_a_second_of_the_result` |
| FR-010 | Busy shown by label and indicator | `micold-client/tests/gates/composer_fits.rs::busy_and_idle_say_so_and_only_busy_offers_stop` |
| FR-011 | Busy marked wherever sessions are listed | `micold-daemon/tests/structured_input.rs::the_sessions_activity_follows_the_turn` (the summary's activity, which the sidebar and overview draw as for a terminal session) |
| FR-012 | Stop control and interrupt key while busy | `micold-client/tests/features_composer.rs::escape_interrupts_only_a_busy_session`; `micold-client/tests/features_composer.rs::stop_is_offered_only_while_busy_and_stop_session_only_while_overdue` |
| FR-013 | Interrupt cancels turn, session stays running | `micold-daemon/tests/structured_input.rs::an_interrupt_ends_the_turn_and_the_same_process_runs_on` |
| FR-014 | Interrupted turn closes with a notice | `micold-client/tests/gates/composer_fits.rs::an_interrupted_turn_closes_with_its_notice_and_a_message_names_its_pictures` |
| FR-015 | 10 s unanswered interrupt reported | `micold-daemon/tests/structured_input.rs::an_ignored_interrupt_goes_overdue_and_may_be_asked_again` |
| FR-016 | Several clients interrupt once | `micold-daemon/tests/structured_input.rs::an_interrupt_that_names_no_open_turn_writes_nothing`; `micold-daemon/tests/structured_input.rs::an_interrupt_asked_before_its_message_is_written_follows_it` |
| FR-017 | Terminal Ctrl-C interrupt unchanged | `micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_interrupt_session_types_ctrl_c_and_leaves_it_running` (unchanged by 941) |
| FR-018 | Busy send queued at end | `micold-core/tests/structured_input.rs::a_message_sent_while_busy_is_queued_and_nothing_is_delivered` |
| FR-019 | Result delivers first queued message | `micold-core/tests/structured_input.rs::one_turn_takes_one_queued_message_in_order`; `micold-core/tests/structured_input.rs::a_result_never_holds_the_queue` |
| FR-020 | Queued message removable and editable | `micold-core/tests/structured_input.rs::a_removed_entry_is_never_delivered`; `micold-core/tests/structured_input.rs::an_edit_replaces_the_text_keeps_chosen_pictures_and_the_position`; `micold-core/tests/structured_input.rs::an_edit_leaving_nothing_to_send_is_refused` |
| FR-021 | Queue is the session's, held by the service | `micold-core/tests/structured_input.rs::two_sessions_never_share_a_queue`; `micold-daemon/tests/structured_input.rs::one_sessions_input_never_reaches_another`; `micold-daemon/tests/structured_input.rs::a_deleted_session_drops_its_input_state` |
| FR-022 | Queue kept and shown on hold when not running | `micold-core/tests/structured_input.rs::a_restored_queue_is_held_with_fresh_ids`; `micold-daemon/tests/structured_input_saved.rs::a_saved_queue_survives_a_restart_on_hold`; `micold-daemon/tests/structured_input_saved.rs::a_damaged_queue_file_restores_an_empty_queue_and_the_events_load`; `micold-client/tests/features_composer.rs::send_queued_is_offered_only_for_a_held_queue_of_a_running_session` |
| FR-023 | History-back/forward keys on first/last line | `micold-client/tests/features_composer.rs::up_steps_back_only_on_the_first_line_and_down_forward_only_on_the_last` |
| FR-024 | Forward past newest restores draft | `micold-client/tests/features_composer.rs::stepping_forward_past_the_newest_restores_the_draft_unchanged` |
| FR-025 | History holds the transcript's user messages | `micold-client/tests/features_composer.rs::history_back_recalls_newest_first_down_to_the_oldest` |
| FR-026 | Drops go to the session's composer | `micold-client/tests/pane_drop_target.rs::a_pane_showing_a_structured_primary_is_the_composers_drop_target` |
| FR-027 | PNG/JPEG/GIF/WebP attach as pictures | `micold-core/src/structured/attachment.rs::sniff_knows_the_four_formats_by_their_magic_bytes`; `micold-core/src/structured/attachment.rs::a_drop_splits_into_pictures_and_paths_in_drop_order` |
| FR-028 | Other files/folders inserted as text | `micold-client/tests/features_composer.rs::dropped_paths_are_inserted_at_the_caret_as_a_terminal_drop_plans_them` |
| FR-029 | Unreadable/oversize/too many images refused and named | `micold-core/src/structured/attachment.rs::the_twenty_first_picture_is_too_many`; `micold-core/src/structured/attachment.rs::a_picture_of_exactly_the_limit_is_admitted_one_byte_more_is_too_large`; `micold-core/src/structured/attachment.rs::pictures_over_the_message_total_are_refused`; `micold-client/tests/features_composer.rs::twenty_two_pictures_attach_the_first_twenty_and_name_the_rest` |
| FR-030 | Transcript shows an attachments line | `micold-client/tests/gates/composer_fits.rs::an_interrupted_turn_closes_with_its_notice_and_a_message_names_its_pictures` |
| FR-031 | Per-session state kept apart | `micold-daemon/tests/structured_input.rs::one_sessions_input_never_reaches_another`; `micold-core/tests/structured_input.rs::two_sessions_never_share_a_queue` |
| FR-032 | Simultaneous sends: one delivered, one queued | `micold-daemon/tests/structured_input.rs::two_windows_sending_at_once_deliver_one_and_queue_the_other`; `micold-core/tests/structured_input.rs::two_releases_with_their_pumps_deliver_once` |
| FR-033 | User guide describes the composer | docs/user-guide/worktrees-and-sessions.md (Structured sessions > Writing to a structured session); docs, no test |
| SC-001 | Every send case delivers exactly one line | `micold-daemon/tests/structured_input.rs::a_send_to_an_idle_session_is_one_line_equal_to_its_encoding` |
| SC-002 | Busy/idle in every window within a second | `micold-daemon/tests/structured_input.rs::every_window_sees_busy_then_idle_within_a_second_of_the_result` |
| SC-003 | Interrupt ends turn, session stays running and idle | `micold-daemon/tests/structured_input.rs::an_interrupt_ends_the_turn_and_the_same_process_runs_on` |
| SC-004 | Queue cases lose or duplicate no message | `micold-core/tests/structured_input.rs::one_turn_takes_one_queued_message_in_order`; `micold-daemon/tests/structured_input.rs::a_busy_send_is_queued_and_each_result_delivers_one`; `micold-daemon/tests/structured_input.rs::a_failed_delivery_returns_to_the_head_held` |
| SC-005 | Refusal/lost connection keep text and attachments | `micold-client/tests/features_composer.rs::a_refused_send_comes_back_unchanged_with_the_reason`; `micold-client/tests/features_composer.rs::a_lost_connection_restores_every_pending_draft` |
| SC-006 | Recall and resend in two key presses | `micold-client/tests/features_composer.rs::up_then_enter_resends_the_previous_message` |
| SC-007 | 10,000-line draft responsive (ignored by default) | `micold-client/tests/composer_large_draft.rs::every_keystroke_into_a_ten_thousand_line_draft_takes_under_100_ms` |
| SC-008 | Terminal-session tests unchanged | `micold-client/tests/gates/composer_fits.rs::a_terminal_session_has_no_composer`; `micold-client/tests/pane_drop_target.rs::a_file_dropped_on_the_pane_view_is_reported_with_the_pane_under_the_pointer` |

Untested: none. SC-007 is `#[ignore]`d (release timing, run by hand: quickstart Part A).
