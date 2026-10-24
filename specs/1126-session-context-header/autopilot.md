# Autopilot ledger — #1126 session-context-header

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1126 "Layout: worktree context header and session tabs above the terminal" (app redesign issues, started by dependency order); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1126
- **Worktree branch**: claude/project-thread-y28cqn
- **Started**: 2026-10-24
- **Phase**: implement M1
- **Next step**: M1 verify (scoped gate + review A, review B, full gate), then PR (M1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1318 | design (spec, plan, tasks) | merged | f8eeaa43e5f151a08a2c20b055c5c9ce64bf9584 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020 | full | context header above the terminal (title, state, branch, PR chips); bar unchanged | — | in progress |
| M2 | T021–T042 | full | tab strip replaces the bar; chords and focus unchanged (US2 + US4) | — | pending |
| M3 | T043–T055 | full | Changes button with counts opens/closes the Changes view | — | pending |

## Decisions

- Plan (2026-10-24): header facts from the Overview card's own `CardModel` (R2); counts from the Changes view's list reader in the background for the shown location (R5, R6); strip = session tabs + shown session's shells and "+", status/restart, New session, splits at the right; keyed row for presses (R7–R10); header stays above the open Changes view, strip does not (R12). Edits confined to `ui/mod.rs`'s active-session/Changes branch for #1124/#1125 rebases.
- Plan review 1: tab right-click menu stays on the shown session's tab and its shell tabs; another session's tab opens no menu (`TerminalRestartRequested` acts on the shown session). `StripTab` + predicates re-exported from `ui/terminal.rs`; `features/session.rs` imports from `features::session_strip`.
- Clarify round 1 (strip tab contents, US2 sc. 3): option A, session tabs (main, then helpers) with the shown session's tab as its AI conversation, its shell tabs and "+" after it; New session runs **Start another session...** with its confirmation. Orchestrator's provisional pick of the recommended option, user decision pending (decision card). Applied to spec.md Terms, US2 sc. 3/7, FR-008 to FR-010, Key Entities (Tab), SC-004.
- Tasks (2026-10-24): M2 keeps US2 whole with US4 (22 tasks, over the ~10 split guide): no acceptance-scenario split leaves main without a bar control (FR-009: bar removal and strip land together); the behaviour-preserving moves (StripTab and predicates into `features::session_strip`, `cdk::Keyed`) go in Foundational (M1) to shrink M2. Polish T056–T057 is code-free, left to the close unit.
- Clarify round 2: session tab order and press behaviour agent-resolved from worktrees-and-sessions.md; no new requirement, clarify ends.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 0c554b9b6b3c47d8f964a5440a77a10f44444bf6:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CHANGES: 4 MAJOR, 3 MINOR; all fixed |
| Spec | 2 | d6925d167b7f9c841828824458dcbad6a7efa800:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (sonnet); 1 MINOR fixed, prose only |
| Plan | 1 | 4d4fb75608f438dc27c9078b4aa4db62f62e68a1:938d94c5b529d63aa2ac171bcb132cfb2383899a | CHANGES: 3 MAJOR (tab menu not mapped, StripTab importers, containment gate), 3 MINOR; all fixed |
| Plan | 2 | 29583b02c933d279616339822384ea215db890db:3616cce290dbc0e95358c640c3b0c58fabf0feaa | CLEAN (sonnet); 1 MINOR fixed (containment retarget detail) |
| Tasks | 1 | 8ab80bc5203f8fcd8be0c73b112b09dc4539067a:2fa1a219553a964ec33e235d36e1d193a87f7128 | CLEAN; 3 MINOR: checklist ticks + note fixed, provisional clarification noted in follow-ups |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| tasks | speckit-analyze | C1 HIGH: `gates/session_bar_fits.rs` not retargeted though the bar is removed | that gate holds feature 944's transcript session bar (`session-transcript-insights`), not the bottom bar under the terminal (`layout_snapshot.rs:222`) |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Clarification 1 (strip contents, New session) is still the orchestrator's provisional pick; confirm with the user before M2 starts. If it changes, T021–T023, T032, T033, T037 change with it.
