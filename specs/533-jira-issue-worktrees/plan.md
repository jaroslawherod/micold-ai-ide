# Implementation Plan: Create Worktrees from Jira Issues, Behind an Opt-In Integration Switch

**Branch**: `feat/533_create-worktrees-from-jira-issues` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/533-jira-issue-worktrees/spec.md`

## Summary

The create-worktree dialog's issue source (feature 034, rows from 038) gains a second provider,
**Jira Cloud**. A new app-wide switch (off by default) and a per-project connection (site origin,
project key, account email, API token) make the dialog list the project's open Jira issues, search
them, and fill ticket, name and type exactly as a GitHub pick does. Nothing about Jira exists, and
no request leaves, until the switch is on and a connection is stored.

The design turns on five decisions (each in [research.md](./research.md)):

1. **Reuse the GitHub issue pipeline; do not fork it** ([R1](./research.md#r1--reuse-the-034038-pipeline-with-a-second-provider)).
   `micold_core::github::Issue` gains an optional Jira key (its ticket and row prefix); the
   existing `typeahead` ranking, `name_from_title`, `type_for_labels`, `IssueList`/`SearchState`
   state machine, staleness counter, notice/Retry and `Typeahead` widget carry both providers.
   A new render-free `micold_core::jira` module holds only what is Jira-specific: URL and key
   validation, JQL building, response parsing (including Atlassian Document Format flattening),
   failure classification, a `JiraSource` trait with a fake, and the paging/cap loop shared with
   `github::load_listing` through one extracted helper.
2. **HTTPS through one new, vetted crate: `ureq` 3 with `rustls` and the platform certificate
   verifier**, constructed only in `micold-client`, behind the `JiraSource` trait
   ([R2](./research.md#r2--http-client-ureq-3-in-the-client-not-curl-not-reqwest)). The workspace
   has no HTTP crate today and Jira, unlike GitHub, has no CLI that owns the sign-in.
3. **The switch and the connections are service-owned settings**, like `pr_status_enabled`
   ([R3](./research.md#r3--where-the-switch-and-the-connection-live)): `settings.json` has two
   writers and a client-only field is reverted by the daemon's next whole-struct save; the daemon's
   `SettingsChanged` echo is also the only existing path that reaches every window (FR-022). The
   daemon stores the fields and never reads them (FR-021). **Protocol 40 -> 41.** Like `pr_status_enabled`, the switch and the connection therefore follow the service the client is connected to; the token file stays on the client host, so a second daemon shows the connection and asks for the token again (R3 records the multi-daemon behaviour).
4. **The token is a separate owner-only file written by the client** with `owner_only::write`
   ([R4](./research.md#r4--the-token-an-owner-only-file-beside-settings-never-on-the-wire)), never
   in `Settings`, never on the wire, never in `Debug`. The settings carry only the connection's
   non-secret fields; "a token is stored" is a file-existence probe.
5. **The remembered source per project is client state** in the per-project state file the
   client's `JsonFileStore` already keeps ([R5](./research.md#r5--the-remembered-source-per-project)).

Everything with a decision in it (validation, JQL, parsing, classification, the form's provider
state and staleness, the draft validation, the keep-or-remove prompt) is render-free and tested
first; `src/ui/` only composes (Principle I's GUI exception, quickstart §B).

## Technical Context

**Language/Version**: Rust, edition 2021, workspace MSRV (unchanged)

**Primary Dependencies**: `iced` 0.14, `serde`/`serde_json`, `tokio` (`Task::perform` +
`spawn_blocking`), `micold-core` modules `github`, `issue_types`, `naming`, `typeahead`,
`settings`, `owner_only`, `store`. **New crate: `ureq` 3 (rustls, platform-verifier), `micold-client`
only**; a direct `base64` dependency for the Basic header (already in `Cargo.lock`
transitively) in the same crate.

**Storage**: `settings.json` gains `jira_enabled: bool` and `jira_connections` (additive,
defaulted; `SETTINGS_VERSION` stays 4). The token is `<data_dir>/jira/<project-id>.token`, owner-only.
The per-project source choice goes in the project state file (additive). Issues are never
persisted (FR-019).

**Testing**: `mise run test-core` for the render-free rules; `mise run gate` for the workspace;
reducer tests under `crates/micold-client/tests/`; daemon settings test under
`crates/micold-daemon/tests/`; a loopback fake Jira server (std `TcpListener`, plain HTTP behind a
test-only constructor) for the `ureq` adapter; `quickstart.md` §B for render glue.

**Target Platform**: Linux, macOS, Windows desktop. The only OS-specific parts are the owner-only
file (existing `owner_only.rs` arms) and the TLS verifier (`rustls-platform-verifier` uses each
OS's store, so a private-CA Jira site works the same on all three).

**Project Type**: desktop application (three-crate Cargo workspace).

**Performance Goals**: local filtering of 1,000 issues stays inside the existing rank budget
(< 50 ms release, test-held, SC-003 of 034); a failed or slow site reported within 10 s per
request; switch-off effective within one second (SC-006).

**Constraints**: no request unless switch on + connection + Jira source showing (FR-020); only the
project key, search text and credentials sent; token never in `Debug`, logs, notices, settings,
wire or session history (FR-009); GitHub path and every other creation path unchanged and offline
capable (FR-003, FR-018).

**Scale/Scope**: one new core module (`jira`) + extended `github` (Issue key, shared paging),
`settings`, `owner_only` user, `store` (token dir, source choice), `protocol` (two fields, v41);
client: provider state in `worktree_form`, Settings **Jira** page (a rail entry that is always present and shows only the switch while it is off: FR-001's "its own switch"), shell effect module `shell/jira.rs`, `ureq` adapter; docs on three pages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Verdict | Evidence |
|---|---|---|
| I. Test-First | PASS | Validation, JQL, ADF flattening, parsing, classification, paging, the form's provider/staleness rules, draft validation and the settings round trip are in `micold-core` or render-free client reducers, written test-first. The `ureq` adapter is a thin call + status mapping, tested against a loopback server. GUI exception claimed only for `src/ui/` composition (§B). |
| II. Multi-Session | PASS | No session starts, stores or reads Jira (FR-021). Per-form state only; the connection is app data, not session data; two projects keep separate connections and tokens. |
| III. Worktree Integration | PASS | A Jira-sourced worktree is created by the unchanged new-branch path with its pre-flight and conflict prompt; no new create mode. |
| IV. Local-First (NON-NEGOTIABLE) | PASS | Off by default; no request, no credential field, no control while off (FR-001). Contact only after the user's explicit opt-in (the switch plus a saved connection), with the notice naming what is sent (FR-005b). Offline, every other path works and Jira fails inside 10 s with a Retry. |
| V. Rust + iced | PASS | Rust and iced only. `JiraConnection` and `SiteOrigin`/`ProjectKey` newtypes make an invalid connection unrepresentable; `ApiToken` has no `Serialize` and a redacting `Debug`. |
| VI. Cross-Platform | PASS | One code path. Token file via the existing per-OS owner-only arms; TLS trust via the OS store on all three; `micold-core` suite runs on all three CI OSes. The loopback test needs no OS-specific code. |
| VII. Documentation | PASS | FR-024: user guide gains "From a Jira issue" and the Settings integration section, each in the milestone whose behaviour it describes; `architecture.md` records where the Jira request runs; the dependency is recorded in research R2. |
| VIII. Reusable UI | PASS | The picker is the existing `Typeahead`; the source choice is `ToggleChip`; Settings rows are `TextField`, `Switch`/toggle, `Button`, `IconButton` as the GitHub page uses. No new widget. |

### Re-check after Phase 1 design

All eight still PASS. Two things recorded rather than waved through:

- **A new crate (`ureq` and its TLS tree)** is the first HTTP dependency of the workspace. The
  vetting (maintenance, licences, alternatives) is in R2; it lives in the client only, so the
  daemon and `micold-core` (which Windows-host sandbox placements must keep small) do not link it.
- **A protocol change** (two settings fields) changes `SCHEMA_HASH`; a mixed client/daemon pair
  refuses at the handshake by design, so both halves land in one milestone.

See Complexity Tracking below; neither is a constitutional violation.

## Requirement -> design map

| Requirement | Where |
|---|---|
| FR-001, FR-003 | `Settings.jira_enabled` default false; every Jira control gated on `jira_visible(settings)` ([issue-picker-ui §1](./contracts/issue-picker-ui.md)); the opt-in gate test |
| FR-002 | `Settings.jira_enabled`, `#[serde(default)]` ([jira-connection §1](./contracts/jira-connection.md)) |
| FR-004, FR-005 | Settings Jira section; `JiraAvailability` in the form; switch-off drops provider state and bumps `issue_request_seq` ([data-model §4](./data-model.md)) |
| FR-005b | Notice text beside the switch and above the picker ([issue-picker-ui §2](./contracts/issue-picker-ui.md)) |
| FR-006 | `JiraConnection::parse` / `validate` ([jira-connection §2](./contracts/jira-connection.md)) |
| FR-007, FR-022 | `Settings.jira_connections` keyed by project path; daemon echo |
| FR-008, FR-009, FR-010 | `ApiToken`, `jira_token_dir`, `owner_only::write` ([jira-connection §3-4](./contracts/jira-connection.md)) |
| FR-011 | `JiraCloud::list_open`, JQL `statusCategory != Done ORDER BY updated DESC`, shared cap ([jira-cloud-source §2-4](./contracts/jira-cloud-source.md)) |
| FR-012 | Shared `typeahead::rank` over `Issue::row_text` (key in place of number); `search_open` ([jira-cloud-source §5](./contracts/jira-cloud-source.md)) |
| FR-013, FR-014 | `type_for_labels` over the issue-type name plus the labels: it is already first-match in mapping order ([issue-naming §2](./contracts/issue-naming.md)); Settings intro text |
| FR-015 | `naming_for_issue` uses `Issue::ticket()`; `Tag::Issue` already reads `ABC-123` |
| FR-016, FR-017 | `IssueProvider`, per-project choice in the project state file ([data-model §4-5](./data-model.md)) |
| FR-018 | `JiraError` -> `IssueLoadError` + notice text ([jira-cloud-source §6](./contracts/jira-cloud-source.md)) |
| FR-019 | `Issue` has no `Serialize`; Debug already redacts reporter and description |
| FR-020 | Single call site in `shell/jira.rs`, gate test `jira_is_requested_only_on_named_events.rs`; request carries only key, text, credentials |
| FR-021 | No daemon read of the fields; `jira` not in `micold-daemon` |
| FR-023 | Parse accepts only the Cloud v3 shape; anything else is `NotJiraCloud` |
| FR-024 | Documentation table below |

