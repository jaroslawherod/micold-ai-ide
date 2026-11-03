---
description: "Task list for collapsed summaries for raw events in the structured view"
---

# Tasks: Collapsed Summaries for Raw Events in the Structured View

**Input**: `specs/1664-collapsed-raw-events/` (plan.md, spec.md, data-model.md, contracts/ui-contract.md, research.md, quickstart.md)
**Tests**: mandatory (Constitution I). Every story lists its failing tests before its implementation.
**Documentation**: each story's user-guide task is in that story's milestone (Constitution VII).

## Format: `[ID] [P?] [Story] Description`

Paths: core `crates/micold-core/src/`, client `crates/micold-client/src/` and `crates/micold-client/tests/`.

## Phase 1: Setup

None. The feature adds no crate, dependency or config; the work starts in the first story.

## Phase 2: Foundational

None separate. Each story below brings the core pieces it needs.

## Phase 3: User Story 1 - Read the conversation without plumbing in the way (P1) 🎯 MVP

**Goal**: each of the four low-signal kinds is one folded summary line that expands to its detail.
**Independent test**: a transcript with one of each between two messages renders four one-line cards, no raw text; expanding shows the detail.

### Tests first

- [x] T001 [P] [US1] Unit tests for the summary text functions (title and facts per kind: every raw reason, the `(type)` fact present when the raw text is JSON with a `type` and absent otherwise, empty raw text reads "Empty event", footer ok/error with and without duration/turns, session start with and without model, orphan ok/error) in `crates/micold-client/src/features/transcript_selection/shown.rs`
- [x] T002 [P] [US1] View tests in `crates/micold-client/tests/transcript_entry_view.rs`: the four kinds draw as folded cards with the summary title; no raw text painted while collapsed (SC-001); open state shows raw detail byte-equal to the entry text (FR-004, tabs to spaces only); error footer/orphan titles carry the word "failed"/"error"; a footer arriving later updates its summary in place; a very large raw detail is height-bounded
- [x] T003 [P] [US1] Geometry test in `crates/micold-client/tests/transcript_follow_layout.rs`: a collapsed summary card is one line tall; appending one moves the reading offset by at most that (SC-003, US1 scenario 4)
- [x] T004 [P] [US1] Parity test in `crates/micold-client/tests/transcript_selection_parity.rs`: `body_units` for Raw, SessionStart, Footer, OrphanResult yields the summary line as unit 0 and detail units only while open; copy equals what is drawn

### Implementation

- [x] T005 [US1] Summary title/facts functions in `crates/micold-client/src/features/transcript_selection/shown.rs` (wording table in data-model.md; raw type shown only if the raw text parses as JSON with a `type`)
- [x] T006 [US1] Folded `DisclosureCard` arms for the four kinds in `crates/micold-client/src/ui/transcript/entries.rs`, raw detail in a height-bounded scrollable (reuse the bound used by tool results in `blocks.rs`), error variants use the card's `failed` outline
- [x] T007 [US1] `body_units` for the four kinds (summary unit 0, detail only when open) in `crates/micold-client/src/features/transcript_selection.rs`
- [x] T008 [US1] User-guide page `docs/user-guide/structured-transcript.md` (summary lines and expanding) and its `docs/SUMMARY.md` entry

## Phase 4: User Story 2 - Hide low-signal entries altogether, and get them back (P2)

**Goal**: a persisted per-user "Show raw events" toggle (off by default) in the view header hides the low-signal entries; errors stay visible.
**Independent test**: toggle off hides all but error footers/orphans; on shows all in order; restart keeps the choice; two views follow.

### Tests first

