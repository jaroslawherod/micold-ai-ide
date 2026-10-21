# Tasks: Jira Ticket Status

**Input**: `specs/1101-jira-ticket-status/` (plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md)

**Tests**: MANDATORY (Constitution I): in each story the test tasks come first and must fail before the implementation tasks.

**Documentation**: each story's user-guide task is in that story's phase (Constitution VII).

## Format: `- [ ] [ID] [P?] [Story] Description with file path`

## Phase 1: Setup

- [X] T001 Create `crates/micold-core/src/jira_status.rs` as an empty module and register it in `crates/micold-core/src/lib.rs` (`pub mod jira_status;`)

## Phase 2: Foundational (types and parsing shared by every story)

- [X] T002 [P] Write failing core unit tests in `crates/micold-core/src/jira_status.rs` for `TicketStatus::from_jira(name, category_key)` and `StatusCategory`: `new`/`indeterminate`/`done` map to ToDo/InProgress/Done, unknown key gives `category: None`, blank or missing name gives `None`, name is trimmed; `accessible_text(false/true)` gives "In Review" / "In Review, may be out of date" (FR-001, FR-009, FR-013)
- [X] T003 Implement `StatusCategory` (enum `ToDo | InProgress | Done`) and `TicketStatus { name: String (trimmed, non-empty), category: Option<StatusCategory> }` with `from_jira` and `accessible_text(stale: bool)` ("In Review" / "In Review, may be out of date", FR-013) in `crates/micold-core/src/jira_status.rs`
- [X] T004 [P] Write failing tests in `crates/micold-core/src/jira.rs` (and `github.rs` where `Issue` lives): `parse_page` reads `fields.status.name` and `statusCategory.key` into `Issue::status`; a missing status gives `None`; `list_jql`/`search_jql` are unchanged (FR-001, FR-011)
- [X] T005 Add `status: Option<TicketStatus>` and `Issue::with_status` to `Issue` in `crates/micold-core/src/github.rs` (None for GitHub issues; no effect on matching or ordering); make `parse_page` in `crates/micold-core/src/jira.rs` set it
- [X] T006 Write a failing test in `crates/micold-client/src/shell/jira_http.rs` that the list and search request bodies ask for `status`, then add `"status"` to `FIELDS` (now `[&str; 7]`) there

## Phase 3: User Story 1 - status in the issue picker (P1) 🎯 MVP

**Goal**: picker rows (list and search) show the ticket's status chip.
**Independent test**: open the form with the fake Jira source; rows show To Do, In Progress, In Review.

