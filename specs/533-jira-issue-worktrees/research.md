# Research: Create Worktrees from Jira Issues

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Each section: Decision, Rationale, Alternatives rejected. No `NEEDS CLARIFICATION` remains.

---

## R1 - Reuse the 034/038 pipeline with a second provider

**Decision**: Extend `micold_core::github::Issue` with an optional Jira key and keep one picker,
one state machine, one ranking. `Issue::jira(key, number, summary, labels, updated_at)` stores the
key; `Issue::ticket()` returns the key or the decimal number; `title_line()` shows the key where
GitHub shows `#number`; `number` is the numeric part of the key (unique inside one project key)
and stays the identity for de-duplication, `picked_issue` and `issue_number_at`. The paging and
cap loop in `load_listing` is extracted into one helper both providers call. `IssueLoadError`
gains `CredentialsRefused` and `NotJiraCloud`; its `message()` names the provider of the failure
(`gh` wording stays for GitHub).

**Rationale**: 034 and 038 hold roughly 40 tests and the whole form reducer around this type;
the spec asks for "the same shape of row, the same matching rule and emphasis" (FR-011, FR-012).
A second `Issue` type would fork `rematch_issues`, the row builder, `IssuePicked`, and the
duplication budget (`mise run duplication`) would flag it.

**Rejected**: (a) a separate `JiraIssue` plus a parallel `JiraList` state machine: doubles the
reducer and the tests for no behavioural difference. (b) Replacing `number: u64` by an
`IssueId` enum: touches every 034/038 call site and test for a gain the optional key already gives.

## R2 - HTTP client: `ureq` 3 in the client, not `curl`, not `reqwest`

**Decision**: Add `ureq` 3.x (features: `rustls`, `platform-verifier`, default JSON off; we parse
with `serde_json`) and `base64` to `micold-client` only. A `JiraSource` adapter in
`shell/jira_http.rs` builds `POST /rest/api/3/search/jql` with `Authorization: Basic
base64(email:token)`, a 10 s global timeout, redirects disabled, and maps every error to our
own enum at the boundary. Proxy settings are read from the environment by `ureq`
(`HTTPS_PROXY`), which the spec's proxy edge case needs.

**Vetting (constitution: dependencies)**: `ureq` is MIT/Apache-2.0, maintained for years by one
active maintainer team (1.x/2.x/3.x lines), blocking and small, widely used; `rustls` is
Apache-2.0/MIT/ISC, actively maintained; `rustls-platform-verifier` (the `platform-verifier`
feature) is MIT/Apache-2.0 and uses SChannel, Security.framework and the system store on
Windows, macOS and Linux, so a private-CA Jira behind a company proxy works as it does in the
browser. `base64` is MIT/Apache-2.0 and already in `Cargo.lock`. The tasks unit re-checks licences
and the `ring`/`aws-lc-rs` backend choice against the CI matrix before adding the lock entries.

**Rationale**: GitHub is reached through `gh` because `gh` owns its sign-in; Jira has no such CLI,
and the credentials are the user's own. A blocking client runs inside `spawn_blocking` like the
`gh` call, so no async runtime coupling. Keeping it in the client keeps the daemon and core (also
built for Windows-host sandbox placements) free of a TLS stack, and the adapter is one of the
constructions `no_concrete_implementations` already counts.

**Rejected**: (a) shelling out to `curl`: not present on minimal Linux images; on Windows older
than 10 1803 absent; the token would travel on a command line (visible in the process list) unless
fed through `--config -`; certificate and proxy behaviour differ per OS. (b) `reqwest`: pulls the
tokio/hyper tree and a second async stack for one POST; larger audit surface. (c) `native-tls`/
OpenSSL: a system library dependency that breaks the "one binary" Linux and Windows packaging.
(d) `isahc`/`curl-sys`: C library to build on three CI OSes.

## R3 - Where the switch and the connection live

**Decision**: Two additive, defaulted `Settings` fields, service-owned: `jira_enabled: bool`
(default `false`) and `jira_connections: BTreeMap<String, JiraConnection>` keyed by the project
path as the client names it. The switch rides `SettingsSet` as an `Option<bool>`; a connection rides it as one per-project
change `jira_connection: Option<JiraConnectionChange { project, connection: Option<JiraConnection> }>`
(upsert or removal) which the daemon applies to its map; `SettingsChanged` carries both whole
(`settings_version` stays 4; `PROTOCOL_VERSION` 41 -> 42). `JiraConnection` holds `site`,
`project_key`, `email`; no token.

**Rationale**: `settings.json` has two writers and the split is by field (the doc on
`default_ai_cli` explains it): a field the client alone writes is reverted by the daemon's next
whole-struct save. The echo of `SettingsChanged` to every client is the one existing channel that
makes a change in one window show in the others without a restart (FR-022) and gives "last save
wins, never half-written" for free (the struct is written whole, atomically). A pre-feature file
reads as off with an empty map (FR-002, US2 AS6). The daemon never reads the fields (FR-021). **Several daemons**: the client's registry (feature
491) keeps daemons independent, and these fields, like `pr_status_enabled`, live with the daemon the
client is connected to. The token file lives on the client host, not the daemon's: after switching
daemon a project shows "not connected" (no entry) or "enter your token" (entry, no file) and
sends nothing. This amends the spec's Edge Case wording "not in a session or a daemon" to "no session
depends on it" (recorded in the ledger).

**Rejected**: (a) a client-only `jira.json`: needs its own cross-window refresh and an
atomic-whole story. (b) keeping the connection in `StoredProjectState`: no daemon echo, so the
second window would show stale state until reopened (FR-022). (c) putting the connection in the
repository (`.micold/`): "never in a project file" (FR-008), and it would be committed by accident.