- [x] T009 [P] [US2] Core unit tests in `crates/micold-core/src/structured/transcript/visibility.rs`: every `EntryKind` classified; error footer and error orphan always shown (FR-013); ok ones follow the flag; `hidden_count` excludes errors and is 0 when shown; transcript length unchanged by any flag value (FR-010); every non-low-signal kind is always shown (FR-012)
- [x] T010 [P] [US2] Settings tests in `crates/micold-core/tests/settings_roundtrip.rs`: `show_raw_events` defaults false, a file without the key reads false, round-trips true, other keys untouched
- [x] T011 [P] [US2] Client tests in `crates/micold-client/tests/transcript_entry_view.rs`: `rows()` with flag off/on over a mixed transcript (counts, order, no raw text off; summary lines plus hidden count equal the total, SC-002)
- [x] T012 [P] [US2] Selection tests in `crates/micold-client/tests/transcript_selection_parity.rs` and a unit in `crates/micold-client/src/features/transcript_selection.rs`: hidden entries absent from `rows()` and copy text, revealed ones present; `ShowRawEvents` clears that session's selection (selection ending on a low-signal entry)
- [x] T013 [P] [US2] Reducer tests in `crates/micold-client/src/main_tests.rs` and `crates/micold-client/src/shell/persist.rs`: toggle updates `core.settings` and writes the store; a fresh `App` from that store starts on (SC-004); two sessions' views change in one reducer step; a Save of another field leaves `show_raw_events` as the window holds it; a write failure shows "Couldn't save your settings" and the value still applies

### Implementation