- [X] T007 [P] [US1] Write failing client unit test in `crates/micold-client/src/ui/material/status_chip.rs`: `StatusChip` tint class differs for ToDo, InProgress, Done and no-category (FR-004); `None` status draws nothing (FR-009); the chip's tooltip equals `accessible_text` (FR-013)
- [X] T008 [P] [US1] Write failing client integration test in `crates/micold-client/tests/jira_status_picker.rs` with `FakeJiraSource`: list and search rows each show their status name, a Jira row without status shows no chip, GitHub rows are unchanged (US1 scenarios 1-4, FR-002)
- [X] T009 [P] [US1] Write failing geometry test in `crates/micold-client/tests/jira_status_geometry.rs`: a long status name truncates without moving key/title; a row without a chip has unchanged layout (FR-009, edge case)
- [X] T010 [US1] Implement `StatusChip::new(&TicketStatus).stale(bool)` built on `Tag` in `crates/micold-client/src/ui/material/status_chip.rs` (ToDo = secondary role, InProgress = primary, Done = tertiary, none = neutral; stale = dimmed; `Ellipsized` within a max width; accessible text as tooltip), and export it from `crates/micold-client/src/ui/material/mod.rs`
- [X] T011 [US1] Draw the chip beside the key in the picker row in `crates/micold-client/src/ui/worktree_form.rs` from `Issue::status`
- [X] T012 [P] [US1] Update `crates/micold-client/examples/fake_jira.rs` to report statuses (To Do, In Progress, In Review, Done) with categories on list and search
- [X] T013 [US1] Update `docs/user-guide/worktrees-and-sessions.md` (picker shows each Jira ticket's status)

## Phase 4: User Story 2 - linked ticket's current status on the sidebar (P1)

**Goal**: each linked worktree's sidebar row shows its ticket's current status, refreshed on project open and on focus after 5 minutes.
**Independent test**: change the fake source from In Progress to In Review, trigger focus after the clock passes 5 minutes: the row shows In Review.

- [x] T014 [P] [US2] Write failing core tests for `statuses_jql(project, keys)` in `crates/micold-core/src/jira_status.rs`: keys validated `<PROJECT>-<digits>` of the connected project and uppercased, others dropped (no quote can enter the JQL), chunks of 100 in order (101 keys give 2 chunks), no done filter (FR-007)
- [x] T015 [US2] Write failing core tests for `parse_statuses` in `crates/micold-core/src/jira_status.rs`: `{"issues":[{"key","fields":{"status":{"name","statusCategory":{"key"}}}}]}` gives `(key, Option<TicketStatus>)`; non-JSON or no `issues` gives `NotJiraCloud`
- [x] T016 [US2] Write failing core tests for `StatusBoard` in `crates/micold-core/src/jira_status.rs` per data-model.md: `due(now, Open|Focus)` (Focus only when `last_read` is None or older than 5 min, false while `in_flight`), `begin` (empty keys gives None), `apply` Ok (replace, remove absent, `failures = 0`, `last_read = now`), Err (all `stale`, `failures += 1`, third clears), stale-epoch apply dropped, `clear()` bumps epoch (FR-005, FR-008, FR-010)
- [x] T017 [P] [US2] Write failing core/client tests for `JiraCloud::statuses` in `crates/micold-client/src/shell/jira_http.rs`: POST body shape, same headers/timeout/no redirects/redaction, a `400` bisects the chunk (depth cap 7; the bisection lives in `jira_http.rs`, the client crate) and a key alone in a `400` is absent, other errors abort the read (contract §3)
- [x] T018 [P] [US2] Write failing client integration tests in `crates/micold-client/tests/jira_status_sidebar.rs` with `FakeJiraSource`: sidebar row shows the chip beside the key tag; follows a changed fake after a focus past 5 minutes with an injected clock (SC-002); Done status stays shown; failure keeps a dimmed chip and no dialog; not-found retires the chip; a key of another project shows no chip; switching the integration off or removing the connection clears chips; a 429 leaves the last status and makes no extra request before the next FR-005 event (US2 scenarios 1-5, SC-005, FR-008, edge cases)
- [x] T019 [P] [US2] Extend `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs`: statuses are requested only on project open and on focus past 5 minutes; zero requests with the switch off or the project unconnected (FR-006, FR-006b, SC-004); the restored-project path at startup triggers one read; two triggers at once make one request (coalescing); 50 linked tickets give 1 request and 101 give 2 (SC-003); the request runs off the UI thread (the update returns before the source answers); extend `jira_token_never_leaks` to the statuses path (FR-012), including the chip tooltip and accessible text
- [x] T020 [US2] Implement `statuses_jql`, `parse_statuses` and `StatusBoard` (`StatusEntry { status, stale }`, `due`, `begin`, `apply`, `clear`, `status_of`) in `crates/micold-core/src/jira_status.rs`
- [x] T021 [US2] Add `JiraSource::statuses` (default body `Ok(vec![])`) and `FakeJiraSource::with_statuses` / `status_calls()` in `crates/micold-core/src/jira.rs`; implement `JiraCloud::statuses` in `crates/micold-client/src/shell/jira_http.rs`; update the `Recording` impl in `crates/micold-client/src/main_tests.rs` where a test needs statuses
- [x] T022 [US2] Widen `jira_request` / `JiraRequest` to `pub(crate)` in `crates/micold-client/src/shell/issues.rs`; add the refresh driver `crates/micold-client/src/shell/jira_refresh.rs` (gate through `jira_request`, linked Jira keys, blocking-thread request, `JiraStatusesRead { keys, result }` message applying `StatusBoard::apply`, `on_project_changed`) and the board field on the core state in `crates/micold-client/src/shell/mod.rs` (`app.core`; use the file that defines it if different)
- [x] T023 [US2] Wire the triggers: `WindowFocusChanged(true)` in `crates/micold-client/src/main.rs`; project change in `crates/micold-client/src/shell/daemon_sync.rs` and the restored project in `crates/micold-client/src/shell/startup.rs`; `clear()` on switch off, connection removed and project switch
- [x] T024 [US2] Draw `StatusChip` (from `StatusBoard::status_of(key)`, `.stale(entry.stale)`) right of the `Tag::Issue` tag in `crates/micold-client/src/ui/sidebar.rs`
- [x] T025 [US2] Add the FR-006b disclosure text (linked tickets' keys sent with the project key and credentials, also with the dialog closed) in `crates/micold-client/src/ui/settings/jira.rs` and the picker's disclosure in `crates/micold-client/src/ui/worktree_form.rs`, with a test that it is shown
- [x] T026 [P] [US2] Update `fake_jira.rs` (`crates/micold-client/examples/fake_jira.rs`) to serve `key in (...)` searches and let a script change a ticket's status
- [x] T027 [US2] Update `docs/user-guide/settings.md` (disclosure, refresh events) and `docs/user-guide/worktrees-and-sessions.md` (sidebar chip, dimmed on failure)

## Phase 5: User Story 3 - Overview card (P2)

**Goal**: the Overview card of a worktree linked to a Jira ticket shows the same chip.
**Independent test**: same ticket on sidebar and card shows the same status.

- [x] T028 [P] [US3] Write failing client integration test in `crates/micold-client/tests/jira_status_overview.rs`: the card for a Jira-linked worktree shows the chip from the same board as the sidebar; a ticket shown in two places shows the same status (US3 scenarios 1-2, FR-010)
- [x] T029 [US3] Add an optional chip slot to the card builder in `crates/micold-client/src/ui/material/overview_card.rs` and fill it in `crates/micold-client/src/features/overview.rs` from the board by the Jira key from `micold_core::naming::ticket`
- [x] T030 [US3] Update `docs/user-guide/overview.md` (status chip on cards)

## Dependencies and order

Setup -> Foundational -> US1 (MVP) -> US2 (needs `StatusChip`, `TicketStatus`) -> US3 (needs the board from US2). T022 needs T020 and T021; T026 needs T012. Within a story, tests before implementation. `[P]` tasks touch different files.

## Implementation Strategy

MVP is US1: the picker reads status for free from the requests that already run. US2 adds the one new request kind and its gate. US3 is a card slot. The quickstart Part B visual pass and the task ticks are done by the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Status chip in the issue picker 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: Jira ticket rows in the New worktree picker (list and search) show a tinted status chip with the status name.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-004, FR-009, FR-011, FR-013; SC-001
- **Verify**: `mise run test-core` and `cargo test -p micold-client --test jira_status_picker`
- **Depends on**: —
- **Tier**: full

### M2 — Current status on the sidebar

- **Tasks**: T014–T027
- **Deliverable**: A worktree linked to a Jira ticket shows the ticket's current status beside its key tag, refreshed on project open and on focus after 5 minutes, dimmed when a refresh fails.
- **Satisfies**: US2 acceptance scenarios 1–5; FR-003, FR-005, FR-006, FR-006b, FR-007, FR-008, FR-010, FR-012; SC-001 to SC-005
- **Verify**: `mise run test-core`, then `cargo test -p micold-client --test jira_status_sidebar --test jira_is_requested_only_on_named_events`
- **Depends on**: M1
- **Tier**: full

### M3 — Status on Overview cards

- **Tasks**: T028–T030
- **Deliverable**: Overview cards of Jira-linked worktrees show the same status chip as the sidebar.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-010
- **Verify**: `cargo test -p micold-client --test jira_status_overview`
- **Depends on**: M2
- **Tier**: full
