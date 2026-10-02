# Contract: the history saving setting

Research: [R10](../research.md#r10-the-setting),
[R9](../research.md#r9-removing-sweeping-and-turning-saving-off).

## 1. Stored

`settings.json` gains `"save_terminal_history": <bool>`. `#[serde(default)]` gives `true`, so a file
written before this feature reads as on (FR-026, FR-029). `SETTINGS_VERSION` is unchanged: the
field is additive.

## 2. On the wire

| Message | Change |
|---|---|
| `DaemonSettings` | new field `save_terminal_history: bool` |
| `ClientMsg::SettingsSet` | new field `save_terminal_history: Option<bool>`; `None` leaves it unchanged, as its other fields do |

`PROTOCOL_VERSION` goes up by one (the next free number when the milestone is implemented) and
`SCHEMA_HASH` changes. The setting is service-owned: the service stores it, applies it and
broadcasts `SettingsChanged`, so every window shows the same value. The path copied end to end is
`pi_activity_component`'s.

## 3. What a change does

The service applies a change after storing it and before replying to `SettingsSet`.

| Change | Effect | Requirement |
|---|---|---|
| on → off | `HistoryStore::set_enabled(false)`: every saved history is deleted; a save in flight writes nothing or its file is deleted, because both hold the store's mutex. No confirmation. Failed deletions are logged once each as a warning with the session and the reason and retried every 30 s | FR-027, FR-033 |
| on → off | `carried` and every running `Term` are untouched: the terminals show what they showed, and a stop and start in this service run still shows the earlier output | FR-033, FR-015 |
| off → on | `HistoryStore::set_enabled(true)`; a file whose deletion had failed is still retried and is never loaded; the schedule of every running covered terminal is marked due, so each is saved at the next tick (within 5 s, inside the 60 s allowed) | FR-027 |
| off → on | Histories deleted earlier do not come back; a stopped session's carried snapshot is not written until the session runs again | story 2 scenario 7 |

At service start: saving off → `purge()` before the first connection is accepted; saving on →
`sweep()` ([saved-history-file §6](./saved-history-file.md)).

While the setting is off, a start after a service restart finds nothing to load and shows no
separator (FR-028).

## 4. The control

Settings → Terminal, below *Scrollback lines*. The shared `Checkbox` with `field_note` (FR-031).

| Part | Text |
|---|---|
| Label | Save terminal history |
| Note | Terminal output is written to this computer's disk while this is on. Turning it off deletes the saved history. |

It is saved with the section's other fields by the existing Save action; there is no separate
confirmation (spec Assumptions). The entry is added to the section's `SETTINGS` list so the
settings search finds it.