## Test strategy by layer

| Layer | What it holds | Requirements |
|---|---|---|
| `micold-core` unit (`mise run test-core`) | site/key/email validation, origin normalisation, JQL building and escaping, ADF flattening, page parsing, classification fixtures, paging and cap, `Issue` key rows and ticket, type match order, settings round trip and pre-feature file, `ApiToken` Debug, token file path | FR-002, 006, 009, 011-015, 018, 019, 023 |
| `micold-core` release budget | existing 1,000-row rank case extended with Jira rows | SC-003 |
| `micold-daemon` integration | `SettingsSet`/`SettingsChanged` carry the two fields and echo to a second client; stored file round trip | FR-002, FR-007, FR-022 |
| `micold-client` reducer | form provider state, staleness, source choice and memory, switch-off discard, Settings draft validation, keep-or-remove prompt, token-presence derivation | US1-US4 scenarios, FR-004, 005, 010, 016, 017 |
| `micold-client` adapter | `ureq` adapter against a loopback server: status mapping, timeout, header content, redirect refusal, no token in errors | FR-018, FR-020, FR-009 |
| `micold-client` source gates | requested only on named events; `no_concrete_implementations` (adapter named only in `Capabilities::real()`); token-never-logged scan; `features_are_render_free`; `material_builder_api` | FR-001, 009, 020, VIII |
| Layout/geometry gates | new covered states (source choice shown, Jira Loading/Failed/Loaded, Settings Jira section off/on/with error) in `tests/support/covered_states.rs`; `layout_snapshot.txt` regenerated | US2, US3, US4 |
| quickstart §B (visual-pass) | render glue, real Jira Cloud site (or the loopback fake), two windows | SC-001, SC-004, SC-006 |

