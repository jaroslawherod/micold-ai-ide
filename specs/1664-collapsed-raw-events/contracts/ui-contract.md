# UI contract

**Messages**
- `TranscriptMsg::ShowRawEvents(bool)`: emitted by the toggle and by the notice's "Show" button (`true`). The root forwards it to `SettingsMsg::ShowRawEventsToggled(bool)`, so the settings feature stays the only writer of its field (check `tests/feature_write_isolation.rs`). The reducer sets `settings.show_raw_events` and calls `persist_show_raw_events`; a write failure shows the existing "Couldn't save your settings" error and the in-memory value still applies.
- Expanding a summary line reuses `TranscriptMsg::Toggle { session, key }`.

**Rows** (`entries::rows(transcript, show_raw)`): `Omitted?`, `Hidden(n)?`, entries filtered by `visibility::shown`, or `Empty` when there are no entries. Selection (`transcript_selection::rows`/`entries_in`) uses the same filter and has no unit for `Hidden`.

**Layout**: header row = session bar then, at the trailing end, the `LabelledToggle` "Show raw events" (tooltip: "Show session start, turn summaries and unrecognised events"). Summary line = folded `DisclosureCard`, one line tall. Error summaries use the card's `failed` outline plus the word "failed"/"error" in the title (never colour alone).

**Persistence**: `settings.json` key `show_raw_events` (bool, absent reads false).
