---
feature: 039-session-attention-notifications
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 48 # US1 1-13, US2 1-23, US3 1-6, US4 1-6
planned_at: a8521731
updated_at: 2026-10-04
suite_baseline: green # 4352 passed, 0 failed of the tests that existed, 9 ignored, 384 binaries at 6b4b6fa9; see cycle-log.md
---

# Test List: Notify When a Session Needs Attention, and Track Unread Sessions

Ids in this file (`A1`…, `U1`…) are **behavior ids**. In `tasks.md` they appear in square brackets
(`[A1]`, `[U69]`). The contracts have ids of their own that look alike: `A1`–`A6` in
`data-model.md`, `U1`–`U8` in `contracts/unread-mark.md`, `W…` and `N…` in the other two contracts.
`tasks.md` writes those in round brackets, and this file never uses them.

## Outer loop: acceptance behaviors

The profile's acceptance runner (`sandbox_real_*`) drives the container runtime and cannot see a
desktop notification, a sidebar row or the switcher. The outer loop is therefore **integration
tests**, headless, in two halves that meet at the catalog snapshot:

- **Service half**, `crates/micold-daemon/tests/` with two connections: an activity signal goes in,
  `attention_seq`, `unread`, `AttentionGranted`, `RevealSession` and `SettingsChanged` come out
  (`attention_events.rs`, `attention_claims.rs`, `unread_state.rs`, `session_reveal.rs`,
  `settings_desktop_notifications.rs`).
- **Window half**, `crates/micold-client/tests/` over the real reducers with a recording
  `DesktopNotifier`: a snapshot or a grant goes in, a view report, a claim, a call to `show`, a
  selection message or a count comes out (`attention_view_report.rs`, `attention_notify.rs`,
  `unread_rows.rs`, `switcher_unread.rs`, `attention_reveal.rs`).

No automated test joins the two halves in one process. That is weaker than an end-to-end test: the
real notification on a real desktop, the click, and the session service in a container are
quickstart §B (`visual-pass`, T115) and §C (T116). A "no notification" scenario is asserted as "the
service raises no attention event" or "the window makes no claim and calls `show` zero times".

The `tests` column names the task that writes the outer test; that task carries the `[A…]` marker.
The `final` column names the story's final run task, which must be green before the story is
complete. T119 to T121 carry their story's `[A…]` markers. T118 carries none: A1 to A13 are all
done by the end of M2, and T118 is the run of those tests on CI's macOS and Windows legs in M3,
ticked by `speckit-implement`.

