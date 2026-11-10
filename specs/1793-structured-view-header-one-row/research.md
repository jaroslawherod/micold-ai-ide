# Research: feature 1793

## R1 Where status, Changes, meter and `⋮` render
Decision: `toolbar::view` adds `.action(...)` items before settings/menu, driven by a
`HeaderBarModel` from the client's render-free `features/header_bar.rs` (None on Overview). Rejected: a second bar under the
title bar (re-adds a row); drawing in `session_area` (not the top bar).

## R2 Collapse order
Decision: `GiveWayRow` ranks alone, the mechanism `status_cluster` uses today (that fn is removed, the
pattern kept): ring text rank 0, `Changes` label rank 1, others `keep`. The model only assigns the
ranks (unit-tested); layout does the collapsing, so there is one source of truth. Task 1 confirms
`Toolbar::action` accepts a `GiveWayRow` (it takes any `Element`). Rejected: width thresholds in the
model (second source of truth, drifts per platform).

## R3 Ring component
Decision: new `micold-material::ContextRing` (builder: `.ratio().text().warning().tooltip()`),
coloured from the `Roles` tokens (`primary`, `secondary_container`, `error` for warning); the
client's `ProgressBar` (`ui/material/progress_bar.rs`) stays in the client and is not imported.
Exported from `micold-material/src/lib.rs` and given a showcase entry. Rejected: feature-local canvas in `ui/` (Principle VIII).

## R4 Segmented control
Decision: add `SegmentedControl` to `micold-material` (exported in `lib.rs`, showcase entry); two segments, selected state, `on_select`.
Rejected: two `ToggleChip`s (no group semantics, no single-selection accessibility name); keeping
today's single switch button (spec FR-009 names a segmented control).

## R5 View toggle content
Decision: segments Structured / Terminal from `switch_entries` data; shown only for Claude Code
sessions (as today); other CLIs show no segmented control. Picking the other segment sends the
existing `Composer::Msg::SwitchRequested` (its confirm dialog stays).

## R6 Session-bar leftovers (unspecified by the spec)
Decision (recommended, flagged): totals and running count join the ring tooltip (turns/cost already
there); the background-tasks accordion and Bypass mode label stay available as one slim first row of
the transcript body, not a header row, so FR-012 holds and SC-001 counts header rows only. Rejected:
drop them (violates FR-012); keep in header (violates FR-001).

## R7 `+` menu
Decision: `MenuOverlay` anchored to the `+` bounds (as `ContextArea` reports bounds for the project
switcher); items "Another session" (sends `AdditionalRequested(location)`, which still raises the AI
CLI and kind choice) and "Terminal" (`ShellInstanceOpenRequested`, always a new instance, verified).
Rejected: `SplitAction` (needs a primary action; the spec wants a menu only).

## R8 Menu lifetime
Decision: a typed `HeaderMenu` enum (none / plus / worktree) in client state; closed on any selection
change and on `on_dismiss`. Rejected: two booleans (both could be true).

## R9 Activity dot
Decision: per-session `session_status` (existing) drives the `Member::Session` leading mark; no
`running` text. Rejected: new per-session state.
