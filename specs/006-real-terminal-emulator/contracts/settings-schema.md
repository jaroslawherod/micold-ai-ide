# Contract: Settings Schema — `scrollback_lines`

Extends the settings document defined by feature 003
(`specs/003-material-design-layout/contracts/settings-schema.md`). Reuses
`JsonFileSettingsStore` at `<data_dir>/settings.json`. Governs FR-019, FR-020, FR-021.

## Change

Add one field to both the in-memory `Settings` and the on-disk `StoredSettings`:

```
Settings {
    theme: ThemePreference,           // existing
    scrollback_lines: usize,          // NEW — per-session terminal scrollback limit
}
```

On-disk (`StoredSettings`):

```jsonc
{
  "settings_version": 2,              // bumped from 1 (documentation only)
  "theme": "FollowSystem",
  "scrollback_lines": 10000           // NEW
}
```

## Rules

- **Default**: `scrollback_lines` defaults to `10_000` (matches `alacritty_terminal 0.25.1`
  `Config::scrolling_history` default). Implemented with `#[serde(default = "default_scrollback")]`.
- **Backward compatibility**: an existing `settings.json` written by 003/005 has no
  `scrollback_lines`; on load it takes the default. A missing/corrupt file still degrades to
  `Settings::default()` (Principle IV recovery — unchanged).
- **Validation**: on save from the Settings form, the value MUST be parsed and constrained to a
  sane inclusive range (recommended `100..=1_000_000`); out-of-range input is rejected with a
  message and not persisted (FR-020, FR-021).
- **Version**: `settings_version` becomes `2`. Readers MUST NOT reject an unknown newer version
  destructively; unknown fields are ignored (forward compatibility, per the 003 contract).

## Application

- Each session's `Term` is created with `Config { scrolling_history: settings.scrollback_lines,
  ..Config::default() }`. The value applies to sessions spawned after a change (FR-020 minimum);
  applying to already-running terminals (`Grid::update_history`) is optional and out of required
  scope.

## Tests (`tests/settings_scrollback.rs`, written first — TDD)

- Serialize→deserialize roundtrip preserves `scrollback_lines`.
- A JSON document without the field loads with the default `10_000`.
- Out-of-range values are rejected/clamped by the validator with a clear message.
- A corrupt file still yields `Settings::default()` (regression against 003 behavior).
