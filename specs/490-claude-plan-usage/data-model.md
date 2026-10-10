# Data Model: Show Claude Plan Usage and the Next Limit Reset

All types are new unless marked. Core types live in `crates/micold-core/src/plan_usage.rs` (new
module, render-free, no I/O except where marked).

## UsageReading (core, wire)

The account's plan usage at one moment. Source-neutral: every source produces it (research R1).

| Field | Type | Rule |
|---|---|---|
| `windows` | `Vec<LimitWindow>` | Non-empty. Ordered as the source gave them. |
| `obtained_at` | `i64` (Unix seconds) | Stamped by the daemon when it accepts the reading (R5). |

Derives `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`.

Operations (pure, `now: i64`):

- `current(&self, now) -> Option<CurrentUsage>`: the windows with `resets_at` absent or
  `> now`, with the headline re-picked among them. `None` when none remain (FR-009, R7). A window with no `resets_at` stays current until a
  newer reading replaces it.
- `same_windows(&self, other) -> bool`: equal ignoring `obtained_at` (R6 dedupe).

## LimitWindow (core, wire)

| Field | Type | Rule |
|---|---|---|
| `key` | `String` | The source's key (`five_hour`, `seven_day`, …). Non-empty. |
| `used_tenths` | `u32` | Percent × 10, rounded. Negative input → window dropped. Above 1000 kept as is (spec Edge Cases). |
| `resets_at` | `Option<i64>` | Unix seconds. A value `<= obtained_at` when parsed → window dropped. Absent → shown without a reset. |

- `name(&self) -> String`: "5-hour", "Weekly", else the key humanised (R12).
- `percent_label(&self) -> String`: whole percent, rounded half up (`"42%"`).
- `at_or_above(&self, threshold: WarnPercent) -> bool`: `used_tenths >= threshold * 10`.

## CurrentUsage (core, not on the wire)

What a view draws, from `UsageReading::current`.

| Field | Type | Rule |
|---|---|---|
| `windows` | `Vec<LimitWindow>` | Current windows only (FR-009). |
| `headline` | index into `windows` | Highest `used_tenths`; tie → earliest `resets_at` (absent last); tie → first (spec Terms, "Next limit reset"). |
| `obtained_at` | `i64` | Copied. |

- `warning(&self, threshold) -> Vec<&LimitWindow>`: windows at or above the threshold (FR-010).
- `label(&self, now, offset) -> String`: `"42% · 15:30"`, or `"42%"` when the headline has no reset.
- `details(&self, threshold, now, offset) -> Vec<String>`: one line per window
  (`"5-hour: 42%, resets 15:30"`), then `"At or above 80%: 5-hour"` when any, then
  `"Updated 14:02"` (FR-006, FR-010).

## WarnPercent (core)

`u8` newtype, 50..=100. `WarnPercent::clamped(u64)` (read, FR-005);
`WarnPercent::parse(&str) -> Result<_, FieldError-compatible message>` refuses out of range on save
with "Enter a whole number from 50 to 100". Default `80`.

## Status-line extraction (core, source A only: `plan_usage::status_line`)

- `extract(stdin: &[u8]) -> Option<serde_json::Value>`: the `rate_limits` object of a status-line
  input, `None` when stdin is not JSON or has none. Used by the relay; only this object is posted.
- `parse(body: &serde_json::Value, obtained_at: i64) -> Option<UsageReading>`: every member of
  `rate_limits` except `spend_limit` whose value is an object with a numeric `used_percentage`
  becomes a `LimitWindow` (rules above). `None` when no window survives (spec Edge Cases, empty or
  partial reading). Used by the daemon on the `/status` body.

## UserStatusLine (core, `plan_usage::user_status_line`, reads files)

The user's own status line, resolved at session prepare time (R4).

| Field | Type |
|---|---|
| `command` | `String` |
| `padding` | `Option<u32>` |
| `refresh_interval` | `Option<u32>` |

- `resolve(locations: &ConfigLocations, cwd: &Path) -> Option<UserStatusLine>`: first of
  `cwd/.claude/settings.local.json`, `cwd/.claude/settings.json`,
  `(claude_config_dir or home/.claude)/settings.json` whose `statusLine` is a `type: "command"`
  object with a non-empty `command`. Deserialises only `statusLine`. Absent, unreadable or
  malformed files name nothing. `ConfigLocations` is existing (`mcp/binding.rs`); it gains a
  `claude_settings_dir()`: `CLAUDE_CONFIG_DIR` when set, else `home/.claude` (not the directory of
  `claude_json()`, which is `~/.claude.json` in the home directory when the variable is unset).

## RelayConfig (core, serde; written by the daemon, read by the relay)

`<settings_dir>/<session-uuid>.status.json`, owner-only (`micold_core::owner_only::write`).

| Field | Type |
|---|---|
| `url` | `String` — `http://127.0.0.1:<port>/status/<uuid>` |
| `token` | `String` — the session's existing hook token |
| `user_command` | `Option<String>` |

Lifecycle: written by `prepare_settings` when the switch is on; deleted when the switch is turned
off, and by `HookReceiver::forget` at session end (with its token).

## Settings (existing `micold_core::settings::Settings`, extended)

| Field | Type | Default | Rule |
|---|---|---|---|
| `plan_usage_enabled` | `bool` | `true` (`DEFAULT_PLAN_USAGE_ENABLED`) | serde default (FR-001). |
| `plan_usage_warn_percent` | `u8` | `80` | serde default; clamped 50–100 on read (FR-005). |

Both mirrored in `DaemonSettings` (existing, wire) and `ClientMsg::SettingsSet` (existing, gains two
`Option` fields). Additive: `settings_version` unchanged.

## Daemon state (existing `DaemonState`, extended)

- `plan_usage: Option<UsageReading>` in `Inner`, memory only (FR-014).
- `note_plan_usage(&self, session, reading) -> bool`: ignored (returns false) while
  `plan_usage_enabled` is off or the session is unknown; replaces the stored reading (FR-019);
  returns whether to broadcast (R6).
- `set_plan_usage_enabled(bool)`: persists, rewrites live sessions' settings and relay files
  (R9), and on `false` clears the reading; broadcasts `SettingsChanged` and, when cleared,
  `PlanUsageChanged { reading: None }`.

## Client state (new render-free feature module `features/plan_usage.rs`)

| Field | Type |
|---|---|
| `reading` | `Option<UsageReading>` |

Updated from `Welcome.plan_usage` and `PlanUsageChanged`; reset to `None` on disconnect. The view
asks `reading.current(now)` with the settings' threshold; nothing is drawn when the switch is off
or `current` is `None`. Subscription: `Tick` every 30 s only while `reading.is_some()` (R7).

## State transitions (one account reading)

```
None --(accepted reading, switch on)--> Some(r)
Some(r) --(accepted reading r')--> Some(r')            (newest wins, FR-019)
Some(r) --(switch off)--> None                         (FR-002)
Some(r) --(every window's reset passes)--> drawn as hidden; dropped on next arrival/connect (FR-009)
any --(daemon restart)--> None                          (FR-014)
```
