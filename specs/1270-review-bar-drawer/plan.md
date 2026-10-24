# Implementation Plan: Review bar and comments drawer

**Branch**: `claude/project-thread-ttad83` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1270-review-bar-drawer/spec.md` (issue #1270, slice 1
of `docs/development/review-redesign.md`). Blocked by #1269 (Review as a per-entry tab, not
merged) and #1124 (status bar, merged).

## Summary

The review panel's header (Clear sent, Discard drafts…, Send review, counts) moves into a fixed
44 px **review bar** at the bottom of the entry's Review tab, above the status bar; the right-hand
column becomes a **comments drawer** stacked over the diff (360 px, at most half) with **Drafts**
and **Replies** tabs, closed by Esc, ×, or the bar's **Comments** toggle, and opening itself on
Replies at the first report of a send this window started. Comment cards show at most 4 reply
lines, then **Show all**.

The work is split in two layers around #1269 (research R11). **Layer A**, independent of #1269:
every rule (counts and their shortened forms, send button, overflow availability, error wording,
tab membership and order, drawer state machine with Esc and auto-open, drawer width, card reply
preview) in new `micold-core::review::{bar, drawer}` modules and `review::reply`, unit-tested
without drawing (FR-023); and the shared components (`ReviewBar`, `SideSheet`,
`LineClamped::when_cut`, `ReviewPanelRow` reply mode) with their own tests and showcase entries.
**Layer B**, after #1269 merges: the Changes reducer, shell and Esc wiring that attach the bar and
drawer to the per-entry review view, then the Review tab's layout, layout snapshot re-record,
geometry gate, user guide and visual pass.

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise`

**Primary Dependencies**: iced 0.14 (existing); no new crate

**Storage**: none new; comments stay in the daemon's review file, drawer state is in memory per
window (FR-019, Edge Cases "Two windows")

**Testing**: `mise run test-core` (core units), `mise run gate` (material component tests,
`crates/micold-client/tests/features_changes.rs`, app escape tests, layout snapshot and geometry
gates, docs), quickstart §B visual pass for `ui/changes.rs` glue

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (workspace: `micold-core`, `micold-daemon`, `micold-client`)

**Performance Goals**: counts, rows and cards follow a `ReviewChanged` within 2 s (FR-022, SC-005;
the push path is unchanged); 200 comments lay out in the drawer with no frame over 100 ms (946
SC-006's bar)

**Constraints**: send button 0 px movement (SC-001); diff 0 px / 0 lines change on drawer toggle
(SC-002); no protocol or store change (Out of scope); offline, local only

**Scale/Scope**: one entry's comments (hundreds); three files rewritten in the client
(`features/changes.rs`, `ui/changes.rs`, `shell/daemon_sync.rs` op), two new core modules, four
component changes

No NEEDS CLARIFICATION: research R1–R11 settle every open choice.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase
1: all PASS, no deviation.

- [x] **I. Test-First**: every decision is in tested render-free code written Red first: core
  `review::bar`, `review::drawer`, `review::reply::card_preview`, `tokens::anatomy`; client
  `features::changes` read models and messages (`tests/features_changes.rs`), `app::on_escape`,
  overlay registry; components' layout tests (`ReviewBar` form choice and fixed trailing bounds,
  `SideSheet` width, `LineClamped::when_cut`). Only `ui/changes.rs` composition uses the §B pass,
  backed by the layout snapshot and a new geometry gate.
- [x] **II. Multi-Session**: no session state added. Drawer state is per entry's review view and
  per window; comments and send state stay per entry in the daemon, shared as today. The auto-open
  watch exists only in the window that sent (R9).
- [x] **III. Worktree Integration**: no file or VCS operation added; scopes and entries
  (worktree or Default) are today's.
- [x] **IV. Local-First**: nothing stored or sent off-device; no remote service assumed.
- [x] **V. Rust + iced**: Rust and iced only; `DrawerTab::of(Status)` is total, so a comment in
  no tab or two is unrepresentable; `SendButton.primary` is derived, not set.
- [x] **VI. Cross-Platform**: core rules platform-agnostic; the overflow menu is the app's own
  `MenuOverlay`, the drawer and bar are app widgets; Esc goes through the shared `on_escape`;
  quickstart §C repeats the interactive steps on macOS and Windows.
- [x] **VII. Documentation**: `docs/user-guide/reviewing-changes.md` (*Sending a review*,
  *The review panel* → *The comments drawer*, *Clearing and discarding*, *Statuses and replies*)
  and the layout snapshot ship in M4 with the UI; the showcase entries in M2 with the components
  (FR-024). The stale "Clearing and discarding" wording (ledger follow-up) is fixed there.
- [x] **VIII. Reusable UI**: new `ReviewBar` and `SideSheet` in `ui/material/` with builders
  ending in `.into()`; `LineClamped`, `ReviewPanelRow`, `ReviewCommentCard`, `MenuOverlay`,
  `MenuTrigger`, `Tab`/`TabStrip`, `Scrollable`, `Button` reused or extended; all appear in the
  showcase.

## Project Structure

### Documentation (this feature)

