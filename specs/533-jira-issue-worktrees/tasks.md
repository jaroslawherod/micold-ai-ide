---

description: "Task list for creating worktrees from Jira issues behind an opt-in integration switch"
---

# Tasks: Create Worktrees from Jira Issues, Behind an Opt-In Integration Switch

**Input**: Design documents from `/specs/533-jira-issue-worktrees/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Constitution I (Test-First) makes tests mandatory. Within each story, every test task sits
before the implementation it covers and is written failing first. `src/ui/` composition is the only
GUI exception (quickstart §B).

**Documentation**: each story's user-guide task is in that story's milestone (Constitution VII; CI's
`scripts/check-user-guide-updated.sh`).

**Cross-platform**: one code path; the token file uses the existing `owner_only` arms; the
`micold-core` suite runs on all three CI OSes.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: different files, no dependency on an incomplete task
- Paths are from the repository root. `core` = `crates/micold-core`, `daemon` = `crates/micold-daemon`,
  `client` = `crates/micold-client`.
- Run `mise run test-core` for core tests and `mise run gate` before every push.

## Phase 1: Setup

- [X] T001 Add `pub mod jira;` to `crates/micold-core/src/lib.rs` and an empty `crates/micold-core/src/jira.rs` with the module doc (render-free, Jira-specific rules only, per plan R1); create `crates/micold-core/tests/fixtures/jira/` with a `README.md` line naming the Cloud v3 captures it will hold.
- [X] T002 Make `store::project_id` reachable through a public accessor: keep `fn project_id` private and add `pub fn project_id_of(path: &Path) -> String` (delegating to it) in `crates/micold-core/src/store.rs`, with a doc comment that the Jira token file name uses it; add a test in the existing store test module that the accessor equals the id of `project_state_path`'s file stem.

## Phase 2: Foundational (blocking: the validated connection types and the service-owned settings fields)

**Purpose**: the switch and the connection fields exist in `Settings`, on the wire and in the daemon, so US2, US3 and US1 can build on them. Nothing requests Jira yet.

### Tests first

- [X] T003 [P] Write `crates/micold-core/tests/jira_connection.rs`: `SiteOrigin::parse` accepts `https://host[:port]`, drops path and trailing slash, refuses `http`, user-info, no host, query or fragment (and accepts loopback `http` only under `cfg(debug_assertions)`); `ProjectKey::parse` trims, upper-cases and accepts only `[A-Z][A-Z0-9_]{1,9}`; `AccountEmail::parse` needs one `@`, non-empty local part, domain with a dot, no whitespace; `JiraConnection::parse` reports every refused field at once as `Vec<FieldProblem>` (FR-006, US3 AS5).
- [X] T004 [P] Write `crates/micold-core/tests/jira_site_release.rs`, `#[cfg_attr(debug_assertions, ignore)]` like `typeahead_budget.rs`: in a release build `SiteOrigin::parse("http://127.0.0.1:8080")` is refused (FR-006, R10).
- [X] T005 [P] Write `crates/micold-core/tests/settings_jira.rs`: `jira_enabled` defaults false and `jira_connections` empty; a settings file written before this feature reads as off with every other field unchanged and `SETTINGS_VERSION` still 4 (US2 AS6); round trip of an enabled switch and a connection; a stored connection that no longer parses is dropped on read, like `known_entries`.
- [X] T006 [P] Update the `PROTOCOL_VERSION` expectation and the `SCHEMA_HASH` pin in `crates/micold-core/tests/schema_hash.rs` to 43 (it fails until T011); add a `protocol` message test in `crates/micold-core/tests/` that `SettingsSet` and `SettingsChanged` round trip `jira_enabled` and a per-project `JiraConnectionChange` (upsert and removal).
- [X] T007 [P] Write `crates/micold-daemon/tests/jira_setting.rs`: `SettingsSet{jira_enabled}` is stored and echoed as `SettingsChanged` to a second connected client; a `JiraConnectionChange` upsert and a removal for one project are applied to the stored map and echoed; two projects' changes arriving back to back both survive (no whole-map overwrite); a save of an unrelated field keeps `jira_enabled` and `jira_connections`; the daemon never reads them (FR-007, FR-021, FR-022).
- [X] T008 [P] Add the daemon catalog tests in the test module of `crates/micold-daemon/src/catalog.rs`: (a) two catalogs on one store, one saves a connection through the Jira setter, the other then saves an unrelated service setting, and the connection is still on disk (the plan's "several daemons share settings.json" risk); (b) an unrelated setter's save leaves the Jira map untouched; (c) `persist_service_settings` copies `jira_enabled` but never `jira_connections`.

### Implementation

- [X] T009 Implement `SiteOrigin`, `ProjectKey`, `AccountEmail`, `FieldProblem`/`JiraField` and `JiraConnection::parse` in `crates/micold-core/src/jira.rs` per data-model §1 and contracts/jira-connection.md §2, including the refusal texts ("Enter the https address of your Jira site, e.g. https://acme.atlassian.net.", "Enter the project key, e.g. ABC.", "Enter the email of your Atlassian account."); `JiraConnection` derives serde and re-validates on deserialisation (T003, T004 green).
- [X] T010 Add `jira_enabled: bool` (default false) and `jira_connections: BTreeMap<String, JiraConnection>` to `Settings` and `StoredSettings` in `crates/micold-core/src/settings.rs`, both `#[serde(default)]`; add a `JiraConnectionChange { project, change: Upsert(JiraConnection) | Remove }` type; keep `SETTINGS_VERSION` 4 (T005 green).
- [X] T011 Carry `jira_enabled` and `jira_connection: Option<JiraConnectionChange>` on `SettingsSet` and `jira_enabled` plus the full `jira_connections` map on `SettingsChanged` in `crates/micold-core/src/protocol/messages.rs`; raise `PROTOCOL_VERSION` 45 -> 45 in `crates/micold-core/src/protocol/version.rs`; update every `SettingsSet`/`SettingsChanged` construction the compiler flags, including fixtures (T006 green).
- [X] T012 Daemon: in `crates/micold-daemon/src/state.rs` and `server.rs` add the setters beside `set_pr_status_enabled`: `set_jira_enabled`, and `set_jira_connection` which merges its `JiraConnectionChange` into the map read from disk inside its own `update_reporting` closure (never a boot-snapshot copy); broadcast the echo; the daemon stores the fields and never reads them (FR-021).
- [X] T013 Daemon: in `crates/micold-daemon/src/catalog.rs` make `settings_wire` carry both fields; `persist_service_settings` copies `jira_enabled` like the other service fields and does NOT touch `jira_connections` (T007, T008 green).
- [X] T014 Client: wherever `pr_status_enabled` is applied from `SettingsChanged` and sent in `SettingsSet` (grep `pr_status_enabled` in `crates/micold-client/src/`), carry `jira_enabled` and `jira_connections` the same way, so every window re-derives from the echo (FR-022); add the failing-then-green reducer assertion in `crates/micold-client/tests/features_settings.rs`.

- [X] T034 Extend the existing release CI step in `.github/workflows/ci.yml` (the "Test (type-ahead frame budget, release)" step) to also run `cargo test --release -p micold-core --test jira_site_release` (R10), so the release-only site-origin test actually runs.

**Checkpoint**: `mise run gate` green; the switch and connections persist and echo; nothing requests Jira.

## Phase 3: User Story 2 - Nothing about Jira exists until I turn it on (Priority: P1) 🎯 MVP

**Goal**: default settings look and behave exactly as before; the only Jira surface is the off switch in Settings; turning it on shows the enabled status.

**Independent Test**: with default settings open Settings and the dialog and compare; enable and see the status line; restart and see it persisted.

### Tests first (US2)

- [X] T015 [P] [US2] Extend `crates/micold-client/tests/features_settings.rs`: `SettingsSection::Jira` exists and `JiraDraft { enabled }` follows the persisted switch; with the switch off the Jira page model holds only the switch and its one-line description; toggling on yields the status "Open a project to connect it to Jira." with no project and "Enabled. This project has no Jira connection yet." with one (US2 AS1, AS3, FR-001, FR-004).
- [X] T016 [P] [US2] Write `crates/micold-client/tests/jira_opt_in_gate.rs`: with default `Settings`, `jira_visible(settings)` is false, no create-worktree form state names Jira and no Jira request function is reachable (SC-001, US2 AS1-2); extend `crates/micold-client/tests/support/covered_states.rs` with "Settings Jira off" and "Settings Jira on, no connection" and regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`.
- [X] T017 [P] [US2] Extend `crates/micold-client/tests/icons.rs` expectations: the Jira rail icon uses an existing Material Symbols codepoint that no other icon shares.

### Implementation (US2)

- [X] T018 [US2] Add `jira_visible(&Settings) -> bool` in `crates/micold-core/src/jira.rs` (the one gate every Jira control and request checks, issue-picker-ui §1).
- [X] T019 [US2] Add `SettingsSection::Jira`, `JiraDraft { enabled, .. }` and `JiraEnabledToggled(bool)` to `crates/micold-client/src/features/settings.rs`; Save sends `SettingsSet{jira_enabled}` (T015 green).
- [X] T020 [P] [US2] Create `crates/micold-client/src/ui/settings/jira.rs`: the switch **Enable Jira integration** with "Lets Micold read open issues of a Jira Cloud project to start worktrees from. Off: nothing is sent to Jira.", the status line, and the FR-005b data-use line beside the switch; add the rail entry and glyph in `crates/micold-client/src/icons.rs` and register the page in the Settings view (T016, T017 green).
- [X] T021 [US2] User guide: add the "Jira" section to `docs/user-guide/settings.md` covering the switch, what enabling does and that off sends nothing (FR-024, first part).

**Checkpoint**: US2 AS1-3 and AS6 hold; AS4 completes with the picker (M3), AS5 with the connection (M2).

## Phase 4: User Story 3 - Connect, change and remove a Jira connection (Priority: P2)

**Goal**: enter site, key, email and token in Settings; see it described; remove it and delete the token.

**Independent Test**: save, restart, confirm persistence and no token in any file; remove and confirm the token file is gone.

### Tests first (US3)

- [x] T022 [P] [US3] Add to `crates/micold-core/tests/jira_connection.rs`: `ApiToken::parse` refuses empty and whitespace-only input, `Debug` prints `ApiToken(<redacted>)`, `jira_token_dir()` in `crates/micold-core/src/store.rs` is `<data_dir>/jira` and the token path is `<dir>/<project_id_of(path)>.token`; the absence of `Serialize`/`Display` on `ApiToken` is held by the leak scan T024 (it greps `jira.rs` for a derive on `ApiToken`).
- [x] T023 [P] [US3] Extend `crates/micold-client/tests/features_settings.rs`: `JiraDraft` fields `site`, `project_key`, `email`, `token`, `token_stored`, `project`, `remove_requested`; `validate` reports `FieldError { section: Jira, field }` for each refused field and saving stores nothing (US3 AS5); a blank token with `token_stored` keeps the stored one; turning the switch off with a stored connection raises `KeepOrRemovePrompt` and the draft is not applied until answered, Keep leaves it stored, Remove deletes it (US3 AS4, US2 AS5); the token message type redacts in `Debug`.
- [x] T024 [P] [US3] Write `crates/micold-client/tests/jira_token_never_leaks.rs` (save, remove part): with a sentinel token, run save and remove through the shell fakes and scan the settings file, project files, wire frames, `Debug` of every type reachable from `App` and the log buffer for the sentinel (US3 AS6, SC-004); later milestones extend it with load and each failure kind.
- [x] T025 [P] [US3] Write `crates/micold-client/tests/jira_connection_shell.rs`: save writes the owner-only token file only when a new token was typed, then sends `SettingsSet{jira_connection: Some(upsert)}`; remove deletes the token file (ignoring not-found) then sends the removal; the token probe is `Path::exists` and runs on Settings open and `SettingsChanged` (contracts/jira-connection.md §4); the token file is mode 0600 in a 0700 directory on Unix.
- [x] T026 [P] [US3] Extend `crates/micold-client/tests/support/covered_states.rs` with "Settings Jira on with connection", "on with a field error" and "keep-or-remove prompt"; regenerate `layout_snapshot.txt`.

### Implementation (US3)

- [x] T027 [US3] Add `ApiToken` (no `Serialize`/`Deserialize`/`Display`, redacting `Debug`, one `expose`) to `crates/micold-core/src/jira.rs` and `jira_token_dir()` to `crates/micold-core/src/store.rs` (T022 green).
- [x] T028 [US3] Create `crates/micold-client/src/shell/jira.rs` (declare it in `crates/micold-client/src/shell/mod.rs`) with the connection operations of contracts/jira-connection.md §4: save, remove, switch off keep/remove, token probe; the token is written with `owner_only::write` and read only here (T025, T024 green).
- [x] T029 [US3] Extend `crates/micold-client/src/features/settings.rs`: the draft fields, the messages `JiraSiteChanged`, `JiraKeyChanged`, `JiraEmailChanged`, `JiraTokenChanged`, `JiraRemoveRequested`, `JiraKeepOrRemoveChosen`, `validate` over `JiraConnection::parse` and the token rule, and `KeepOrRemovePrompt` (T023 green).
- [x] T030 [US3] Extend `crates/micold-client/src/ui/settings/jira.rs`: fields **Site URL**, **Project key**, **Account email**, **API token** (masked; placeholder "A token is stored" when `token_stored`), `FieldError` under each field, **Remove connection**, the "Keep the Jira connection for {project}?" prompt with **Keep**/**Remove**, and the status line "Enabled. Connected to {site} - {KEY}."; reuse `TextField`, `Switch`, `Button` as the GitHub page does (T026 green).
- [x] T031 [US3] User guide: extend the "Jira" section of `docs/user-guide/settings.md` with connecting a project, changing and removing it, and where the token file is stored per OS.

**Checkpoint**: a connection can be saved, persists across restart and is removed with its token; nothing requests Jira.

## Phase 5: User Story 1 - Start a worktree from a Jira issue (Priority: P1)

**Goal**: with the switch on and a connection, the dialog lists the project's open Jira issues, a pick fills ticket, name and type, and creating gives `${type}/${KEY-123}_${name}` with the issue tag; failures show a notice with Retry.

**Independent Test**: against the loopback fake Jira site (three issues), enable, save a connection, open the dialog, pick, and read ticket, name, type, preview and worktree name.

### Dependency vetting and setup (US1)

- [x] T032 [US1] Vet the new dependencies before locking them: record in `specs/533-jira-issue-worktrees/research.md` (R2 addendum) the licences of `ureq` 3, `rustls`, `rustls-platform-verifier`, `base64` and their transitive tree as resolved by `cargo tree -p micold-client -e normal` (every one MIT, Apache-2.0, ISC or equivalent permissive), which rustls crypto backend (`ring` or `aws-lc-rs`) the `ureq` features pull, and that it builds on every CI matrix leg (Linux, macOS, Windows, Windows ARM64: `aws-lc-rs` needs a C/NASM toolchain, `ring` needs only a C compiler); choose the feature set that builds without new CI toolchain steps, or escalate when it cannot.
- [x] T033 [US1] Add `ureq` 3 (features per T032, JSON off) and `base64` to `crates/micold-client/Cargo.toml` and update `Cargo.lock`; confirm `cargo tree -p micold-daemon` and `-p micold-core` show neither (plan Re-check).

### Tests first (US1)

- [x] T035 [P] [US1] Write `crates/micold-core/tests/jira_jql.rs`: `list_jql(key)` is `project = "KEY" AND statusCategory != Done ORDER BY updated DESC` (FR-011); no JQL carries anything but the key, search text and credentials-free fields (FR-020).
- [x] T036 [P] [US1] Write `crates/micold-core/tests/jira_parse.rs` with captured Cloud v3 bodies in `crates/micold-core/tests/fixtures/jira/`: key `ABC-123` becomes `Issue::jira`; a key without `-<digits>` is skipped; reporter is `displayName`, else the email local part, else the ghost login, never the account id; ADF description flattened over `text`, `hardBreak` and paragraph boundaries, capped at 600 chars; at most 20 labels; a non-JSON body or one without `issues` is `NotJiraCloud` (FR-011, FR-019, FR-023).
- [x] T037 [P] [US1] Write `crates/micold-core/tests/jira_classify.rs`: connect/DNS/TLS/proxy -> `Offline` "Couldn't reach {host}: {reason}." (reason <= 80 chars, no token); 10 s -> `TimedOut` "{host} did not answer within 10 seconds."; 401/403 -> `CredentialsRefused` naming Settings; redirect, Data Center shape or non-listing -> `NotJiraCloud`; the sentinel token is absent from every message (FR-018, US4 AS5-6, Edge "Data Center").
- [x] T038 [P] [US1] Write `crates/micold-core/tests/jira_load.rs`: the shared paging helper extracted from `github::load_listing` loads pages through `FakeJiraSource` until `isLast` or 1,000, marks the listing incomplete past the cap, and `github::load_listing` still passes its existing tests unchanged (FR-011, Edge ">1,000").
- [x] T039 [P] [US1] Write `crates/micold-core/tests/issue_jira_rows.rs` and extend `crates/micold-core/tests/issue_types.rs`: `Issue::ticket()` is the key or the number; `title_line()` and `row_text()` put the key in place of `#n`; `type_for_issue(mapping, type_name, labels)` picks the first mapping entry, nearest the top, matching the type name or any label ignoring case and surrounding spaces, `Bug` -> `fix` by default, none -> unset (FR-013, US1 AS3-4, AS6); `naming_for_issue` yields `feat/ABC-123_add-login-rate-limit` and the `ABC-123` tag (FR-015, US1 AS5); `Debug` still redacts reporter and description; a Jira key that the `naming.rs` issue-tag rule does not accept as a tag still yields a ticket the user can see and edit and a created worktree (Edge "Key that naming cannot read"); the `ProjectKey` rule `[A-Z][A-Z0-9_]{1,9}` is checked against that rule.
- [x] T040 [P] [US1] Write `crates/micold-client/tests/jira_http_adapter.rs` against a loopback `TcpListener` fake: the request is `POST /rest/api/3/search/jql` with `Authorization: Basic base64(email:token)`, `Accept`, `Content-Type`, `User-Agent: micold-ai-ide` and nothing else; the body holds only JQL, fields and page token; a redirect is not followed (`NotJiraCloud`); a stalled server times out at the 10 s budget (test with an injected shorter timeout); the sentinel token is absent from every error string and `Debug` (FR-009, FR-018, FR-020).
- [x] T041 [P] [US1] Write `crates/micold-client/tests/jira_source_state.rs` (reducer): `JiraAvailability` derives `Off`/`Connected` from settings; with no GitHub remote and Jira connected the form's provider is Jira and no source choice is offered (US4 AS3); `issues` leaves `NotRequested` for Jira only on `SourceChanged(Issue)` while `Connected` (invariant 3); `JiraSettingsChanged(Off)` resets issues, clears the query, bumps `issue_request_seq` and a late result drops (invariants 1, 4, US2 AS4); a Jira listing never enters `DescriptionPass::Loading` (invariant 5); pick fills ticket, name and type and a later hand edit stands (US1 AS3, AS7); failures give the notice, Retry reloads and the inputs keep working (US4 AS5-7); a connection with no token file on this host shows the "enter your token" state and makes no request, and a daemon with no stored connection shows "not connected" (plan risks, multi-daemon); a switch-off in one update leaves no request pending and no Jira control (SC-006); rotating the token surfaces the refused-credentials notice and saving a new token fixes it (Edge "Token rotated").
- [x] T042 [P] [US1] Write `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs`: the Jira source is called only from the load and retry arms, after `jira_visible` and a connection are checked in the same function (FR-001, FR-020, SC-001); extend `jira_token_never_leaks.rs` with load and each failure kind; scan that no Jira issue type derives `Serialize` (FR-019); extend `no_concrete_implementations` expectations so the adapter is constructed only in `Capabilities::real()`.
- [x] T068 [P] [US1] Extend `crates/micold-client/tests/features_settings.rs`: the Settings GitHub issues section's intro states that the label-to-type list also applies to Jira issue types and labels (FR-014); the default entries are unchanged so `Bug` selects `fix`.
- [x] T043 [P] [US1] Extend `crates/micold-client/tests/support/covered_states.rs` with "dialog Jira Loading", "Failed + Retry (credentials refused)", "Loaded with cap caption" and the zero-issues state; regenerate `layout_snapshot.txt`.

### Implementation (US1)

- [x] T044 [US1] `crates/micold-core/src/github.rs`: give `Issue` the private `key: Option<String>` and issue-type name, `Issue::jira`, `Issue::ticket`, `with_type_name`, key-aware `title_line`/`row_text`; add `IssueLoadError::{CredentialsRefused, NotJiraCloud}` and the provider-aware `message(&self, provider)`; extract the paging/cap helper from `load_listing` into one function both providers call (T038, T039 green).
- [x] T045 [US1] `crates/micold-core/src/jira.rs`: `list_jql`, `parse_page` (with ADF flattening), `classify`, the `JiraSource` trait, `FakeJiraSource` and `load`; define `IssueProvider { Github, Jira }` here (serde, `Default` = Github) so `micold-core::store` and the client share one type over the shared helper (T035-T038 green).
- [x] T046 [P] [US1] `crates/micold-core/src/issue_types.rs`: `type_for_issue(mapping, type_name, labels)` over the existing first-match order; `crates/micold-core/src/naming.rs`: `naming_for_issue` uses `Issue::ticket()` (T039 green).
- [x] T047 [US1] Create `crates/micold-client/src/shell/jira_http.rs` (declare it in `crates/micold-client/src/shell/mod.rs`): `JiraCloud` implements `JiraSource` with `ureq` 3 (10 s global timeout, redirects off, errors mapped to our enum at the boundary, `HTTPS_PROXY` from the environment, token read only at request time); add the factory to `crates/micold-client/src/shell/capabilities.rs` (T040 green).
- [x] T048 [US1] `crates/micold-client/src/shell/jira.rs` and `shell/issues.rs`: load and retry effects and the provider dispatch, checking `jira_visible` and the connection in the same function; a connection or token removed while loading bumps the seq (T041, T042 green).
- [x] T049 [US1] `crates/micold-client/src/features/worktree_form.rs`: `JiraAvailability` (importing `IssueProvider` from `micold_core::jira`), the `provider` and `jira` fields, messages `JiraSettingsChanged` and the provider-aware existing ones, invariants 1, 3-5 of data-model §4 (T041 green).
- [x] T050 [US1] `crates/micold-client/src/ui/worktree_form.rs`: the "Issue" chip label while Jira is connected, the FR-005b notice "Reads open issues of project {KEY} from {origin}. Sends the project key, your search text and your credentials.", Loading `StageProgress`, Failed caption + **Retry**, "{KEY} has no open issues.", the `Typeahead` with the cap caption, and the shared `naming_inputs` (T043 green).
- [x] T069 [US1] In `crates/micold-client/src/ui/settings/github.rs` add the "also applies to the issue type and labels of Jira issues" line to the GitHub issues section intro (T068 green).
- [x] T051 [P] [US1] Add `crates/micold-client/examples/fake_jira.rs`, a dev-only loopback server over the same fake used in T040 (scripted pages, a switch to refuse credentials or go silent) for quickstart §B; the `examples/` directory is new, Cargo auto-discovers it, and it reuses the T033 dependencies.
- [x] T052 [P] [US1] User guide: add "From a Jira issue" to `docs/user-guide/worktrees-and-sessions.md` (prerequisites, what is sent, each failure notice and its remedy) and the "also applies to Jira issue types and labels" sentence to the GitHub issues section of `docs/user-guide/settings.md` (FR-014, FR-024); say that in a project with a GitHub remote the picker stays on GitHub until the source choice ships in a later release of this feature.
- [x] T053 [P] [US1] `docs/development/architecture.md`: record that the Jira request runs in the client's `shell/jira_http.rs`, the new `ureq` dependency and why the daemon and core do not link it (Constitution VII).

**Checkpoint**: a Jira-only project lists issues, picks fill the form and the failure notices work.

## Phase 6: User Story 1 (continued) - Search

**Goal**: typing narrows the list by key, summary or label; beyond the loaded 1,000, Jira is asked.

**Independent Test**: type in the picker over the fake and see matches within the rank budget; with a cap-exceeding fake, a text beyond the loaded issues is searched on the site.

### Tests first

- [x] T054 [P] [US1] Extend `crates/micold-core/tests/jira_jql.rs`: `search_jql(key, text)` is `... AND (summary ~ "<t>*" OR labels = "<t>" [OR key = "<KEY-n>"])`; `text` trimmed, `\` and `"` escaped, control characters removed, at most 100 chars; the `key =` clause only when text matches `^[A-Za-z][A-Za-z0-9_]+-\d+$` after upper-casing with the project's prefix; empty text builds no search (FR-012, FR-020).
- [x] T055 [P] [US1] Extend `crates/micold-core/tests/issue_jira_rows.rs` and the existing 1,000-row case in `crates/micold-core/tests/typeahead_budget.rs` with Jira rows (release budget, SC-003); extend `jira_source_state.rs`: local narrowing by key, summary and label ignoring case (US1 AS2); a server search runs only when the listing is incomplete and the query non-empty after the 300 ms debounce, merges de-duplicated by number and keeps only locally matching results.
- [x] T056 [P] [US1] Extend `crates/micold-client/tests/jira_http_adapter.rs` and `jira_is_requested_only_on_named_events.rs`: the search request body carries only the search JQL; `search_open` is called only from the search-due arm under the same gate.

### Implementation

- [x] T057 [US1] `crates/micold-core/src/jira.rs`: `search_jql` and `search_open` on `JiraSource` and `FakeJiraSource` (T054 green).
- [x] T058 [US1] `crates/micold-client/src/shell/jira_http.rs` and `shell/jira.rs`: the search effect with the debounce and `merge_searched`, behind the same gate (T055, T056 green).

- [x] T070 [P] [US1] User guide: extend "From a Jira issue" in `docs/user-guide/worktrees-and-sessions.md` with searching by key, summary and label and that Jira is asked beyond the 1,000 loaded issues.

**Checkpoint**: US1 AS1-7 hold.

## Phase 7: User Story 4 - Choose between GitHub and Jira (Priority: P2)

**Goal**: a project with both sources offers a choice, remembered per project; Jira failing never blocks a hand-made worktree.

**Independent Test**: with a GitHub remote and a connection, switch sources and reopen; take the fake offline and create by hand.

### Tests first (US4)

- [x] T059 [P] [US4] Extend `crates/micold-core/tests/` (store round trip, e.g. `crates/micold-core/tests/store_issue_source.rs`): `StoredProjectState.issue_source: Option<IssueProvider>` is additive, `None` reads as GitHub, a pre-feature project file loads unchanged.
- [x] T060 [P] [US4] Extend `crates/micold-client/tests/jira_source_state.rs`: with a GitHub remote and `Connected` the form offers the choice and opens on the remembered source, default GitHub (US4 AS1-2); `IssueProviderChanged(p)` resets issues, clears query, matches and `searched`, bumps the seq so the other provider's in-flight result drops, and asks the shell to remember `p` (invariant 2, FR-016, FR-017); with the integration off or no connection the dialog is exactly the 034 dialog with no choice (US4 AS4); `Jira` is never selected while not `Connected` (invariant 1).
- [x] T061 [P] [US4] Extend `crates/micold-client/tests/support/covered_states.rs` with "dialog with source choice (GitHub shown)" and regenerate `layout_snapshot.txt`.

### Implementation (US4)

- [x] T062 [US4] `crates/micold-core/src/store.rs`: add `issue_source: Option<micold_core::jira::IssueProvider>` to `StoredProjectState` with its load/save accessors (T059 green).
- [x] T063 [US4] `crates/micold-client/src/features/worktree_form.rs` and `shell/jira.rs`: `IssueProviderChanged`, default from the remembered choice, remembering through the project state file (T060 green).
- [x] T064 [US4] `crates/micold-client/src/ui/worktree_form.rs`: the second `ToggleChip` row **GitHub / Jira** under the chips, shown only when both sources are available (T061 green).
- [x] T065 [P] [US4] User guide: extend "From a Jira issue" in `docs/user-guide/worktrees-and-sessions.md` with the source choice, its per-project memory and that GitHub stays the default.

**Checkpoint**: US4 AS1-4 hold; AS5-7 held since Phase 5.

## Phase 8: Polish and cross-cutting (no code changes: left to the close unit)

- [ ] T066 Record quickstart §B (B1-B9) with the `visual-pass` skill against the `fake_jira` example, including the two-window and token-file-permission arms; per-OS arms with no access are recorded as missing access (category 4) in `specs/533-jira-issue-worktrees/quickstart.md`.
- [ ] T067 Record the follow-up "put the GitHub picker under an integrations switch" as the open issue it stays, in the ledger and the close PR body.

## Dependencies and execution order

- Phase 1 -> Phase 2 -> Phase 3 (US2). Phase 4 (US3) needs Phase 2's types and the Jira page of Phase 3.
- Phase 5 (US1) needs US3's connection (a connection must exist to list) and the Phase 3 gate; Phase 6 needs Phase 5; Phase 7 (US4) needs Phase 5.
- Within a phase, tests (red) precede implementation; `[P]` tasks touch disjoint files.
- T034 sits in Phase 2 beside T004 (its test), so the release-only test runs in CI from M1.

## Parallel examples

- Phase 2 tests: T003-T008 in parallel (distinct files).
- Phase 5 tests: T035-T043 in parallel; T046, T051, T052, T053 beside the core implementation.

## Implementation strategy

MVP is the P1 pair: the opt-in switch (M1) and the picker (M3, which needs M2's connection to be usable by a person). Each milestone merges on its own with its tests and docs; the `jira` module and the wire fields land first so later milestones only add behaviour. The GitHub dialog is unchanged at every step: Jira controls appear only while `jira_visible` and a connection are true.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Jira switch, settings fields and the opt-in gate 🎯 MVP

- **Tasks**: T001–T021, T034
- **Deliverable**: Settings has a **Jira** page whose off-by-default switch persists in `settings.json`, echoes to every window through protocol 45, and shows its enabled status; with default settings no Jira control or request exists anywhere else.
- **Satisfies**: US2 acceptance scenarios 1–3 and 6; FR-001, FR-002, FR-004, FR-005b (Settings part), FR-006 (types), FR-021, FR-022; AS4 and AS5 complete in M3 and M2
- **Verify**: `mise run test-core` (`jira_connection`, `settings_jira`, `jira_site_release` with `cargo test --release -p micold-core --test jira_site_release`, `schema_hash`), `cargo test -p micold-daemon --test jira_setting` through `mise run gate`, and quickstart §B B1
- **Depends on**: —
- **Tier**: full

### M2 — Connect, change and remove a project's Jira connection

- **Tasks**: T022–T031
- **Deliverable**: In Settings a user saves, edits and removes a project's site, key, email and API token; the connection and "token stored" persist across restart, the token lives only in an owner-only file and is deleted on removal.
- **Satisfies**: US3 acceptance scenarios 1–6; US2 acceptance scenario 5; FR-006–FR-010
- **Verify**: `mise run gate` with `features_settings`, `jira_connection_shell`, `jira_token_never_leaks`; quickstart §B B2 and B8
- **Depends on**: M1
- **Tier**: full

### M3 — Start a worktree from a Jira issue

- **Tasks**: T032–T033, T035–T053, T068–T069
- **Deliverable**: With the switch on and a connection, the create-worktree dialog of a Jira-only project lists open Jira issues (a project that also has a GitHub remote keeps the GitHub picker until M5 adds the source choice), and a pick fills ticket, name and type and creates `${type}/${KEY-123}_${name}` with the issue tag; unreachable, timed-out, refused-credential and not-Jira-Cloud sites show a notice with Retry and hand creation keeps working; switching the integration off drops the picker and discards the in-flight result.
- **Satisfies**: US1 acceptance scenarios 1 (for a Jira-only project), 3–7; US2 acceptance scenario 4; US4 acceptance scenarios 3 and 5–7; FR-011, FR-013–FR-015, FR-018–FR-020, FR-023, FR-014, FR-024 (pick, notices), FR-005b (picker part), FR-005
- **Verify**: `mise run gate` with `jira_jql`, `jira_parse`, `jira_classify`, `jira_load`, `issue_jira_rows`, `jira_http_adapter`, `jira_source_state`, `jira_is_requested_only_on_named_events`; quickstart §B B3, B5, B6, B7
- **Depends on**: M2
- **Tier**: full

### M4 — Search Jira issues, including beyond the loaded list

- **Tasks**: T054–T058, T070
- **Deliverable**: Typing in the Jira picker narrows the list by key, summary and label ignoring case, and asks the site for matches beyond the 1,000 loaded issues when the list is incomplete.
- **Satisfies**: US1 acceptance scenario 2; FR-012, FR-024 (search), SC-003
- **Verify**: `mise run gate`, `cargo test --release -p micold-core --test typeahead_budget`, `jira_source_state` search cases; quickstart §B B3
- **Depends on**: M3
- **Tier**: full

### M5 — Choose between GitHub and Jira issues per project

- **Tasks**: T059–T065
- **Deliverable**: A project with a GitHub remote and a Jira connection shows a GitHub / Jira source choice in the dialog, opens on the project's remembered source (default GitHub) and clears list and search on switching.
- **Satisfies**: US4 acceptance scenarios 1, 2 and 4; FR-016, FR-017
- **Verify**: `mise run gate` with `jira_source_state` and `store_issue_source`; quickstart §B B4
- **Depends on**: M3
- **Tier**: full

### Not a milestone

T066–T067 change no code (quickstart results, follow-up record): the close unit does them in the close PR.