## Project Structure

### Documentation (this feature)

```text
specs/533-jira-issue-worktrees/
├── plan.md
├── research.md             # Phase 0 - R1-R10
├── data-model.md           # Phase 1
├── quickstart.md           # Phase 1 - §A automated, §B recorded manual pass
├── contracts/
│   ├── jira-cloud-source.md   # the trait, requests, JQL, parsing, classification, adapter
│   ├── jira-connection.md     # settings fields, validation, token file, wire
│   ├── issue-naming.md        # key rows, ticket, type match order
│   └── issue-picker-ui.md     # dialog, notice, Settings section, gating, docs
├── checklists/requirements.md
└── tasks.md                # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/
│   ├── lib.rs                 # + pub mod jira
│   ├── jira.rs                # NEW - SiteOrigin, ProjectKey, JiraConnection, ApiToken, jql, parse_page, classify, JiraSource, FakeJiraSource, load
│   ├── github.rs              # Issue gains optional key; paging helper extracted; IssueLoadError + CredentialsRefused, NotJiraCloud
│   ├── issue_types.rs         # + type_for_issue(mapping, type_name, labels)
│   ├── naming.rs              # naming_for_issue uses Issue::ticket()
│   ├── settings.rs            # + jira_enabled, jira_connections (StoredSettings, Settings)
│   ├── store.rs               # + jira_token_dir(), StoredProjectState.issue_source
│   ├── protocol/messages.rs   # SettingsSet/SettingsChanged carry the two fields
│   └── protocol/version.rs    # PROTOCOL_VERSION 40 -> 41
└── tests/
    ├── jira_site_release.rs   # NEW - release-only: loopback http refused; #[cfg_attr(debug_assertions, ignore)]
    ├── jira_connection.rs  jira_jql.rs  jira_parse.rs  jira_classify.rs  jira_load.rs
    ├── issue_jira_rows.rs  issue_types.rs (extended)  settings_jira.rs  schema_hash.rs (pin -> 41)
    └── fixtures/jira/        # captured Cloud v3 bodies per outcome

crates/micold-daemon/
├── src/state.rs, server.rs    # setters beside set_pr_status_enabled; echo; no read
├── src/catalog.rs             # settings_wire carries both fields; `jira_enabled` is copied in persist_service_settings, `jira_connections` is NOT (only the Jira setter merges its JiraConnectionChange into on_disk)
└── tests/jira_setting.rs      # NEW; includes "a save of another field keeps the Jira fields"

crates/micold-client/
├── Cargo.toml                 # + ureq, base64
├── src/
│   ├── features/worktree_form.rs  # IssueProvider, JiraAvailability, provider-aware messages
│   ├── features/settings.rs       # SettingsSection::Jira, JiraDraft, validation, keep-or-remove
│   ├── shell/jira.rs              # NEW - effects: load/search/retry, token read/write/remove
│   ├── shell/jira_http.rs         # NEW - the ureq adapter (JiraCloud implements JiraSource)
│   │                              #   (+ examples/fake_jira.rs: dev-only loopback server for §B)
│   ├── shell/capabilities.rs      # + jira factory
│   ├── shell/issues.rs            # provider dispatch
│   ├── ui/settings/jira.rs        # NEW - the Jira section
│   ├── ui/worktree_form.rs        # source choice + provider notice
│   └── icons.rs                   # + Jira rail glyph on an existing Material Symbols codepoint
└── tests/
    ├── jira_source_state.rs  jira_is_requested_only_on_named_events.rs  jira_http_adapter.rs
    ├── jira_token_never_leaks.rs
    ├── features_settings.rs (extended)  support/covered_states.rs  fixtures/layout_snapshot.txt

.github/workflows/ci.yml               # release step also runs --test jira_site_release

docs/
├── user-guide/worktrees-and-sessions.md  # From a Jira issue; source choice; failure notices
├── user-guide/settings.md                # Jira page; token storage
└── development/architecture.md           # where the Jira request runs (client), the dependency
```