| id | behavior | traces | kind | state | tests (tasks.md) | final |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | With A in view and B working, B's change to awaiting input is granted to one window, which shows exactly one notification naming B's project, worktree and row label | US1-1, FR-001, FR-004 | example | DONE | T021, T022 | T118 |
| A2 | A's change to awaiting input while a window reports A in view raises no attention event | US1-2, FR-002 | example | DONE | T005 | T118 |
| A3 | A selected in a window that lost keyboard focus is reported as not in view, and its change raises one attention event | US1-3, FR-001 | example | DONE | T005, T006 | T118 |
| A4 | A granted session of a project that is not the active one is shown with its own project's name, its worktree and its label | US1-4, FR-004 | example | DONE | T022 | T118 |
| A5 | A repeated waiting signal for a session already awaiting input raises no further attention event | US1-5, FR-003 | example | DONE | T005 | T118 |
| A6 | A session that worked again and changes to awaiting input a second time raises one more attention event | US1-6, FR-003 | example | DONE | T005 | T118 |
| A7 | A session of the Default entry is shown with the project, the Default entry's sidebar name and the session | US1-7, FR-004 | example | DONE | T022 | T118 |
| A8 | A claim sent over the window's existing connection is granted over that connection, and nothing else reaches the service: the path names no runtime (the guard is U49; the real container is quickstart §C3) | US1-8, FR-007, SC-007 | example | DONE | T021 | T118 |
| A9 | With Settings filling the main area the window reports nothing in view, so the selected session's change raises one attention event | US1-9, FR-001 | example | DONE | T006 | T118 |
| A10 | A selected session in a focused window is in view whichever of its tabs is shown, so its change raises nothing | US1-10, FR-002 | example | DONE | T001, T006 | T118 |
| A11 | After a reconnect, a session last seen working and found awaiting input with a higher sequence is claimed once and shown once | US1-11, FR-006 | example | DONE | T022 | T118 |
| A12 | Sessions that change at the same moment are each granted once, each grant naming its own session | US1-12, FR-009, SC-005 | example | DONE | T021 | T118 |
| A13 | A change that happened with no window open is never claimed: the first snapshot a window receives yields no claim and no call to `show` | US1-13, FR-008, FR-005 | example | DONE | T022 | T118 |
| A14 | A session that changes to awaiting input while not in view is `unread` in the snapshot every window receives, and its row is marked | US2-1, FR-016, FR-018 | example | DONE | T048, T051 | T119 |
| A15 | A session that changes while a window reports it in view is not `unread` | US2-2, FR-016 | example | DONE | T048 | T119 |
| A16 | When the user brings an unread session into view, its project's unread count falls by one at once | US2-3, FR-019, FR-021 | example | DONE | T063 | T119 |
| A17 | With two unread sessions in Q and none in P, Q's switcher row carries `● 2 unread` and P's row carries no unread count | US2-4, FR-021 | example | DONE | T063, T064 | T119 |
| A18 | The active project's switcher row carries its unread count like any other | US2-5, FR-021 | example | DONE | T063 | T119 |
| A19 | A session selected in an unfocused window becomes unread on its change, and the report sent when the window regains focus clears it | US2-6, FR-019 | example | DONE | T048 | T119 |
| A20 | A report naming a background project's unread session, sent when the user switches to that project, clears it | US2-7, FR-019 | example | DONE | T048 | T119 |
| A21 | An unread session that starts working again stays unread | US2-8, FR-020 | example | DONE | T048 | T119 |
| A22 | An unread session that is removed is in no later snapshot, so no row and no count includes it | US2-9, FR-020 | example | DONE | T048 | T119 |
| A23 | With desktop notifications off, a change while not in view sets `unread` exactly as with them on | US2-10, FR-017, SC-003 | example | DONE | T104 | T121 |
| A24 | An unread session selected in a focused window behind Settings is cleared by the report sent when Settings closes | US2-11, FR-019 | example | DONE | T048 | T119 |
| A25 | A session whose window shows one of its regular terminal tabs is reported in view and does not become unread | US2-12, FR-016 | example | DONE | T048 | T119 |
| A26 | A session whose window lost its connection is in view nowhere, so its change to awaiting input sets `unread` | US2-13, FR-006 | example | DONE | T048 | T119 |
| A27 | A project's unread count includes a session of a worktree the sidebar's filter hides | US2-14, FR-022 | example | DONE | T062 | T119 |
| A28 | With P active, two unread in Q and one in R, the switcher's button carries `● 3` | US2-15, FR-023, SC-010 | example | DONE | T063, T065 | T119 |
| A29 | With the only unread session in the active project, the switcher's button carries no unread count | US2-16, FR-023 | example | DONE | T063 | T119 |
| A30 | After a switch to Q the button's total counts R's unread session only, whether or not Q's were read | US2-17, FR-023 | example | DONE | T062, T063 | T119 |
| A31 | A change with no connection sets `unread`, and the first snapshot a later window receives carries it | US2-18, FR-008 | example | DONE | T048 | T119 |
| A32 | `unread: true` survives a restart of the session service | US2-19, FR-008a | example | DONE | T048 | T119 |
| A33 | A read session awaiting input whose activity did not change is not unread after a restart of the service | US2-20, FR-008a | example | DONE | T048 | T119 |
| A34 | A read session that works and changes to awaiting input again with no connection is unread | US2-21, FR-008 | example | DONE | T048 | T119 |
| A35 | A session that became unread with no connection and then works again is still unread | US2-22, FR-008, FR-020 | example | DONE | T048 | T119 |
| A36 | The report a window sends after `Welcome`, naming the unread session it selected, clears it, and the row hides the mark at once | US2-23, FR-019 | example | DONE | T048, T051 | T119 |
| A37 | A click on a notification for a session of the active project raises the window and selects that session | US3-1, FR-011 | example | DONE | T077 | T120 |
| A38 | A click for a session of a background project raises the window, reopens that project, then selects the session | US3-2, FR-011 | example | DONE | T077 | T120 |
| A39 | After a click has shown the session, its row carries no unread mark | US3-3, FR-019 | example | DONE | T077 | T120 |
| A40 | A click for a session that no longer exists raises the window, changes no selection and pushes `That session is no longer available.` | US3-4, FR-013 | example | DONE | T077 | T120 |
| A41 | A click changes no session and no attachment in the service, and the window sends nothing but the two selection messages | US3-5, FR-014 | example | DONE | T076, T077 | T120 |
| A42 | With two windows, the reveal is sent to the one attached to the session's project and to no other | US3-6, FR-012 | example | DONE | T076 | T120 |
| A43 | On default settings the Settings draft holds **Desktop notifications** and it is on | US4-1, FR-026 | example | DONE | T101, T105 | T121 |
| A44 | While the setting is off no claim is granted, for a session in view or not | US4-2, FR-027 | example | DONE | T104 | T121 |
| A45 | Turning the setting off while sessions run reaches every connection, and the next claim is refused with no restart | US4-3, FR-027 | example | DONE | T104 | T121 |
| A46 | `desktop_notifications: false` survives a restart of the service | US4-4, FR-026 | example | DONE | T104 | T121 |
| A47 | After the setting is turned on the next event is granted, and events made while it was off are never granted | US4-5, FR-027 | example | DONE | T104 | T121 |
| A48 | The settings hold one notification field and none per AI CLI, and the grant rule is the same for a session of each of the three | US4-6, FR-028 | example | DONE | T101, T104 | T121 |
| A49 | A page opened before another window turned both switches off, saved after changing another setting, sends `None` for both switches, leaves them off in the store and in the window, and changes the other setting (BUG-570) | US4-7, FR-026a | example | DONE | T127 | T129 |
| A50 | An open Settings page shows another window's save in every field its user has not edited, and keeps the field they edited (BUG-475) | US4-8, FR-026b | example | DONE | T137 | T138 |

