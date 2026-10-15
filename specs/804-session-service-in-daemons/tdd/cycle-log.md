# TDD cycle log: 804 M1

Honest record. The M1 unit hit its context cap mid-implementation and the tests (T001, T003,
T005-T007) were written in the same pass as the code, then committed together in one WIP commit.
No red run was captured for them, so test-first is not shown by git history.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `Settings::sync_legacy_daemon` keeps the legacy daemon in step | none captured | `cargo test -p micold-core` |
| T003 | `parse_budget` and limit text | none captured | `features_settings` |
| T005 | profile fields round trip through `save`; bad limit names its field | none captured | `daemons_feature::profile_fields_round_trip_through_save`, `a_bad_limit_or_archive_names_its_field_and_changes_nothing` |
| T006 | two daemons edit independently; running daemon asks first | none captured | `two_daemons_edit_independently_and_drafts_are_kept_apart`, `a_running_daemon_asks_before_new_container_settings_apply` |
| T007 | SSH shows no container profile; switch drops it | none captured | `an_ssh_entry_shows_no_container_profile_and_switching_to_ssh_drops_it` |

Later milestones (M2, M3) write red before green and record it here.
