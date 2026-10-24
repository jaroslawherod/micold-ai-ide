# Autopilot ledger — #1272 review-source-dropdown

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1272 — issue #1272 "Review UI: source dropdown and two-column layout"; labels: enhancement, flow:feature (in-progress added by the orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1272
- **Worktree branch**: claude/project-thread-43e62l
- **Started**: 2026-10-24
- **Phase**: milestone M2
- **Next step**: orchestrator merges #1368, moves the M2 commits (branch m1272-m2 in /home/user/wt-1272-m1) onto the run branch, pushes and opens the M2 PR (title/body in scratchpad m2/; full gate green at 1eaf6de8)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1355 | Design PR: spec, plan, tasks | merged | db1c1bcf |
| #1368 | M1: core `review::source_menu` model | open (CI running) when M2 started | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T008 | full | core `review::source_menu` model + unit tests; client PR section/wording from core | #1368 | done, gate green at 703c8ff5; PR open |
| M2 | T009–T015 | full | reducer: control state, picks via SourceChosen/FilesPaneToggled, chips keep open, chip-while-PR | | done, gate green at 1eaf6de8; PR to open |
| M3 | T016–T018 | full | shared `MenuSelect` + geometry gate + showcase | | pending |
| M4 | T019–T026 | full | header control, emptied left column, 260 px two columns, snapshot, guide, visual pass — **not before #1270 (and #1269 if open) merged** (decision 2) | | blocked on #1270 |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | How does the Files option combine with the shown source (Story 3 s2)? | As today: on/off mode over the shown source, stays on across source picks (tree and browsed file re-read, Back forgotten); label `Files · <source>`; Open diff leaves source and Files unchanged | agent | docs/user-guide/reviewing-changes.md#Browsing the repository; specs/1150-diff-full-file-view/spec.md#Edge Cases; features/changes.rs `reread_after_source_change`; spec FR-006/FR-007/SC-004 |
| 2 | plan | How to sequence against unmerged #1270/#1269? | M1 core model, M2 reducer, M3 shared `MenuSelect` ship first; M4 view wiring (header, left column, 260 px two columns, snapshot, docs, visual pass) waits on #1270 merged | agent | plan.md#Milestones and sequencing; research.md R8 |
| 3 | tasks | Milestones by story (milestones.md rules 1–2) or by layer? | By layer, per plan and decision 2: M1 ships the P1 stories' (US1, US2) core decisions and tests under milestones.md rule 6, M2 their reducer behaviour, both unreachable from the UI until M4; no story's UI can land before #1270, so a per-story cut would put every milestone behind it. T027 (Polish, no code) left to the close unit | agent | plan.md#Milestones and sequencing; tasks.md header note |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 0f203fb2717e1fa99d2ac2b68c94c24f64c916f3:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | 5cb0e7aeb46f21b103a07c493f87bb563ef37b13:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 3 MINOR (fixed, prose only) |
| Clarify | 1 | 1 question, agent-resolved (Story 3 s2) | requirements changed: FR-001, FR-002, FR-007, Story 3 s1–s5 |
| Clarify | 2 | rescan of the clarified spec | no critical ambiguities |
| Plan | 1 | 6f35131b36e2a60c6aab6421cd8d93d2789984e8:1ce7434bb25151d52292a1c7868132c7a0ef2313 | CLEAN: 3 MINOR (fixed, prose only: F1 `source_menu_stops` without a clock, F2 shared `show_working_tree`, F3 *The Default entry* in the M4 docs); dispatched by the orchestrator |
| Tasks | 1 | bd07a428b03bf252be362cfcff0c9d6697890417:454bb4718cbe9cf836b80ff8eae4a20e85a5b483 | CHANGES: 2 MAJOR, 2 MINOR (F1 partly, F2–F4 fixed); speckit-analyze 0 CRITICAL/HIGH, 1 MEDIUM + 9 LOW (C1, C2, C3, I1, I2, U1, D1, T1 fixed; I3, U2 left) |
| Tasks | 2 | 0f7ff1a9986747a18ac55db014ecf7046638b679:454bb4718cbe9cf836b80ff8eae4a20e85a5b483 | CLEAN: 2 MINOR (ledger wording; fixed, prose only) |
| M1 A (code-review high, headless claude -p) | 1 | 11db4a7af4275acae7dd7700532cd0d8a5a89e28:48cb9959b134050aa6cb38dd873c65410e7fb795 | CLEAN: 3 MINOR, all fixed (F1 `List` with no pull request read as `Empty`, tested; F2 `files` prefix built only where used; F3 duplicate assert removed) |
| M1 B (conformance, sonnet, headless) | 1 | 11db4a7af4275acae7dd7700532cd0d8a5a89e28:48cb9959b134050aa6cb38dd873c65410e7fb795 | CLEAN: 1 MINOR (Default-entry label row missing from data-model §6; fixed, prose only). Ran beside A, before the gate; reviewers could not run cargo, Verify output supplied by the implementer |
| M2 A (code-review high, headless claude -p) | 1 | a5076ddfcb5a1f1d65c7ba6c1e637859be9fabe9:1eaf6de89bd3c1bf198ff6a5218cd2413a9c8ec4 | CLEAN: 2 MINOR, both declined (see *Declined review findings*) |
| M2 B (conformance, sonnet, headless) | 1 | a5076ddfcb5a1f1d65c7ba6c1e637859be9fabe9:1eaf6de89bd3c1bf198ff6a5218cd2413a9c8ec4 | CLEAN, no findings; first run returned a bare CLEAN, re-run with a required evidence block (Checked: per rubric item, cited tests and lines). Ran beside A and the gate; reviewers could not run cargo, Verify output supplied by the implementer |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Tasks 1 | F1 re-cut (core + reducer as M1, view wiring before #1270) | View wiring before #1270 contradicts R8 (both rewrite the same header/body rows of `ui/changes.rs`). Taken: Phase 2 relabelled as the P1 stories' core (rule 6), deviation note under *Milestones* citing decision 3 and rules 1, 2, 6, 3; "decision 2" reference fixed. Core + reducer kept apart: 15 tasks (rule 3). |
| — | Analyze | I3 FR-008 "about 260 px" vs exact 260 | the plan fixes the constant at 260.0; the spec's "about" is user-level wording. No change. |
| — | Analyze | U2 split T021 | T021 is one view function's wiring; M4 has 8 tasks. No change. |
| M2 | A | F1 `menu_input` computes `stale` at `now = 0` then clears it | correct and commented (`None` never marks it); a refactor would re-run the full gate for no behaviour change |
| M2 | A | F2 inline `Source` → `Stop` mapping in `SourceMenuToggled` | used once; the other mapping (`menu_input`) builds a `Shown`, not a `Stop` |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Depends on unmerged #1270 (R1, bottom bar + drawer; removes the right column) and possibly #1269 (R0, Review tab). FR-008's two-column result is verifiable only after #1270 merges; plan must sequence or gate on it.
