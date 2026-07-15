# Contract: `settings.json` On-Disk Schema

Durable format for the persisted application settings. Separate from `projects.json`; same
directory and the same write/recovery discipline as `contracts/storage-schema.md` in feature 002.

## Location

`<data_dir>/settings.json`, where `<data_dir>` is
`directories::ProjectDirs::from("", "", "micold-ai-ide").data_dir()` — the same tuple as the
projects store, so both files sit together. The application tuple MUST stay stable across releases.

## Shape

```json
{
  "settings_version": 1,
  "theme": "follow_system"
}
```

| Field              | Type   | Required | Notes                                                        |
|--------------------|--------|----------|--------------------------------------------------------------|
| `settings_version` | number | yes      | Current schema version. Starts at `1`.                       |
| `theme`            | string | no       | One of `"follow_system"`, `"light"`, `"dark"`. Serde default → `"follow_system"`. |

`theme` serializes `ThemePreference` in snake_case (`#[serde(rename_all = "snake_case")]`).

## Compatibility rules

- **Unknown fields** are ignored on read (forward compatibility).
- **Missing `theme`** takes its serde default (`FollowSystem`).
- **Missing file** → `Settings::default()` (`FollowSystem`), `LoadStatus::Missing` (first run).
- **Unparseable file** → `Settings::default()` and the bad file is preserved to
  `settings.json.bak` (best-effort), `LoadStatus::Recovered` (FR-019). Never crashes (Principle IV).
- **Writes are atomic**: serialize to `settings.json.tmp`, then rename over `settings.json`, so a
  crash mid-save cannot truncate settings.

## Versioning

`settings_version` gates future migrations. A reader encountering a newer version it does not
understand recovers to defaults rather than failing. v1 readers write `settings_version: 1`.
