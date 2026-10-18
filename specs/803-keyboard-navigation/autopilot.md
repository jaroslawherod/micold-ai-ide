# Autopilot ledger — #803 keyboard-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937, 686  and 803 in separate threads using speckit autopilot — issue #803 "Keyboard shortcuts to navigate worktrees, sessions and projects"; labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #803
- **Worktree branch**: claude/project-thread-ah7xxz
- **Started**: 2026-10-17
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #949 | design: spec, plan, tasks | merged | c410a749a8c6a417aa91f9011fd2a16fe2e340a8 |
| #967 | M1 | merged | c5e61c3ea91a464695dcf9fb42b53121fac37490 |
| #984 | M2 | merged | 380ceef79760b7f896b15b683276bdd0924aa959 |
| #987 | M3 | merged | 2107a1a117dfa8aa2f6e45cddcb5af83a78850a9 |
| #990 | M4 | merged | 68ddfea358f14bb1811220c44e1ab67a8112d580 |
| — | close | orchestrator opens it (title/body in the scratchpad) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Location chords step through locations (both focus states, stop, 0 PTY bytes), docs rows | #967 | merged |
| M2 | T017–T023 | full | Keyboard lands on the sidebar row with a focus ring (FR-010), showcase variant | #984 | merged |
| M3 | T024–T029 | light | Session chords cycle a location's sessions, docs rows | #987 | merged |
| M4 | T030–T037 | full | Project chords; all six chords in the user guide | #990 | merged |

US1 split along acceptance scenario 4 (FR-010 sidebar ring) into M1/M2: one milestone would exceed ~800 changed lines. Polish (T038–T039) changes no code: close unit.

