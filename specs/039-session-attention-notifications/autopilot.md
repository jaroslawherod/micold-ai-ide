# Autopilot ledger — 039-session-attention-notifications

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #481: https://github.com/jaroslawherod/micold-ai-ide/issues/481 — Issue title: Notify when a session needs attention and track unread sessions. Issue body: ## Problem — The sidebar already shows each session's activity dot (working / awaiting input), but only while the user is looking at the window. With several agents running in parallel, a session that finished its turn or is waiting for a permission prompt goes unnoticed until the user happens to check it. There is also no way to tell which sessions have new output the user has not looked at yet. ## Proposal — When a session moves to **awaiting input** (turn ended, permission or idle prompt), show an OS desktop notification naming the project, worktree and session. Clicking it focuses the window and switches to that session. — Mark a session **unread** when it produces a turn the user has not viewed since it became active, and clear the mark when the user opens the session. — Show an unread count on the project in the switcher, so attention needed in a background project is visible. — Suppress the notification when the session is the one currently focused. — Settings: on/off for desktop notifications, and optionally per provider. ## Acceptance criteria — A session that reaches awaiting input while another session is focused (or the window is unfocused) raises exactly one desktop notification on Linux and macOS. — Clicking the notification opens that session. — The unread mark appears on the session and its project, and clears when the session is opened. — Nothing is sent when notifications are turned off. — Works in both the host and the sandboxed daemon runtime.
- **Kind**: feature
- **Issue**: #481
- **Worktree branch**: feat/notify-session-needs-attention
- **Started**: 2026-10-02
- **Phase**: 4-milestone (M1)
- **Next step**: Continue M1 from *Handover*: first build of the WIP commit (core green, daemon red), daemon green, cycle 6 (client), T016, then the gate and reviews A and B, then PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #528 | Spec | merged | 824e0a9bc58ad5f977ef56fa813e58217c563ea5 |
| #539 | Design | merged | 6b4b6fa9afc27f134a5f4fc80048dc3f9a115a38 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016, T123 | full | The service knows what is in view and counts attention events (integration test; nothing new on screen) | | in progress |
| M2 | T017–T034 | full | One desktop notification on Linux for a session not in view | | pending |
| M3 | T035–T044, T118, T122 | full | The same notification on macOS and Windows | | pending |
| M4 | T045–T061 | full | The unread mark on a session's row, kept across restarts | | pending |
| M5 | T062–T072, T119 | full | Unread counts on the switcher's rows and button | | pending |
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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 188c880b773ff483e9791fdb9842587dcb7f9515:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR ("in view" defined two ways, unread after reconnection undefined, click with no window open undefined), 3 MINOR; all six fixed |
| Spec | 2 | b217dc29a9ef4507039fd9ddace7063a33e1ac63:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN (1 MINOR: FR-019 did not name leaving Settings; fixed) |
| Plan | 1 | 7dfdd00bfdb595aa473a2abd3e119814606b521d:21c79c9d9bf6fb59b7945d2b4bd0596a9a9aefe7 | CHANGES: 5 MAJOR (`SessionChanged` is never sent, the carrier is `CatalogChanged`; the Wayland token did not reach the window that is raised; no rule for a click after the raising window closed; the raise decision was untested glue; no test layer for FR-015, FR-015a, FR-017, FR-025, FR-028), 3 MINOR (an event while the switch is off claimable after reconnecting; counts need the session in view; `SettingsSet` field is an `Option`); all eight fixed |
| Plan | 2 | 6f5590e2f6b7bd8ef3510df31aee2a75532299d2:80af04fa661ea74bd8f32172b5dd8b92c2813ecb | CLEAN (1 MINOR: wrong line cited for the second `note_activity` caller; fixed) |
| Tasks | 1 | c85dac2f537bb711fcce428ed56ceeef88af19dc:897f84cedd8e9e405d34fa1ff8b79c1fbf646097 | CHANGES: 5 MAJOR, 3 MINOR. Fixed: F1 the macOS and Windows backend tests ran on no CI leg (new T122 edits `ci.yml`; client test files join the enumerated list in T006, T051, T063, T077); F3 T118 would be ticked in M2 (markers removed); F4 a failed probe left T092–T100 open and version 25 untaken (T091 closes them as `DROPPED`; versions are "next free"); F5 the rules of the view report sat untested in `main.rs` (new T123 `State::view_facts` in the lib, tested in T006 with `[A9]`, `[A10]`, U176–U178; this replaces the reason given for analysis finding G6); F6 dependencies (M4 on M2, M6 on M3 and M4, M8 on M7); F7 the "next free number" rule in R10, wire.md and the plan; F8 the Windows notification-centre sentence moved to T090, milestone ids in D15 and D16, the header's glue list. F2 (the D15 Edge Case narrowed FR-011 by agent decision): escalated, decided by the user (D19), FR-015 and FR-015a amended, `checklists/requirements.md` line 17 ticked again |
| Tasks | 2 | 87798c9ef7f66cc737705e1027bdd8ac800df22f:3e048fa2f8aedc3fdf5b0c9a8a0acfe60351f003 | CLEAN (1 MINOR: T016 named no dependency on T123; fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1, handed over a second time at the 150k context cap (2026-10-03). No PR is open. The branch is
rebased on `origin/main` (`34cd2c85`); do NOT run `branch-start.sh` again unless `main` moved (it is
harmless: it rebases). The stash entry `039-m1-cycles-2-3-red-wip` is applied and committed; it is
dropped.
- **Done and observed**: cycle 1 (committed by part 1). Red of cycles 2 and 3 (`tdd/cycle-log.md`).
- **Written, committed as WIP, NEVER BUILT** (commit `wip(039): M1 cycles 2 to 5`):
  - core green (T008, T009): `StoredSession::attention_seq`, `ClientMsg::WindowView`, the
    `WindowView` struct and `SessionSummary::attention_seq` in `protocol/messages.rs` (with
    `mod attention_wire_tests`), `PROTOCOL_VERSION` 21, samples in `tests/protocol_roundtrip.rs`.
    `attention_seq: 0` added to the `SessionSummary` literals of the client (`catalog_sync.rs`,
    `shell/daemon_sync.rs`, `tests/start_failure_notice.rs`, `tests/session_title_sync.rs`).
  - cycle 4 tests and STUB (T004/T010): `crates/micold-daemon/src/attention.rs` (`Views`:
    `set_view`, `remove` do nothing, `is_in_view` answers `false`), `pub mod attention` in `lib.rs`.
  - cycle 5 tests and STUBS (T005/T011–T013): `crates/micold-daemon/tests/attention_events.rs`
    (U69–U79 and A3); `Catalog::mark_attention` returns `Ok(false)`; `session_summary` sends
    `attention_seq: 0`; `Inner::views` and `DaemonState::set_window_view` (does nothing) in
    `state.rs`; the `ClientMsg::WindowView` arm in `server.rs` (final).
- **Next step** (one build-lock run, detached, `hold.sh`): `scripts/build-lock.sh bash -c 'cargo test -p micold-core --all-targets --no-fail-fast; cargo test -p micold-daemon --lib attention:: --test attention_events --no-fail-fast'`.
  Expected: core all green (cycles 2, 3 green); daemon red: `Views` tests U50, U52, U53 fail, U51
  passes (mutant later: store the report as it came); `attention_events` U69, A3, U72, U73, U74,
  U75, U77 fail, U70, U71, U76, U78, U79 pass (mutants later). Fix any compile error first: nothing
  was compiled. Then the green:
  - `Views`: `set_view` inserts `WindowView { focused, in_view: if focused { in_view } else { None } }`;
    `remove`; `is_in_view` is `views.values().any(|v| v.in_view == Some(session))`.
  - `Catalog::mark_attention`: `find_session_mut`, `attention_seq += 1`, `persist()?`, `Ok(true)`
    (copy `record_session_name`, `catalog.rs:626`); `session_summary`: `attention_seq: session.attention_seq`.
  - `state.rs`: `set_window_view` calls `self.lock().views.set_view(id, view)`; `deregister` adds
    `inner.views.remove(id)`; `note_activity` (after `live.name_stale |= …`): when `before` is not
    `AwaitingInput`, the signal after is, and `!inner.views.is_in_view(session)`, call
    `inner.catalog.mark_attention(session)` and `tracing::warn!` on `Err` (reborrow the guard as
    `let inner = &mut *inner;` so `sessions`, `views` and `catalog` borrow apart).
  - Mutants, each one targeted run, restored from a copy (not `git checkout`): U6 drop
    `#[serde(default)]`; U8 bump `SCHEMA_VERSION`; U12 `#[serde(skip)]` on
    `SessionSummary::attention_seq`; U51 store the report as it came; U70/U76 the `server.rs` arm
    does nothing; U71 drop the `before` condition; U79 count any change of signal; U78 is a
    characterization of existing behaviour (`sessions_for` filters `archived`).
  - `docs/daemon.md:481` says "It is version 20 today": make it 21.
- **Cycle 6 (client; T006, T014, T015, T123) and T016: not started. Design, decided here because
  the gate `tests/feature_registration_cost.rs::only_the_root_drives_a_feature` forbids `main.rs`
  and `shell/` to call a feature function that takes a `State` mutably** (record it under
  *Decisions* as agent-resolved when it is built):
  - `crates/micold-client/src/features/attention.rs`: `pub struct State { pub sent_view: Option<WindowView> }`
    (`Debug, Clone, Default, PartialEq, Eq`, as `features/help.rs:36`);
    `pub fn view_report(state: &mut State, facts: ViewFacts) -> Option<WindowView>` (the feature's
    own `State`): derives `WindowView { focused: facts.window_focused, in_view: micold_core::attention::in_view(facts) }`,
    returns it and stores it when it differs from `sent_view`, else `None`;
    `pub fn connection_started(state: &mut State)` sets `sent_view = None`. No `Msg` enum.
  - `pub mod attention;` in `features/mod.rs`; `pub attention: crate::features::attention::State`
    on `app::State`; `State::view_facts(&self, window_focused) -> ViewFacts` beside
    `terminal_focused` (`app.rs:342`): `main_area_taken: self.settings.settings_draft.is_some()`,
    `selected: self.session.active` (check that `session.active` is the active project's selected
    session). Two root helpers in `app.rs`, because only `app.rs` may call a feature's reducer:
    `State::view_report(&mut self, window_focused: bool) -> Option<WindowView>` and
    `State::view_report_forgotten(&mut self)`.
  - `main.rs:417` (in `update`, beside `shell::daemon_sync::report_color_scheme(app)`): a
    `shell::daemon_sync::report_window_view(app)` that calls `app.core.view_report(app.window_focused)`
    and sends `ClientMsg::WindowView`; only while `app.daemon` is `Some`, or the report is marked
    sent without being sent. `daemon_sync.rs:264` and `:1119` clear `reported_scheme`: call
    `app.core.view_report_forgotten()` beside both, and `report_window_view(app)` after
    `report_color_scheme(app)` at `:1120`. Check where `Welcome` is handled relative to `:1119`
    (W1.1: the first report follows `Welcome`).
  - Tests: `tests/attention_view_report.rs` (U111–U115, U176–U178; T006), and
    `tests/features_attention.rs` must exist (`every_feature_module_has_an_isolation_test`): put
    the `view_report` tests that build only `features::attention::State` there, or a short
    isolation test, and keep T006's file for the `app::State` ones. `catalog_sync.rs` tests: U116,
    then copy `existing.attention_seq = summary.attention_seq` at `catalog_sync.rs:118` and set it
    on the `Session::restored` branch. Add `--test attention_view_report` (and
    `--test features_attention`) to the `cargo test -p micold-client` list at
    `.github/workflows/ci.yml:295`. Other client gates to expect: `feature_write_isolation.rs`
    (`every_state_field_has_an_owner`, `every_method_called_on_state_is_classified`),
    `root_state_is_shared.rs`, `root_is_routing_only.rs`.
- **Then**: tick T002–T006, T008–T016, T123 in `tasks.md`; set the test list rows to their state;
  finish the cycle log; squash or reword the WIP commit; phase steps 2 to 4 (gate with review A,
  review B, PR with `docs-not-needed`, body ends `Refs #481`).
- **Build lock**: waits of 10 minutes and more. One detached run per step, `hold.sh` on its log.
- **Open findings**: none. No review has run.

## Open escalation

None.

## Token usage

## Follow-ups not done

- `crates/micold-daemon/tests/mcp_create_session.rs::a_pi_session_start_event_makes_pi_ready` failed once in a full-suite run on a docs-only branch (`left: ""`, line 600) and passed when rerun alone: a timing flake in code this flow does not own. Not fixed here.
