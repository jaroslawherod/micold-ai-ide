# Autopilot ledger — #1125 sidebar-state-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1125 "Layout: sidebar grouped by state with Overview as its first row" (body verbatim in the issue and in spec.md *Input*); the user asked to start the app redesign issues in dependency order. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1125
- **Worktree branch**: claude/project-thread-22fhnj
- **Started**: 2026-10-24
- **Phase**: milestone M3
- **Next step**: M3 implement T032–T041

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1320 | Design: spec, plan, tasks | merged | 663899346b1c6c44682895eb6d8f4e4402f714ed |
| #1325 | M1: sidebar grouped by state | merged (rebase) | 0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 |
| #1345 | M2: two-line rows, primary selection | merged (rebase) | b54fa75959d6535260875f8859776e458e3851aa |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T019 | full | Sidebar grouped by state (headers with counts, Done collapsed, toggles, reveals/chords open a closed group) | #1325 | done |
| M2 | T020–T031 | full | Two-line rows, no type/issue chips, primary-container selection | #1345 | done |
| M3 | T032–T041 | full | Overview as the sidebar's first row, top bar's badge and tooltip | — | in progress |
| M4 | T042–T051 | full | Chip bar (All, tag chips, add, hide, ⋯ menu) replaces the filter panel | — | todo |
| M5 | T052–T058 | full | Keyboard and hover stay on a row that moves group; KeyedColumn in cdk | — | todo |

