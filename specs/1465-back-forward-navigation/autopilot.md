# Autopilot ledger — #1465 back-forward-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1465 (issue #1465: Back/forward navigation with mouse side buttons and Alt+←/→); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1465
- **Worktree branch**: 1465-m1 (M1, stacked on the design commits; design on claude/project-thread-e4cve5)
- **Started**: 2026-10-27
- **Phase**: M1 implemented (T001–T016 ticked); full gate run; review A and B pending (orchestrator)
- **Next step**: review A (code-review high) and review B (conformance) on 23915f816..HEAD; then PR title/body for M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1559 | design | merged | ed945a97b54efe6e58c68c39a91c29f019c3741d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Mouse side buttons walk the history in the active project (other projects' places passed over as gone until M4) | — | in progress |
| M2 | T017–T023 | full | Alt+←/→ (Cmd+[ / Cmd+]) walk it unless a field or terminal holds the keyboard; guide table | — | planned |
| M3 | T024–T026 | light | The Review tab's own Back first | — | planned |
| M4 | T027–T033 | full | Back/Forward across projects, skipping their gone places | — | planned |
| M5 | T034–T036 | light | The Review sheet lists Back and Forward | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | macOS chord: Option+←/→ or Cmd+[ / Cmd+]? | Cmd+[ / Cmd+] on macOS; Alt+←/→ on Linux/Windows; same focus rule everywhere | agent | terminal-panes.md app-chord table (all macOS forms use Cmd, none Option); keymap.rs `is_primary_shift`; Cmd+Shift+[ / ] already prev/next session |
| 2 | clarify | Only the Review tab's sheet lists Back/Forward? | Yes; app-wide sheet out of scope | agent | spec.md Out of Scope |
| 3 | tasks | Cut of M1 (16 tasks, over ~10)? | Kept whole: Foundational alone is never a milestone and US1 needs all of it; cross-project (R5 lookup, `runs.known`, `Reopened`) split out to M4 (US4), so M1 passes another project's places over as gone until M4 | agent | milestones.md rules 1, 3; tasks.md T010/T012/T028 |
| 4 | tasks | Guide updates per milestone | Each feat milestone adds its own line to terminal-panes.md §Keyboard shortcuts (plan D7: the only page), so the user-guide gate passes without docs-not-needed | agent | milestones.md rule 4; tasks T016, T023, T026, T033, T036 |
| 5 | tasks | speckit-analyze | Not available in this environment (no skill, no subagent); coverage checked by grep: every FR-001–015 and SC-001–005 named by a task or milestone. Tasks review round 1 stands in for analyze (reviewer checked coverage, paths, anchors, test-first order and milestone deps: no gaps) | agent | tasks.md; tasks review r1 F3 |

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
| Review A M1 (code-review high) + Review B M1 (conformance) | 1 | 0023312419ed37e71de2f2b81292fc770186f678:02623b87546736987e7fa4ecee318e1c9ddfb070 | pending (dispatched by orchestrator) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