## Inner loop: unit behaviors

Grouped by the component from `plan.md` that owns them. States were brought up to date from `cycle-log.md` at close
(2026-10-04): all `DONE`; `verification.md` lists the ten with no recorded red. The `tasks` column gives the test task first, then the
implementation task(s).

### `crates/micold-core/src/attention.rs`: `in_view`

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U1 | A focused window whose main area is not taken has its selected session in view | FR-002, FR-016 | example | DONE | T001 / T007 |
| U2 | An unfocused window has no session in view | US1-3, US2-6 | example | DONE | T001 / T007 |
| U3 | A window whose main area is taken has no session in view | US1-9, US2-11 | example | DONE | T001 / T007 |
| U4 | A window with no selected session has no session in view | FR-016 | example | DONE | T001 / T007 |
| U5 | The result is the same whichever tab of the session is shown: `ViewFacts` has no field for it | US1-10, US2-12 | example | DONE | T001 / T007 |

### `crates/micold-core/src/session.rs`, `store.rs`: stored fields

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U6 | A `StoredSession` written without `attention_seq` reads as `0` | FR-008a | example | DONE | T002 / T008 |
| U7 | A store round trip keeps `attention_seq` | FR-008a | example | DONE | T002 / T008 |
| U8 | `schema_version` is the same with and without the new fields | FR-008a | example | DONE | T002 / T008 |
| U9 | A `StoredSession` written without `unread` reads as `false` | FR-008a, Edge: first start with this feature | example | DONE | T045 / T052 |
| U10 | A store round trip keeps `unread: true` | FR-008a, US2-19 | example | DONE | T045 / T052 |
| U11 | `unread` is written to the catalog file and to no other file of the store | FR-025 | example | DONE | T045 / T052 |
| U175 | After a load that recovered from a store file it could not parse, no session is unread and the load reports nothing it did not report before | FR-008a, Edge: stored unread state unreadable | characterization | DONE | T045 / (none) |

### `crates/micold-core/src/protocol/`: the wire

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U12 | `ClientMsg::WindowView` and `SessionSummary::attention_seq` encode and decode at version 21 | FR-002, FR-016 | example | DONE | T003 / T009 |
| U13 | `ClientMsg::AttentionClaim` and `DaemonMsg::AttentionGranted` encode and decode at version 23 | FR-001, FR-006a | example | DONE | T019 / T026 |
| U14 | `SessionSummary::unread` encodes and decodes at version 24 | FR-016, FR-024 | example | DONE | T046 / T053 |
| U15 | `ClientMsg::SessionReveal` and `DaemonMsg::RevealSession` encode and decode at version 25 | FR-011, FR-012 | example | DONE | T074 / T082 |
| U16 | `activation: Option<String>` on both reveal messages encodes and decodes at version 26 | FR-011 | example | DONE | T092 / T097 |
| U17 | `DaemonSettings::desktop_notifications` and `SettingsSet::desktop_notifications` encode and decode at version 27 | FR-026, FR-027 | example | DONE | T102 / T107 |

### `crates/micold-core/src/attention.rs`: `AttentionTracker::observe`

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U18 | A session not seen before is adopted with no claim, also when it is `AwaitingInput` | FR-005, US1-13 | example | DONE | T017 / T024 |
| U19 | A higher `attention_seq` in `Phase::Live` yields one claim for that sequence | FR-001 | example | DONE | T017 / T024 |
| U20 | The same sequence observed again yields no claim | FR-003, FR-006a | example | DONE | T017 / T024 |
| U21 | In `Phase::Reconnected`, a session last seen not awaiting, now awaiting, with a higher sequence and not in view, yields one claim | FR-006, US1-11 | example | DONE | T017 / T024 |
| U22 | In `Phase::Reconnected`, a session last seen awaiting input yields no claim and its sequence is adopted | FR-006 | example | DONE | T017 / T024 |
| U23 | In `Phase::Reconnected`, a session that is in view yields no claim | FR-006, FR-002 | example | DONE | T017 / T024 |
| U24 | In `Phase::Reconnected`, a session with a higher sequence that is no longer awaiting input yields no claim | FR-006 | example | DONE | T017 / T024 |
| U25 | A lower sequence than the one seen is adopted with no claim | FR-003 | example | DONE | T017 / T024 |
| U26 | Ten sessions with higher sequences yield ten claims, each naming its own session | FR-009, SC-005 | example | DONE | T017 / T024 |
| U27 | A session absent from the snapshot is dropped from what the tracker has seen | FR-020 | example | DONE | T017 / T024 |

