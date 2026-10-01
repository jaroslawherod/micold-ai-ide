# Data Model: Explain Why an AI CLI Is Not Offered

Nothing is persisted. Every type below lives in memory or on the wire for the length of one
availability answer.

## `SpawnEnv` (new, `micold_core::cli_reason`, on the wire)

The state of the environment a session in one directory gets. One variant per row of FR-001.

| Variant | FR-001 row | Reports an attempt (FR-004a) | Script applied |
|---|---|---|---|
| `IncludeOff` | Environment-include is off | no | no |
| `NoScriptPath` | On, script path is blank | no | no |
| `ScriptNotFound` | On, script path names no file | yes | no |
| `ScriptFailed` | On, last attempt exited with an error | yes | no |
| `ScriptTimedOut` | On, last attempt timed out | yes | no |
| `Applied` | On, last attempt succeeded | yes | yes |

- `Copy`, `Eq`, `Serialize`, `Deserialize`. No payload: the script's output is never carried
  (FR-007).
- `SpawnEnv::classify(enabled: bool, script_path: &str, attempt: Option<&EnvIncludeOutcome>) ->
  Option<SpawnEnv>` is the only constructor outside tests (research R2). `None` when the settings
  call for an attempt and none is known.
- `script_applied()` is true only for `Applied`. FR-002's limit is written against it.

## `ResolvedEnv` (new, `micold-daemon`, replaces the cached `Vec`)

| Field | Type | Meaning |
|---|---|---|
| `vars` | `Vec<(String, String)>` | What the cell holds today: the resolved variables merged with `TERM` |
| `env` | `SpawnEnv` | The state that produced `vars` |

- One per directory in `Inner::env_include_cache`, inside the existing `OnceLock` cell. Written
  once by the attempt that fills the cell. Removed by the invalidations that exist today
  (`set_env_include`, `invalidate_env_include`).
- For the two settings states no cell is taken, as today: `spawn_env_for` returns
  `ResolvedEnv { vars: merge_with_term(&[]), env: IncludeOff | NoScriptPath }`.
- Invariant: `vars` and `env` come from one call of `env_include::resolve`. Nothing writes one
  without the other.

## `DaemonMsg::AiCliAvailability` (changed)

| Field | Type | Change |
|---|---|---|
| `req` | `u64` | unchanged |
| `available` | `Vec<AiCli>` | unchanged |
| `env` | `Option<SpawnEnv>` | **new**. The state of the environment `available` was walked in. `None` when the service could not resolve a directory at all |

`PROTOCOL_VERSION` 18 → 19.

## `CliAvailability` (changed, `micold-client` `features/session.rs`)

| Field | Type | Change |
|---|---|---|
| `available` | `Vec<AiCli>` | unchanged |
| `source` | `AvailabilitySource` | unchanged: this computer, or the image |
| `env` | `Option<SpawnEnv>` | **new**, from the wire |
| `asked_for` | `AvailabilityKey` | **new**. `Home` or `Dir(path)`: the key this answer was filed under, stamped by `AvailabilityAnswers::answered` |

- The offer (`available`) and the reason (`env`) are fields of one value, so a surface cannot pair
  one answer's offer with another's reason (FR-012).
- A row that has no answer of its own reads the home answer, whose `asked_for` is `Home` (FR-004a).
- Replaced as a whole when a newer answer for the key is filed (FR-013). Dropped with the
  connection, as today.

## `Place`, `AttemptDir`, `Explanation` (new, `micold_core::cli_reason`)

| Type | Shape | Source |
|---|---|---|
| `Place<'a>` | `ThisComputer` \| `Image(&'a str)` | client: `AvailabilitySource`. Service: `MICOLD_IMAGE_REFERENCE` |
| `AttemptDir<'a>` | `Home` \| `Dir(&'a Path)` | client: `CliAvailability::asked_for`. Service: the session's or the request's directory |
| `Explanation` | `{ reason: String, action: String }` | `explain(..)` |

`Explanation` is the spec's **Unavailability reason**. It is built on demand from a
`CliAvailability` (client) or a `ResolvedEnv` (service) and never stored.

## State transitions

`SpawnEnv` for a directory changes only when the directory's cell is replaced:

```text
settings saved (env-include field changed)  → every cell removed → next ask classifies afresh
worktree deleted                            → that directory's cell removed
otherwise                                   → the cell, and so the state, stays
```

The client's copy changes only when a newer answer is filed for the key (033 contract C2).
