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
the private `entry_card_state` method of `features/sidebar.rs` stays; run group rows use `run_group_state` (kept).

**Rejected**: recomputing from `LocationState` (cannot give Done or attention).

## R3 — Where `run_group_state` / `RunCard` live

**Decision**: keep them in core, delete the rest of `sidebar_groups.rs`, and do not rename the module
(updating its docs only), to keep the diff small.
**Rejected**: moving into `overview.rs` (large file).

## R4 — Opt-in grouping, sort, filter

Out of scope per spec (issue alternatives 2 to 5: attention count with jump, state filter, opt-in grouping, debounced reordering). Rejected because each adds a control or setting beyond the stable order the issue's alternative 1 asks for.

## R5 — Persistence

`group_choices` is `sidebar::State` memory only; nothing to migrate (FR-008).

## R6 — Fate of 905 leftovers and group-only parts

- **Tooltip** (905 FR-009): the state line becomes the card state's word (`CardState::label()`), so the
  icon and its tooltip agree. Rejected: keeping `LocationState` words (would say Running beside a Done icon).
- `LocationState` stays in core (it feeds `card_state`); `StateIcon::role` is re-keyed on `CardState`;
  `Icon::StateBlocked` stays in the icon font (unused by the row; removing it churns `icons_font.rs` for no gain).
- `TypeRole::SidebarGroup`, the `TreeItem` group-heading builder, their tests and the showcase
  catalogue entry are deleted: nothing draws a group header (FR-007). Rejected: keep them unused (dead code).
- `ListRow` is replaced by `SidebarEntry`: `sidebar_list()` returns `Vec<SidebarEntry>`; `row_heights`
  and `row_index` take `&[SidebarEntry]`. Rejected: a one-variant enum.

## R7 — Row highlight and the cleanup candidate (decision 2026-11-09)

**Finding**: the "merged, nothing running, can be cleaned up" signal already exists as
`CardState::Done` (`micold_core::overview::card_state` row 9: PR merged AND `pr_status.removable`
AND no running session; Working outranks it) with its own glyph `CardDone` (e86c). A closed
(unmerged) PR is `PrState::Closed`, read but deliberately never removable (feature 813 FR-017).

**Decision**: no sixth state; Done is the cleanup candidate. Row tint by card state, from the
`Roles` already in `micold_core::tokens` (no new token): Needs action `error_container`, Needs
attention `tertiary_container` (the louder pair), Done `surface_container_high` (quieter, tonal),
Working and Idle none. Text and glyph on the tint use the matching `on_*_container` role (Done:
`on_surface`), checked in `composition_contrast.rs`. The tint is the row's resting background and
the hover and selected state layers draw over it. The glyph and the "Done (merged)" tooltip state line stay, so
colour never carries the state alone (FR-004, FR-012). Position is untouched (FR-003).

**Rejected**: (a) a sixth `CardState::CleanupCandidate` (merged-removable OR closed): edits feature
813's table, ordering, Overview and filters, and widens "can be deleted" to unmerged closed work
(data-loss risk). (b) Highlight only, drop the cleanup request: leaves merged rows unmarked.
**Follow-up**: closed-unmerged PRs as candidates, own issue (spec Out of Scope).