```text
specs/1270-review-bar-drawer/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/  review-bar-ui.md  comments-drawer-ui.md
└── tasks.md   (tasks unit)
```

### Source Code (repository root)

```text
# Layer A — independent of #1269
crates/micold-core/src/review/
├── bar.rs        NEW ScopeCounts, CountsForm, counts_text, SendButton, send_button, Overflow, overflow, error_text
├── drawer.rs     NEW DrawerTab, tab_counts, rows_of, Drawer, DrawerEvent, SendWatch, newly_reported, width
├── reply.rs      + CARD_LINES, Preview, card_preview
└── mod.rs        pub mod bar; pub mod drawer;
crates/micold-core/src/tokens/anatomy.rs      + review_bar::{HEIGHT, SEND_WIDTH}, comments_drawer::WIDTH
crates/micold-client/src/ui/material/
├── review_bar.rs        NEW ReviewBar (counts form choice at layout, fixed trailing group)
├── side_sheet.rs        NEW SideSheet (stacked right sheet: title, ×, content, width rule)
├── line_clamp.rs        + LineClamped::when_cut
├── review_comment.rs    reply drawn through LineClamped (4 lines) with Show all when cut
├── review_panel_row.rs  + whole-reply mode, Mark applied / Answer… actions
└── mod.rs               exports
crates/micold-client/src/showcase/sections/review.rs   review_bar, side_sheet, card and row poses

# Layer B — after #1269 merges
crates/micold-client/src/features/changes.rs   Panel → ReviewDrawer; review_bar, drawer_rows, drawer_groups;
                                               Msg::{Drawer, DrawerRowSelected, OverflowToggled, OverflowDismissed,
                                               SendFailed, SendErrorDismissed}; ReviewOverflowMenu surface;
                                               ReviewChanged → newly_reported → auto-open
crates/micold-client/src/overlay/registry.rs   register ReviewOverflowMenu (Popover)
crates/micold-client/src/app.rs                on_escape: last arm closes the shown drawer
crates/micold-client/src/shell/changes.rs      PendingOp::ReviewSend gets the entry key
crates/micold-client/src/shell/daemon_sync.rs  failure → Msg::SendFailed (snackbar kept)
crates/micold-client/src/ui/changes.rs         header without Review(n)/Send; diff + SideSheet stack; bar row;
                                               drawer tabs and rows; cards' 4-line replies
crates/micold-client/tests/features_changes.rs bar, drawer, send error, auto-open, Esc state tests
crates/micold-client/tests/support/covered_states.rs  review-bar / drawer-drafts / drawer-replies / overflow states
crates/micold-client/tests/gates/review_bar_fixed.rs  NEW SC-001, SC-002, SC-004 geometry gate
crates/micold-client/tests/layout_snapshot.rs          #[path = "gates/review_bar_fixed.rs"] mod registration
crates/micold-client/tests/fixtures/layout_snapshot.txt  re-recorded
docs/user-guide/reviewing-changes.md
```

