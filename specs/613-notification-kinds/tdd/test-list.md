---
feature: 613-notification-kinds
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 29 # US1 1-11, US2 1-13, US3 1-5
planned_at: 0c2b4a1d
updated_at: 2026-10-07
suite_baseline: green # see cycle-log.md; written after the fact (T070, verification finding 4)
---

# Test List: Notification kinds, per-kind settings and icons

Written at close (T070), after the loop ran from the test-first order of `tasks.md` (see the
deviation note at the top of `cycle-log.md`). It maps each acceptance scenario and each test task
to its tests, so `traces` and the `[X]`-against-`DONE` check can be done mechanically. Ids `A…` and
`U…` are behavior ids of this file only; the contracts' `C…` and `T…` ids are not used here.

The outer loop is headless integration tests in two halves, as in 039: the **service half**
(`crates/micold-daemon/tests/attention_claims.rs`, `attention_error_notice.rs`,
`settings_notification_kinds.rs`, `settings_long_task_threshold.rs`) and the **window half**
(`crates/micold-client/tests/attention_notify.rs`, `features_settings.rs`, `notification_icon.rs`,
the `layout_snapshot` gates, and the dispatch tests in `src/shell/daemon_sync.rs`). The real
notification on a real desktop is quickstart §B (visual pass, `visual-pass.md`).

## Outer loop: acceptance behaviors

| id | behavior | traces | kind | state | tests (tasks.md) |
| --- | --- | --- | --- | --- | --- |
| A1 | A short turn of a session not in view raises no notification; the session still becomes unread | US1-1, FR-005, FR-018, SC-001 | example | DONE | T005, T010, T073 |
| A2 | A turn at or above the threshold raises one Long task finished notification | US1-2, FR-001, FR-005 | example | DONE | T005, T010, T011, T067, T071 |
| A3 | A mid-turn ask for permission or an answer raises one Needs permission notification | US1-3, FR-002 | example | DONE | T005, T010, T011, T067 |
| A4 | An error ending (give-up, Copilot `session.error`) raises one Session error notification in one window | US1-4, FR-004, FR-007 | example | DONE | T021, T022, T024, T025, T067 |
| A5 | An ending without error (user stop, normal exit) raises no notification | US1-5, FR-004 | example | DONE | T021, T024, T074 |
| A6 | A session in view raises no notification of any kind | US1-6, FR-006 | example | DONE | T009, T010 |
| A7 | After an answered permission the turn's kind is decided by the whole turn duration | US1-7, FR-003 | example | DONE | T005, T010 |
| A8 | A repeated waiting signal with no work in between raises nothing more | US1-8, FR-006 | example | DONE | T009, T010 |
| A9 | Claude Code, Copilot and Pi long turns each raise one Long task finished | US1-9, FR-014 | example | DONE | T008, T010 |
| A10 | With no window open nothing is notified, then or later | US1-10, FR-006 | example | DONE | T010, T022, T024 |
| A11 | A helper agent finishing inside a turn (`SubagentStop`) is ignored | US1-11, FR-024 | example | DONE | T008 |
| A12 | Settings shows one switch per kind under Desktop notifications, in FR-009 order, with its icon | US2-1, FR-009, FR-022 | example | DONE | T001, T034, T036, T075 |
| A13 | Turn finished, when on, notifies a short turn | US2-2, FR-005 | example | DONE | T033 |
| A14 | A kind switched off notifies nothing for that kind; unread changes as with it on | US2-3, FR-005, FR-018, SC-002, SC-003 | example | DONE | T009, T033, T068 |
| A15 | A switch changed while sessions run applies to the next event; no backlog | US2-4, FR-013 | example | DONE | T033 |
| A16 | With Desktop notifications off the kind switches keep their positions, are disabled, and nothing notifies | US2-5, FR-012 | example | DONE | T035, T036, T077 |
| A17 | Turning Desktop notifications back on restores the kind switches' decisions | US2-6, FR-012 | example | DONE | T033 |
| A18 | Kind switches and threshold survive a restart | US2-7, FR-011, SC-006 | example | DONE | T002, T033, T058 |
| A19 | A settings file from before this feature keeps Desktop notifications and takes default kinds | US2-8, FR-010 | example | DONE | T002 |
| A20 | Kind switches apply alike to every AI CLI; there is no switch per AI CLI | US2-9, FR-014 | example | DONE | T033, T035 |
| A21 | The threshold field shows 60 seconds with its range, next to Long task finished | US2-10, FR-025, FR-022 | example | DONE | T056, T060, T075 |
| A22 | A threshold of 20 s makes a 30 s turn Long task finished and a 15 s turn Turn finished | US2-11, FR-025, SC-008 | example | DONE | T058, T059 |
| A23 | A threshold below 10, above 3600 or not whole is refused on save with the range named | US2-12, FR-026 | example | DONE | T058, T060 |
| A24 | A missing threshold loads as 60 and an out-of-range one as the nearest bound | US2-13, FR-010, FR-025 | example | DONE | T056 |
| A25 | Each kind has its own icon and no two kinds share one | US3-1, FR-015, FR-017 | example | DONE | T034, T046 |
| A26 | A notification's icon is the glyph shown beside its kind's switch | US3-2, FR-015 | example | DONE | T034, T036, T046 |
| A27 | Each notification's title says its kind in words | US3-3, FR-008 | example | DONE | T006, T011, T025 |
| A28 | Where the OS shows no app icon the notification still appears and its title tells the kind | US3-4, FR-016, FR-020 | example | DONE | T047, T006 |
| A29 | Each kind icon is recognisable on the light and the dark theme | US3-5, FR-017 | example | DONE | T046 |

