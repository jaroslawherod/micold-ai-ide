# Autopilot ledger — 031-clickable-terminal-links

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: the links shown at terminal or ai cli session should be real links. User should be able to click them and open webside or other related application
- **Kind**: feature
- **Worktree branch**: feat/links-in-terminal-should-be-clickable
- **Started**: 2026-09-14
- **Phase**: 3-design
- **Next step**: merge PR 2 (design), then Phase 4 milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #336 | Spec | merged | 411711c1 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T012 | Link recognition core (`micold_core::link`, SC-002 corpus) | — | pending |
| M2 | T013–T022 | Opening pipeline: `LinkActivated` through `update_inner` to the system opener | — | pending |
| M3 | T082, T083, T037, T023–T034 | Clickable web, mail and declared links in the pane | — | pending |
| M4 | T035, T036, T038–T041 | Session terminal identity and the FORCE_HYPERLINK opt-in | — | pending |
| M5 | T084, T087, T042–T048, T088, T049–T054 | File links on the host: open, reveal runnables, not-found | — | pending |
| M6 | T085, T055–T067, T069, T068 | Sandboxed file links, translated and confirmed | — | pending |
| M7 | T086, T070–T077 | Link context menu | — | pending |
| M8 | T078–T081 | Close: SC-005 measurement, full visual walkthrough | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | 1-spec | Spec review still had a MAJOR after 3 rounds: how to proceed? | Run one more fresh review; open PR 1 if clean | decided by user | round-3 findings fixed in spec.md |
| 2 | 1-spec | Round 4 still had 2 MAJOR (fixed): open PR 1 or review again? | Open PR 1 now | decided by user | round-4 findings fixed in spec.md |
| 3 | 1-spec | `specs/030-*` landed on main from another flow (030-settings-rail-slide) during specify: which number? | Renumbered to 031 | agent-resolved | `git ls-tree origin/main specs/` |
| 4 | 2-clarify | Which gesture opens a link? (FR-004) | Ctrl+click on Linux/Windows, Cmd+click on macOS | decided by user | spec.md Clarifications |
| 5 | 2-clarify | Should sessions advertise hyperlink support? (FR-006) | No; document the FORCE_HYPERLINK opt-in | decided by user | Claude Code 2.1.270 detection reads FORCE_HYPERLINK / TERM_PROGRAM list |
| 6 | 2-clarify | May application-specific schemes open? (FR-011) | Never | decided by user | spec.md Clarifications |
| 7 | 2-clarify | Round 2 scan: further critical ambiguities? | None; hover display, notification and confirmation reuse existing tooltip/notification/confirm components (plan-level) | agent-resolved | crates/micold-client/src/ui/confirm_*.rs, features/notifications.rs |
| 8 | 3-design | Plan review round 3 still had 2 MAJOR (fixed): review again or proceed? | Proceed to speckit-tasks | decided by user | round-3 findings fixed in plan/research/data-model/contracts/quickstart |
| 9 | 3-design | US1 is 34 tasks: one milestone or several? | Split into recognition (M1), opening (M2) and pane (M3); M1–M2 unreachable from the UI until M3. Deliberate departure from rule 3: the smallest scenario half (scenarios 1, 2, 4, 5) still needs nearly all of recognition, opening and the pane, so a scenario split would not shrink the first PR; rule 6 keeps M1–M2 unreachable from the UI. M6 kept whole (translation without confirmation would violate FR-018a) | agent-resolved | .claude/skills/speckit-autopilot/references/milestones.md rules 3 and 6; tasks.md ## Milestones |
| 10 | 3-design | Where do outer-loop acceptance tests run, with no GUI end-to-end runner? | `#[cfg(test)] mod acceptance` in shell/links.rs, driving the headless `TerminalPane` through `update_inner` (integration of composed modules); rendered pixels via visual-pass | agent-resolved | .specify/memory/tdd-profile.md (acceptance runner is sandbox-only); crates/micold-client/tests/terminal_size_reporting.rs headless pattern |
| 11 | 3-design | Tasks review round 1: declared-link hover was built in M3 but tested in M4 (tests-after) | Moved T083 (A7–A11) and T037 (U129) into M3 ahead of T028; M4 keeps session identity and the opt-in | agent-resolved | milestones.md rule 7; constitution I |
| 12 | 3-design | Local baseline is red on 2 `micold-daemon` pi tests (exclusivity, pi_launch_wiring): block the loop? | No; recorded as pre-existing, not fixed here — out of flow, and green in CI on main | agent-resolved | tdd/cycle-log.md Baseline; CI run on b27ffe62 success |
| 13 | 3-design | Tasks review round 3 still had 2 MAJOR (fixed): review again or open PR 2? | Open PR 2 now | decided by user | round-3 findings fixed in tasks.md, tdd/test-list.md, research.md R15 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Open escalation

None.

## Follow-ups not done

None.
