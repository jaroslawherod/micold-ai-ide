# Data Model: Report a Missing Environment-Include Script

Everything here is in memory. Nothing is persisted (FR-010). `Settings` (feature 011) is
unchanged: the enabled flag, the path and the timeout.

## Core: `micold_core::script_path_check` (new)

### `ScriptPathState` (enum)

The spec's *Script Path Check* entity, without its path.

| Variant | Meaning | Produced when |
|---|---|---|
| `Present` | a regular file the current user can open for reading | absolute path, `metadata` is a file, `File::open` succeeds |
| `NotFound { tilde: bool }` | no such file | absolute path and `metadata` reports `NotFound`; or the string is `~` or starts with `~/` or `~\` (`tilde: true`, no probe; a string test on every OS) |
| `NotReadable` | it exists but cannot be sourced | a directory or other non-file, an `open` error, or any `metadata` error other than `NotFound` |
| `Relative` | not checked: the answer depends on the session's directory | `!Path::is_absolute()` and not `~` |
| `Unchecked` | no answer within the bound | `check_bounded` timed out (2 s) |

Invariant (Principle V): there is no "blank" variant. A blank path has no check at all
(`Option::None`, FR-011).

### `ProbeAnswer` (enum), the capability's raw answer

`File`, `NotAFile`, `Missing`, `Unreadable`. `classify` maps these to `ScriptPathState`. It is
separate so that the fake can answer without touching a filesystem.

### `CheckedScriptPath` (struct)

| Field | Type | Notes |
|---|---|---|
| `path` | `String` | the stored path exactly as checked (not trimmed or expanded) |
| `enabled` | `bool` | the stored enabled flag when the check started (research R8) |
| `state` | `ScriptPathState` | |

### Functions

- `classify(path: &str, probe: &dyn ScriptPathProbe) -> Option<ScriptPathState>`: blank gives
  `None`. For `~` and relative paths the probe is never called (R2).
- `check_bounded(probe: Arc<dyn ScriptPathProbe + Send + Sync>, path: String, bound: Duration) ->
  Option<ScriptPathState>`: `classify` on a worker thread. A timeout gives `Some(Unchecked)`.
- `SCRIPT_PATH_CHECK_BOUND: Duration = 2 s`.

## Client: `features::settings` (additions)

### `CheckOrigin` (enum)

`Opened` (Settings shown, or another window's change while shown) and `Saved` (this window's
save). Only `Saved` can post a notification (FR-004, FR-007).

### `ScriptCheck` (enum), on `features::settings::State`

| Variant | Meaning |
|---|---|
| `Idle` | no check started yet (boot), or the path is blank |
| `Pending { seq: u64, last: Option<CheckedScriptPath> }` | a check is in flight. `last` is the previous `Done` answer, still shown so a re-check does not blank the notice; with none, the page shows 011's note as it does today |
| `Done(CheckedScriptPath)` | the latest applied answer |

Also on `State`:

- `script_check_seq: u64`, the sequence number of the latest check started (display gate)
- `script_check_save_seq: Option<u64>`, the latest check a save started and has not yet notified
  for (notification gate)

State transitions:

```text
Idle/Done ──start(origin)──▶ Pending{seq = ++script_check_seq, last = previous Done, if any}
                               (Saved also sets script_check_save_seq = Some(seq))
Pending{seq} ──ScriptPathChecked{seq, Some(state)}──▶ Done(checked)
Pending{seq} ──ScriptPathChecked{seq, None}─────────▶ Idle           (blank path)
any ──ScriptPathChecked{other seq}──▶ unchanged (display); notification step still runs
```

### `NoticeLine` (enum), the output of `script_path_notice`

`Caution(String)` renders through `ui::settings::caution`. `Note(String)` renders through
`ui::settings::note`. The order is the order on the page.

## Validation rules (from the spec)

- The path is taken literally, and `~` is not expanded (FR-001).
- A relative path is never probed (FR-001).
- A check never runs or sources the script (FR-003). The real probe only calls `metadata` and
  `open`.
- A check never starts from a session launch or terminal restart (FR-006).
