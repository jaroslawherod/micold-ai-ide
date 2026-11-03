# Research

## R1: Where the classification lives
Decision: `micold-core::structured::transcript::visibility`, a pure `Entry::low_signal()` and `shown(kind, show_raw)`.
Why: renderer (`entries::rows`) and selection (`transcript_selection::rows`/`entries_in`) must agree on what is hidden, or copy and selection ranges would include invisible rows. One core function serves both and tests without iced.
Rejected: filter inside `ui/transcript` only (selection drifts); filter when building the `Transcript` (loses FR-010, breaks resume/era logic, revisions).

## R2: Persistence
Decision: `show_raw_events: bool` on `micold_core::settings::Settings` (serde default false), copy in `features::settings::State`, written on toggle with `write_setting` through a new `persist_show_raw_events`.
Why: the spec puts it with the app settings; `filter_open` and `folded_origins` set the precedent for a live control that writes one field without the Settings draft/save cycle, and `update_reporting` keeps every other field.
Rejected: per-session or per-worktree storage (spec: per user); adding it to the settings draft (the toggle is in the transcript header, not on the Settings page, and a draft would make it Save-gated); a daemon-side setting (client-owned like `suggestion_chips`).

## R3: Summary line widget
Decision: folded `DisclosureCard` (title, summary, no status) with the existing `State.open` set for expansion.
Why: already one line when folded, keyboard and ripple included, expansion state and selection units already work for it; no new widget (Principle VIII).
Rejected: new `SummaryLine` widget (a fork of the card); `AccordionSection` directly (loses the card chrome the other entries share).

## R4: Toggle control and placement
Decision: `micold-material::LabelledToggle` labelled "Show raw events" at the trailing end of the session-bar row built in `view_full`.
Rejected: a menu item (spec assumption: header control); a Settings-page-only switch (not reachable from the transcript).

## R5: Hidden notice
Decision: one row `Row::Hidden(n)` ("N events hidden") with a "Show" button emitting the same toggle message; drawn only while `n > 0`; not a selection unit (like "No output yet").
Rejected: a notice in the bar (spec FR-008 says the view shows it; the transcript is where the gap is).

## R6: Bounded raw detail
Decision: raw detail inside a scrollable with a fixed max height, following how tool results are bounded in `blocks.rs`; implementer picks the existing constant if one exists.
Rejected: truncating text (FR-004 forbids dropping content).

## R7: Cross-view follow
Decision: all structured views of a window read `core.settings.show_raw_events`, so changing it in one repaints the others in the same reducer step.
Open assumption: the client is one process with one `settings::State`; a second app process reads the stored value at start only. Matches the spec's "two views open at once" within a window.