### `crates/micold-core/src/attention.rs`: `notification_text`

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U28 | The title is `{session} is waiting for input` and the body `{project} — {worktree}` | FR-004, US1-1 | example | DONE | T018 / T025 |
| U29 | The Default entry's name is passed through unchanged | FR-004, US1-7 | example | DONE | T018 / T025 |
| U30 | A placeholder session label is passed through unchanged | FR-004, Edge: placeholder name | example | DONE | T018 / T025 |
| U31 | Neither string holds any text besides the three names and the fixed words | FR-004, Edge: screen sharing | example | DONE | T018 / T025 |

### `crates/micold-core/src/attention.rs`: `resolve_reveal`

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U32 | A known, available project holding the session resolves to `Show` | FR-011 | example | DONE | T073 / T081 |
| U33 | A session that was removed resolves to `Unavailable` | FR-013, US3-4 | example | DONE | T073 / T081 |
| U34 | A forgotten project resolves to `Unavailable` | FR-013, Edge: project forgotten | example | DONE | T073 / T081 |
| U35 | A project whose folder is unavailable resolves to `Unavailable` | FR-013, Edge: project forgotten | example | DONE | T073 / T081 |

### `crates/micold-core/src/workspace.rs`: unread counts

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U36 | `unread_session_count` counts unread sessions of the Default entry and of every worktree, whatever the sidebar's filter hides | FR-022, US2-14 | example | DONE | T062 / T066 |
| U37 | The count leaves out the session passed as in view | FR-019 | example | DONE | T062 / T066 |
| U38 | The count is zero for a project with no unread session | FR-021 | example | DONE | T062 / T066 |
| U39 | `other_projects_unread` is the sum over every project but the active one | FR-023, US2-15 | example | DONE | T062 / T066 |
| U40 | Unread sessions of the active project are not in that sum | FR-023, US2-16 | example | DONE | T062 / T066 |
| U41 | With Q active the sum counts R's session only | FR-023, US2-17 | example | DONE | T062 / T066 |

### `crates/micold-core/src/settings.rs`: the setting

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U42 | `desktop_notifications` is `true` in default settings | FR-026, US4-1, Edge: settings file unreadable | example | DONE | T101 / T106 |
| U43 | A settings file without the field reads as `true` | FR-026 | example | DONE | T101 / T106 |
| U44 | `false` survives a round trip | FR-026, US4-4 | example | DONE | T101 / T106 |
| U45 | The serialised settings hold exactly one key naming notifications, and none per AI CLI | FR-028, US4-6 | example | DONE | T101 / T106 |

### `crates/micold-core/tests/notification_registers_nothing.rs`: source scan

U47 to U49 hold against today's sources: they are guards, green when written, and their red is a
deliberate violation shown once in the cycle log (T037 says so). U175, under the stored fields, is
the same kind.

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U46 | The installer's Start-menu shortcut carries `AppUserModelID: "MicoldAiIde.Client"`, the same string as `APP_USER_MODEL_ID` | FR-029 | example | DONE | T037 / T040, T042 |
| U47 | The installer registers no toast activator and no protocol handler, and the macOS `Info.plist` template no URL scheme | FR-015a | characterization | DONE | T037 / (none) |
| U48 | The client's entry point reads no command-line argument | FR-015a | characterization | DONE | T037 / (none) |
| U49 | `crates/micold-daemon/Cargo.toml` names none of the three notification crates: the service shows nothing itself | FR-007 | characterization | DONE | T037 / (none) |

### `crates/micold-daemon/src/attention.rs`: `Views`

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U50 | `set_view` keeps one report per connection: a second report replaces the first | FR-002 | example | DONE | T004 / T010 |
| U51 | A report with `focused: false` is stored with `in_view: None` whatever it carried | US1-3, FR-016 | example | DONE | T004 / T010 |
| U52 | `is_in_view(session)` is true while any stored report names the session, and false otherwise | FR-002, FR-016 | example | DONE | T004 / T010 |
| U53 | `remove(client)` forgets that connection's report | FR-016 | example | DONE | T004 / T010 |
| U54 | `grant` is true the first time a sequence is claimed | FR-006a | example | DONE | T020 / T027 |
| U55 | `grant` is false for the same sequence again | FR-006a | example | DONE | T020 / T027 |
| U56 | `grant` is false for a sequence above the session's current one | FR-001 | example | DONE | T020 / T027 |
| U57 | `grant` is true for a later sequence of the same session | US1-6, FR-003 | example | DONE | T020 / T027 |
| U58 | A grant for one session does not use up another session's | FR-009 | example | DONE | T020 / T027 |
| U59 | `set_view` returns the session that came into view | FR-019 | example | DONE | T047 / T055 |
| U60 | `set_view` returns `None` when the report names the same session as before, or none | FR-019 | example | DONE | T047 / T055 |
| U61 | `focus_order` puts the connection that last reported `focused: true` last | FR-012 | example | DONE | T075 / T083 |
| U62 | `remove` takes a connection out of `focus_order` | FR-012 | example | DONE | T075 / T083 |
| U63 | `reveal_target` is the connection that holds the project | FR-012, US3-6 | example | DONE | T075 / T083 |
| U64 | With no holder, `reveal_target` is the last of `focus_order` | FR-012 | example | DONE | T075 / T083 |
| U65 | With no holder and an empty `focus_order`, `reveal_target` is the sender | FR-012 | example | DONE | T075 / T083 |
| U66 | `grant` with `enabled: false` is false and records nothing | FR-027 | example | DONE | T103 / T108 |
| U67 | `note_event` with `enabled: false` records the sequence as granted, so a later `grant` for it is false | FR-027, US4-5 | example | DONE | T103 / T108 |
| U68 | `note_event` with `enabled: true` records nothing | FR-027 | example | DONE | T103 / T108 |

