# Data model: The start affordance answers for its own directory

Client-side, in memory only (FR-003, 026 R11: never persisted). No wire type and no persisted file
changes. Rationale for each choice: [research.md](./research.md).

## `AvailabilityKey` (new, `crates/micold-client/src/features/session.rs`)

What one answer is *about*.

| Variant | Meaning | Asked with |
|---|---|---|
| `Home` | The user's home directory: the Settings default (FR-008) and every row's fallback (FR-005) | `cwd: None` |
| `Dir(PathBuf)` | One row directory: `SessionLocation::cwd(project_root)`, a project root or a worktree (FR-007) | `cwd: Some(dir)` |

A closed enum rather than `Option<PathBuf>` (Principle V): "home" is a fixed place with its own
consumers, not an absent directory.

## `CliAvailability` (existing, unchanged)

`{ available: Vec<AiCli>, source: AvailabilitySource }`: one answer, stamped on arrival with what it
describes (027 FR-023c). An empty `available` is a real answer.

## `AvailabilityAnswers` (new, replaces `session::State::available_providers`)

| Field | Type | Rule |
|---|---|---|
| `home` | `Option<CliAvailability>` | `None` = not answered since the last connect |
| `dirs` | `HashMap<PathBuf, CliAvailability>` | At most one per directory (FR-003). Only for wanted directories (R3) |
| `latest` | `HashMap<AvailabilityKey, u64>` | The `req` of the newest request per key (R4) |
| `in_flight` | `HashMap<u64, AvailabilityKey>` | Which key each unanswered request named (R1) |

### Operations

| Operation | Effect | Requirement |
|---|---|---|
| `asked(req, key)` | `latest[key] = req`, `in_flight[req] = key` | FR-004, FR-009 |
| `answered(req, CliAvailability) -> bool` | Look up and remove `in_flight[req]`. Unknown `req` → drop and return `false`. `latest[key] != req` → drop and return `false`. Otherwise store under `key` (`home` or `dirs[dir]`) and return `true` | FR-002, FR-009 |
| `for_dir(&Path) -> Option<&CliAvailability>` | `dirs[dir]` if held, else `home` | FR-001, FR-005 |
| `home() -> Option<&CliAvailability>` | `home` only, never a directory's answer | FR-002, FR-008 |
| `retain(&BTreeSet<PathBuf>)` | Drop `dirs`, `latest` and `in_flight` entries whose `Dir` is not wanted. `Home` is kept | FR-003, FR-012 |
| `unasked(&BTreeSet<PathBuf>) -> Vec<PathBuf>` | Wanted directories with no held answer and no request in flight | FR-006 (once per distinct directory) |
| `clear()` | Everything, `home` included | FR-011 |

### State transitions for one directory `d`

```text
            appears (sync)               answer, req == latest[d]
 absent ──────────────────▶ in flight ─────────────────────────▶ held
   ▲                         │  ▲                                │
   │   leaves wanted set /   │  │ refresh: start list opened,    │
   │   reconnect (clear)     │  │ env-include changed            │
   └─────────────────────────┴──┴────────────────────────────────┘
                               (held stays readable while re-asked; a newer answer replaces it)
```

While `d` is absent or in flight with nothing held, `for_dir(d)` returns the home answer (FR-005).

## Readers on `session::State` (changed signatures)

Each takes the answer to read instead of reading one window-wide set:

| Reader | Today | After |
|---|---|---|
| `known_available()` | window-wide slice | `available_in(key_dir: Option<&Path>) -> &[AiCli]`: `None` = home, `Some(d)` = `for_dir(d)`. Empty when nothing is held |
| `default_ai_cli_is_available()` | window-wide | takes `Option<&Path>` |
| `offered_providers()` | window-wide | takes `Option<&Path>` |
| `start_affordance_offers_a_choice()` | window-wide | takes `&Path` (the row directory) |
| `start_intent(target)` | window-wide | `start_intent(target, &Path)` |

`crate::app::State::location_dir(&SessionLocation) -> Option<PathBuf>` resolves a row to its
directory: `workspace.active` joined through `SessionLocation::cwd`. It is the only place the row →
directory rule is applied on the client, and it is the same rule as the spawn's.

## Wanted directories (derived, never stored)

`features::session::wanted_availability_dirs(&app::State) -> BTreeSet<PathBuf>`: the active
project's root plus `cwd` of each worktree with `can_start_session()`. Empty when no project is
active.

## Removed

- `session::State::available_providers`, replaced by `AvailabilityAnswers`.
- `App::cli_availability_asked`, replaced by `AvailabilityAnswers::latest` / `in_flight`.
