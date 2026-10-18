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