### `crates/micold-daemon/tests/attention_events.rs`: the attention event

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U69 | A change into `AwaitingInput` with the session in view nowhere adds one to `attention_seq` in the `CatalogChanged` both connections receive | FR-001, FR-024 | example | DONE | T005 / T011, T012 |
| U70 | With one connection reporting the session in view, the change adds nothing | FR-002 | example | DONE | T005 / T012, T013 |
| U71 | A repeated waiting signal adds nothing | FR-003, US1-5 | example | DONE | T005 / T012 |
| U72 | Working and awaiting input again adds one more | FR-003, US1-6 | example | DONE | T005 / T011, T012 |
| U73 | Three sessions changing at once each add one to their own sequence | FR-009, US1-12 | example | DONE | T005 / T011, T012 |
| U74 | With no connection the change still adds one | FR-008 | example | DONE | T005 / T011, T012 |
| U75 | After the connection that had the session in view closes, the next change adds one | FR-016 | example | DONE | T005 / T012 |
| U76 | A `WindowView` from a connection attached to no project is accepted and gets no `OperationOk` | FR-002 | example | DONE | T005 / T013 |
| U77 | `attention_seq` survives a restart of the service on the same store directory | FR-008a | example | DONE | T005 / T011 |
| U78 | A removed session is in no later catalog snapshot | FR-020 | example | DONE | T005 / T012 |
| U79 | A session that ends adds nothing to its sequence | FR-005, Edge: session ended or crashed | example | DONE | T005 / T012 |

### `crates/micold-daemon/tests/attention_claims.rs`: claim and grant

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U80 | Two connections claim the same sequence and exactly one `AttentionGranted` is sent, to that claimer only | FR-006a, Edge: several windows | example | DONE | T021 / T028 |
| U81 | A claim for an unknown session is not answered | FR-005 | example | DONE | T021 / T028 |
| U82 | Ten sessions with one event each give ten grants | FR-009, SC-005 | example | DONE | T021 / T028 |
| U83 | A claim gets no `OperationOk`, and a connection attached to no project may claim | FR-001 | example | DONE | T021 / T028 |

### `crates/micold-daemon/tests/unread_state.rs`: unread

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U84 | An attention event sets `unread` | FR-016, US2-1 | example | DONE | T048 / T054 |
| U85 | A change while the session is in view does not set it | FR-016, US2-2, US2-12 | example | DONE | T048 / T055 |
| U86 | A `WindowView` naming an unread session clears it, and every connection receives the `CatalogChanged` | FR-019, FR-024, US2-3 | example | DONE | T048 / T054, T055 |
| U87 | An unread session that works again stays unread | FR-020, US2-8 | example | DONE | T048 / T054 |
| U88 | A claim and a grant leave `unread` as it was | FR-017 | example | DONE | T048 / T054 |
| U89 | With no connection the event sets `unread` | FR-008, US2-18 | example | DONE | T048 / T054 |
| U90 | With no connection, an unread session that works again stays unread | FR-008, US2-22 | example | DONE | T048 / T054 |
| U91 | A restart of the service keeps `unread: true` | FR-008a, US2-19 | example | DONE | T048 / T054 |
| U92 | A restart of the service keeps `unread: false` for a session that did not change | FR-008a, US2-20 | example | DONE | T048 / T054 |
| U93 | A read session that works and awaits input again with no connection is unread | FR-008, US2-21 | example | DONE | T048 / T054 |
| U94 | A removed unread session is gone from the snapshot | FR-020, US2-9 | example | DONE | T048 / T055 |
| U95 | A session named by a connection that dropped is not in view, so its change sets `unread` | FR-006, US2-13 | example | DONE | T048 / T055 |
| U96 | A session created with no connection that reaches `AwaitingInput` is unread | FR-008 | example | DONE | T048 / T054 |

