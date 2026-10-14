# Data Model: Create Worktrees from Jira Issues

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

## 1. Connection - `micold_core::jira` (NEW module)

| Type | Shape | Rules |
|---|---|---|
| `SiteOrigin` | newtype over `String`, `scheme://host[:port]` | `parse(&str)`: `https` only (loopback `http` additionally under `cfg(debug_assertions)`, R10), a host, no user-info, no query or fragment; path and trailing slash dropped (FR-006, Edge "Site URL"). `Display` is the origin. |
| `ProjectKey` | newtype over `String` | `parse(&str)`: trimmed, upper-cased input accepted only when `[A-Z][A-Z0-9_]{1,9}` (Atlassian's key rule). |
| `AccountEmail` | newtype over `String` | `parse`: one `@`, non-empty local part and domain with a dot, no whitespace. |
| `ApiToken` | newtype over `String` | non-empty after trim; no `Serialize`/`Deserialize`/`Display`; `Debug` is `ApiToken(<redacted>)`; one `expose(&self) -> &str` used only by the adapter's header builder. |
| `JiraConnection` | `{ site: SiteOrigin, project_key: ProjectKey, email: AccountEmail }` | serde (it is in `settings.json`); built only through `JiraConnection::parse(site, key, email) -> Result<_, Vec<FieldProblem>>`; deserialisation re-validates and drops an entry that no longer parses (as `known_entries` does for the label map). |
| `FieldProblem` | `{ field: JiraField, reason: &'static str }`, `JiraField = { Site, ProjectKey, Email, Token }` | every refused field is reported at once (FR-006). |

## 2. Issue - `micold_core::github::Issue` (extended)

| Change | Meaning |
|---|---|
| `key: Option<String>` (private) | The Jira key `ABC-123`; `None` for GitHub. |
| `Issue::jira(key, number, summary, labels, updated_at)` | `number` = numeric part of the key; identity inside one project key. |
| `Issue::ticket() -> String` | key, or `number.to_string()`. Used by `naming_for_issue` (FR-015). |
| `Issue::issue_type()` | Jira issue-type name kept as a private extra label source; `labels()` stays the labels only. A `with_type_name` ctor sets it; GitHub issues carry `None`. |
| `title_line()` | `KEY-123 summary` when keyed, `#n title` otherwise. |
| `row_text()` | the same text GitHub builds with the key in place of `#n` (FR-012). |
| `Debug` | unchanged: reporter and description `<redacted>` (FR-019). No `Serialize`. |

`IssueLoadError` gains `CredentialsRefused` and `NotJiraCloud`; `message(&self, provider)` is
provider-aware ([jira-cloud-source §6](./contracts/jira-cloud-source.md)).

## 3. Settings and storage

| Field | Where | Type | Default | Owner |
|---|---|---|---|---|
| `jira_enabled` | `Settings`, `StoredSettings`, `SettingsSet`, `SettingsChanged` | `bool` | `false` | service |
| `jira_connections` | same (written through a per-project `JiraConnectionChange`) | `BTreeMap<String, JiraConnection>` | empty | service |
| token file | `<data_dir>/jira/<project-id>.token`, owner-only | raw token bytes, UTF-8 | absent | client |
| `issue_source` | `StoredProjectState` | `Option<IssueProvider>` | `None` = GitHub | client |

`SETTINGS_VERSION` stays 4 (additive); `PROTOCOL_VERSION` 41 -> 42; `schema_hash.rs` pin moves.
A project key in `jira_connections` is the same string the client uses for the project path in
`SettingsSet` elsewhere; the token file name is `store::project_id(path)` (new public accessor).

## 4. Form state - `micold_client::features::worktree_form` (extended)

```text
IssueProvider
├── Github
└── Jira

JiraAvailability                        // derived from settings at open and on SettingsChanged
├── Off                                 // switch off, or no connection for this project
└── Connected(JiraSite)                 // JiraSite { origin, key }: what the notice names
```

| New `WorktreeForm` field | Type | Meaning |
|---|---|---|
| `provider` | `IssueProvider` | Which provider the Issue source lists. Defaults to the project's remembered choice, else `Github`; `Jira` is forced when there is no GitHub remote and Jira is connected (US4 AS3). |
| `jira` | `JiraAvailability` | Gates the source choice. |

`BranchSource::Issue` is unchanged; the existing `issues: IssueList`, `issue_query`, `issue_matches`,
`searched`, `SearchState` and `issue_request_seq` serve both providers.

**Invariants** (each with a reducer test in `tests/jira_source_state.rs`):

1. `provider == Jira` only while `jira` is `Connected`; a `SettingsChanged` that turns it `Off`
   moves `provider` to `Github` (or leaves the source with no list when there is no GitHub remote),
   resets `issues` to `NotRequested`, clears the query and bumps `issue_request_seq`
   (FR-005, FR-017, US2 AS4).
2. `IssueProviderChanged(p)` resets `issues`, clears `issue_query`/`issue_matches`/`searched`,
   bumps the seq (an in-flight result of the other provider is then stale, invariant 2 of 034) and
   asks the shell to remember `p` for the project (FR-016, FR-017).
3. `issues` leaves `NotRequested` for `Jira` only on `SourceChanged(Issue)` or
   `IssueProviderChanged(Jira)` while `jira` is `Connected` (FR-020).
4. A connection or token removed while `Loading`/`Searching`: the shell bumps the seq; the late
   result drops (Edge "Token removed during a load").
5. A Jira listing never enters `DescriptionPass::Loading`: descriptions arrive with the page.

| Message | Effect |
|---|---|
| `JiraSettingsChanged(JiraAvailability)` | Applies invariant 1; no effect when unchanged. |
| `IssueProviderChanged(IssueProvider)` | Invariant 2. |
| existing `IssuesLoaded`, `IssueRetry`, `IssueQueryChanged`, `IssueSearched`, `IssuePicked` | unchanged reducers; the shell chooses the source of the request by `provider`. |

## 5. Settings draft - `micold_client::features::settings` (extended)

- `SettingsSection::Jira` (label "Jira", rail icon on an existing Material Symbols codepoint,
  `tests/icons.rs` rule: no two icons share one).
- `JiraDraft { enabled: bool, site: String, project_key: String, email: String, token: String,
  token_stored: bool, project: Option<PathBuf>, remove_requested: bool }`. `token` is a
  password-masked `TextField` value, never copied into a message `Debug` (the message type holds
  an `ApiToken`-wrapping `Secret` string with the same redaction).
- Messages: `JiraEnabledToggled(bool)`, `JiraSiteChanged(String)`, `JiraKeyChanged(String)`,
  `JiraEmailChanged(String)`, `JiraTokenChanged(String)`, `JiraRemoveRequested`,
  `JiraKeepOrRemoveChosen(Keep | Remove)`.
- `validate` runs `JiraConnection::parse` and a token check -> `FieldError { section: Jira, field }`;
  saving is refused and nothing is stored (FR-006, US3 AS5). A blank token with `token_stored` keeps
  the stored one (editing the site does not force retyping).
- Turning the switch off with a stored connection raises `KeepOrRemovePrompt`; the draft is
  not applied until it is answered (FR-005, US3 AS4).

## Lifetime and persistence

| Datum | Lives | Persisted |
|---|---|---|
| issues, listing, query, matches | the open form | never (FR-019) |
| `provider` | the open form | its choice in the project state file |
| switch, connections | `settings.json` | yes, daemon-written |
| token | the owner-only file | yes; read per request, never cached across a removal |