## Inner loop: unit behaviors

| id | behavior | traces | state | tests (tasks.md) |
| --- | --- | --- | --- | --- |
| U1 | `NotificationKind::ALL` order, names and defaults | A12, A27 | DONE | T001 |
| U2 | Settings load: defaults, pre-feature files, unreadable file | A18, A19 | DONE | T002 |
| U3 | `TurnClock` transition table and the kind each cell returns | A1, A2, A3, A7 | DONE | T005 |
| U4 | `notification_text` titles and bodies per kind | A27, A28 | DONE | T006 |
| U5 | `AttentionGranted.kind` and `SessionErrorNotice` on the wire | A2, A4 | DONE | T007, T023 |
| U6 | `SubagentStop` classified ignored and not registered | A9, A11 | DONE | T008 |
| U7 | `Views` pending kinds, grants and off-kind events used up | A6, A8, A14 | DONE | T009, T068 |
| U8 | Copilot `session.error` sets `error: true`; every other end `false` | A4, A5 | DONE | T021 |
| U9 | `error_notice_target` picks one window, or none | A4, A10 | DONE | T022 |
| U10 | Client builds the notification from the grant's kind; dispatch honours the kind | A2, A3, A4, A27 | DONE | T011, T025, T067 |
| U11 | `notification_kinds` and `long_task_threshold_secs` on the wire | A15, A18 | DONE | T032, T057 |
| U12 | Kind icons in `Icon::ALL` with their codepoints | A12, A25 | DONE | T034 |
| U13 | Settings draft: kind toggles and threshold text, save and refusal | A16, A20, A21, A23 | DONE | T035, T060 |
| U14 | `Checkbox::icon` keeps label, state and toggle | A12, A26 | DONE | T036 |
| U15 | Threshold bounds and clamping in core settings | A21, A24 | DONE | T056 |
| U16 | Threshold edges at 10 s, 60 s and 3600 s; a turn keeps the threshold it started under | A22 | DONE | T059 |
| U17 | Rasteriser size, tile luminance and per-kind distinctness | A25, A26, A29 | DONE | T046 |
| U18 | Linux `image-data` hint, Windows/macOS PNG per kind | A28 | DONE | T047 |
| U19 | Kind rows and threshold field sit under the switch, in `ALL` order, and take input only with the master switch on | A12, A16, A21 | DONE | T075, T077, T079 |

## Verification commands

From `.specify/memory/tdd-profile.md`: single test `scripts/build-lock.sh cargo test -p <crate>
--test <binary> <name>`; full suite `cargo test --workspace`.

## Source-text scans (T078)

Two tests in `crates/micold-client/tests/settings_sections.rs` read `ui/settings/environment.rs` as
text. Each behavior they name is also asserted on the rendered page, so the scans are kept only as
cheap wiring checks, not as the evidence for any behavior above:

| scan | what it reads | rendered behavior that stands for it |
| --- | --- | --- |
| `the_desktop_notifications_section_lists_the_four_kind_rows` | the rows' builder calls, master switch before `NotificationKind::ALL`, no per-CLI toggle | A12, A20: `the_kind_rows_and_the_threshold_sit_indented_under_the_master_switch` (five rows directly under the switch) and `the_kind_rows_and_the_threshold_take_input_only_while_the_master_switch_is_on` (row *i* toggles `ALL[i]`) |
| `the_threshold_field_sits_under_the_long_task_row` | label and supporting text of the field after the Long task finished row | A21: the geometry gate's fourth row is the taller threshold field; the input test types into it |

The scan that stood for A16 (`the_kind_rows_are_disabled_while_the_master_switch_is_off`) was
deleted in T077: A16 is the input test above.
