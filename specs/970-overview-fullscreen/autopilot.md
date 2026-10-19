# Autopilot ledger — #970 overview-fullscreen

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #970 — "Overview: full-screen mode hides the sidebar, animated" (title and body as in spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #970
- **Worktree branch**: claude/project-thread-ag6ana
- **Started**: 2026-10-18
- **Phase**: implement M2
- **Next step**: M2 — see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1066 | design (spec, plan, tasks) | merged (rebase) | d7005808fd4f200f8ec2fa1afb23e4d640f53aeb |
| #1074 | M1 (full screen hides the sidebar) | merged (rebase) | 3603ce06621499419e86a60e106d605b14a7904c |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Full screen hides the sidebar and gives it back (US1 + US4) | #1074 | merged |
| M2 | T018–T026 | full | The cards zoom with the sidebar (US2 #1–4) | — | implemented (T018–T026 ticked), reviews next |
| M3 | T027–T031 | full | The zoom keeps your place (US2 #5) | — | pending |
| M4 | T032–T041 | full | Reduce motion (US3) | — | pending |

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
| Spec review | 1 | 0f936e933b45470ec78960e325652da519ed1345:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CHANGES: 4 MAJOR, 2 MINOR |
| Spec review | 2 | 89236166dc9f65c1c4ccda6390048046cc91bcdd:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CLEAN (2 MINOR, fixed prose only) |
| Plan review | 1 | 492e6e945eaee1a4aadbe9442ad1423220268194:32e91cce3fcb38094295f0fa9ffdcd4c201aecb4 | CLEAN (1 MINOR: tree state renamed `ZoomState`, fixed prose only) |
| Tasks review | 1 | de0b3a34548aa9cfd42aed76365f2eab957ca1a1:a2c8ebd18334b6968d29ad0003a6e495dfeffbb5 | CLEAN (3 MINOR: M1 Satisfies, T012/T013 order, guide [P]; fixed prose only) |
| M1 review A (code-review high) | 1 | 43fe79e5d9dd39a93cefe98ee80f754afe264105:cfbf874fbc35d2b5ecbe356f3992e6fcc0c869c1 | CHANGES: 2 MAJOR fixed (chord under a card menu now ignored; release reaches a gone drawer's children), 1 declined, 5 MINOR not fixed |
| M1 review A (code-review high) | 2 | ed7739855c19f2a74e84e6499240565858f777fc:89c9bc412a899e37005bfa718a1d0324bdda0745 | CLEAN (9 findings, none BLOCKER/MAJOR on triage: dialogs/switcher are modal and not grid-anchored, ResizeHandle has no touch drag, key repeat shared with the Overview chord; not fixed) |
| M1 review B (conformance, sonnet) | 1 | ed7739855c19f2a74e84e6499240565858f777fc:89c9bc412a899e37005bfa718a1d0324bdda0745 | CLEAN (1 MINOR: card-menu guard recorded in data-model + user guide, prose only) |
| M1 visual pass | 1 | — | FAIL → fixed: Tab reached a retracting drawer (operate now gated on `retracted`); Escape dead over the Overview (pre-existing: `dismiss_topmost` asked only the registry); showcase drawer section blank (pre-existing, follow-up) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | Full-screen chord not heard while a terminal has the keyboard | By design (R7: not in the terminal's reserved tier, so terminals keep Ctrl+Shift+F); opening the Overview moves the keyboard to the first card (R13). Keymap comment and user-guide sentence corrected (prose only) |

## Handover

M2 implemented and committed (T018–T026 ticked, cycle-log written). Done since: fmt; the scoped gate's
only failure was `idle_requests_no_frames` (frame requests must go through `Progress`); fixed by replacing
`ZoomState.started: Instant` with `clock: Progress` (ZOOM_LIMIT = SLIDE + 100 ms, `on_layout_frame` in
`Grid::update`); data-model.md's `started` mention should now read `clock` (update it). Next:
1. Commit the working tree (fmt + clock fix, uncommitted at hand-over) as `fix(970): ...`.
2. Review A (round 1) was started detached on snapshot `ecb6d8c4e7d818f7cd319b4870d4b812095fe700:4ff93b1c3d965bda0ffa49b2acee062c108e0590`: prompt `/tmp/claude-0/-home-user-micold-ai-ide/610acce1-41f7-5bc4-9972-1f899e9527fb/scratchpad/promptA.txt`,
   output `/tmp/claude-0/-home-user-micold-ai-ide/610acce1-41f7-5bc4-9972-1f899e9527fb/scratchpad/review-A.md` (ends with `REVIEW_EXIT=`; if empty/absent, re-run the same command:
   `claude -p --agent autopilot-reviewer --permission-mode auto "$(cat promptA.txt)"`). Act on it per
   review-rounds.md (it saw the pre-clock-fix tree).
3. Scoped gate again (`scripts/autopilot/scoped-gate.sh`, detached, with CARGO_INCREMENTAL=0
   CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0), then review B (conformance rubric, sonnet) and the
   visual pass (zoom pose in showcase; quickstart §B 1, 3, 8), full gate (`scripts/gate.sh`), push
   (see prompt override: ls-remote then --force-with-lease), PR title/body into the scratchpad.
Known gap: no separate behavioural red for T021 (compile red only; recorded in cycle-log).

## Open escalation

None.

## Follow-ups not done

- Showcase `NavigationDrawer` section renders blank (its pre-existing open/closed poses too; `size()` Shrink×Fill unchanged from main) — seen by the M1 visual pass, `specs/970-overview-fullscreen/visual-pass.md` finding 3.

## Where things went (M2)

- Zoom lives in `ui/material/card_grid.rs`: `Zoom`, `zoom_fraction`, `rest_width`, `ribbon`, `Piece`, `ZoomState` (+ `advance`), private `Grid` widget (the `Scrollable` content; `CardGrid::into_grid`). `SLIDE` is `pub(super)` in `navigation_drawer.rs`. `RESIZE_HANDLE_WIDTH` re-exported from `material/mod.rs`.
- Glue: `ui/overview.rs` `freed_width`; `ui/sidebar.rs` `STRIP_WIDTH` is `pub(crate)`. Showcase pose "zoom" in `showcase/sections/surfaces.rs` (`card_grid`), catalogue entry now interactive.
- Deviations: a flip's start width is the previous *layout's* width (`last_width`), not the flip layout's; the zoom end rule heads for the larger end when entering and the smaller when leaving (so the flip layout itself, at the start width, never ends it). On the ribbon the gap at a row end is hidden, so a split card's pieces sum to `w` minus at most the gap (the spec's "sum to w" holds up to the gap). Pointer events over any piece of a card reach the card, translated to its primary box.