## R4 - The token: an owner-only file beside settings, never on the wire

**Decision**: `<data_dir>/jira/<project-id>.token`, written by the client with
`owner_only::write` (0600 / protected DACL, atomic rename), read by the client only at request
time into an `ApiToken` newtype (`Zeroize`-free, no `Serialize`, `Debug` prints `ApiToken(<redacted>)`,
no `Display`). `project-id` is the existing `store::project_id` hash, exposed through one new
accessor. "A token is stored" is `Path::exists`, recomputed when the Settings page opens, on every
`SettingsChanged`, and when the dialog opens. Removing a connection deletes the file first, then
the settings entry; a crash between the two leaves an entry with no token, which reads as
"enter your token again" and sends nothing.

**Rationale**: spec FR-008 and the assumption list: the app has no keychain integration; the owner-only
file is the app's mechanism for private data. The daemon may run in a container (feature 027) and
must neither need nor see the token (FR-009, FR-021), so it is not a `Settings` field and not in any
protocol message. A sentinel-token scan test covers settings, logs and the wire.

**Rejected**: the OS keychain (a new platform integration per OS, and the spec says there is none);
storing the token in `settings.json` (daemon-visible, and world-readable by default umask).

## R5 - The remembered source per project

**Decision**: `StoredProjectState.issue_source: Option<IssueProvider>` (additive, defaulted),
read when the dialog opens and written when the user switches, through the client's existing
`JsonFileStore` project-state update. Absent or unknown value reads as GitHub (FR-016).

**Rationale**: it is a per-project UI preference of the client, not a service setting, and the
project state file already holds `last_session` (feature 025) for the same reason. No wire change.

**Rejected**: a field in `Settings` (a UI memory crossing the wire for no reader on the daemon);
remembering in `WorktreeForm` only (lost on close; the spec asks to reopen on that source).

## R6 - Jira Cloud requests: enhanced JQL search, one endpoint

**Decision**: `POST {origin}/rest/api/3/search/jql` with body
`{"jql": "...", "fields": ["summary","issuetype","labels","reporter","description","updated"],
"maxResults": 100, "nextPageToken": ...}`. List JQL:
`project = "KEY" AND statusCategory != Done ORDER BY updated DESC`. Search JQL adds
`AND (key = "KEY-n" OR summary ~ "text*" OR labels = "text")` built from validated pieces only
(key from the validated project key; text escaped for `\` and `"`, control characters removed,
capped at 100 characters). Paging follows `nextPageToken` until `isLast` or the cap. Total open
count: `POST /rest/api/3/search/approximate-count` once, only to print "of N open"; failure there
degrades to "at least 1,000" without a notice.

**Rationale**: the classic `/rest/api/3/search` endpoint was removed from Cloud in 2025; the
enhanced search is the documented replacement and the only one that needs the user's credentials
and nothing else. `description` comes back in the list, so Jira has no second description pass.

**Rejected**: GET with query string (puts the search text in a URL that proxies log); JQL typed
by the user (out of scope); `/rest/api/2` (retired for search; v3 is what FR-023 names).

## R7 - Failure classification

**Decision**: pure `classify(status, body, transport) -> IssueLoadError`: transport error or
timeout -> `Offline`/`TimedOut` (reason shortened to 80 chars, host only); 401, or 403 with no
JSON error body -> `CredentialsRefused`; 403 on the project, or 400 whose message says the
project does not exist or is not permitted -> `NoAccess` (shown with the credentials wording, since
Jira answers both alike); 429 -> `RateLimited` (honours `Retry-After` in the message); 200 whose
body lacks `issues`, 404 on the endpoint, an HTML body or a redirect -> `NotJiraCloud`. Messages
are fixed text with the host or key interpolated; the response body is never echoed.

**Rationale**: FR-018 and the Data Center edge case; fixtures per outcome keep wording changes
visible, as 034's `fixtures/gh/` does.

## R8 - Matching order between issue type and labels

**Decision**: call the existing `type_for_labels(mapping, [type_name, labels...])`. It returns the
first mapping entry in list order whose label equals any of the supplied strings after trimming
and lower-casing, which is exactly FR-013 ("the entry nearest the top wins among the labels and
the issue-type name together"). No new matcher.

## R9 - The switch's surfaces and the notice

**Decision**: one pure `jira_visible(&Settings) -> bool` read by every view that would show a Jira
control; the Settings page shows the switch and its one-line description at all times and the rest
only when on. The notice above the picker is built from the connection's origin, key and the
fixed list "project key, search text, credentials" (FR-005b), as 034's `issue_notice` is built
from `owner/name`. Switch-off bumps `issue_request_seq`, resets provider state and closes any
Jira list in every window on the `SettingsChanged` that carries it.

## R10 - What the loopback test proves

**Decision**: the adapter is exercised against a `std::net::TcpListener` fake on `127.0.0.1`.
`SiteOrigin::parse` accepts `http` only for a loopback host (`127.0.0.1`, `::1`, `localhost`) and
only under `cfg(debug_assertions)`; a release build refuses it as FR-006 says, held by a
release-only test in `micold-core` beside `SiteOrigin` (`tests/jira_site_release.rs`,
`#[cfg_attr(debug_assertions, ignore)]` like `typeahead_budget.rs`), which the existing release CI step
(`cargo test --release -p micold-core --test typeahead_budget`) is extended to run
(`--test jira_site_release`).
This is what lets the GUI visual pass (debug build) and US1's fake-site test save a loopback
connection. Rejected: TLS on the fake (needs a CA installed into the OS store); a runtime flag
(a switch a hand-edited file could flip). The test
holds: the Basic header is `email:token`, the body carries only the JQL, fields and page token, the
redirect is not followed, a stalled server times out, and a sentinel token is absent from every
error string and `Debug` output.
