# TDD test list — 890 daemon-settings-accordion

Written by the M1 unit: the design PR left no list, and T001/T002 already name one behaviour per
lettered case. M2 and M3 add their own rows.

| ID | Kind | Behaviour | Trace | Task | State | Test |
|---|---|---|---|---|---|---|
| U1 | unit | Closed with a summary paints it under the title, caption role, muted tint, both themes | FR-011, US3-1 | T001 a, g | DONE | `accordion_section::tests::a_closed_section_paints_its_summary_under_the_title` |
| U2 | unit | Open with a summary paints no summary | FR-011, US3-1 | T001 b | DONE | `accordion_section::tests::an_open_section_paints_no_summary` |
| U3 | unit | A narrow header ellipsizes the summary; the chevron stays inside | FR-011 | T001 c | DONE | `accordion_section::tests::a_narrow_header_ellipsizes_the_summary_and_keeps_the_chevron_inside` |
| U4 | unit | Closed and required marks the title in its role and tint; open does not; both themes | FR-012, US3-3 | T001 d, g | DONE | `accordion_section::tests::a_closed_required_section_marks_its_title` |
| U5 | unit | Without summary (marker or not) the header height is today's 40dp | FR-013, US3-2 | T001 e | DONE | `accordion_section::tests::a_plain_header_is_as_tall_as_today` |
| U6 | unit | Summary + marker keep one toggle per press/Space/Enter, no focus stop in a closed body | FR-013, SC-005 | T001 f | DONE | `accordion_section::tests::a_summary_and_marker_leave_the_headers_behaviour_alone` |
| U7 | unit | The showcase entry poses closed with summary, open, closed required, live toggle | FR-015, US3-4 | T002 | DONE | `tests/showcase_completeness.rs::the_accordion_section_poses_its_summary_and_marker` |
| U8 | unit | An empty summary is drawn as none | FR-013 | T003 | DONE | `accordion_section::tests::an_empty_summary_is_drawn_as_none` |
| M2-1 | unit | `Section::of`, `title`, `holding_error` and `holding` give each kind's FR-002 sections and map every field to one | FR-002, FR-003 | T006 | DONE | `tests/features_daemon_sections.rs` (`each_kind_lists_its_sections_in_order`, `one_concept_has_one_title_and_limits_is_gone`, `every_refusable_field_has_its_section`, `every_required_or_marked_control_has_its_section`) |
| M2-2 | acceptance | A form opens on its first section only (add, edit, kind change); a toggle touches neither draft nor registry; discard and reopen forgets flips | FR-004, US1-2 | T007 | DONE | `tests/daemons_feature.rs::sections_open_by_toggle_and_by_error_without_touching_the_draft`, `every_kind_opens_on_connection_only_and_a_kind_change_resets` |
| M2-3 | acceptance | A refused save or a required field left empty opens its section, in every kind; a click on a held-open section is not recorded; `allowed` opens Credentials | FR-004, FR-005 | T007, review A | DONE | `tests/daemons_feature.rs::a_refused_field_opens_its_section_in_every_kind`, `a_click_on_a_section_an_error_holds_open_is_not_recorded`, `the_allowed_credentials_refusal_opens_credentials` |
| M2-4 | acceptance | Each kind paints its titles in order with only Connection's body; no group heading | FR-002, FR-003 | T008 | DONE | `tests/daemons_section_geometry.rs` (`the_container_form_paints_six_titles_with_only_connection_open`, `every_kind_paints_name_runs_in_its_sections_in_order_and_the_actions`, `a_closed_connection_paints_none_of_its_fields`) |
| M2-5 | acceptance | The Kubernetes add form fits 800 px at the canonical width | SC-002 | T009 | DONE | `tests/gates/daemon_form_fits.rs` (red measured afterwards, see cycle-log) |
| M3-1 | unit | `FieldGroup` stacks label, note and children at one left edge, both themes | FR-008 | T016, T021 | DONE | `field_group::tests` (3), `showcase_completeness::the_field_group_is_listed_with_and_without_a_note` |
| M3-2 | unit | Every summary row and fallback of the contract; the marker per required field | FR-014 | T017, T023 | DONE | `tests/features_daemon_sections.rs` (`container_*`, `kubernetes_sections`, `ssh_and_host`, `the_marker_follows_the_required_fields`) |
| M3-3 | acceptance | One field gap and one section gap across the kinds; group labels share the fields' left edge; context select above *Context*; summaries and `Connection*` painted | SC-003, FR-008, FR-009 | T018, T020, T024–T026 | DONE | `tests/daemons_section_geometry.rs` (`each_group_label_sits_above…`, `explanations_sit_where…`, `found_contexts_are_a_select…`, `field_and_section_gaps_take_one_value_across_kinds`, `closed_headers_paint_their_summary…`) |