Polish T059–T060 changes no code: close unit. M1 kept whole at 19 tasks (grouping, collapse and reveal-into-collapsed only work together; tasks.md M1 *Size*). M2 kept whole at 12 (two lines and dropped chips are one row change). US3 before US2 (both P1): US2's selected look uses US3's `Selection::Primary`.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Overview button in the top bar: removed here? | No: it stays; #1127 removes it ("Blocked by: #1125 (Overview moves into the sidebar first)") | agent-resolved | issue #1127 body |
| D2 | spec | Order of Done vs Idle | Done last, after Idle | agent-resolved | issue #1125 body; mockup board sidebar |
| D3 | clarify 1 | Needs attention: own group or joins Needs action? | Joins Needs action; no separate group | agent-resolved | issue #1125 lists four groups; mockup puts fix/1117 "checks failing" (NeedsAttention, overview.rs card_state) under "Needs action · 2" |
| D4 | clarify 1 | Where do refresh, attach existing, run in parallel go? | A more (⋯) menu at the chip bar's end, beside add and hide | decided by user | jaro, 2026-10-24 |
| D5 | clarify 1 | Which groups are collapsible? | Every group; Done starts collapsed, the others expanded | decided by user | jaro, 2026-10-24 |
| D6 | clarify 2 | More menu's look and dismissal | Shared `Menu` primitive + feature 017 overlay rules | agent-resolved | ui/material/menu.rs, micold-core/src/overlay.rs module docs |
| D7 | plan | Overview row badge under a tag filter: all locations or the listed ones? | The listed ones, the same cards as the top bar badge; spec FR-008 and US2.6 amended | agent-resolved | ui/overview.rs `cards` walks `sidebar_entries()` (filtered); 813 FR-001 one card per listed location; SC-002 requires equality (research R10) |
| D8 | M1 | Where do T003 and T005 tests live? | T003 in `tests/overview.rs`, T005 in `tests/sidebar_state.rs` (`state_groups`), T008 order test in `tests/navigation_apply.rs`; one shared `five_states()` fixture in `tests/support/sidebar_fixture.rs` | agent-resolved | an in-crate `mod tests` has no app-state builder; `features_sidebar.rs` is the isolation test that builds no `State` (its module doc) |
| D9 | M1 | The Overview's reveal and the chord/current-session reveal: one target? | Two calls: `open_group_of_reveal_target()` (keyboard row, else current session) in `reveal_scroll` while armed, `open_group_of_revealed()` (`sidebar.revealed`) in `revealed_scroll` | agent-resolved | one chain would open the current session's group, not the revealed row's, when both exist |
| D10 | M1 | `sidebar_fixture::session_at` sessions are `Starting`, which counts as Working | The fixture's sessions are Idle, so existing navigation/sidebar tests keep today's order inside one Idle group | agent-resolved | `location_state::session_state` maps Starting to Running |
| D11 | M1 | Headers in the Tab order | A heading's keyboard wrapper is always enabled (rows only when focused), so Enter/Space reach it (US5.2) | agent-resolved | `TakesTheKeyboard::new(_, enabled)` skips a disabled wrapper |
| D12 | M1 | Rebase onto origin/main 4910aeee (feature 943 M9) | Clean rebase; the layout snapshot regenerated unchanged | agent-resolved | `UPDATE_LAYOUT_SNAPSHOT=1 … --test layout_snapshot`: 105 passed, no diff |
| D13 | M2 | The second line's sessions phrase: the spec's "1 session · 1 running" or today's Overview phrase? | Today's: `sidebar_second_line` composes `branch_line` + `meta_sessions` ("fix/1117 · 1 session running", "project root · 2 sessions, 1 running", "No sessions" alone); spec US3, Assumptions, contract T1, quickstart B3, tasks amended | agent-resolved | `sessions_line` became `meta_sessions` in BUG-1298 FR-023 (e55ff583) after the design; FR-011 and T1 say "no copy" of 1120's phrase |
| D14 | M2 | Tooltip: where do the full title and `Branch` go? | `features::sidebar::with_title_line` prepends the bare title as the first line; today's tooltip follows unchanged, its `Branch: <full branch>` line included (composed like `with_state_line`) | agent-resolved | FR-012 "in addition to what it shows today"; 27 existing `worktree_tooltip` tests keep today's lines |
| D15 | M2 | Where do T023's height tests and T029's showcase rows live? | Heights: `tests/features_sidebar.rs` and `tests/sidebar_state.rs` (where `row_heights` is tested; `sidebar_tree.rs` has none); showcase: `showcase/sections/surfaces.rs` TreeView entry (beside M1's group headers), catalogue lists "two-line row", "two-line row, selected" | agent-resolved | `grep -n row_heights crates/micold-client/tests/`; catalogue.rs TreeView entry's `posed` |
| D16 | M2 | A selected row's tint under `Selection::Primary` | Twisty, icon, label and supporting text take `on_primary_container`; the state icon keeps its state tint; only a neutral tint (`on_surface`, `on_surface_variant`) is replaced: a failed session's `error`, an interrupted one's `primary` and a missing worktree's error-tinted name keep their tint, checked ≥ 4.5:1 on `primary_container` (amended after M2 review A r1 F2) | agent-resolved | research R7; FR-010 "on-primary-container text and icons"; the lifecycle tints carry meaning (FR-006a of the session feature) |
| D17 | M3 | Where do T039's showcase rows live, and how does the Overview row take Tab and Enter? | The `TreeView` entry in `showcase/sections/surfaces.rs` (beside M1/M2's sidebar poses; catalogue lists "Overview row", "Overview row, with a badge"), drawn by the sidebar's own `ui::overview_row_view`; a new `TreeItem::tab_stop()` makes a row with a press a Tab stop answering Enter and Space, as a heading does | agent-resolved | D15 (the sidebar's poses are in surfaces.rs); contract S2 "Pressed (pointer, Enter, Space)", T035 "first focusable" |
| D18 | M3 | The Overview row shifts the sidebar column's children by one | Layout paths into the sidebar re-pointed (+1 at the sidebar column: `containment.rs` CLIP_REVEALED/SCROLL_CONTENT, `context_menu_anchor.rs`, `covered_states.rs` SIDEBAR_SESSION_ROW, row anchors, filter reveal node); snapshot regenerated | agent-resolved | layout_snapshot's anchor and staleness assertions named each path |


## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | da2d807c9816acd2626754adf775bd75bfdf81bb:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | e7874cdbc745d323aa39bf2af5b3eb37e903f5d6:14cef2c07be917f0d69a3be91fbcea6f77f1b079 | CLEAN |
| Analyze | 1 | — | 0 CRITICAL/HIGH; 4 MEDIUM, 7 LOW: I1 (spec FR-007/US2.5 now say no Overview row with no project open, matching Edge Cases), C1 (T012 and the test migration split), C2 (Tab-order tests T035, T045), C3 (revealed row in T033), C4, C5, D1 (plan Docs list), D2, A2, U1 fixed; A1 left (G5 already cross-referenced by FR-005/FR-006) |
| Tasks | 1 | c3988cc2b123a2d2b824ce43111671dff99979ae:a53d7012cf8f7b5295432a203dac8857c4efd1dc | CLEAN (3 MINOR: test callers named in T014, M1 and M4 Verify widened, US5.1 moved to M4 Satisfies; fixed) |
| M1 review A (code-review high) | 1 | aa345f554d65d9fd7a1b953deeb7a72508a0915f:9c2e4ba039aabe727a2b5933e8a90cf7a20669c7 | CHANGES: F1 header focus-loss sent KeyboardRowLeft (fixed); F2, F3 repeated sidebar derivations (fixed); F4 unreachable fallback (fixed); F6 unused From (fixed); F5 declined |
| M1 review A (code-review high, sonnet) | 2 | c656359f69d7d2b23bfdc07e5431aa3337fb346f:e11c2f7e8bb8d6cadc64b82f86df2e9ed1ae9100 | CLEAN of BLOCKER/MAJOR: r1 fixes hold; MINORs fixed (reveal opens a group only while its row is not drawn, stale comments, a test for a toggle of a hidden group) |
| M1 review B (conformance, sonnet) | 1 | e54c4f32191630712064a90d938fd3be236f14bf:f7ca36b5d89ba51b9d774db606ed4a96cc3124f5 | CHANGES: F1 a chord into a closed group did not open it while the current session's row was drawn (fixed: `State::reveal_ready`); F2 the test bypassed that gate (fixed: tests go through it, new case with a drawn current session) |
| M1 review B (conformance, sonnet) | 2 | 801128dd52fac679a288a6e71919acc608b17553:a72f9964ab30605790f03aa05a0d81ed83d18b56 | CLEAN: F1, F2 fixes hold |
| M2 review A (code-review high) | 1 | 66f5278f77aeaf3c85facc0d72387b01d8bdcc1a:34bf5238bde08351387c0396b0c75cf531aa8eb8 | CHANGES: F1 a rename lost the first line (fixed: `row_title` takes the rename, it wins); F2 Primary selection replaced lifecycle tints (fixed: neutral tints only, contrast pairs); F3 titles read only on Overview open (fixed: read when the worktree list changes); F5, F6 MINOR fixed; F4 declined |
| M2 review A (code-review high, sonnet) | 2 | ec2c9336949b1368872b8dbf32b907159948d404:a7213fdae80ddd2f0dd6cfd1409445afbd0b3c29 | CLEAN: F1–F3, F5, F6 fixes hold; F4 decline stands |
| M2 review B (conformance, sonnet) | 1 | 092fa3acba03841019c0235d725522fea9aecfa0:bf2c21dfec749c3aae73d8db08e4ab95052cf4e8 | CLEAN (1 MINOR: T029 named `atoms.rs`; fixed, prose only) |
| M2 visual pass | — | bf2c21df | B3, B4 (revealed row, both themes), B8 PASS; issue titles and a session's selected row not seen live (no GitHub, no agent session): `visual-pass/m2-record.md` |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 F5 | `card()` and `location_card_state` each filter the location's sessions | a constant factor per card; one function owning the card state (R2) is what makes the sidebar and the Overview agree |
| M2 | A r1 F4 | the second line's supporting text is `Length::Fill`; use `.shrink()` | iced lays out Shrink children before Fill ones, so a shrinking long second line would take the row's width first and squeeze the chips out in a narrow sidebar; Fill keeps the chips whole and ellipsizes the text |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M2 A r1 F1: a renamed worktree's sidebar row now leads with the rename, while its Overview card (feature 1120 FR-006, `ui/overview.rs` `card_title`) still leads with the issue title or branch. Whether the card should honour the rename too is 1120's call; not changed here.
