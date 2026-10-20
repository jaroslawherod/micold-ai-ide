# Autopilot ledger — #970 overview-fullscreen

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #970 — "Overview: full-screen mode hides the sidebar, animated" (title and body as in spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #970
- **Worktree branch**: claude/project-thread-ag6ana
- **Started**: 2026-10-18
- **Phase**: close unit running (tdd-verify, T042, T043)
- **Next step**: tdd-verify, visual pass T042, full gate T043, close PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1066 | design (spec, plan, tasks) | merged (rebase) | d7005808fd4f200f8ec2fa1afb23e4d640f53aeb |
| #1074 | M1 (full screen hides the sidebar) | merged (rebase) | 3603ce06621499419e86a60e106d605b14a7904c |
| #1143 | M2 (the cards zoom with the sidebar) | merged (rebase) | 34fccd299512b1ea90e592c8a920a631ba26ebf7 |
| #1144 | M3 (the zoom keeps your place) | merged (rebase) | 0914629b79db4e4dd0d59658bba82df746e986fe |
| #1145 | M4 (reduce motion) | merged (rebase) | 5f54853a04f27a1048a9148df4eb184d3efb1dd5 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Full screen hides the sidebar and gives it back (US1 + US4) | #1074 | merged |
| M2 | T018–T026 | full | The cards zoom with the sidebar (US2 #1–4) | #1143 | merged |
| M3 | T027–T031 | full | The zoom keeps your place (US2 #5) | #1144 | merged |
| M4 | T032–T041 | full | Reduce motion (US3) | #1145 | merged |

Polish T042–T043 (visual pass, final gate) changes no code: close unit. M1 keeps US1 whole at 15
tasks (+ US4's 2): toggle, drawer retraction and exits are one state change, and no split along a
scenario leaves a deliverable without half-wired UI (a header button with no retraction); estimated
diff under 800 lines. US4 rides in M1: it has no code of its own (structural from D1/D2), so alone
it would be a tests-only milestone with nothing new to observe. US2 split along scenario 5 (anchor,
R6) to stay near 10 tasks. M4 is 10 tasks; its serde field is a persistence-format change, hence
`full`.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | FR-001: how is full-screen mode entered? | Toggle in the Overview header + shortcut, off each time the Overview opens, not remembered | user (2026-10-18) | decision card |
| 2 | clarify r1 | FR-008: source of reduced motion? | New *Reduce motion* setting, off by default, this zoom only; OS preference not read | user (2026-10-18) | decision card |
| 3 | clarify r2 | Shortcut while the Overview is closed? | Does nothing | agent | spec.md#Assumptions |
| 4 | clarify r2 | Reduce motion before Save / on Cancel? | Save applies, Cancel discards | agent | docs/user-guide/settings.md |
| 5 | tasks | Where do the header-text and mode-state tests live? | `ui/overview.rs` in-crate (toolbar's `overview_tooltip` pattern) and `tests/features_overview.rs`; plan's test-location lines fixed to match (no behaviour change) | agent | crates/micold-client/src/ui/toolbar.rs:215 |
| 6 | M1 implement | T003's chord tests: `keymap.rs` `mod tests` or `tests/keymap.rs`? | `tests/keymap.rs`, beside the existing `is_overview_chord` tests (the in-file module has none) | agent | crates/micold-client/tests/keymap.rs:243 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Close review | 1 | 97e7f19d9a90b880f82a4b75878ab7efcf65595f:e74be4d14e3b08e7c28a628c351ae413bb7a1138 | CHANGES: 1 MAJOR (Status said Closed while T052–T055 open: T052–T054 done, T055 deferred with reason in verification.md Resolution, Status stays Closed as the close PR closes it), 2 MINOR fixed (stale comments, U35 wording) |
| Spec review | 1 | 0f936e933b45470ec78960e325652da519ed1345:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CHANGES: 4 MAJOR, 2 MINOR |
| Spec review | 2 | 89236166dc9f65c1c4ccda6390048046cc91bcdd:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CLEAN (2 MINOR, fixed prose only) |
| Plan review | 1 | 492e6e945eaee1a4aadbe9442ad1423220268194:32e91cce3fcb38094295f0fa9ffdcd4c201aecb4 | CLEAN (1 MINOR: tree state renamed `ZoomState`, fixed prose only) |
| Tasks review | 1 | de0b3a34548aa9cfd42aed76365f2eab957ca1a1:a2c8ebd18334b6968d29ad0003a6e495dfeffbb5 | CLEAN (3 MINOR: M1 Satisfies, T012/T013 order, guide [P]; fixed prose only) |
| M1 review A (code-review high) | 1 | 43fe79e5d9dd39a93cefe98ee80f754afe264105:cfbf874fbc35d2b5ecbe356f3992e6fcc0c869c1 | CHANGES: 2 MAJOR fixed (chord under a card menu now ignored; release reaches a gone drawer's children), 1 declined, 5 MINOR not fixed |
| M1 review A (code-review high) | 2 | ed7739855c19f2a74e84e6499240565858f777fc:89c9bc412a899e37005bfa718a1d0324bdda0745 | CLEAN (9 findings, none BLOCKER/MAJOR on triage: dialogs/switcher are modal and not grid-anchored, ResizeHandle has no touch drag, key repeat shared with the Overview chord; not fixed) |
| M1 review B (conformance, sonnet) | 1 | ed7739855c19f2a74e84e6499240565858f777fc:89c9bc412a899e37005bfa718a1d0324bdda0745 | CLEAN (1 MINOR: card-menu guard recorded in data-model + user guide, prose only) |
| M1 visual pass | 1 | — | FAIL → fixed: Tab reached a retracting drawer (operate now gated on `retracted`); Escape dead over the Overview (pre-existing: `dismiss_topmost` asked only the registry); showcase drawer section blank (pre-existing, follow-up) |
| M2 review A (code-review high) | 1 | ecb6d8c4e7d818f7cd319b4870d4b812095fe700:4ff93b1c3d965bda0ffa49b2acee062c108e0590 | CHANGES: 1 MAJOR fixed (`cursor_for_state` mixed absolute and grid-relative coordinates; test with a nonzero grid origin and a split card added) |
| M2 review A (code-review high, sonnet) | 2 | e915063c705611ab260afb599fb42f45c0593e1b:c4f236d890ae55fc6ea27212f0565a008c8ccd4e | CLEAN |
| M2 review B (conformance, sonnet) | 1 | e915063c705611ab260afb599fb42f45c0593e1b:c4f236d890ae55fc6ea27212f0565a008c8ccd4e | CHANGES: 1 MAJOR (no behavioural red for T021) fixed by a mutation run recorded in the cycle-log (no re-review: record only); 1 MINOR (test count) fixed |
| M2 visual pass | 1 | — | PASS (zoom 3→4 columns, rest layouts, showcase pose; split-card mid-frames not observed, covered by `ribbon` unit tests) |
| M3 review A (code-review high) | 1 | c8483b7d07bdf0a4a36969e4f8711dd9aeaa4761:34fccd299512b1ea90e592c8a920a631ba26ebf7 | CLEAN |
| M3 review B (conformance, sonnet) | 1 | 3cd7cdcfe0ecbcc3c43be3158a6a371c6687c908:34fccd299512b1ea90e592c8a920a631ba26ebf7 | CLEAN (2 MINOR: tests-after-code gap, recorded in the cycle-log; T030 names app.rs, the Task is in main.rs, recorded under Where things went) |
| M3 visual pass | 1 | — | PASS (30 cards, scrolled to the bottom, card 20 stays in its row after entering full screen; no overlap mid-zoom). Not observed: card 20 as the very first card, exit, the chord |
| M4 review A (code-review high) | 1 | afd2fd0973e0969385ee1ea0e807f62d72e0da6f:a0883430df135c44ac6b91bd5a0cb7ec2db8c089 | CLEAN |
| M4 review B (conformance, sonnet) | 1 | afd2fd0973e0969385ee1ea0e807f62d72e0da6f:a0883430df135c44ac6b91bd5a0cb7ec2db8c089 | CLEAN (3 MINOR: no red for the persist and grid tests, Esc route untested (Esc maps to `Cancelled`), plan D4/T039 still said `LabelledToggle`: the last fixed, prose only) |
| M4 visual pass | 1 | — | PASS (Appearance page, Save/Cancel, on: no intermediate frame entering or leaving, off: still animates). Not exercised: header button with the setting on, the sidebar's own slide with it on |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | Full-screen chord not heard while a terminal has the keyboard | By design (R7: not in the terminal's reserved tier, so terminals keep Ctrl+Shift+F); opening the Overview moves the keyboard to the first card (R13). Keymap comment and user-guide sentence corrected (prose only) |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Close visual pass (T042) PASS on top bar, connection lost, 650 px; showcase NavigationDrawer still blank, so the follow-up stays open: file it as a bug (pre-existing, `surfaces.rs:820`, all four poses missing).
- Showcase `NavigationDrawer` section renders blank (its pre-existing open/closed poses too; `size()` Shrink×Fill unchanged from main) — seen by the M1 visual pass, `specs/970-overview-fullscreen/visual-pass.md` finding 3.

## Where things went (M2)

- Zoom lives in `ui/material/card_grid.rs`: `Zoom`, `zoom_fraction`, `rest_width`, `ribbon`, `Piece`, `ZoomState` (+ `advance`), private `Grid` widget (the `Scrollable` content; `CardGrid::into_grid`). `SLIDE` is `pub(super)` in `navigation_drawer.rs`. `RESIZE_HANDLE_WIDTH` re-exported from `material/mod.rs`.
- Glue: `ui/overview.rs` `freed_width`; `ui/sidebar.rs` `STRIP_WIDTH` is `pub(crate)`. Showcase pose "zoom" in `showcase/sections/surfaces.rs` (`card_grid`), catalogue entry now interactive.
- Deviations: a flip's start width is the previous *layout's* width (`last_width`), not the flip layout's; the zoom end rule heads for the larger end when entering and the smaller when leaving (so the flip layout itself, at the start width, never ends it). On the ribbon the gap at a row end is hidden, so a split card's pieces sum to `w` minus at most the gap (the spec's "sum to w" holds up to the gap). Pointer events over any piece of a card reach the card, translated to its primary box.

## Where things went (M3)

- `card_grid.rs`: `CardGrid::on_reflow` / `scroll_id`, `OnReflow`, `first_visible`, `card_top`; `ZoomState.anchor` / `was_zooming`; the anchor logic is the top block of `Grid::update`. Test rig: `Rig::frame` (+ `lay_element`).
- Glue: `features/overview.rs` `Msg::Reflowed` (an `f32` y, not `AbsoluteOffset`: `features_are_render_free` bars iced types from feature modules, a deviation from contracts/ui.md; + hand-written `impl Eq for Msg`); `ui/overview.rs` `GRID_SCROLL_ID`; the root's `scroll_to` is in `main.rs` `overview_message` (the Task side; `app.rs` has no Tasks). User guide: one sentence in `docs/user-guide/overview.md`.

## Where things went (M4)

- Core: `Settings.reduce_motion` + `StoredSettings` in `micold-core/src/settings.rs` (`reduce_motion_tests`). Client: `features/settings.rs` (`AppearanceDraft`/`ValidSettings`/`SettingsState.reduce_motion`, `Msg::ReduceMotionToggled`, `reduce_motion_toggled`); applied in `shell/persist.rs` (`persist_settings`, `window_settings`, `set_changed`, the Save apply) and seeded in `shell/startup.rs`; the page is `ui/settings/appearance.rs` (a `Checkbox` + `field_note`, not the plan's `LabelledToggle`, an icon button).
- Drawer: `NavigationDrawer::instant` snaps `presence` in `layout` (`restart_at`), `ui/mod.rs` passes `state.settings.reduce_motion`. Grid needed no code: `ZoomState::advance` already ends a zoom whose width reaches the end (test only). Docs: `settings.md#reduce-motion`, one paragraph in `overview.md`.