- 2026-10-17: M1 rebased onto origin/main 70f58726 (PR #967 conflicted, #813 M4 + #937 M1); conflicts in `session.rs` (`selected` keeps 803's KeyboardRowLeft/MainAreaTaken and 813's RevealMarkCleared), `clipboard.rs` (all three outcomes listed), `ui/mod.rs` (813's overview arrow listener kept beside 803's `app_chord` listener; no chord collision).

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Wrap or stop at list ends? | Wrap at every level | agent-resolved | spec.md#Assumptions |
| D2 | spec | User-configurable bindings? | Out of scope, follow-up | agent-resolved | spec.md#Out of Scope; no key-binding settings exist |
| D3 | clarify | Do FR-001 defaults collide with system/IME/app chords? | No (IBus Ctrl+. is exact-modifier; Win+. emoji; macOS reserves none) | agent-resolved | spec.md#Clarifications; docs/user-guide/terminal-panes.md |
| D4 | clarify | macOS project chords need Fn: keep PageUp/PageDown? | Keep Ctrl/Cmd+Shift+PageUp/PageDown everywhere | CONFIRMED by the user on 2026-10-17 | spec.md#Clarifications |
| D5 | clarify | "Jump directly" in scope? | Out of scope; quick-switcher palette follow-up | CONFIRMED by the user on 2026-10-17 | spec.md#Clarifications |
| D6 | tasks | speckit-analyze findings | Spec amended: SC-004 scoped for stops, US1 AC5 and FR-010 state the stop and where the keyboard stays, FR-012 adds the filter panel, SC-001 ⌊N/2⌋, Terms: current location survives the filter. Plan D5/ui.md U3 amended: render-free `State::finish_navigation` so main.rs holds no branch (Principle I) | agent-resolved | speckit-analyze report I1–I4, A1–A3, T2 |
| D7 | implement | Plan names `Message::Navigate(NavAction)`; the root-vocabulary, overlay-subscription and feature-registration guards refuse a single-feature root variant | Feature wrapper `Message::Navigation(navigation::Msg::Pressed(NavAction))`, shape-B entry `shell::navigation::update`; `navigation`→`sidebar` read allowlisted; `tests/features_navigation.rs` isolation test | agent-resolved | tests/root_vocabulary_is_cross_cutting.rs, feature_registration_cost.rs, overlay_registry.rs |

## Review rounds

- M4: A (headless, round 1) tree 0e6bcec8…: CLEAN, 2 MINOR fixed (refused-switch guard, reveal scroll). B (sonnet) CHANGES: MAJOR missing M4 cycle-log (fixed), MINOR unavailable-active test (added). Round 2 (sonnet, fix diff): CLEAN. Full gate (`scripts/gate.sh`) green. Visual pass: not run, nothing drawn differently (project chords reuse the M2 ring).

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 3570ac4feb529b41dda29f1758629810e99a726c:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 1 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | 9bd245752e2eea4f7d52a5c833ceb957c3b44fca:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN |
| Review plan | 1 | a5368675fa0da6d90c2d114a39f92b52516249ca:89f9e66558d2c1507c412e6c77e4f61181a88dc9 | CHANGES: 3 MAJOR, 3 MINOR (all fixed) |
| Review plan | 2 | d47f661a5157eef58b9da4fd0191667ca463746b:0dc00fc7e758a81c7a68fdc8efd44c8ef14d31a8 | CLEAN |
| Review tasks | 1 | 5fd429a37a9d1d845b1988c23a153ea3f2524fdf:e48aa579382d1917198e8b478d11eabc68785ce3 | CHANGES: 1 MAJOR ([P] tasks sharing tests/navigation.rs: apply tests moved to navigation_apply.rs), 1 MINOR (SC-001 bound test added) |
| Review tasks | 2 | ea280ce8681586436fafe3903453010ab4c577a5:e48aa579382d1917198e8b478d11eabc68785ce3 | CLEAN (sonnet, scoped) |
| Review A M1 (code-review high) | 1 | 6c2da2fa240fc273d8be5a262d19b689ee24be3c:b69d9a7f1c50957529a7b62c4656082b219a8d59 | CLEAN: 1 MINOR (declined) |
| Review B M1 (conformance, sonnet) | 1 | 6c2da2fa240fc273d8be5a262d19b689ee24be3c:b69d9a7f1c50957529a7b62c4656082b219a8d59 | CLEAN (headless, Read-only on prepared diff files; Verify from the unit's runs) |
| Review A M2 (code-review high, headless) | 1 | 18d14e8fd29c699459bc7387d9d4c84af422d0fb:025714b3e366e8d011828262937d3442cb12b550 | CLEAN: 3 MINOR (declined) |
| Review A M3 (code-review high, headless) | 1 | 20cc549b4c94aa5d14ed040724232845d092b355:380ceef79760b7f896b15b683276bdd0924aa959 | CLEAN: 1 MINOR (fixed) |
| Review B M3 (conformance, sonnet, headless) | 1 | 20cc549b4c94aa5d14ed040724232845d092b355:380ceef79760b7f896b15b683276bdd0924aa959 | CHANGES: MAJOR no cycle-log rows for T024–T029 (fixed, rows added); 1 MINOR (fixed). Rows are evidence text only, no re-review |
| Review B M2 (conformance, sonnet, headless) | 1 | 18d14e8fd29c699459bc7387d9d4c84af422d0fb:025714b3e366e8d011828262937d3442cb12b550 | CLEAN: 2 MINOR (1 fixed in the ledger, 1 declined) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | A F1-F3 | alias placement, unused `terminal_had_focus`, duplicated row builders | MINOR; code frozen once the full gate ran. `terminal_had_focus` stays: T018's specified signature. |
| M2 | B F2 | no diff-path test for a disabled wrapper given focus | MINOR; the mounted-wrapper path is covered by the ring test and `focused && enabled` is one expression. |
| M1 | A | MINOR: `SidebarMsg::KeyboardRowLeft` has no production sender | T012 requires it; M2 T019 sends it from the focus ring's `on_change` |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- User-configurable key bindings (spec Out of Scope).
- Quick-switcher palette / direct jumps (FR-014, D5): follow-up issue.
- Visual pass M2: showcase ring checked on Xvfb (`visual-pass/m2.md`); quickstart B1/B3/B7 run live in the close unit (`visual-pass/t038.md`).
- §C (macOS, Windows) not run: no such host. TDD verify: M1–M3 red runs not captured (`tdd/verification.md`); no cargo-mutants, three hand mutants killed.
- Visual pass skipped for M1 (M2 does it): nothing drawn changes (the keyboard-row ring is M2); the stop only moves the reveal scroll.
- Spec reviews ran as headless `claude -p` sessions (no Agent tool in this unit's environment); fresh context, same prompt parts.
