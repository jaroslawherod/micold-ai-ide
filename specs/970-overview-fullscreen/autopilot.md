# Autopilot ledger — #970 overview-fullscreen

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #970 — "Overview: full-screen mode hides the sidebar, animated" (title and body as in spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #970
- **Worktree branch**: claude/project-thread-ag6ana
- **Started**: 2026-10-18
- **Phase**: implement M1
- **Next step**: M1 implement (T001 written, T002 next)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1066 | design (spec, plan, tasks) | merged (rebase) | d7005808fd4f200f8ec2fa1afb23e4d640f53aeb |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Full screen hides the sidebar and gives it back (US1 + US4) | — | in progress |
| M2 | T018–T026 | full | The cards zoom with the sidebar (US2 #1–4) | — | pending |
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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 0f936e933b45470ec78960e325652da519ed1345:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CHANGES: 4 MAJOR, 2 MINOR |
| Spec review | 2 | 89236166dc9f65c1c4ccda6390048046cc91bcdd:7dded4dabfe494a9aa98b069d0cec860beee2dd2 | CLEAN (2 MINOR, fixed prose only) |
| Plan review | 1 | 492e6e945eaee1a4aadbe9442ad1423220268194:32e91cce3fcb38094295f0fa9ffdcd4c201aecb4 | CLEAN (1 MINOR: tree state renamed `ZoomState`, fixed prose only) |
| Tasks review | 1 | de0b3a34548aa9cfd42aed76365f2eab957ca1a1:a2c8ebd18334b6968d29ad0003a6e495dfeffbb5 | CLEAN (3 MINOR: M1 Satisfies, T012/T013 order, guide [P]; fixed prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 1 handed over at the 150k context cap (most of it spent reading code). Done: branch reset
to origin/main (d7005808, = #1066's merge); T001's four tests appended to
`crates/micold-client/tests/features_overview.rs` (red: no `FullScreenToggled`/`full_screen()`
yet; committed as the red phase). Next: T002–T006 tests, then T007 (record red in a new
`tdd/test-list.md` + `tdd/cycle-log.md`, precedent `specs/1024-overview-larger-cards/tdd/`: no
test list existed, the tasks' own T001–T007 red phase is the loop), T008–T017.
Findings from reading, so the next unit need not re-read:
- `.specify/extensions.yml` `before_implement` runs `speckit.tdd.run` (mandatory); it needs
  `tdd/test-list.md`, absent: write it for M1's behaviours as 1024 did.
- T012 settings_open: the root fills nothing. `ui/mod.rs::subscription` drops `app_chord_listener`
  while `settings_draft.is_some()`, so the chord's `settings_open: false` is always true; the header
  button fills it from `state.settings.settings_draft.is_some()` (as `ui/toolbar.rs::overview_press`).
  No `app.rs` change. Settings replaces the whole body (`ui/mod.rs` ~351), so the drawer is rebuilt
  on close: built-retracted-at-0 gives FR-011 with no zoom. `settings::Msg::Opened`/`Cancelled`
  don't touch the Overview.
- Drawer: `ui/mod.rs` ~396 builds it; `Progress` (`ui/cdk/motion.rs`) `.value()` is already eased
  when built with `.easing(EMPHASIZED..)`; `on_layout_frame`, `animating`, `restart_at` exist.
- App chord: `ui/mod.rs::app_chord` ~1252 (add after the `is_overview_chord` arm); chord helpers
  `keymap.rs` ~396–408 (`is_primary_shift_chord` private, same file).
- Icons: `icons.rs` enum ~173, `ALL` ~245, `glyph` ~335; `tests/icons.rs` count 66 → 68 and the
  overview test ~199; `tests/icons_font.rs` checks the font; PROVENANCE table ends at `MoveDown`.
- Showcase: `showcase/sections/surfaces.rs::navigation_drawer` ~706 (use `showcase.shown(index)` +
  `Message::Reversed(index)` as `accordion` ~206 does); poses list in `showcase/catalogue.rs` ~764.
- Overview view: `ui/overview.rs::view` ~29 (header row goes above the `CardGrid` container).
- Docs: `overview.md` *Opening and closing it* l.14 / *Keyboard* l.126; `terminal-panes.md` l.38;
  `worktrees-and-sessions.md` l.1684. The chord is not terminal-reserved: say so in the table row.
- Env: no mise, gh unauthenticated; use raw cargo with CARGO_INCREMENTAL=0; full gate
  `scripts/gate.sh` detached + hold.sh; push `--force-with-lease=claude/project-thread-ag6ana:`.

## Open escalation

None.

## Follow-ups not done

None.
