# TDD cycle log: M4 (#1092)

T034 accepted: M1-M3 were not run through the TDD extension, so they have no red phase on record.

| Task | Test | Red | Green |
| --- | --- | --- | --- |
| T039 | `features_daemons drift_behind_the_form::a_drift_prompt_raised_while_details_are_open_waits_for_them_to_close` | Failed before the fix: `no second modal over Details` (`features_daemons.rs:1309`, prompt shown with Details open). | `show_next_drift` holds while `details` is set and `DetailsClosed` calls it. |
| T031 | `daemon_dialog_buttons` (form, details, page) | The behaviour already existed, so no red against the code. Proven by mutants instead: Save daemon sending `DraftDiscarded`, Edit sending `AddOpened`, Details sending `EditOpened`, Close sending `AddOpened` each fail it. | passes |
| T032 | `daemons_section_geometry::the_details_dialog_paints_the_errors_and_the_log_...` | Mutants: dropping the `for error` loop and the `for line` loop fail it. | passes |
| T033 | `daemon_dialog_buttons::clicking_the_scrim_*` | Mutant: removing `modal.on_dismiss(cancel)` fails the details scrim test (the form's also asserts the discard). | passes |
| T035 | `layout_snapshot daemon_form_fits` | The absolute bottom is 2336 in the 4000 px window the gate forces (the dialog is centred), so it asserts the dialog sits inside that window. | passes |
| T036 | `shell::daemons::tests::a_closed_dialog_returns_...`, `daemon_dialog_buttons::the_add_and_row_buttons_carry_the_ids_...` | Mutants: Details id equal to Edit's, and the focus return dropped, fail them. | passes |
| T037 | `daemons_section_geometry::the_rows_keep_their_status_and_actions_...` | Mutant: renaming the Stop button fails it. | passes |
| T038 | `daemons_feature::add_dialog::a_daemon_saved_from_the_dialog_survives_a_restart` | Pure addition over existing behaviour. | passes |
