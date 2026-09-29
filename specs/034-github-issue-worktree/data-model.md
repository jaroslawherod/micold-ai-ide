# Data Model: Create a Worktree from a GitHub Issue

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

Types marked **NEW** do not exist yet; everything else is an existing type being extended. Module
paths are where the type lives.

---

## 1. Repository identity — `micold_core::github` (NEW module)

### `GitRemote` (NEW, `micold_core::git`)

| Field | Type | Notes |
|---|---|---|
| `name` | `String` | `origin`, `upstream`, … |
| `url` | `String` | `remote.<name>.url` from the repository-local config, **not** rewritten by global `insteadOf` (research R5). |

Serialized over the protocol (serde + postcard), so it derives `Serialize`/`Deserialize`.
Order is config order. Produced by the pure `parse_remote_list(&str) -> Vec<GitRemote>` from
`git config --local --get-regexp ^remote\..+\.url$` output (first URL per remote name).

### `GithubRepo` (NEW)

| Field | Type | Validation |
|---|---|---|
| `owner` | `String` | Non-empty; GitHub login charset (`[A-Za-z0-9-]`), as parsed. |
| `name` | `String` | Non-empty; `.git` suffix and trailing `/` stripped. |

`Display` → `owner/name` (the text FR-025 shows). Constructed only by
`GithubRepo::from_remote_url(&str) -> Option<GithubRepo>` (host `github.com`, or `ssh.github.com`
for SSH-over-443, ignoring case — FR-002; accepted URL forms in research R5; any userinfo is
discarded), so an Enterprise or non-GitHub repository is unrepresentable and a token in a URL can
never reach the display.

### `choose_remote(&[GitRemote]) -> RemoteChoice` (NEW, pure)

```text
RemoteChoice
├── Github { remote: String, repo: GithubRepo }   // origin if it is GitHub, else the first GitHub remote
└── NoGithubRemote                                // no remote, or none on github.com (FR-002)
```

---

## 2. Issues — `micold_core::github`

### `Issue` (NEW)

| Field | Type | Notes |
|---|---|---|
| `number` | `u64` | FR-009 sets the ticket to `number.to_string()`. |
| `title` | `String` | As GitHub returns it. |
| `labels` | `Vec<String>` | Label names, at most 20 (research R2). |
| `updated_at` | `String` | RFC 3339 from GitHub; used for ordering only, compared as returned. |
| `row_text` | `String` | Derived at construction: `#<number> <title>` + `  ·  <l1>, <l2>` when labelled. The string the picker ranks **and** shows (research R12). |

Held only in the open form (FR-023); never serialized to disk (SC-006). No `Serialize` derive, so
no code path can persist it by accident.

### `IssueListing` (NEW)

| Field | Type | Notes |
|---|---|---|
| `issues` | `Vec<Issue>` | Most recently updated first (FR-004), ≤ 1,000. |
| `total_open` | `u64` | GitHub's `totalCount`. |
| `complete` | `bool` | `issues.len() as u64 >= total_open`. Derived; decides whether search may contact GitHub (FR-005a). |

### `IssuePage` (NEW)

`{ issues: Vec<Issue>, total_open: u64, next_cursor: Option<String> }` — one GraphQL page. The pure
`load_listing(source, repo)` loop concatenates pages until `next_cursor` is `None` or 1,000 issues
are held (`ISSUE_LOAD_CAP = 1_000`), and stops at the first error.

### `IssueLoadError` (NEW)

```text
IssueLoadError
├── ToolMissing          // gh not found (R3)
├── NotSignedIn          // exit 4, 401 (R8)
├── NoAccess             // 404/403/SAML (R8)
├── Offline              // network errors (R8)
├── RateLimited          // rate limit (R8)
├── TimedOut             // 10 s, killed (R6)
└── Other(String)        // first non-empty stderr line
```