- [x] T014 [US2] `LowSignal` closed enum (`Raw`, `SessionStart`, `Footer`, `OrphanResult`), `Entry::low_signal()` as an exhaustive `match` on `EntryKind` with no `_` arm, `shown(&Entry, show_raw)`, `Transcript::hidden_count(show_raw) -> u64` in new `crates/micold-core/src/structured/transcript/visibility.rs`, re-exported from `crates/micold-core/src/structured/transcript/mod.rs`
- [x] T015 [US2] `show_raw_events: bool` (`#[serde(default)]`, default false) in `crates/micold-core/src/settings.rs`: `Settings`, the on-disk mirror struct and both conversions, the `Default` impl, and every struct literal of `Settings` in the workspace (all literal sites in this one task)
- [x] T016 [US2] `State.show_raw_events` and `SettingsMsg::ShowRawEventsToggled(bool)` in `crates/micold-client/src/features/settings.rs`; `ValidSettings::into_settings` emits the default for the key (the `set_changed` change is T017's)
- [x] T017 [US2] `persist_show_raw_events`, `window_settings` taking the window copy, and the single `set_changed` change (destructure `show_raw_events: _`, a Save never changes the key; the settings.rs side only emits the default) in `crates/micold-client/src/shell/persist.rs`
- [x] T018 [US2] `TranscriptMsg::ShowRawEvents(bool)` in `crates/micold-client/src/features/structured_transcript.rs`, routed by the root to `SettingsMsg::ShowRawEventsToggled` in `crates/micold-client/src/main.rs` (settings feature stays the only writer; check `tests/feature_write_isolation.rs`)
- [x] T019 [US2] `TranscriptView` gains `show_raw` (all 4 construction sites incl. test/parity helpers in this one task); `rows()`/`entries_in` filtered by `visibility::shown`; reconcile clears the session's selection on toggle, in `crates/micold-client/src/features/transcript_selection.rs`
- [x] T020 [US2] `rows(transcript, show_raw)` filter in `crates/micold-client/src/ui/transcript/entries.rs`; `LabelledToggle` "Show raw events" (tooltip "Show session start, turn summaries and unrecognised events") at the trailing end of the session bar in `view_full` and `show_raw` passed down in `crates/micold-client/src/ui/transcript/mod.rs`
- [x] T021 [US2] User-guide section on the toggle (off by default, errors stay visible, remembered per user) in `docs/user-guide/structured-transcript.md`

## Phase 5: User Story 2 (notice) and User Story 3 - Hidden count and resumed transcripts (P2)

**Goal**: hidden entries are announced with a count and a Show button; live and resumed transcripts behave the same.
**Independent test**: toggle off shows "N events hidden" with Show; a transcript of only hidden entries shows the notice, not Empty; live and resumed transcripts give equal `rows()`.

### Tests first

- [x] T022 [P] [US2] Tests in `crates/micold-client/tests/transcript_entry_view.rs`: `Row::Hidden(n)` present only when n > 0, after `Omitted`; a transcript of only hidden entries shows the notice (no `Empty`); the Show button emits `TranscriptMsg::ShowRawEvents(true)`; notice text unit test in `shown.rs`
- [x] T023 [US3] Test in `crates/micold-client/tests/transcript_entry_view.rs`: the same transcript built live and via the resume path gives equal `rows()` for both flag values (FR-011)

### Implementation

- [x] T024 [US2] Notice text function in `crates/micold-client/src/features/transcript_selection/shown.rs`; `Row::Hidden(u64)` emitted by `rows()` and drawn as "N events hidden" with a "Show" button in `crates/micold-client/src/ui/transcript/entries.rs`; `RowKey::Hidden` in `crates/micold-client/src/ui/transcript/mod.rs`; not a selection unit
- [x] T025 [US2] User-guide note on the hidden-events notice and resumed sessions in `docs/user-guide/structured-transcript.md`

## Phase 6: Polish

- [x] T026 Record quickstart Part B (visual pass, `visual-pass` skill) results in `specs/1664-collapsed-raw-events/quickstart.md`; tick tasks (close unit, no code)

## Dependencies & Execution Order

- T001–T004 before T005–T007; T008 after T006.
- T009–T013 before T014–T020; T014 before T019/T020; T015 before T016/T017; T016/T017 before T018; T018 before T020; T019 before T020.
- Phase 5 depends on Phase 4 (needs `show_raw` plumbing); T022/T023 before T024.
- Cross-window sync is out of scope: all views of one app process share `settings::State` (research R7).
- Parallel: all `[P]` tests within a phase touch different files or independent sections.

## Implementation Strategy

MVP = Phase 3 (summary lines, always shown). Then the toggle and persistence, then the notice. Phases 4 and 5 form one milestone.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Collapsed summary lines 🎯 MVP

- **Tasks**: T001–T008
- **Deliverable**: raw, session-start, footer and orphan-result entries show as one folded summary line each that expands to its detail, and copy matches what is drawn.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-004, FR-012; SC-001 (collapsed), SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test main -- transcript_entry_view transcript_selection_parity transcript_follow_layout`
- **Depends on**: —
- **Tier**: full

### M2 — "Show raw events" toggle, persisted, with hidden notice

- **Tasks**: T009–T025
- **Deliverable**: a header toggle (off by default) hides low-signal entries except error footers/orphans, shows "N events hidden" with a Show button while hidden, is remembered across restarts and shared by all views, live and resumed transcripts behave alike, and hidden entries are neither selected nor copied.
- **Satisfies**: US2 acceptance scenarios 1–5; US3 acceptance scenarios 1–2; FR-005 to FR-011, FR-013; SC-002, SC-004
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test main -- transcript_entry_view transcript_selection_parity`; quickstart §B steps 2–5
- **Depends on**: M1
- **Tier**: full

M2 has 17 tasks (over the ~10 guideline) and no acceptance-scenario split leaves a shippable half: a toggle without the hidden-count notice hides entries silently (US2 scenario 4), and the persisted flag has no use without the toggle. Per-task diffs are small (one field, one filter, one row); kept whole.

T026 (Polish) changes no code and is left to the close unit.

## Phase 7: TDD remediation

The feature is not done until T027 and T028 are cleared (tdd-verify FAIL, `tdd/verification.md`).

- [x] T027 Fix F1: add a test in `crates/micold-client/tests/transcript_entry_view.rs` that presses the header toggle (`ui/transcript/mod.rs:224`) with `show_raw` off and on and asserts the emitted `TranscriptMsg::ShowRawEvents(true)` / `(false)`. Proof: the mutant `ShowRawEvents(ctx.show_raw)` fails `scripts/build-lock.sh cargo test -p micold-client --test main -- transcript_entry_view`.
- [x] T028 Fix F2: add a test that presses the hidden-notice "Show" button (`ui/transcript/entries.rs:190`) and asserts `ShowRawEvents(true)`. Proof: the mutant `ShowRawEvents(false)` fails the same command.
- [x] T029 Fix F3: correct the M1 and M2 Verify commands above to `--test main -- <modules>`. Proof: the command runs.
- [x] T030 Fix F5: assert the orphan and footer titles exactly in `error_footers_and_orphans_say_failed_or_error_in_their_title`.