**Structure Decision**: the existing three-crate workspace, unchanged. Decision logic in
`micold-core` beside `github`, `naming`, `typeahead`; the client gains effects, reducer state and
composition; the daemon only stores and echoes two fields.

## Delivery order

| Slice | Delivers | Gate |
|---|---|---|
| **1 (US2 + US3 core)** | Settings fields + protocol v41; switch, connection draft/validation, token file, Jira section with the switch-off prompt; nothing requested yet | `mise run gate`; §B Settings |
| **2 (US1, P1)** | `jira` module, `ureq` adapter, provider in the form, Jira-only project picker, pick fills, naming, type match, notice and failures with Retry; user-guide section | `mise run gate`; §B pick |
| **3 (US4, P2)** | Source choice for GitHub + Jira, per-project memory, switch/discard rules | §B source choice |
| **Polish** | architecture doc; §B two-window and sandbox/desktop passes recorded | quickstart §B complete |

## Risks

| Risk | Handling |
|---|---|
| Atlassian retires or reshapes the search endpoint (the classic `/search` was removed in 2025) | Only `POST /rest/api/3/search/jql` is used; fixtures captured from it; any unknown shape is `NotJiraCloud`, never a wrong list. |
| ADF description shapes vary | Flatten text nodes only, cap at 600 chars (existing `description_from`); unknown node kinds contribute nothing. |
| Token leaks through an error chain of `ureq`/`rustls` | Errors are mapped to our own enum at the adapter boundary; messages built from fixed text plus host; the scan gate runs a failing request with a sentinel token and greps all outputs. |
| A private-CA Jira site fails with a bundled root store | Platform verifier chosen (R2); the failure is "could not be reached" with a shortened reason. |
| Two writers of `settings.json` | New fields are service-owned, saved through `SettingsSet` like `pr_status_enabled`; `catalog.rs` copies `jira_enabled` like the other fields; `jira_connections` is merged per project by the Jira setter alone, never copied from the boot snapshot (daemon tests: two catalogs on one store, and an unrelated setter's save leaves the map untouched). |
| Several daemons (feature 491 registry): switch and connections follow the connected service | Same as `pr_status_enabled`; the token file is client-side, so a daemon with no stored connection for the project shows "not connected" and a connection without a token shows "enter your token". Never a request without both. |
| Several daemons share `settings.json` | `persist_service_settings` merges the daemon's own `JiraConnectionChange` into the map read from disk instead of copying its boot snapshot, so a save through daemon A keeps a connection saved through daemon B; the other service fields keep the existing snapshot staleness class. |
| Two windows save different projects' connections at once | The wire carries one per-project upsert or removal, applied by the daemon to its map; no whole-map overwrite ([jira-connection §1](./contracts/jira-connection.md)). |
| The visual pass needs a non-`https` fake site | `SiteOrigin` accepts loopback `http` only under `cfg(debug_assertions)`; a release-build gate test in `micold-core` (`jira_site_release.rs`, run by the existing release CI step extended with `--test jira_site_release`) asserts it is refused ([R10](./research.md#r10--what-the-loopback-test-proves)). |
| Windows/macOS passes cannot run on this Linux host | Everything platform-sensitive is in existing, already cross-tested arms; the per-OS §B arms recorded in Polish or escalated as missing access (category 4). |

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected |
|---|---|---|
| New crates `ureq` (+ rustls, platform verifier) and `base64` in the client | Jira has no CLI that owns the sign-in; HTTPS with the user's own credentials is the minimum | `curl` subprocess, `reqwest`, native-tls: [R2](./research.md#r2--http-client-ureq-3-in-the-client-not-curl-not-reqwest) |
| `PROTOCOL_VERSION` 40 -> 41 | A change in one window must reach every window (FR-022) and the daemon is the only echo | Client-only file with its own refresh: [R3](./research.md#r3--where-the-switch-and-the-connection-live) |
