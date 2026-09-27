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
  serde default when the document omits it, so an absent field is never a reason a document cannot
  be read. *(Amended by BUG-003: `settings_version` was the one required field, so a hand-written
  file that omitted the one number nothing reads was treated as corrupt and every setting in it was
  lost.)*
- A **present** field can still make the document unreadable, and that is deliberate: a value of the
  wrong JSON type, or a string naming a variant this build does not know (`"theme": "Dark"` — the
  values are snake_case, so only `"dark"` parses; a `default_ai_cli` naming a CLI added by a later
  release) fails the whole read and takes the recovery path below. Declining to load beats guessing
  at a value the user chose. The two halves are not symmetric on purpose: an absent field is a
  question the defaults answer, a present-but-unreadable one is not.
- One consequence of the first rule, stated so it is not discovered: absence is silent. A key
  misspelled by hand is an unknown key, and an unknown key is ignored, so the field it was meant to
  set takes its default and nothing is reported. A document therefore loads cleanly whether it set a
  field or merely tried to.
- **Missing `theme`** takes its serde default (`FollowSystem`).
- **Missing file** → `Settings::default()` (`FollowSystem`), `LoadStatus::Missing` (first run).
- **Unparseable file** → `Settings::default()` and the bad file is preserved to
  `settings.json.bak` (best-effort), `LoadStatus::Recovered` (FR-019). Never crashes (Principle IV).
- **Writes are atomic**: serialize to `settings.json.tmp`, then rename over `settings.json`, so a
  crash mid-save cannot truncate settings.

## Versioning

`settings_version` was intended to gate future migrations. v1 readers write `settings_version: 1`,
and each reader writes its own version.

No migration gate has ever been written, and every schema version since v1 has been reached by
adding defaulted fields precisely so that none was needed. The field is therefore documentation on
disk: readers write the current version and **no reader branches on the value** — a document
declaring a version this build has never heard of loads as a current document, because every field
in it is read on its own terms. *(Amended by BUG-003, which also retires this section's earlier
claim that an unrecognised newer version recovers to defaults. No reader has ever done that, and it
described the field as load-bearing when nothing loads on it.)*

Because nothing consumes the field, it is not required on read: a field nothing reads must not be a
field a document can be rejected for omitting. Absent, it reads as `0`. `0` means **the document
named no version**. A reader MUST NOT take it as a claim that the current build wrote the document,
and a migration gate, should one ever be written, MUST NOT take it as a version older than v1 — a
document without the number is most often a current one written by hand.