### `crates/micold-daemon/tests/session_reveal.rs`: reveal routing

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U97 | `SessionReveal` is forwarded as `RevealSession` to exactly one connection: the one attached to the project | FR-012, US3-6 | example | DONE | T076 / T083 |
| U98 | With the project attached nowhere, it goes to the connection that last reported focus | FR-012 | example | DONE | T076 / T083 |
| U99 | With neither, it goes back to the sender | FR-012 | example | DONE | T076 / T083 |
| U100 | It is forwarded for a session that does not exist | FR-013 | example | DONE | T076 / T083 |
| U101 | No session, attachment or stored state differs after a reveal | FR-014 | example | DONE | T076 / T083 |
| U102 | `activation` reaches the target connection unchanged, also when the target is not the sender | FR-011 | example | DONE | T093 / T097 |

### `crates/micold-daemon/tests/settings_desktop_notifications.rs`: the setting in the service

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U103 | `SettingsSet { desktop_notifications: Some(false) }` reaches every connection as `SettingsChanged` | FR-027, US4-3 | example | DONE | T104 / T109 |
| U104 | The value survives a restart of the service | FR-026, US4-4 | example | DONE | T104 / T109 |
| U105 | `SettingsSet` with `None` leaves the value unchanged | FR-026 | example | DONE | T104 / T109 |
| U106 | While it is off a claim is not granted | FR-027, US4-2 | example | DONE | T104 / T109 |
| U107 | While it is off the event still sets `unread` | FR-017, US2-10, SC-003 | example | DONE | T104 / T109 |
| U108 | After it is turned on, the next event is granted | FR-027, US4-5 | example | DONE | T104 / T109 |
| U109 | Events made while it was off are not granted afterwards, also to a connection that reconnects | FR-027, US4-5 | example | DONE | T104 / T109 |
| U110 | The rule is the same for a session of each of the three AI CLIs | FR-028, US4-6 | example | DONE | T104 / T109 |

### `crates/micold-client/src/features/attention.rs`: the view report

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U111 | One report follows each `Welcome`, also when nothing is in view | FR-019, US2-23 | example | DONE | T006 / T015 |
| U112 | Afterwards a report is sent only when the derived value differs from the last one sent | FR-019 | example | DONE | T006 / T015 |
| U113 | Losing focus reports `focused: false, in_view: None` | US1-3, US2-6 | example | DONE | T006 / T015 |
| U114 | Opening Settings reports `in_view: None`, and leaving it reports the session again | US1-9, US2-11 | example | DONE | T006 / T015 |
| U115 | A reconnect resets what was sent, so the next report is sent whatever its value | FR-006 | example | DONE | T006 / T015 |
| U116 | `reconcile_catalog` copies `attention_seq` into `Workspace::sessions` | FR-001 | example | DONE | T006 / T014 |
| U176 | `view_facts` names the active project's selected session as `selected` and passes `window_focused` through | US1-1, FR-001 | example | DONE | T006 / T123 |
| U177 | With Settings open `view_facts` has `main_area_taken: true` | US1-9, US2-11 | example | DONE | T006 / T123 |
| U178 | Showing another tab of the selected session leaves `view_facts` equal | US1-10, US2-12 | example | DONE | T006 / T123 |

### `crates/micold-client/src/features/attention.rs`: claim, grant and show

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U117 | A snapshot with a higher sequence yields one `AttentionClaim` | FR-001 | example | DONE | T022 / T030 |
| U118 | The first snapshot yields no claim | FR-005, US1-13 | example | DONE | T022 / T030 |
| U119 | `AttentionGranted` calls `show` once, with the title and body built from the labels the sidebar shows and with the project path and session id | FR-004, US1-1 | example | DONE | T022 / T030 |
| U120 | Without a grant `show` is not called | FR-006a | example | DONE | T022 / T030 |
| U121 | An `Err` from `show` is logged once and pushes no in-app notice | FR-010 | example | DONE | T022 / T030 |
| U122 | A second `Err` in the same run is not logged | FR-010 | example | DONE | T022 / T030 |
| U123 | The first snapshot after a reconnect is observed with `Phase::Reconnected` | FR-006, US1-11 | example | DONE | T022 / T030 |
| U124 | A granted session of a project that is not the active one is named by its own project | FR-004, US1-4 | example | DONE | T022 / T030 |

### `crates/micold-client/src/features/attention.rs`, `features/project.rs`: rows and counts

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U125 | `row_unread` is true for an unread session that is not the one in view | FR-018, US2-1 | example | DONE | T051 / T056 |
| U126 | `row_unread` is false for the session in view, before the service's answer arrives | FR-019, SC-006 | example | DONE | T051 / T056 |
| U127 | `reconcile_catalog` copies `unread` | FR-024 | example | DONE | T051 / T056 |
| U128 | Every `SwitcherEntry` carries `unread_count`, the active project's too | FR-021, US2-4, US2-5 | example | DONE | T063 / T067 |
| U129 | The count falls at once for the session in view | FR-019, US2-3 | example | DONE | T063 / T067 |
| U130 | The button's total equals `other_projects_unread` | FR-023 | example | DONE | T063 / T067 |