`IssueLoadError::message(&GithubRepo) -> String` is the plain-language text of FR-007, and names
what to do. The exact wording is defined once, in
[github-issue-source §5](./contracts/github-issue-source.md#5-failure-outcomes-as-the-user-reads-them-fr-007-spec-edge-cases). Every variant offers retry; the view does not branch on variant for that.

---

## 3. Label-to-type mapping — `micold_core::issue_types` (NEW module)

### `LabelTypeEntry` (NEW)

| Field | Type | Serialized as | Validation (FR-019) |
|---|---|---|---|
| `label` | `String` | `"label"` | Not blank after trim; unique among entries ignoring case. |
| `type_` | `ConventionalType` | `"type"`, the `as_str()` token | Closed enum — invalid unrepresentable. |

### `LabelTypeMapping` = `Vec<LabelTypeEntry>` (order significant, FR-017)

- `default_mapping()` → `[bug→fix, enhancement→feat, documentation→docs]` (FR-021).
- `type_for_labels(&[LabelTypeEntry], &[String]) -> Option<ConventionalType>` — the **first entry,
  in mapping order,** whose label equals any issue label ignoring case (Unicode simple case fold via
  `to_lowercase`) (FR-013; AS2 "mapping entry listed first wins", AS5 case-insensitive).
- `validate_mapping(&[LabelTypeEntry]) -> Result<(), MappingError>` with
  `MappingError { index: usize, kind: Blank | Duplicate { of: usize } }` — the first offending entry
  (FR-019).

`ConventionalType` gains `Serialize`/`Deserialize` as its lowercase token. On read, an entry whose
`type` token is unknown is dropped (hand-edited file), not the whole document.

### Persistence — `micold_core::settings::Settings` (extended)

| New field | Type | Default |
|---|---|---|
| `issue_label_types` | `Vec<LabelTypeEntry>` | `default_mapping()` via `#[serde(default = …)]` |

Also on `StoredSettings`. `SETTINGS_VERSION` unchanged (4). Client-owned: written only by the
Settings save through `SettingsStore::update` (research R10).

---

## 4. Naming — `micold_core::naming` (extended)

- `name_from_title(&str) -> String` (NEW, research R11): the longest whole-word prefix of the title
  whose `slugify` is ≤ `ISSUE_NAME_SLUG_MAX = 50`; else the first 50 characters of the title's slug;
  `""` when the title slugs to nothing (FR-010).

`derive`, `slugify`, `WorktreeNaming`, `DerivedNames` are unchanged — an issue-sourced worktree is a
new-branch worktree (FR-012).

---

## 5. Form state — `micold_client::features::worktree_form` (extended)

### `BranchSource` (extended)

```text
BranchSource
├── New
├── Existing
└── Issue          // NEW — FR-001
```

`preview()` and `can_submit()` treat `Issue` **exactly as `New`** (type/ticket/name derivation),
which is FR-012 by construction.

### `State` (feature state, extended)

| New field | Type | Why here |
|---|---|---|
| `issue_request_seq` | `u64` | Monotonic, never reset — outlives any one form so a closed form's late result cannot match a new form's request (research R9, FR-007a). |

### `WorktreeForm` (extended)

| New field | Type | Meaning |
|---|---|---|
| `github` | `GithubAvailability` | Whether the Issue chip is usable, and for which repo. |
| `issues` | `IssueList` | The picker's load state. |
| `issue_query` | `String` | Search text as typed. |
| `issue_matches` | `Vec<(usize, Match)>` | Derived: ranked indices into the displayed issue set (loaded ∪ searched). |
| `issue_list_open` | `bool` | Same rule as `branch_list_open`. |
| `issue_highlight` | `Option<usize>` | Index into `issue_matches`. |
| `picked_issue` | `Option<u64>` | The issue last picked, for the row's selected marker. |

```text
GithubAvailability
├── Checking                           // RemoteList in flight — chip disabled, "Checking for a GitHub remote…"
├── Unavailable(String)                // FR-002 reason: "This repository has no GitHub remote." / lookup failed
└── Available(GithubRepo)              // chip enabled; the caption under the switch names owner/name
                                       // and says it contacts GitHub — before the choice (FR-025)

IssueList
├── NotRequested                                   // before the Issue source is first chosen (FR-003)
├── Loading { seq }                                 // FR-006
├── Failed { error: IssueLoadError }               // FR-007 + retry
└── Loaded {
      listing: IssueListing,                        // FR-004
      gh: PathBuf,                                  // located once per load, reused by its searches (R3)
      searched: Vec<Issue>,                         // beyond-cap matches (FR-005a), deduped by number
      search: SearchState,
    }

SearchState
├── Idle                                  // listing complete, or query empty
├── Pending { seq }                       // debounce timer running (R9)
├── Searching { seq }                     // gh request in flight — "Searching GitHub…"
└── Failed { error: IssueLoadError }      // loaded matches stay; "Search beyond the loaded issues failed" + retry
```

**Invariants** (each has a reducer test in `crates/micold-client/tests/issue_source_state.rs`):

1. `issues` leaves `NotRequested` only on `SourceChanged(Issue)` while `github` is `Available`
   (FR-003). `IssueRetry` acts only from `Failed` (→ `Loading`) or `SearchState::Failed`
   (→ `Searching`); it never starts a first load.
2. A result is applied only if its `seq` equals the one the current state awaits; otherwise dropped
   (FR-007a). With no form open, every result is dropped.
3. `issue_highlight`, when `Some(i)`, satisfies `i < issue_matches.len()`.
4. `searched` never contains a number already in `listing.issues` (FR-005a "without duplicates").
5. A searched issue is displayed only when it matches `issue_query` (FR-005, FR-005a) — it is
   ranked with the loaded ones and unmatched rows are dropped.
6. `SearchState` is never `Pending`/`Searching` when `listing.complete` (FR-005a).
7. Switching the source away from `Issue` keeps `type_`, `ticket`, `name` (spec Edge Cases) and moves
   `issues` back to `NotRequested` — a later return loads afresh; an in-flight result is then stale by
   invariant 2.

### Messages (`worktree_form::Msg`, extended)

| Message | Effect |
|---|---|
| `RemotesListed(Result<Vec<GitRemote>, String>)` | `github` ← `choose_remote` result. |
| `SourceChanged(BranchSource::Issue)` (existing variant) | Only when `github` is `Available`: `issues` ← `Loading{seq}`; the shell starts the load. |
| `IssuesLoaded { seq, result: Result<(IssueListing, PathBuf), IssueLoadError> }` | → `Loaded { listing, gh, … }` / `Failed` (inv. 2). The shell also caches any env-include snapshot the load resolved (it travels beside the message, not in the reducer). |
| `IssueRetry` | New seq; `Failed` → `Loading`, or `SearchState::Failed` → `Searching`. |
| `IssueQueryChanged(String)` | Re-rank locally at once (FR-005); if the listing is incomplete and the query non-empty, `SearchState` ← `Pending{seq}`. |
| `IssueSearchDue { seq }` | If `Pending{seq}` is current: → `Searching{seq}`; the shell runs the search. |
| `IssueSearched { seq, result: Result<Vec<Issue>, IssueLoadError> }` | Merge into `searched` (inv. 4), re-rank (inv. 5), or `SearchState::Failed`. |
| `IssueFocused` / `IssueHighlightMoved(Direction)` / `IssueDismissed` | As the branch picker's. |
| `IssueRowPicked(usize)` | **View message**, an index into `issue_matches`. The shell calls the pure `State::issue_number_at(index) -> Option<u64>` (unit-tested in `issue_source_state.rs`; `None` for an out-of-range index), reads the mapping, and dispatches `IssuePicked`; on `None` it dispatches nothing. |
| `IssuePicked { number, mapping: Vec<LabelTypeEntry> }` | FR-009/010/010a/013/014: `ticket ← number`, `name ← name_from_title(title)`, `type_ ← type_for_labels(mapping, labels)` (clears when `None`), `error ← None`, list closes. The shell fills `mapping` from the settings store at the moment of the pick (FR-014a). A `number` the listing does not hold (loaded or searched) is a no-op. |

---

## 6. Settings draft — `micold_client::features::settings` (extended)

- `SettingsSection::GithubIssues` (NEW, 5th in `ALL`; label "GitHub issues"; icon
  `Icon::IssueMapping`, NEW variant on the existing Material Symbols `label` glyph).
- `ValidSettings` (existing, `features/settings.rs`) gains `issue_label_types`, and
  `into_settings()` copies it, so the client's whole-half save carries the mapping; a save that
  changes only the theme keeps the stored mapping (test in `features_settings.rs`).
- `SettingsDraft.github: GithubDraft` (NEW) — `entries: Vec<(String /*label as typed*/, ConventionalType)>`.
- Messages (NEW): `IssueMappingLabelChanged(usize, String)`, `IssueMappingTypeChanged(usize,
  ConventionalType)`, `IssueMappingAdded` (appends `("", Feat)`), `IssueMappingRemoved(usize)`,
  `IssueMappingMoved(usize, Direction)`, `IssueMappingDefaultsRestored`.
- `SettingsDraft::validate` runs `validate_mapping` and maps `MappingError { index, … }` to
  `FieldError { field: FieldId::IssueMappingLabel(index), section: GithubIssues, message }` —
  saving is refused and nothing is written (FR-019, AS5).

---

## 7. Protocol — `micold_core::protocol::messages` (extended)

| Direction | Variant | Payload |
|---|---|---|
| client → daemon | `ClientMsg::RemoteList` (NEW) | `{ req: u64, project: PathBuf }` |
| daemon → client | `OperationResult::RemoteList` (NEW) | `{ remotes: Vec<GitRemote> }` |
| daemon → client | `OperationError` (existing) | `kind: GitFailed` on a git failure; `reject_non_repo` for a non-repository, as `BranchList` does |

`PROTOCOL_VERSION` 15 → 16 (`protocol/version.rs`), with the pin in `tests/schema_hash.rs`.

Details: [contracts/remote-list-rpc.md](./contracts/remote-list-rpc.md).

---

## Lifetime and persistence

| Data | Lives | Persisted |
|---|---|---|
| `GithubAvailability`, `IssueList`, query, matches | the open form | never (FR-023) |
| `issue_request_seq` | the client process | never |
| `LabelTypeMapping` | `settings.json` | yes (FR-020) |
| GitHub credential | `gh`'s own store | never touched by this app (FR-022, SC-006) |
