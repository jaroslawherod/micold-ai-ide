# Autopilot ledger — 031-clickable-terminal-links

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: the links shown at terminal or ai cli session should be real links. User should be able to click them and open webside or other related application
- **Kind**: feature
- **Worktree branch**: feat/links-in-terminal-should-be-clickable
- **Started**: 2026-09-14
- **Phase**: 4-milestones
- **Next step**: M1 (T001–T012): review round 2 A fixed (U152); review B round 2 (interrupted, re-dispatch), gate, then PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #336 | Spec | merged | 411711c1 |
| #341 | Design (clarify, plan, tasks, milestones) | merged | 226d3a8b |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T012 | Link recognition core (`micold_core::link`, SC-002 corpus) | — | in progress |
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
| 14 | 4-milestones | Which suite is "the full suite" inside an M1 cycle, at ~343 s a workspace run? | The core fast subset (`cargo test -p micold-core --all-targets`) per cycle, since M1 touches only `micold-core`; the workspace suite and `mise run gate` at the milestone's end | agent-resolved | .specify/memory/tdd-profile.md fast-subset note |
| 15 | 4-milestones | U25: a wide char's spacer cell holds a space on the wire (alacritty 0.26 writes `' '`; messages.md calls it meaningless), so `LinkRows` as designed cannot tell it from real text and detection would stop at it | Add `LinkRows::spacer(row, col)`, answered by the client from the style-run flags; the logical line skips spacers in its text and covers them with the char before them on their row. Contract §1 and L5, data-model §1 and T028 updated | agent-resolved | alacritty_terminal 0.26 `Term::input`; specs/010 contracts/messages.md §Wide characters; crates/micold-daemon/src/framer.rs:362,368 |
| 16 | 4-milestones | M1 review A: a detected address nested in one whose start lies past the top cut (`?next=https://…`) was offered, since L7 dropped only a candidate at column 0 | Widen L7's upper edge: drop a candidate when only address characters precede it on the logical line (U151); contract L7 updated | agent-resolved | contract L7 "A truncated address is never recognised (FR-003)"; research R4 "safer than recognising a truncation" |
| 17 | 4-milestones | M1 review A round 2: `detect` rescanned the rest of the line for every candidate that failed, 3.4 s on a capped line of `mailto:` repeated. The suggested fix (skip ahead to the failed scan's end) would stop finding an address nested in a rejected one, which the contract keeps | Precompute each start's stop, the next `'`, `@` and `/`, and the trailing punctuation run in one pass, so each candidate costs O(1) plus its authority (U152); checked against the old `detect` on 200,000 random texts | agent-resolved | research R4 "bounds the work on pathological output"; contract §3 nested addresses |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | B | F5 (MINOR): contract §3 rows `http://localhost:5173/`, `mailto:team@example.com,`, `file:///home/u/My%20Doc.pdf`, `team@example.com`, `src/main.rs:42`, `data:text/html,x` have no `detect` unit test of their own | Each is a line of the SC-002 corpus (`tests/fixtures/link_corpus.txt`, section "Contract link-recognition §3") checked cell by cell through `link_at`, which calls `detect`; a second table test would pass on arrival and duplicate it |

## Open escalation

None.

## Follow-ups not done

None.