### `crates/micold-client/src/features/attention.rs`: reveal and raise

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U131 | `NotifierEvent::Activated` yields one `SessionReveal` | FR-011 | example | DONE | T077 / T084 |
| U132 | `RevealSession` resolving to `Show` for a project that is not active yields `ProjectMsg::Reopened` then `SessionMsg::Selected`, and no other message | FR-011, FR-014, US3-2 | example | DONE | T077 / T084 |
| U133 | For the active project it yields `SessionMsg::Selected` alone | FR-011, US3-1 | example | DONE | T077 / T084 |
| U134 | `Unavailable` pushes `That session is no longer available.` at `Level::Info` and changes no selection | FR-013, US3-4 | example | DONE | T077 / T084 |
| U135 | The session shown by a reveal is in view, so `row_unread` is false for it | FR-019, US3-3 | example | DONE | T077 / T084 |
| U136 | `raise_plan` off Wayland is `[Unminimize, Focus]` | FR-011 | example | DONE | T077 / T084 |
| U137 | `raise_plan` on Wayland is `[Unminimize, RequestAttention]` | FR-011, FR-015 | example | DONE | T077 / T084 |
| U138 | `raise_plan` on Wayland with a token is `[Unminimize, Activate(token)]` | FR-011 | example | DONE | T094 / T098 |
| U139 | Without a token the Wayland plan is as before, and off Wayland the token changes nothing | FR-011 | example | DONE | T094 / T098 |
| U140 | `after_activation(false)` is `Some(RequestAttention)` and `after_activation(true)` is `None` | FR-011, FR-015 | example | DONE | T094 / T098 |
| U141 | The token of `Activated` is put into `SessionReveal`, and the token of `RevealSession` is the one passed to `raise_plan` | FR-011 | example | DONE | T094 / T098 |

### `crates/micold-client/src/features/settings.rs`: the switch's draft

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U142 | The draft holds `desktop_notifications`, and its message changes the draft | FR-026, US4-1 | example | DONE | T105 / T110 |
| U143 | Saving sends `SettingsSet` with `Some(value)` when the user changed the switch on the page (narrowed by BUG-570: `None` otherwise, U182) | FR-027 | example | DONE | T105 / T110 |
| U144 | `SettingsChanged` updates the value the draft is built from | FR-027, US4-3 | example | DONE | T105 / T110 |
| U182 | A save sends `Some` for each service-owned field the user changed and `None` for every other; a save changing none sends no `SettingsSet` (BUG-570) | FR-026a | example | DONE | T128 / T129 |
| U183 | A save sets only the changed fields on the stored document: a theme and sandbox settings another window stored are kept (BUG-570) | FR-026a | example | DONE | T128 / T129 |
| U184 | A field retyped to the value the page opened with counts as unchanged (BUG-570) | FR-026a | example | DONE | T128 / T129 |

### `crates/micold-client/src/ui/material/`: `UnreadMark` and its hosts

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U145 | The mark is an 8dp filled circle in the `primary` role | FR-018 | example | DONE | T049 / T057 |
| U146 | The mark meets 3:1 against its surface in the light and the dark scheme | FR-018, FR-023 | example | DONE | T049 / T057 |
| U147 | `count(0)` renders nothing | FR-021, FR-023 | example | DONE | T049 / T057 |
| U148 | `.worded(true)` adds the word `unread` | FR-021 | example | DONE | T049 / T057 |
| U149 | A `TreeItem` with `.unread(true)` has the height of one without | FR-032 | example | DONE | T050 / T058 |
| U150 | The row's label truncates before the mark is pushed out | FR-018 | example | DONE | T050 / T058 |
| U151 | The badge slot and its `ActivityBadge` are the same node as before | FR-018, FR-032 | example | DONE | T050 / T058 |
| U152 | `MenuItem` with `trailing_mark: Some(n)` renders `● n unread` after `trailing_text` | FR-021 | example | DONE | T064 / T068 |
| U153 | With `trailing_mark: None` the row renders what it renders today | FR-032 | example | DONE | T064 / T068 |
| U154 | With no running count the mark alone trails | FR-021 | example | DONE | T064 / T068 |
| U155 | The menu row's height is unchanged by the mark | FR-032 | example | DONE | T064 / T068 |
| U156 | `Button::trailing_mark(n, tooltip)` renders `● n` after the label, inside the button | FR-023 | example | DONE | T065 / T069 |
| U157 | The button's height is unchanged by the mark | FR-032 | example | DONE | T065 / T069 |
| U158 | `unread_total_tooltip(1)` is `1 unread session in other projects`, and for `n` above one `{n} unread sessions in other projects` | FR-023 | example | DONE | T065 / T069 |

### `crates/micold-client/src/shell/desktop_notify/`: backend mappings

