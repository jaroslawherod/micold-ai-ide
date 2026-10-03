# Autopilot ledger — 039-session-attention-notifications

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #481: https://github.com/jaroslawherod/micold-ai-ide/issues/481 — Issue title: Notify when a session needs attention and track unread sessions. Issue body: ## Problem — The sidebar already shows each session's activity dot (working / awaiting input), but only while the user is looking at the window. With several agents running in parallel, a session that finished its turn or is waiting for a permission prompt goes unnoticed until the user happens to check it. There is also no way to tell which sessions have new output the user has not looked at yet. ## Proposal — When a session moves to **awaiting input** (turn ended, permission or idle prompt), show an OS desktop notification naming the project, worktree and session. Clicking it focuses the window and switches to that session. — Mark a session **unread** when it produces a turn the user has not viewed since it became active, and clear the mark when the user opens the session. — Show an unread count on the project in the switcher, so attention needed in a background project is visible. — Suppress the notification when the session is the one currently focused. — Settings: on/off for desktop notifications, and optionally per provider. ## Acceptance criteria — A session that reaches awaiting input while another session is focused (or the window is unfocused) raises exactly one desktop notification on Linux and macOS. — Clicking the notification opens that session. — The unread mark appears on the session and its project, and clears when the session is opened. — Nothing is sent when notifications are turned off. — Works in both the host and the sandboxed daemon runtime.
- **Kind**: feature
- **Issue**: #481
- **Worktree branch**: feat/notify-session-needs-attention
- **Started**: 2026-10-02
- **Phase**: 4-milestone (M5)
- **Next step**: M5, continue from *Handover*: T119, then verify.md (scoped gate with review A, review B and the visual pass, full gate) and the PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #528 | Spec | merged | 824e0a9bc58ad5f977ef56fa813e58217c563ea5 |
| #539 | Design | merged | 6b4b6fa9afc27f134a5f4fc80048dc3f9a115a38 |
| #544 | M1 | merged | a8d628b8a9b089033a05b1194c8fc5f5fe5b3c7c |
| #554 | M2 | merged | d4ea296f481afbd1c773adfb52cfa862ca37482a |
| #557 | M3 | merged | f113b4a49a0c9a3146dfc5ada363b0c6d42cb4db |
| #558 | M4 | merged | ea477587200d8efb0a0be2427ec471254cb32fd7 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016, T123 | full | The service knows what is in view and counts attention events (integration test; nothing new on screen) | #544 | merged |
| M2 | T017–T034 | full | One desktop notification on Linux for a session not in view | #554 | merged |
| M3 | T035–T044, T118, T122 | full | The same notification on macOS and Windows | #557 | merged |
| M4 | T045–T061 | full | The unread mark on a session's row, kept across restarts | #558 | merged |
| M5 | T062–T072, T119 | full | Unread counts on the switcher's rows and button | | in progress |
| M6 | T073–T090, T120 | full | A click on the notification opens the session | | pending |
| M7 | T091–T100 | full | Keyboard focus from a click on Wayland (probe first) | | pending |
| M8 | T101–T112, T121 | full | The Desktop notifications switch | | pending |
| M9 | T113–T117 | light | Developer docs and the recorded quickstart passes | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which spec number? | 039: 036 is taken by the flow in worktree `fix-issue-430` (not on `main`), 037 and 038 are on `main`, no open PR or worktree holds 039. | agent-resolved | `ls <worktree>/specs` across `git worktree list`; `gh pr list --state open`, 2026-10-02 |
| D2 | spec | The issue names Linux and macOS. Is Windows in scope? | Yes. Principle VI makes parity on all three a condition of done (FR-029). | agent-resolved | `.specify/memory/constitution.md` §VI; spec.md#Assumptions |
| D3 | spec | What is "the session currently focused"? | The selected session of a window with keyboard focus ("in view"). The same condition decides notification and unread. | agent-resolved | spec.md#Terms; issue acceptance criterion 1 ("or the window is unfocused") |
| D4 | spec | Do unread marks depend on the notification setting? | No. The switch stops desktop notifications only (FR-017, FR-027). | agent-resolved | spec.md#User Story 4 |
| D5 | spec | Is a session covered by Settings, or showing a regular terminal tab, in view? | Settings: not in view (the user cannot see it). Regular terminal tab: in view (its indicator is beside it). | agent-resolved | spec.md#Terms; Review Spec round 1 F1 |
| D6 | spec | Does a change missed during a lost connection count? | Yes, with a window open: it is treated as a change at reconnection (FR-006). At application start nothing is notified (FR-005); unread then waits on FR-008. | agent-resolved | spec.md FR-005, FR-006; Review Spec round 1 F2 |
| D7 | clarify | FR-008: a session changes to awaiting input while no window is open. What must happen? | (b) No desktop notification; the session is unread when the application is next opened. Unread state outlives the window and is stored on the user's computer (FR-008, FR-008a, FR-024). | user | Orchestrator relayed the user's answer, 2026-10-02; spec.md#Clarifications |
| D8 | clarify | FR-023: does the switcher's button show unread with its panel closed? | (a) Yes: the total of unread sessions in projects other than the active one, nothing when zero. | user | Orchestrator relayed the user's answer, 2026-10-02; spec.md#Clarifications |
| D9 | clarify | FR-028: a notification switch per AI CLI? | (a) No: one switch for every AI CLI; per-CLI is out of scope. | user | Orchestrator relayed the user's answer, 2026-10-02; spec.md#Clarifications |
| D10 | clarify | What is unread on the first start with the feature, or when the stored unread state cannot be read? | Nothing, and no error is shown: there is no record of what the user viewed (FR-008a). Same fallback as an unreadable settings file. | agent-resolved | spec.md#Edge Cases ("Settings file unreadable"); follows from D7 |
| D11 | design | FR-008: how is a change known that happened with no window open? | The session service keeps `attention_seq` and `unread` per session in its catalog and sets them itself; windows report what is in view (`WindowView`). Within FR-007: the existing connection only. | agent-resolved | research.md R1, R2 |
| D12 | design | Several windows are several client processes: who raises the one notification, and which window does a click open? | A window claims an attention event and the service grants each once (FR-006a); a click is sent to the service, which forwards it to the window holding the project (FR-012). | agent-resolved | research.md R3, R6; `messages.rs:92` `ClientInstance`, `RefusalReason::ProjectBusy` |
| D13 | design | Which notification crates? | Linux `zbus` directly, macOS `mac-usernotifications` 0.3.1, Windows `tauri-winrt-notification` 0.8.1, behind one trait. `notify-rust` rejected: no Wayland activation token, `cc` build on macOS. | agent-resolved | research.md R4 |
| D14 | design | Is the switch a client or a service setting? | Service-owned (`DaemonSettings`), so every window has one value at once; the service refuses grants while off. | agent-resolved | research.md R8; pattern `tool_server_enabled` |
| D15 | design | A notification is clicked after the window that raised it was closed, with another window open. Must the session open? | No: the click is reported to the raising process only, on all three systems, so no window learns of it (FR-015's exception). Nothing changes; the unread mark finds the session. Stated in spec Edge Cases, research R6 *Known limit*, contract N9, quickstart B11a, and the user guide in M6 (T090). Sharing ids through the service was rejected: it helps only on Linux services that send the click to every listener. | agent-resolved | spec.md FR-015, Edge Cases; Review Plan round 1 F3; `mac-usernotifications` 0.3.1 `src/delegate.rs`, `tauri-winrt-notification` 0.8.1 `src/lib.rs:485` |
| D17 | design | How are the stories cut into milestones? | Nine: story 1 in three slices (service rules; Linux notification; macOS and Windows), story 2 in two (row mark; switcher counts), story 3 in two (click; Wayland focus, which starts with a probe), story 4, polish. Each story is well past 15 tasks; M1 is observable by its integration test only and its PR takes `docs-not-needed`. The wire is bumped once per milestone that changes it (21 to 26), so the claim pair moved to 22 and the activation token to 25. | agent-resolved | `references/milestones.md` rules 1, 3; tasks.md Milestones; research.md R10 |
| D18 | design | How does the test list bind the tasks? | `tdd/test-list.md`: 48 acceptance behaviors (one per scenario) and 178 unit behaviors (175, and U176–U178 for `view_facts` after the tasks review); the outer loop is two halves of integration tests (service, window) that meet at the catalog snapshot, with the real desktop left to quickstart §B and §C. Markers `[A…]`/`[U…]` on the test and implementation tasks; four final run tasks T118–T121 (M3, M5, M6, M8; T118 carries no marker, as A1–A13 are done in M2); five cases added to T005, T022, T037, T045, T048 for rules no task tested (FR-005 ended session, US1.4, FR-007 guard, FR-008a unreadable store, FR-008 session created with no window) | `speckit-tdd-plan`; a marker's tasks stay inside one milestone so `speckit-tdd-run` can tick them | autopilot |
| D16 | design | Do the facts part 1 left unconfirmed hold? | Code: all four hold (`switch_daemon_attachment` sends `Detach` then `Attach`, `daemon_sync.rs:198`; `ProjectMsg::Reopened` `features/project.rs:531`; `SessionMsg::Selected` `features/session.rs:1554`; `Button` is a builder with a `leading` slot, `button.rs:95`; the sandboxed service's `projects.json` is the host's state directory mounted at `/var/lib/micold-ai-ide`, `sandbox/mod.rs:367`, `:659`). Wayland: the foreign-handle binding is confirmed in the sources (winit selects `client_system`; `smithay-clipboard` does the same in this application); whether a compositor honours the notification's token stays unverified and has a probe task in M7 (T091). Windows: the notification-centre click stays unverified: no primary source found; T116 in M9 checks it on an installed build. | agent-resolved | research.md "Where the code is today", R4, R7 |
| D19 | design | A notification is clicked after the window that raised it was closed, while another window is open. Must the session open? (supersedes D15's `agent-resolved`) | Option 1: accept the limit on all three systems. FR-011 to FR-013 apply while the window that raised the notification is still open; otherwise the click need not open the session, which is found by its unread mark. FR-015, FR-015a and the Edge Case amended; no change to plan or tasks. | decided by user | Orchestrator relayed the user's answer to the escalation of tasks review round 1 F2, 2026-10-02; spec.md#Clarifications |
| D20 | M1 | T016 has `main.rs` call `features::attention::view_report(&mut State, …)`, which the client gate `only_the_root_drives_a_feature` forbids. How is the view report wired? | The feature's functions take its own `features::attention::State`; two root helpers in `app.rs` (`State::view_report`, `State::view_report_forgotten`) call them, and `shell::daemon_sync::report_window_view` calls the root and sends `ClientMsg::WindowView` only while connected. U111–U115 live in `tests/features_attention.rs`, which the gate `every_feature_module_has_an_isolation_test` requires, not in `tests/attention_view_report.rs` as T006 words it. Behaviour unchanged. | agent-resolved | `crates/micold-client/tests/feature_registration_cost.rs`; ledger Handover of M1 part 2 |
| D21 | M4 | T045 says `unread` is written "to the catalog file"; the prep code stores it in the per-project state file. Which holds? | The code: `unread` is a `#[serde(default)]` field of `StoredSession` (as T052 says) in the project's state file, beside `attention_seq`; the store test asserts that file names it and no other does. FR-025 asks only that it stays on the computer. T045's "catalog file" is loose wording, left as written. | agent-resolved | research R1; `store.rs` `unread_tests`; review B M4 (consistent with spec and plan) |
| D22 | M4 | T054 says `mark_read` "clears it and persists"; the prep code clears in memory and lets the supervisor tick write. Accept? | Yes: `set_window_view` sets `attention_unsaved` and the tick writes, as M1 does for `mark_attention`, so a view report never does blocking I/O on the connection loop. This unit added the write at stop (`unwind` calls `persist_attention`) and the test `a_read_is_written_by_persist_attention`. | agent-resolved | `state.rs` `set_window_view`, `persist_attention`; `server.rs` `unwind`; cycle 21 |
| D23 | M4 | Which M4 design choices do later milestones build on? | (1) An unread row's label uses the tree view's existing `selected_label_role`; there is no new setter. (2) `UnreadMark` draws its count and word in `TypeRole::Label`, with no setter for the host's type role or colour: M5's switcher hosts need one, and `count(0)` returns a zero-size `Space` that still takes a spacing slot in a host `row!`. (3) The sidebar takes the session in view from the last view report sent (`features::attention::in_view`), so a row loses its mark before the service answers (FR-019). (4) In the sidebar the mark sits in the trailing slot before the close action, as T058 says. (5) The user guide has *Unread sessions* before *Being told when a session needs you*; read again after M3, the order holds (the mark is the lasting state, the notification the momentary one) and is kept. | agent-resolved | `tree_view.rs`, `unread_mark.rs`, `features/attention.rs`; `visual-pass/M4/B6.png` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 188c880b773ff483e9791fdb9842587dcb7f9515:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR ("in view" defined two ways, unread after reconnection undefined, click with no window open undefined), 3 MINOR; all six fixed |
| Spec | 2 | b217dc29a9ef4507039fd9ddace7063a33e1ac63:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN (1 MINOR: FR-019 did not name leaving Settings; fixed) |
| Plan | 1 | 7dfdd00bfdb595aa473a2abd3e119814606b521d:21c79c9d9bf6fb59b7945d2b4bd0596a9a9aefe7 | CHANGES: 5 MAJOR (`SessionChanged` is never sent, the carrier is `CatalogChanged`; the Wayland token did not reach the window that is raised; no rule for a click after the raising window closed; the raise decision was untested glue; no test layer for FR-015, FR-015a, FR-017, FR-025, FR-028), 3 MINOR (an event while the switch is off claimable after reconnecting; counts need the session in view; `SettingsSet` field is an `Option`); all eight fixed |
| Plan | 2 | 6f5590e2f6b7bd8ef3510df31aee2a75532299d2:80af04fa661ea74bd8f32172b5dd8b92c2813ecb | CLEAN (1 MINOR: wrong line cited for the second `note_activity` caller; fixed) |
| Tasks | 1 | c85dac2f537bb711fcce428ed56ceeef88af19dc:897f84cedd8e9e405d34fa1ff8b79c1fbf646097 | CHANGES: 5 MAJOR, 3 MINOR. Fixed: F1 the macOS and Windows backend tests ran on no CI leg (new T122 edits `ci.yml`; client test files join the enumerated list in T006, T051, T063, T077); F3 T118 would be ticked in M2 (markers removed); F4 a failed probe left T092–T100 open and version 25 untaken (T091 closes them as `DROPPED`; versions are "next free"); F5 the rules of the view report sat untested in `main.rs` (new T123 `State::view_facts` in the lib, tested in T006 with `[A9]`, `[A10]`, U176–U178; this replaces the reason given for analysis finding G6); F6 dependencies (M4 on M2, M6 on M3 and M4, M8 on M7); F7 the "next free number" rule in R10, wire.md and the plan; F8 the Windows notification-centre sentence moved to T090, milestone ids in D15 and D16, the header's glue list. F2 (the D15 Edge Case narrowed FR-011 by agent decision): escalated, decided by the user (D19), FR-015 and FR-015a amended, `checklists/requirements.md` line 17 ticked again |
| Tasks | 2 | 87798c9ef7f66cc737705e1027bdd8ac800df22f:3e048fa2f8aedc3fdf5b0c9a8a0acfe60351f003 | CLEAN (1 MINOR: T016 named no dependency on T123; fixed) |
| M1 A | 1 | f0755331e896218cbb4aee5d412514a99a0285a0:42e5f39236de396343a5def84b90d15f7207c92f | CHANGES: 1 MAJOR (`mark_attention` wrote the store under the state lock on the async runtime; now counted in memory and written by `persist_attention` in the supervisor tick's `spawn_blocking` hop), 2 MINOR (F2 `release_attachments` kept the view: fixed, with a test; F3 a displaced window reports its session in view: follow-up) |
| M1 A | 2 | 2c3a900b1a72b2cf6da590cf8e1a8266e0113d50:c91ce9217f93d0e268f150994b1143aa625b1644 | CLEAN (2 MINOR: a failed `persist_attention` write is retried only by the next event or catalog write, and its doc says the next event; an event counted within 250 ms of a daemon exit is not written. Both follow-ups) |
| M1 B | 1 | 30250d1fa843ee6a0f1a5ec2a9e9db134f181889:dd87f613dd0b54de6d7f8c0ffbcfefeb47e8f433 | CLEAN (Verify: attention_events 12 passed, attention_view_report 5, features_attention 5, test-core exit 0; 3 MINOR: docs/daemon.md version clause fixed; untested tick write and lock held across the write: follow-ups) |
| M2 A | 1 | ce1de609a41f19b0cd16bd454170c54169714416:c0366f52d42b99fc05804fbf4476d12c07623760 | CHANGES: 2 MAJOR (F1 `Notifier::show` blocks the window's update thread on D-Bus with no timeout; F2 the body is sent unescaped to servers that read it as markup), 3 MINOR (F3 a dead bus connection is kept; F4 `Views::granted` is never pruned; F5 the notification re-implements the sidebar's worktree naming) |
| M2 A | 2 | b335b552c718e80437c9096936e32a92acbc77ca:8f5886349019ae746877ee10aef897c156812511 | CLEAN (1 MINOR: `Notifier::connection` holds its mutex across the connect and handshake, which `method_timeout` does not cover: follow-up) |
| M2 B | 1 | f2083e379371594a8e7ba931a1e28f92f7331c6b:267c3ab777d9ca024030b57b56b0fa65e7348d53 | CLEAN (Verify: attention_claims 5 passed, attention_notify 11 passed; the first reviewer ran only Verify, so a second read the diff file by file on the same snapshot; 1 MINOR: US1.8 (container) and US1.13 (no window open) have no M2 test of their own — 13 rests on U18 and M1's count, 8 on the visual pass and §C: follow-up) |
| M2 visual | 1 | same tree | PASS: quickstart §B1–B5 and B13, notification part, on a private bus with a stand-in `org.freedesktop.Notifications` service (no dunst or mako installed); evidence in `visual-pass/M2/` |
| M3 A | 1 | 272bc48cbee8e8b08b48852c798bfec26d9e83a0:a0a230930461a240142697d440b9d66e64073ded | CLEAN: no BLOCKER or MAJOR. Two passes on one snapshot (a reviewer by hand, which read the vendored `mac-usernotifications` 0.3.1 and `tauri-winrt-notification` 0.8.1 sources, then the `code-review` skill at `high`, 10 findings). All judged MINOR: 2 declined, 8 under *Follow-ups not done*; none fixed, as the full gate had already run on this tree |
| M3 B | 1 | 272bc48cbee8e8b08b48852c798bfec26d9e83a0:a0a230930461a240142697d440b9d66e64073ded | CLEAN (Verify: `mise run test-core` all ok, `notification_registers_nothing` 5 passed; `attention_notify` 11 passed; `cargo check -p micold-client --target aarch64-apple-darwin` finished; 1 MINOR fixed: cycle 12 said 8 macOS tests, there are 10) |
| M4 A | 1 | 423aa47295a7aa4ef0b37929a5fe74f7bf4ee31d:37359f9dc9d0d422bb868ede6fdd739328f8d81e | CLEAN: 3 MINOR. F1 fixed in part (`unwind` writes before the stop; the retry after a failed write stays a follow-up), F2 fixed (`unread: true` in the round-trip sample), F3 under *Follow-ups not done* |
| M4 B | 1 | 1bc33e238c5399a29f6f1a31cd4eecb6c2a73415:1b75cdf11c322ea9af4eddea55ea00153957c266 | CLEAN (Verify: `unread_state` 14 passed, `unread_rows` 5 passed; D21 and D22 judged consistent with spec and plan). 1 MINOR fixed: no test saw a read reach the store (cycle 21, `unread_state` now 15) |
| M4 visual | 1 | same tree (binaries from `1b75cdf1`) | PASS: showcase (light and dark), quickstart §B6 (row), B7, B8, on Xvfb `:121` with a private HOME and pin directory; turns driven by posting `UserPromptSubmit` and `Stop` to the session's hook URL, notifications logged by a stand-in service on a private bus; evidence `visual-pass/M4/`. Not covered: see *Follow-ups not done* |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A | `desktop_notify/mod.rs`: the catch-all arm is gone, so `system()` exists for Linux, macOS and Windows only and `NotifyError::Unsupported` has no constructor | T041 asks for exactly the three arms and the constitution (VI) names three systems; the client is built for no fourth. Removing the unused variant is not an M3 task |
| M3 | A | `desktop_notify/windows.rs`: `Toast::show` returns `Ok` on a build no shortcut registers, so nothing is logged (FR-010) | The system reports success, so the client has no failure to log; FR-031 and T044 say in the user guide that Windows shows notifications only for an installed build |

## Handover

M5, written at the 150k context cap. No PR is open. Nothing is pushed.

- **Done and committed** on `feat/notify-session-needs-attention` (on `origin/main` at
  ea477587): T062–T072, all ticked in `tasks.md`, with cycles 22–24 in `tdd/cycle-log.md`.
  U36–U41, U128–U130 and U152–U158 are `DONE` in `tdd/test-list.md`.
- **Runs so far**: `cargo test -p micold-core --all-targets` green; `cargo test -p micold-client
  --lib` (507) and `--test switcher_unread` (5), `showcase_completeness`, `showcase_captions`,
  `material_builder_api` green. **Not run**: clippy, the rest of the client's integration tests,
  `cargo test --workspace`, the scoped gate, the full gate.
- **Next step**: T119. Run `scripts/build-lock.sh cargo test --test unread_state`, `--test
  unread_rows`, `--test switcher_unread`, then tick T119 and set A14–A22 and A24–A36 to `DONE` in
  `tdd/test-list.md` (A23 is T121's). Then `speckit-implement`'s optional `after_implement` hooks,
  then `tasks/verify.md` from step 1: scoped gate with review A (`code-review`, `high`), review B
  (conformance, sonnet) with the visual pass (quickstart §B5, B6 panel and button, B7; evidence
  under `visual-pass/M5/`), the full gate, the PR (`Refs #481`).
- **No review has run** for M5: *Review rounds* has no M5 row yet.
- **For the reviewers and the visual pass**:
  - The showcase's switcher row with both counts is in the project switcher panel
    (`MenuOverlay`'s second opener), not in `sections/atoms.rs` as T071's file list says: a
    menu's rows are built only inside its panel. The button is in the `UnreadMark` entry.
  - On the switcher's button the mark (`primary`) and the number (the text variant's content
    colour, also `primary`) are the same colour, and the gap before the mark is 8dp. Neither has
    been looked at on a display.
  - `Button::trailing_mark` on a **filled** button would draw a `primary` mark on a `primary`
    fill. No host does that; the doc comment says so and nothing enforces it.
  - `other_projects_unread` takes `Option<&Path>`, where data-model.md writes `&active`.
- **Open follow-up from M4 not done here**: review A F3 (`session_tree_item`'s two positional
  `Option<SessionId>`). M5 did not edit `ui/sidebar.rs`.

## Open escalation

None.

## Token usage

## Follow-ups not done

- M1 review B (MINOR): no test observes the supervisor tick writing `attention_seq` (the mutant removing the call survives); and `persist_attention` holds the state lock across the blocking write, as `record_observed_names` does. Snapshot the workspace under the lock and write after releasing it, and add a test that runs one tick.

- M1 review A round 2 and M4 review A F1 (MINOR): `DaemonState::persist_attention` clears `attention_unsaved` before the write, so a failed write is retried only by a later event or catalog write, and a read that failed to write is unread again after a restart. Setting the flag again on `Err` would retry and warn every 250 ms on a read-only data directory, so it needs a back-off or a log-once. (The other half is done in M4: `unwind` calls `persist_attention` before the stop.)

- M1 review A F3 (MINOR): a focused window displaced from or refused its project still reports its selected session in view (`app.rs` `view_facts`), so that session's attention events are not counted. Consider in M2 or M4: treat a displaced project as `main_area_taken`, or have `set_window_view` ignore `in_view` outside the connection's attachments.

- `crates/micold-daemon/tests/mcp_create_session.rs::a_pi_session_start_event_makes_pi_ready` failed once in a full-suite run on a docs-only branch (`left: ""`, line 600) and passed when rerun alone: a timing flake in code this flow does not own. Not fixed here.
- M2 review A F5 (MINOR): `State::attention_notification` (`app.rs`) re-implements the sidebar's worktree naming (the literal `Default`, `worktree_names` then `naming::display_name`) instead of sharing a helper with `worktree_display_name` and the sidebar; a change to either will not reach the notification.
- M2 review A round 2 (MINOR): `Notifier::connection` (`shell/desktop_notify/linux.rs`) holds its mutex across `Builder::build`, and `method_timeout` does not cover the connect or handshake; a session bus that accepts the socket and never answers stalls every queued notification on a blocking-pool thread. Build outside the lock, or bound the connect.
- M2 review B (MINOR): US1 scenario 13 (no window open: nothing then, nothing on the next open) has no single test; it rests on U18 (first snapshot adopted) and M1's `a_change_with_no_connection_still_adds_one`. Scenario 8 (service in a container) rests on the protocol being the same; quickstart §C or a sandbox test should show it (SC-007).
- M2 visual pass, the marks on rows A and B (examined, nothing to do): they are the existing activity badge — the dot between the row's icon and its name, drawn by `ui/sidebar.rs` with `ui/material/activity_badge.rs` (feature 010) for a session awaiting input. `visual-pass/M2/B3.png` shows it at the leading edge, not the trailing edge where M4's unread mark goes, and the M2 diff touches no file under `crates/micold-client/src/ui/`.
- M2 cycle 10 (linux.rs U159, U160): red was not recorded (the worker died); `speckit-tdd-verify` at close should mutation-test `notify_request` and `notify_error`.
- M3 review A (MINOR, macOS, `desktop_notify/macos.rs`): while the system's permission prompt is unanswered, every `show` leaves one detached thread parked in `request_auth()` with its banner; when the user allows, all of them send at once (a burst of banners, some for sessions no longer waiting). Fix when M6 next edits the file: a "request in flight" flag so later `show`s return `NOT_ANSWERED` without a thread, or drop a banner whose `show` timed out.
- M3 review A (MINOR, macOS): the 2 s timeout is reported as `Refused(NOT_ANSWERED)`, which takes the run's one failure line (FR-010 says at most once) although the notification is shown once allowed; a later real failure in that run is then not logged. And a system error on the authorisation request arrives as `Ok(false)` from the crate, so it is logged as the user's refusal (an unsigned staged bundle, for example); the U168 test comment about an unsigned bundle yielding `NoBundleIdentifier` is wrong for a bundle that has a `CFBundleIdentifier`.
- M3 review A (MINOR, Windows, not verified on a machine): only the Start-menu shortcut carries the AppUserModelID (`packaging/windows/micold-ai-ide.iss`); the desktop shortcut, the installer's launch and the process itself do not, so a pinned icon and the running window may show as two taskbar buttons. And a click on the toast has no activator, so Windows may start a second client (FR-015a allows an ordinary start; M6 adds the click). Check both in quickstart §C2 (M9) and in M6's T080.
- M3 review A (MINOR): CI's "Test (desktop notification backends)" step selects by the substring `desktop_notify` and passes on 0 tests if the module is renamed; `notification_registers_nothing.rs::rust_comment` treats any line starting with `*` as a comment and scans only `main.rs` for argument reading; `ToastText`/`Banner` copy the title and body of `DesktopNotification` field for field, and the test `notification()` helper is pasted into three backends.
- M4 review A F3 (MINOR): `session_tree_item` in `ui/sidebar.rs` takes two positional `Option<SessionId>` (`active_session`, `in_view`) that can be swapped without a compile error, and `build_default_item` derives `in_view` a second time. Pass `unread: bool` computed with `row_unread` at the call site. M5 edits this file.
- M4 visual pass, not covered: B6–B8 ran in the dark scheme only (the showcase in both); B8's branch where the session that finished with no window open is *not* the one the application opens on (it must be marked; `unread_state.rs` covers it in the service); the other tree-row entries of the showcase were not compared with `main`. The weight difference of an unread label is real but subtle at showcase size. M9's recorded passes (T113–T117) should run B6–B8 in the light scheme.
- M1 review A F3 is still open after M4: a view report from a window displaced from its project now also reads the session it names (`set_window_view` calls `mark_read`).
