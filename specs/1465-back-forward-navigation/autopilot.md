# Autopilot ledger — #1465 back-forward-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1465 (issue #1465: Back/forward navigation with mouse side buttons and Alt+←/→); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1465
- **Worktree branch**: 1465-m4 (M4, stacked on 1465-m3; M3 on 1465-m3, stacked on 1465-m2; M2 on 1465-m2, stacked on 1465-m1; M1 on 1465-m1, design on claude/project-thread-e4cve5)
- **Started**: 2026-10-27
- **Phase**: M4 implemented (T027–T033 ticked, TDD cycles 15–17); full gate green (scripts/gate.sh at dd6b10896 + ledger, 75974ce87); reviews A and B round 1 pending (snapshot 5fb76795f85b2c0114f83b6245805dab20a57b40:75974ce8725a33caed3f0dc316e755deed529399)
- **Next step**: full gate on M4, then review A (code-review high) and review B (conformance) on range 9b3e348bb..HEAD, then pr-title-m4.md / pr-body-m4.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1559 | design | merged | ed945a97b54efe6e58c68c39a91c29f019c3741d |
| #1567 | M1 | merged | ebd7090c556500a3b530604d4cde73f91b3b7ddc |
| (pending) | M2 + M3 (one PR) | being opened by the orchestrator | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Mouse side buttons walk the history in the active project (other projects' places passed over as gone until M4) | #1567 | merged |
| M2 | T017–T023 | full | Alt+←/→ (Cmd+[ / Cmd+]) walk it unless a field or terminal holds the keyboard; guide table | M2+M3 PR | reviews clean, PR being opened |
| M3 | T024–T026 | light | The Review tab's own Back first | M2+M3 PR | reviews clean (B: 1 MINOR, applied in M4), PR being opened |
| M4 | T027–T033 | full | Back/Forward across projects, skipping their gone places | — | implemented, gate green, reviews pending |
| M5 | T034–T036 | light | The Review sheet lists Back and Forward | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | macOS chord: Option+←/→ or Cmd+[ / Cmd+]? | Cmd+[ / Cmd+] on macOS; Alt+←/→ on Linux/Windows; same focus rule everywhere | agent | terminal-panes.md app-chord table (all macOS forms use Cmd, none Option); keymap.rs `is_primary_shift`; Cmd+Shift+[ / ] already prev/next session |
| 2 | clarify | Only the Review tab's sheet lists Back/Forward? | Yes; app-wide sheet out of scope | agent | spec.md Out of Scope |
| 3 | tasks | Cut of M1 (16 tasks, over ~10)? | Kept whole: Foundational alone is never a milestone and US1 needs all of it; cross-project (R5 lookup, `runs.known`, `Reopened`) split out to M4 (US4), so M1 passes another project's places over as gone until M4 | agent | milestones.md rules 1, 3; tasks.md T010/T012/T028 |
| 4 | tasks | Guide updates per milestone | Each feat milestone adds its own line to terminal-panes.md §Keyboard shortcuts (plan D7: the only page), so the user-guide gate passes without docs-not-needed | agent | milestones.md rule 4; tasks T016, T023, T026, T033, T036 |
| 5 | tasks | speckit-analyze | Not available in this environment (no skill, no subagent); coverage checked by grep: every FR-001–015 and SC-001–005 named by a task or milestone. Tasks review round 1 stands in for analyze (reviewer checked coverage, paths, anchors, test-first order and milestone deps: no gaps) | agent | tasks.md; tasks review r1 F3 |
| 6 | M2 implement | After a key-made move shows a session, its terminal takes the keyboard and swallows the next Alt+← (US2 AS4 broken after one step). Keep U4's click landing for keys? | No: a `Source::Key` move ends with `release_terminal` if it focused one (`finish_history_move(source)`); a side-button move still lands as a click. The user clicks the terminal (or it focuses on the next selection) to type | agent | spec US2 AS4 + FR-012 (each repeat steps until none) vs contracts/ui.md U4 (plan-level); quickstart B5 expects a held key to walk to the oldest entry; tests `three_key_presses_step_back_three_entries`, `a_button_move_lands_on_the_terminal_as_a_click_would`; U4, R8, guide amended |
| 7 | M4 implement | The lookup of another project's worktrees: paths (as T031/T032 said) or `dir_name`s? | `dir_name`s: `SessionLocation::Worktree` holds the worktree's `dir_name` identity, which `reconcile` may disambiguate for included worktrees, so a path's last component is not it. tasks.md T031/T032, data-model.md, contracts/ui.md U2 reworded | agent | crates/micold-core/src/worktree.rs `Worktree::dir_name` doc; cycle-log cycle 16 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 122d576a4ef8abf602bba7274943871a5a80802e:b4c2abed5e49a26d0925bc7011e0fabdc7c4eaa6 | CHANGES: 1 MAJOR, 3 MINOR (all fixed; F2–F4 prose only) |
| Spec review | 2 | 8dd4e55cf8901b03c0b2a3a594c57e3780e9ea71:90b186cd61c01d98ee42519bfcddf69b8bd79acf | CLEAN, 1 MINOR (FR-014/US5: Back/Forward app-wide but only the Review tab's sheet lists them; scoped explicitly, no fix required; noted in Clarifications) |
| Clarify | 1 | 2 questions, both agent-resolved | macOS chord → Cmd+[ / Cmd+]; Review-tab-only sheet confirmed |
| Clarify | 2 | rescan after round 1 | no critical ambiguities |
| Plan review | 1 | 11f3d102ca7aabfb92c1199ff5ca3699e08f8622:0ae4673ae350ac1a5197583750a38820c1241a47 | CHANGES: 1 MAJOR, 2 MINOR (F1 cross-project Compare: `runs.known` per-project group lists, R5 limit narrowed, tests + B10 added; F2 availability refreshed before `exists` (U2 step 0); F3 borrow split via `mem::take` in `resolve`) — all fixed |
| Plan review | 2 | 41ae8cd58f1600b8d9892ebc6d9c08f3694d1578:38cb3341b7aae78134abfabbd81189bd87e758cf | CLEAN, 1 MINOR (data-model runs.rs table: `known[P]` for a removed project unspecified — fixed, prose only: one row saying it is left as is and never read while P is unknown) |
| Tasks review | 1 | 8f4a09d125d94bf6e90b271da7ab675c7845d785:fd6e8bf13a1b2d6f1913c226359527d6733732b2 | CHANGES: 1 MAJOR, 2 MINOR (F1 `history`→`changes` allowlist entry moved from T005 (M1) to T025 (M3), T012 spells `crate::features::navigation::blocked` — dead-entry assert would fail M1; F2 T022/T023 green claims split (docs test greens in T023) — prose only; F3 ledger notes the review stands in for analyze — prose only). No [NEEDS CLARIFICATION] in spec.md confirmed |
| Tasks review | 2 | 5265a22b329483958f659f75aa46b16ae46aeb60:382b7ce61dbd18769aa157d47987bb3797120edb | CLEAN, 1 MINOR (T025 prose named `changes::can_go_back`; fixed, prose only: full `crate::features::changes::can_go_back` path) |
| Review A M1 (code-review high) | 1 | 4742c44666bc7576334aedf07d226cb001a6afdb:4573c0ab874ab208ce66d974a57beb15087369e9 | CLEAN, 2 MINOR (F1 Back with nothing shown skipped the current entry — fixed test-first, `History::step` tries the current entry first, research R3, cycle 9; F2 guide rows' key cells "—" — fixed, cells read "Mouse back/forward button", tasks T016/T017/T023 reworded) |
| Review B M1 (conformance) | 1 | 4742c44666bc7576334aedf07d226cb001a6afdb:4573c0ab874ab208ce66d974a57beb15087369e9 | CHANGES: 1 MAJOR, 1 MINOR (F1 no tdd/cycle-log.md — fixed: red recovered per behaviour by env-gated stubs, cycles 1–8 in tdd/cycle-log.md; F2 = A F2, fixed) |
| Review A M1 (code-review, scoped) | 2 | range 47db7b665..7dcc454a0 (pre-rebase) | CLEAN, 1 MINOR (Forward with nothing shown correctly does not offer the current entry; no fix needed) |
| Review B M1 (conformance, scoped) | 2 | range 47db7b665..7dcc454a0 (pre-rebase) | CLEAN, 1 MINOR (cycle-log red runs for cycles 1–8 not reproducible from committed code since the stubs were not committed; the log states it honestly; no fix needed) |
| Review A M2 (code-review high) + Review B M2 (conformance) | 1 | 637f832282d1952c56e904236b76cf62167ffc9e:ac1817bca4303fa68d3693bbf688cf878c2ae420 | A: CLEAN, no findings; B: CLEAN, no findings (range c359a7871..fa3e7bf39) |
| Review A M3 (code-review high) | 1 | range fa3e7bf39..7e775ab64 | CLEAN, no findings |
| Review B M3 (conformance) | 1 | range fa3e7bf39..7e775ab64 | CLEAN, 1 MINOR (tests/history.rs:637: nothing checks 'list untouched' after BrowseBack is applied and observe_place runs — fixed in M4: the test applies BrowseBack, observes, and asserts entries and position again) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
