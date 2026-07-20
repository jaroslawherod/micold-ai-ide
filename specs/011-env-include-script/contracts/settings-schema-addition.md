# Contract: Settings Schema Addition (Environment-Include Fields)

**Module**: `src/settings.rs`. Extends the schema documented in feature 003's
`contracts/settings-schema.md` (this file documents only the *addition*, mirroring how feature
010's `contracts/persistence-schema.md` documented `StoredSession.mode` as an addition to the
projects store rather than rewriting 003's original contract in place).

## Fields added to `Settings` and `StoredSettings`

| Field | Type | `#[serde(default = ...)]` | Default value |
|---|---|---|---|
| `env_include_enabled` | `bool` | `default_env_include_enabled` | `true` |
| `env_include_script_path` | `String` | `default_env_include_script_path_string` | `default_env_include_path(home).to_string_lossy()` (R7) |
| `env_include_timeout_secs` | `u64` | `default_env_include_timeout_secs` | `10` |

`SETTINGS_VERSION` moves from `2` to `3` (doc-comment bookkeeping — see `settings.rs`'s existing
comment convention: "Bumped to `2` in feature 006 when `scrollback_lines` was added"; this becomes
"Bumped to `3` in feature 011 when the environment-include fields were added").

## Backward compatibility

A settings file written by any prior version of the app (missing all three fields, or missing
`settings_version: 3` entirely) loads successfully: the three `#[serde(default = ...)]` attributes
supply the defaults above, exactly as `scrollback_lines`'s `#[serde(default = "default_scrollback")]`
already does for v1 files. **No migration code, no version-gated branch** — this is purely
additive, matching the "missing field still defaults on read" contract `settings.rs`'s own module
doc comment already states as the established pattern.

## Round-trip contract

`StoredSettings::from_settings` / `into_settings` gain the three fields symmetrically (write what
was read, read back what was written) — `into_settings` additionally clamps
`env_include_timeout_secs` via `clamp_env_include_timeout`, mirroring how `into_settings` already
clamps `scrollback_lines` via `clamp_scrollback` on the read path (so an out-of-range value that
somehow reached disk self-heals on next load, rather than requiring a save to fix).

## Failure/corruption handling

Unchanged from the existing contract: a corrupt or unreadable settings file (any reason) degrades
to `Settings::default()` (all three new fields at their defaults) with `LoadStatus::Recovered`,
never a crash or a hard error — the existing `JsonFileSettingsStore::load` implementation already
guarantees this for the whole document, not per-field, so no new code path is needed here beyond
adding the fields themselves.

## What is explicitly NOT part of this schema

- No captured environment variable values, ever (FR-008).
- No failure diagnostic text, ever (FR-013) — that lives only on `App`'s in-memory
  `EnvIncludeSnapshot` (`data-model.md`), never serialized.
