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
| `settings_version` | number | no       | Current schema version. Starts at `1`. Absent → `0`, the version no build writes *(BUG-003; it was required, and requiring it discarded documents)*. |
| `theme`            | string | no       | One of `"follow_system"`, `"light"`, `"dark"`. Serde default → `"follow_system"`. |

`theme` serializes `ThemePreference` in snake_case (`#[serde(rename_all = "snake_case")]`).

## Compatibility rules

- **Unknown fields** are ignored on read (forward compatibility).
- **No field is required.** Every field of the document, `settings_version` included, takes its
  serde default when the document omits it. A document is unreadable only when it is not a JSON
  object at all. *(Amended by BUG-003: `settings_version` was the one required field, so a
  hand-written file that omitted the one number nothing reads was treated as corrupt and every
  setting in it was lost.)*
- **Missing `theme`** takes its serde default (`FollowSystem`).
- **Missing file** → `Settings::default()` (`FollowSystem`), `LoadStatus::Missing` (first run).
- **Unparseable file** → `Settings::default()` and the bad file is preserved to
  `settings.json.bak` (best-effort), `LoadStatus::Recovered` (FR-019). Never crashes (Principle IV).
- **Writes are atomic**: serialize to `settings.json.tmp`, then rename over `settings.json`, so a
  crash mid-save cannot truncate settings.

## Versioning

`settings_version` gates future migrations. A reader encountering a newer version it does not
understand recovers to defaults rather than failing. v1 readers write `settings_version: 1`.

No migration gate has ever been written, and every schema version since v1 has been reached by
adding defaulted fields precisely so that none was needed. The field is therefore documentation on
disk: readers write the current version and no reader branches on the value. That is why it is not
required on read — a field nothing consumes must not be a field a document can be rejected for
omitting *(BUG-003)*. `0` means the document named no version; readers MUST NOT read it as a claim
that the document was written by the current build.
