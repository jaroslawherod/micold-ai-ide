# Implementation Plan: Collapsed Summaries for Raw Events in the Structured View

**Branch**: `feat/1664_structured-view-raw-events-are-shown-inline-in-the` | **Date**: 2026-11-03 | **Spec**: [spec.md](spec.md)

## Summary

Four kinds of transcript entry (`Raw`, `SessionStart`, `Footer`, `OrphanResult`) are *low-signal*. Each
is drawn as one collapsed `DisclosureCard` summary line (title + facts, detail behind the fold), and a
"Show raw events" toggle in the structured view's header hides them (off by default, a persisted
per-user boolean in `Settings`). Hidden entries stay in the `Transcript`; an error footer and an error
orphan result are never hidden (FR-013). While entries are hidden a one-line notice shows the count and
a "Show" button. The classification is one pure function in `micold-core` that the renderer and the
selection/copy row list both use, so a hidden entry is neither drawn, matched nor copied. Live and
resumed transcripts are the same `Transcript` type, so FR-011 needs no second path.

## Technical Context

**Language/Version**: Rust (workspace `rust-toolchain.toml`), iced.
**Primary Dependencies**: existing crates only: `micold-core` (transcript model, `Settings`), `micold-client` (view, reducer, persistence), `micold-material` (`DisclosureCard`, `LabelledToggle`, `Button`).
**Storage**: the client's local `settings.json` through `SettingsStore` (`write_setting` / `update_reporting`); new key `show_raw_events`, serde default `false`.
**Testing**: `cargo test` via mise; core unit tests, client reducer and view-model tests under `crates/micold-client/tests/`, geometry gate for the one-line height, quickstart §B visual pass.
**Target Platform**: Linux, macOS, Windows (no platform code).
**Project Type**: desktop app (iced).
**Performance Goals**: no extra per-frame work beyond one filter pass over entries; the entry `lazy` keys are unchanged.
**Constraints**: offline, no new dependency, no wire/protocol change.
**Scale/Scope**: transcripts bounded by the existing entry bound; hidden count is O(entries) per rebuild.

## Constitution Check

| Principle | Result |
|---|---|
| I. Test-First | PASS. Tests listed per requirement below; tdd-plan orders them before code. |
| II. Multi-Session | PASS. The toggle is deliberately per user, not per session (spec); expansion stays per session in `structured_transcript::State.open`. No session state leaks. |
| III. Worktree Integration | PASS. No file or VCS operation. |
| IV. Local-First | PASS. One boolean in the local settings file; nothing leaves the device. |
| V. Rust + iced | PASS. `LowSignal` classification is a closed enum matched exhaustively in core, so a new `EntryKind` variant fails to compile until classified. |
| VI. Cross-Platform | PASS. No platform code; the settings file path already abstracts platforms. |
| VII. Documentation | PASS. User-guide page and `SUMMARY.md` entry in the same change (see Project Structure). |
| VIII. Reusable UI | PASS. Reuses `DisclosureCard`, `LabelledToggle`, `Button`; no new widget. Summary text lives in the shared view-model module `shown.rs`. |

## Requirement map

| FR | Where |
|---|---|
| FR-001, 002 | `shown.rs` summary functions (title, facts) + `entries.rs` draws a folded `DisclosureCard` for the four kinds |
| FR-003 | existing `State.open` toggle (`TranscriptMsg::Toggle`); folded when absent |
| FR-004 | raw detail = `plain_text(text)` (tabs to spaces only), in a height-bounded scrollable |
| FR-005 | `LabelledToggle` in the session bar row, `ui/transcript/mod.rs` `view_full` |
| FR-006, 007, 013 | `visibility::shown(kind, show_raw)` in core, used by `rows()` |
| FR-008 | `Transcript::hidden_count(show_raw)` + `Row::Hidden(n)` notice with a "Show" button |
| FR-009 | `Settings.show_raw_events`, window copy in `settings::State`, written by `persist_show_raw_events` |
| FR-010 | filtering is view-side only; `Transcript` is untouched |
| FR-011 | one `Transcript` type for live and resumed sessions |
| FR-012 | non-low-signal kinds never reach the new paths; guarded by a test over every other `EntryKind` |

Details: [data-model.md](data-model.md), [contracts/ui-contract.md](contracts/ui-contract.md),
[research.md](research.md), [quickstart.md](quickstart.md).

## Project Structure

### Documentation

```text
specs/1664-collapsed-raw-events/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
└── contracts/ui-contract.md
```

### Source Code

```text
crates/micold-core/src/structured/transcript/
├── visibility.rs      # NEW: LowSignal, Entry::low_signal, shown(), Transcript::hidden_count
├── mod.rs             # re-export
└── entry.rs           # unchanged
crates/micold-core/src/settings.rs   # + show_raw_events: bool (serde default false)
crates/micold-client/src/
├── features/transcript_selection/shown.rs   # + summary/notice text fns (pure, shared)
├── features/transcript_selection.rs         # rows()/entries_in() filtered; TranscriptView gains show_raw
├── features/structured_transcript.rs        # TranscriptMsg::ShowRawEvents(bool)
├── features/settings.rs                     # State.show_raw_events; SettingsMsg::ShowRawEventsToggled
├── shell/persist.rs                         # persist_show_raw_events; window_settings / set_changed / literals
├── ui/transcript/entries.rs                 # rows(.., show_raw), Row::Hidden, summary-card arms, hidden notice
├── ui/transcript/mod.rs                     # toggle in bar; RowKey::Hidden; pass show_raw
└── main.rs                                  # route the message (check feature_write_isolation)
docs/user-guide/structured-transcript.md     # NEW, + docs/SUMMARY.md line
```

## Test strategy

| Requirement | Layer | Test |
|---|---|---|
| classification, FR-013, FR-010, FR-012 | core unit (`visibility.rs`) | every `EntryKind` classified; error footer/orphan always shown; ok ones follow the flag; `hidden_count` excludes errors; transcript length unchanged by any flag value |
| settings key | core (`tests/settings_roundtrip.rs` style) | default false; a file without the key reads false; round-trips true; other keys untouched |
| FR-002, edge cases | client view-model (`shown.rs` unit + `tests/transcript_entry_view.rs`) | summary text for each raw reason, empty raw text, footer with/without figures, session start with/without model |
| FR-001, 003, 004, 006, 007, 008, SC-001, SC-002 | client (`tests/transcript_entry_view.rs`) | `rows()` with flag off/on over a mixed transcript: counts, order, notice, no raw text painted off; raw detail text byte-equal when open |
| SC-003, US1-4 | client geometry (`tests/transcript_follow_layout.rs` style) | a collapsed summary card is one line tall; appended summary moves the offset by at most that |
| FR-009, SC-004, US2-5 | client (`main_tests.rs` reducer + `shell/persist.rs` tests) | toggle updates `core.settings`, writes the store, a fresh `App` from that store starts on; two sessions' views read the same value; `set_changed` neither loses nor clobbers the key |
| selection/copy | client (`tests/transcript_selection_parity.rs`) | hidden entries absent from `rows()` and from the copy text; revealed ones present |
| FR-011 | client | same transcript built live and via the resume path gives equal `rows()` |
| look | quickstart §B visual pass | summary line weight, toggle placement, notice |

## Complexity Tracking

None: no principle deviation.