Pure functions inside the three backend files, tested where they are (`#[cfg(test)]`); the macOS
and Windows ones run on CI's legs for those systems only (profile, *Windows-only behaviour*), by
the step T122 adds to `.github/workflows/ci.yml`; their red phase is that step's first run.

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U159 | Linux: `notify_request` carries the application name, the summary, the body and the `desktop-entry` hint `micold-ai-ide`, and no other text | FR-004 | example | DONE | T023 / T032 |
| U160 | Linux: a bus failure maps to a `NotifyError` | FR-010, Edge: system refuses | example | DONE | T023 / T032 |
| U161 | Linux: the request offers the `default` action, the same whether or not the service lists the `actions` capability | FR-015 | example | DONE | T078 / T085 |
| U162 | Linux: `ActionInvoked(id, "default")` for an id in the table maps to `Activated` with that id's project and session | FR-011 | example | DONE | T078 / T085 |
| U163 | Linux: an id the table does not hold maps to no event | FR-015, Edge: raising window closed | example | DONE | T078 / T085 |
| U164 | Linux: another action key maps to no event | FR-011 | example | DONE | T078 / T085 |
| U165 | Linux: `NotificationClosed` removes the id from the table | FR-015 | example | DONE | T078 / T085 |
| U166 | Linux: an `ActivationToken` signal that precedes `ActionInvoked` for the same id is carried as `activation: Some(token)`; without it `activation` is `None` | FR-011 | example | DONE | T095 / T099 |
| U167 | macOS: a `DesktopNotification` maps to the title and message passed to the system | FR-004, FR-029 | example | DONE | T035 / T039 |
| U168 | macOS: each error of the notification crate (no bundle, authorisation refused) maps to a `NotifyError` | FR-010 | example | DONE | T035 / T039 |
| U169 | macOS: a response with the default action maps to `Activated` | FR-011 | example | DONE | T079 / T086 |
| U170 | macOS: a dismissal or a timeout maps to no event | FR-011 | example | DONE | T079 / T086 |
| U171 | macOS: an unknown notification id maps to no event | FR-015 | example | DONE | T079 / T086 |
| U172 | Windows: a `DesktopNotification` maps to the toast's title and first text line | FR-004, FR-029 | example | DONE | T036 / T040 |
| U173 | Windows: an error from `show` maps to a `NotifyError` | FR-010 | example | DONE | T036 / T040 |
| U174 | Windows: `on_activated` maps to `Activated` for the session of the toast it was registered on, with two toasts live | FR-011 | example | DONE | T080 / T087 |
| U179 | Linux: a signal for an id in the table, from a sender other than the one that answered its `Notify`, is nothing and leaves the entry and its token as they were; from that sender it is the click (BUG-566) | FR-015b | example | TODO | T124 / T125 |
| U180 | Linux: after the service's owner changes, the new owner's signal for an id the old owner gave is nothing (BUG-566) | FR-015b | example | TODO | T124 / T125 |
| U181 | Linux: `NameOwnerChanged` for `org.freedesktop.Notifications` from `org.freedesktop.DBus` drops the old owner's entries and keeps the new owner's; the same signal from another sender, or for another name, drops nothing (BUG-566) | FR-015b | example | TODO | T124 / T125 |

## Invariants and edge cases still to place

None.

## Out of scope

- The real notification on a real desktop, the click on it, the look of the mark and the counts,
  and the timing criteria SC-001, SC-004 and SC-006 (2 s, 1 s): quickstart §B with `visual-pass`
  (T115), not tests. SC-008 is a question to a user, answered from the same pass.
- macOS and Windows as seen by a user, and the session service in a container on a real runtime
  (SC-007, US1-8 end to end): quickstart §C by hand (T116). The tests cover the path, which names no
  runtime (A8, U49).
- `shell/desktop_notify/*` beyond its pure mappings, `shell/window_raise.rs`, and the `ui/`
  composition in T059, T070 and T111: the constitution's glue exception; no decision is made there.
- The Wayland probe (T091): an experiment recorded in research R7, not a behavior.
- Ended or crashed sessions, long-working sessions, sound, badges, a tray icon, rate limits,
  grouping, withdrawal, a per-CLI switch, a count of unread turns: spec Out of Scope.
- Opening the session from a notification clicked after the application quit: FR-015a. U47 and U48
  hold that nothing is registered for it.
- The user guide and developer docs (FR-031): CI's user-guide gate and review.
- The trial counts of SC-002, SC-003, SC-005, SC-009 and SC-010 (20 of 20, ten sessions): each rule
  is asserted once, deterministically, by A2, A23 and A44, A12, A31 to A33, and A28 to A30.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md`:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- Unit test in `src/`: `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets`
- Acceptance: `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1` (container runtime; not this feature's outer loop)
- Coverage, mutation, property, watch: none installed; deliberate-mutant spot checks by hand.
- The single-test command exits 0 when the name matches nothing: read the `N passed` count.
