# Research: sidebar stable order (#1718)

## R1 — Feature 905's row icon does not satisfy FR-004 / SC-002

**Finding**: `ui/material/state_icon.rs` maps `LocationState` (Running, Blocked, Idle, Unknown) to
`StateRunning`, `StateBlocked`, `StateIdle` and an empty slot. Done has no glyph; Needs action and
Needs attention have the same one (Blocked) or none; an Unknown/no-session location is blank. The
spec's assumption (905 suffices) is false for five states.

**Decision**: re-key `StateIcon` on `CardState` and take its glyph from
`overview_card::state_icon` (five distinct code points: e037, e645, e8f4, e86c, e034). Colour roles:
NeedsAction `error`, NeedsAttention `warning`, Working `primary`, Done `tertiary`-or-`on_surface_variant`
(final choice meets the 4.5:1 / 3:1 gate in `composition_contrast.rs`), Idle `on_surface_variant`.
A location with no session is Idle, so the slot is never blank (spec edge case).

**Rejected**: (a) keep the 905 icon and add a second card-state mark: two marks on one row, clutter,
and 905 FR-008 forbids sharing shapes. (b) State word in the row: the second line already carries
the session text and width is tight. (c) Only add Done: Needs action and Needs attention still collide.

## R2 — Card state source for a row

**Decision**: `features::overview::location_card_state` (already used by `entry_card_state`), the
same call as the Overview card, so row and card agree (feature 813). `entry_card_state` is kept,
renamed `entry_card_state` stays private to sidebar; run group rows use `run_group_state` (kept).

**Rejected**: recomputing from `LocationState` (cannot give Done or attention).

## R3 — Where `run_group_state` / `RunCard` live

**Decision**: keep them in core, delete the rest of `sidebar_groups.rs`, rename the file to
`sidebar_state.rs` only if the churn is small; otherwise leave the module name and update its docs.
**Rejected**: moving into `overview.rs` (large file).

## R4 — Opt-in grouping, sort, filter

Out of scope per spec (issue alternatives 2 to 5).

## R5 — Persistence

`group_choices` is `sidebar::State` memory only; nothing to migrate (FR-008).