**Structure Decision**: the existing three-crate split. Rules in `micold-core` (pure, no
dependency on the client's view state), reusable widgets in `micold-client/src/ui/material/`,
render-free view state and messages in `features/changes.rs`, drawing in `ui/changes.rs`. No
daemon change.

## Test strategy by layer

| Layer | Where | Covers |
|---|---|---|
| Core unit | `review/bar.rs` | FR-002 counts per scope and texts (all zero, singular, zero statuses left out, both scopes); R3 forms; FR-003/FR-005 send button (0, n, 999+, sending, unsendable); FR-007 overflow incl. Applied/Answered count for Clear sent; FR-006 wording |
| Core unit | `review/drawer.rs` | FR-013/SC-006 tab membership over all four statuses and both scopes; FR-014 order and grouping; US3 #7 labels; FR-018 empty texts; FR-011 closed on creation; FR-008/FR-012 Toggle/Close/Escape (Esc on closed drawer not consumed); FR-019 offsets per tab; FR-020 auto-open: first report opens on Replies, second does not, not shown → consumed and stays, closed since → stays, open keeps tab, failed send drops, another send replaces; `newly_reported` transitions; FR-010 width rule |
| Core unit | `review/reply.rs` | FR-021 `card_preview`: ≤ 4 lines kept whole and `cut == false`, wrapped long line counts its lines, over 4 lines cut with `cut == true`, 4,000-char cap still applies, control characters shown |
| Core unit | `tokens/anatomy.rs` | new values listed in the anatomy table |
| Component | `ui/material/review_bar.rs` | B2 longest fitting form at three widths; B3 trailing bounds identical across label, counts, error and drawer flag (in-crate layout test) |
| Component | `ui/material/side_sheet.rs`, `line_clamp.rs`, `review_comment.rs`, `review_panel_row.rs` | D2 width min(360, half); `when_cut` element laid out only when cut; card Show all only when cut; row whole-reply mode and actions |
| Client state | `tests/features_changes.rs` | FR-001 bar absent on a session tab; B4–B8 via `review_bar` and messages; FR-004 `ReviewSend` effect unchanged; FR-006 error set on `SendFailed` for the sending view, cleared on next send and dismiss; D5–D7 `drawer_rows`; FR-017 jumps; FR-019 drawer survives switching with #1269's view map and is dropped with the tab; FR-020 over `ReviewChanged` sequences incl. a report from another window's send; FR-022 focus and dialog cleanup |
| Client state | `app.rs` tests, `overlay/registry.rs` tests | FR-012/SC-003 Esc order: overflow menu, discard dialog, Show all dialog first; then drawer; closed drawer → today's behaviour |
| Geometry gate | `tests/gates/review_bar_fixed.rs` (layout snapshot binary) | SC-001 send button bounds equal across drawer closed/open, 0/99 drafts, error on/off; SC-002 diff pane bounds and first row equal across drawer toggle; SC-004 diff spans the tab body beside the file list with the drawer closed at 1280×800; FR-017 drawer's left edge at or right of the diff area's middle |
| Layout snapshot | `tests/layout_snapshot.rs` + `covered_states.rs` | FR-024 new covered states, fixture re-recorded in M4 |
| Manual | quickstart §B, §C | `ui/changes.rs` glue, look of bar, menu, drawer and cards (SC-007), platforms |

Requirements mapped to where they are built and tested:

| FR | Built in | Tested in |
|---|---|---|
| FR-001 | `features/changes.rs` `review_bar`, `ui/changes.rs` | client state; layout snapshot |
| FR-002 | `review/bar.rs` | core unit; component test |
| FR-003 | `review/bar.rs`, `material/review_bar.rs` | core unit; geometry gate |
| FR-004 | `features/changes.rs` `SendPressed` (unchanged effect) | client state (existing send tests kept green) |
| FR-005 | `review/bar.rs` `send_button` | core unit |
| FR-006 | `review/bar.rs` `error_text`, `features/changes.rs`, `shell/daemon_sync.rs` | core unit; client state; §B step 10 |
| FR-007 | `review/bar.rs` `overflow`, `features/changes.rs` menu surface | core unit; client state; registry test |
| FR-008 | `review/drawer.rs` `Toggle` | core unit; client state |
| FR-009 | `ui/changes.rs` header | layout snapshot; §B |
| FR-010 | `review/drawer.rs` `width`, `material/side_sheet.rs`, `ui/changes.rs` stack | core unit; component; geometry gate |
| FR-011 | `Drawer::default` | core unit |
| FR-012 | `review/drawer.rs`, `app.rs` `on_escape`, overflow surface | core unit; app tests |
| FR-013, FR-014 | `review/drawer.rs` `DrawerTab::of`, `rows_of`, `tab_counts` | core unit (SC-006); client state |
| FR-015, FR-016 | `features/changes.rs` `drawer_rows`, `material/review_panel_row.rs` | client state; component |
| FR-017 | `features/changes.rs` jump (today's) | client state; §B step 8 |
| FR-018 | `review/drawer.rs` texts | core unit |
| FR-019 | `Drawer` in #1269's per-entry view, `Composer.place` | core unit; client state |
| FR-020 | `review/drawer.rs` `Drawer`, `newly_reported`; `features/changes.rs` `ReviewChanged` | core unit; client state |
| FR-021 | `review/reply.rs` `card_preview`, `material/line_clamp.rs`, `review_comment.rs` | core unit; component |
| FR-022 | existing `ReviewChanged` push; read models recomputed | client state |
| FR-023 | all rules above in core or render-free client code | `mise run test-core`, `mise run gate` |
| FR-024 | `docs/user-guide/reviewing-changes.md`, `covered_states.rs`, showcase | docs CI; layout snapshot; §B |

## Delivery order

Milestones (cut in tasks.md); each leaves the app working and the gate green.

1. **M1 — core rules (Layer A, any time)**: `review::bar`, `review::drawer`, `reply::card_preview`,
   anatomy tokens, all Red first. No client change; the client keeps today's panel.
2. **M2 — shared components (Layer A, any time)**: `ReviewBar`, `SideSheet`,
   `LineClamped::when_cut`, `ReviewPanelRow` reply mode, card through `LineClamped`, showcase
   entries. The card's 4-line rule ships here (it uses only the card component), so the existing
   panel already benefits. The layout snapshot changes only if a covered state shows a card with a
   long reply; then it is re-recorded in M2 with that diff alone.
3. **M3 — reducer and shell wiring (Layer B, after #1269 merges)**: `ReviewDrawer` in the per-entry
   view, bar read model, messages, overflow surface, Esc arm, send failure routed with the entry
   key, auto-open over `ReviewChanged`. The old panel's view code calls the new read models so the
   window still draws (the panel shows until M4).
4. **M4 — Review tab layout, snapshot, docs (Layer B)**: `ui/changes.rs` header, bar row, drawer
   stack and tabs; covered states; `layout_snapshot.txt` re-recorded; `review_bar_fixed` gate; user
   guide; quickstart §B visual pass at 1280×800 in both themes.

If #1269 has not merged when M2 is done, the ledger records it and the run waits; the fallback
(build M3 against today's single `State.open`, research R11) needs a decision recorded in the
ledger before use.

## Complexity Tracking

No constitution violation to justify.
