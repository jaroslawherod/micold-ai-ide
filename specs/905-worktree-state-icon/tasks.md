# Tasks: Worktree State Icon (905)

**Input**: spec.md, plan.md, research.md, data-model.md, contracts/state-icon.md, contracts/wire.md,
quickstart.md. Tests first (Constitution I): each test task precedes the code it covers and is run
red, failing for the reason it names.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task).
- **[Story]**: US1, US2, US3 from spec.md.

## Phase 1: Setup

No setup: existing workspace, no new crate (plan *Technical Context*).

## Phase 2: Foundational (state rules, glyphs, component)

**Purpose**: the pure rules (D1), the three glyphs (D3) and the shared component with its host slot
(D4) that every story renders through.

- [ ] T001 Add `pub paused_on_user: bool` to `Session` in `crates/micold-core/src/session.rs` ("Runtime-only, like `activity`; `false` from every constructor": `start_new`, `restored`, and every `Session { .. }` literal, e.g. `crates/micold-daemon/src/ops.rs` and `State::restored_stub` in `crates/micold-daemon/src/state.rs`, and test fixtures that build a `Session` literal) (data-model *Session*)
- [ ] T002 Test (red) in a `#[cfg(test)] mod tests` of the new `crates/micold-core/src/location_state.rs` (module stub registered as `pub mod location_state` in `crates/micold-core/src/lib.rs`): `session_state` over every lifecycle (`Idle`, `Starting`, `Running`, `Restarting { .. }`, `Failed { .. }`, `InterruptedResumable`) × activity (`Unknown`, `Working`, `AwaitingInput`, `Ended { .. }`) × `paused_on_user` (`true`, `false`) gives exactly the data-model rule table (first match wins; rule 7 `Running`+`Unknown` → `Unknown`, never `Blocked`) (SC-004); `location_state`: no session → `Idle`, archived sessions skipped (an archived blocked session beside an idle one → `Idle`), precedence pairs `Blocked > Running > Unknown > Idle` including one `Unknown` beside `Idle` → `Unknown`, and among blocked `NeedsPermission > Failed > Resumable`; `tooltip_line` returns `Running`, `Blocked: needs permission`, `Blocked: failed`, `Blocked: resumable`, `Idle`, `State unknown` (FR-003, FR-004, FR-005, FR-009)
- [ ] T003 Implement in `crates/micold-core/src/location_state.rs`: `enum LocationState { Idle, Unknown, Running, Blocked(BlockedReason) }` and `enum BlockedReason { Resumable, Failed, NeedsPermission }` with derived `Ord` in that ascending order ("`Ord` is derived in that order, so the location's state is the `max` of its sessions' states"), `session_state(&Session) -> LocationState`, `location_state<'a>(impl IntoIterator<Item = &'a Session>) -> LocationState` (skips `archived`; none left → `Idle`), `LocationState::tooltip_line(self) -> &'static str`; T002 goes green (D1, R4, R5)
- [ ] T004 [P] Test (red) in `crates/micold-client/tests/icons.rs`: pin `Icon::StateRunning` = `U+E037` (`play_arrow`), `Icon::StateBlocked` = `U+E160` (`report`), `Icon::StateIdle` = `U+E034` (`pause`); `all_covers_every_variant_without_duplicates` and `crates/micold-client/tests/icons_font.rs` cover the three; an explicit assertion that none of the three equals the house icon, any PR-indicator glyph (`call_split`, `edit_note`, `call_merge`, `block`, `check`, `schedule`, `cancel`) or either activity-dot glyph (`radio_button_checked`/`unchecked`) (FR-007, FR-008, A5)
- [ ] T005 Add `StateRunning`, `StateBlocked`, `StateIdle` to `Icon` and `Icon::ALL` in `crates/micold-client/src/icons.rs`, each name confirmed against the upstream `.codepoints` file named in `assets/fonts/PROVENANCE.md`; T004 goes green (D3, R6)
- [ ] T006 Test (red) in the `mod tests` of `crates/micold-client/src/ui/material/tree_view.rs` (beside the existing geometry tests): with `.state_icon(StateIcon::new(state, roles))` the label's x and the row's height are equal for `Running`, `Blocked(_)`, `Idle`, `Unknown`, one-line and with tags; the tag line starts under the label (`tag_indent` includes `label_role.size() + icon_gap`); at the narrowest width the name ellipsizes and the slot keeps `label_role.size()`; a row without `.state_icon` is laid out exactly as before (FR-010, SC-005, A2–A4)
- [ ] T007 Implement `StateIcon` in the new `crates/micold-client/src/ui/material/state_icon.rs` (`StateIcon::new(state: LocationState, roles: Roles)`, `.size(px)` defaulting to `TypeRole::SidebarLabel`'s size, `impl From<StateIcon> for Element<'_, M>`; glyph and role per the data-model *Glyph* table — `Running` `primary`, `Blocked(_)` `error`, `Idle` `on_surface_variant`; `Unknown` an empty `Length::Fixed(size)` box; the render reads its colour from a private `fn role(state, &Roles) -> Option<Color>`), exported from `crates/micold-client/src/ui/material/mod.rs`, reusing `icon(..)` (S1)
- [ ] T008 Add `TreeItem::state_icon(self, StateIcon) -> Self` in `crates/micold-client/src/ui/material/tree_view.rs`: pushed after the twisty slot and before the leading icon at `label_role.size()`; `tag_indent` adds `label_role.size() + icon_gap` when set; T006 goes green (S2, R7)

**Checkpoint**: rules, glyphs and slot exist and are tested; no row shows them yet.

## Phase 3: User Story 1 - See at a glance which worktrees are working (P1) 🎯 MVP

**Goal**: every worktree row and the Default row show Running, Idle / waiting (or Blocked for a
failed or resumable session, from T003's rules), or an empty slot for Unknown, live.

**Independent Test**: one worktree's session working, another idle, one with no session: the rows
show triangle, two bars, two bars without being selected; the first turns to two bars when its turn
ends.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T009 [US1] Tests (red) in the new `crates/micold-client/tests/sidebar_state_icon.rs` (fixtures from `crates/micold-client/tests/sidebar_attention.rs` / `sidebar_tree.rs`): `WorktreeNode::state()` is `Running` for a working session, `Idle` for a turn-finished (`AwaitingInput`, `paused_on_user == false`) session and for a worktree with no session, `Unknown` for `Running`+`Unknown` activity, `Blocked(Failed)` for a failed session, the same whether the worktree is expanded or collapsed and selected or not; a parallel-run row inside a group has its own state; a worktree filtered out by a tag filter has no row; a missing worktree still has a state; `DefaultNode::state()` follows the project root's sessions by the same rules; with no new snapshot (the connection lost) every row keeps the state it had, and a reconnect snapshot replaces it with the service's current state (Edge: service restart); a catalog snapshot that changes a session's activity changes the row's state with no other message, notification or selection change (FR-006, FR-011, A8); `with_state_line(tooltip, state)` yields `"{tooltip}\n{line}"` and, composed as `with_unread_line(with_state_line(..), n)`, the state line precedes the unread line (FR-001, FR-002, FR-003, FR-009, A1, A7)

### Implementation for User Story 1

- [ ] T010 [US1] Implement `WorktreeNode::state()`, `DefaultNode::state()` (via `micold_core::location_state::location_state` over `sessions`) and `pub fn with_state_line(tooltip: String, state: LocationState) -> String` beside `with_unread_line` in `crates/micold-client/src/features/sidebar.rs`; T009 goes green (D5, S4)
- [ ] T011 [US1] Wire in `crates/micold-client/src/ui/sidebar.rs` (glue): `build_items` worktree rows, top level and run rows in a group, get `.state_icon(StateIcon::new(node.state(), r))` and tooltip `with_unread_line(with_state_line(worktree_tooltip(..), node.state()), unread)`; `build_default_item` the same with `DEFAULT_LOCATION_LABEL`, house icon and everything else unchanged (FR-002); group rows and session sub-rows unchanged (S3, D6)
- [ ] T012 [US1] User guide: new § *Worktree state icon* under *Browsing worktrees* in `docs/user-guide/worktrees-and-sessions.md`: the three states and their icons (triangle, octagon, two bars), what puts a worktree in each as of this milestone (working or starting/restarting → Running; failed or resumable → Blocked; idle, turn finished, no session, process ended → Idle / waiting), the empty slot and *State unknown* line for an unknown state, several sessions by precedence, the tooltip line, the Default entry (FR-013, D7)

**Checkpoint**: on `main`, every listed row shows its state live; a mid-turn permission stop still
reads Idle until US2 carries the flag.

## Phase 4: User Story 3 - Tell the states apart without relying on colour (P2)

**Goal**: shapes and colours proven legible in both schemes, the component in the showcase, the
icons in the icon reference.

**Independent Test**: the showcase's `StateIcon` entry in greyscale, both schemes: three shapes,
each visible.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T013 [P] [US3] Test (red) in `crates/micold-client/src/ui/material/composition_contrast.rs`, beside `the_unread_mark_is_legible_on_every_fill_a_host_draws_it_on`: each `StateIcon` glyph role (`primary`, `error`, `on_surface_variant`) is ≥ 3:1 against the sidebar row's rest, hovered and selected fills in the light and dark schemes; red because `state_icon::role` is still private (FR-007, A6)
- [ ] T014 [P] [US3] Test (red) in `crates/micold-client/tests/showcase_completeness.rs`: a catalogue entry `material/state_icon.rs` / `StateIcon` with variants `Running`, `Blocked`, `Idle` (FR-012, S6)

### Implementation for User Story 3

- [ ] T015 [US3] Make T007's `role` `pub(crate)` in `crates/micold-client/src/ui/material/state_icon.rs`, so the contrast gate reads the same role the row draws; T013 goes green (fix the role table only if a pair fails, and update data-model *Glyph* to match)
- [ ] T016 [US3] Showcase: catalogue entry in `crates/micold-client/src/showcase/catalogue.rs` (beside `UnreadMark`) and `crates/micold-client/src/showcase/sections/atoms.rs::state_icon` posing `Running`, `Blocked`, `Idle`, `Unknown` and one worktree `TreeItem` with tags carrying the icon; T014 goes green (S6, D7)
- [ ] T017 [US3] User guide: three rows (triangle *Running*, octagon *Blocked*, two bars *Idle / waiting*) in `docs/user-guide/icons.md` (FR-013)

**Checkpoint**: M1 complete: the component is shown, proven legible and documented.

## Phase 5: User Story 2 - Spot worktrees that are stuck (P1)

**Goal**: a session that stopped mid-turn to ask for a permission or an answer makes its row
Blocked (*Blocked: needs permission*), live, including when the same turn's `Stop` follows.

**Independent Test**: one worktree's session asks for a tool permission: its row shows the octagon
within 1 s; granting it turns the icon to the triangle.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T018 [P] [US2] Test (red) in the tests of `crates/micold-core/src/attention.rs`: `TurnClock::paused_on_user()` is `true` only after `Working` → `AskedUser`; `false` after a following `Working`, `Finished` or `PromptSubmitted`, for `AskedUser` with no turn, and for a fresh clock (R1)
- [ ] T019 [P] [US2] Test (red) in `crates/micold-core/tests/protocol_roundtrip.rs`: a `SessionSummary` with `paused_on_user: true` round-trips; a frame without the field decodes `paused_on_user == false` (W1)
- [ ] T020 [P] [US2] Tests (red) in `crates/micold-daemon/src/state.rs` tests beside the existing `note_activity` ones: each row of the contract W3 table (`UserPromptSubmit`,`Notification` → `AwaitingInput`/`true`, broadcast; then `PreToolUse` → `Working`/`false`, broadcast; `Notification` then `Stop` → `AwaitingInput`/`false`, **broadcast** though the signal did not change; `UserPromptSubmit`,`Stop` → `false`, broadcast; `Notification` with no turn → `false`, broadcast); `overlay_live_summaries` sets `paused_on_user` from the live clock and `false` with no live entry; attention events and `unread` are unchanged for the same sequences (W2, W3, W5)
- [ ] T021 [US2] Tests (red) in `crates/micold-client/tests/sidebar_state_icon.rs`: a snapshot with `paused_on_user: true` reaches `Session::paused_on_user` on both catalog paths (`crates/micold-client/src/catalog_sync.rs` existing-session update and new-session insert); a worktree whose session is `Running`+`AwaitingInput`+`paused_on_user` is `Blocked(NeedsPermission)` with tooltip line `Blocked: needs permission`; an idle main session beside a helper asking for a permission → `Blocked`; a following snapshot with `Working` → `Running` (US2 scenarios 1, 4; W4)

### Implementation for User Story 2

- [ ] T022 [US2] Add `pub fn paused_on_user(&self) -> bool` (`true` exactly in `TurnState::Paused`) to `TurnClock` in `crates/micold-core/src/attention.rs`; T018 goes green (D2)
- [ ] T023 [US2] Add `#[serde(default)] pub paused_on_user: bool` to `SessionSummary` in `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` 47 → 48 in `crates/micold-core/src/protocol/version.rs` with a doc line naming feature 905's `paused_on_user`; set `false` in every `SessionSummary { .. }` literal (`crates/micold-daemon/src/catalog.rs` `session_summary`, `crates/micold-core/src/attention.rs`, `crates/micold-client/src/catalog_sync.rs`, `crates/micold-client/src/shell/daemon_sync.rs`, and the daemon and client test fixtures that build one); T019 goes green (W1, R2)
- [ ] T024 [US2] In `crates/micold-daemon/src/state.rs`: `overlay_live_summaries` sets `paused_on_user` from `live.turn.paused_on_user()`; `note_activity` returns `changed || paused_changed` (flag compared before and after the turn clock sees the event); `began_waiting` still compares signals only; T020 goes green (W2, W3, R3)
- [ ] T025 [US2] Adopt `summary.paused_on_user` into `Session::paused_on_user` at both places `activity` is adopted in `crates/micold-client/src/catalog_sync.rs`; T021 goes green (W4)
- [ ] T026 [US2] User guide: in § *Worktree state icon* of `docs/user-guide/worktrees-and-sessions.md`, add that a session that stopped mid-turn to ask for a permission or an answer makes the worktree Blocked (*Blocked: needs permission*) until it carries on, and that a turn that simply ended stays Idle / waiting (FR-013)

**Checkpoint**: every Blocked case of the spec shows on `main`; only the visual pass remains.

## Phase 6: Polish & Cross-Cutting Concerns

No code. Left to the close unit (milestones.md rule 4).

- [ ] T027 Run quickstart §B (B1–B11) with the `visual-pass` skill on Linux, light and dark, and save the screenshots and results under `specs/905-worktree-state-icon/visual-pass/` (SC-001–SC-003, FR-006, FR-007)
- [ ] T028 Record quickstart §C (macOS, Windows: B1, B3, B9) as run or as handed to CI's OS matrix in `specs/905-worktree-state-icon/quickstart.md` (Principle VI)

## Dependencies & Execution Order

- Foundational (T001–T008) blocks every story: T001 → T002 → T003; T004 → T005; T003, T005 → T006 → T007 → T008.
- US1 (T009–T012) needs Foundational. US3 (T013–T017) needs Foundational and US1 (the showcase row and the guide). US2 (T018–T026) needs US1 (it extends `sidebar_state_icon.rs` and the guide section).
- Within each story: test tasks red first, then the implementation that turns them green.
- Polish (T027–T028) needs every story merged.

### Parallel Opportunities

- T004 beside T001–T003 (client icons vs core module).
- T018, T019, T020 together (three crates' test files).
- T013, T014 together.

## Parallel Example: User Story 2

```text
T018 attention.rs paused_on_user tests
T019 protocol_roundtrip.rs field tests
T020 daemon state.rs W3 tests
```

## Implementation Strategy

MVP first: Foundational + US1 + US3 is M1, which shows Running, Idle / waiting, Unknown and the
failed and resumable Blocked cases from core rules. US3 travels with it because the shared component
must ship with its showcase entry, its light/dark contrast proof and its icon-reference rows
(Principles VII, VIII). US2 (M2) then carries the one missing signal (protocol 48) so a mid-turn
permission reads Blocked. The visual pass and §C run in the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Worktree rows show their state 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: every worktree row and the Default row show a live state icon (triangle Running, octagon Blocked for a failed or resumable session, two bars Idle / waiting, empty slot when unknown) with its tooltip line; the showcase shows `StateIcon`, the contrast gate proves each glyph legible in both schemes, and the user guide and icon reference describe it.
- **Satisfies**: US1 acceptance scenarios 1–4; US2 acceptance scenarios 2–3; US3 acceptance scenarios 1–3; FR-001–FR-013 (FR-003 rule 5's Blocked case and FR-013's permission line in M2); SC-002 (by gate), SC-004, SC-005
- **Verify**: `mise run test-core` (`location_state` tests); `mise run gate` (`sidebar_state_icon.rs`, `tree_view.rs` geometry tests, `tests/icons.rs`, `icons_font.rs`, `composition_contrast.rs`, `showcase_completeness.rs`); the showcase's `StateIcon` entry
- **Depends on**: —
- **Tier**: full

### M2 — A session asking for permission makes its worktree Blocked

- **Tasks**: T018–T026
- **Deliverable**: a session that stops mid-turn to ask for a permission or an answer turns its worktree's icon to the octagon with *Blocked: needs permission* within 1 s; granting it turns the icon to the triangle, and a `Stop` ending that turn turns it to two bars (protocol 48).
- **Satisfies**: US2 acceptance scenarios 1, 4; FR-003 (rule 5), FR-006, FR-011, FR-013; contract W1–W5
- **Verify**: `mise run test-core` (`TurnClock::paused_on_user`, `protocol_roundtrip.rs`); `mise run gate` (daemon `note_activity` W3 tests, `sidebar_state_icon.rs` permission cases)
- **Depends on**: M1
- **Tier**: full
