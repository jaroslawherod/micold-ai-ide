# Autopilot ledger — 031-clickable-terminal-links

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: the links shown at terminal or ai cli session should be real links. User should be able to click them and open webside or other related application
- **Kind**: feature
- **Worktree branch**: feat/links-in-terminal-should-be-clickable
- **Started**: 2026-09-14
- **Phase**: 4-milestones
- **Next step**: M3 step 3: gate, cross-OS checks and visual pass on T082–T033's commits, then reviews A and B

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #336 | Spec | merged | 411711c1 |
| #341 | Design (clarify, plan, tasks, milestones) | merged | 226d3a8b |
| #356 | M1 Link recognition core | merged | 33f6491c |
| #361 | M2 Opening pipeline | merged | 9995dbdb |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T012 | Link recognition core (`micold_core::link`, SC-002 corpus) | #356 | merged |
| M2 | T013–T022 | Opening pipeline: `LinkActivated` through `update_inner` to the system opener | #361 | merged |
| M3 | T082, T083, T037, T023–T034 | Clickable web, mail and declared links in the pane | — | in progress |
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
| 18 | 4-milestones | M1 review A round 3: alacritty writes the end-of-row padding before a wrapping wide char with the cursor template's hyperlink, so hovering it built a one-cell declared link over the plain char before it | `link_at` reads the hyperlink of the lead cell of the char under the pointer (U153); contract L5 says a spacer's own `hyperlink` is not read | agent-resolved | contract L5 "its spacer cell belongs to the same link as its lead cell" |
| 19 | 4-milestones | T021 has `shell/links.rs` run the session reducer itself, but `tests/feature_registration_cost.rs::only_the_root_drives_a_feature` forbids any caller of a feature reducer outside `app.rs` | Add `State::update_session_for_effects` in `app.rs`: it runs the reducer, drains every outcome but `ClipboardWrite` and `OpenLink`, and returns those two for `shell/links.rs` to perform | agent-resolved | tests/feature_registration_cost.rs SC-002, FR-002 |
| 20 | 4-milestones | The user asked that the pointer change only when a link is clickable: on plain hover or only with the link modifier held? | Only while Ctrl (Cmd on macOS) is held; underline and hint still show on plain hover. Spec clarification 2026-09-16; FR-007, US1 scenario 1, research R7, data-model T1, T025, T030, U123, quickstart B.1 updated before M3 | user | user, 2026-09-16: "only while Ctrl is held" |
| 21 | 4-milestones | The hover key in data-model §3 has no scroll position: scrolling the view under a resting pointer would reuse a link from other rows | Add `display_offset` to `HoverKey`/`HoverCache`, so scrolling re-resolves | agent-resolved | terminal_pane.rs `hover_refresh`; U156 `rows_follow_the_scrollback_offset` |
| 22 | 4-milestones | The acceptance tests cannot reach `ui::material` (`pub(crate)`) from the bin crate | Build the pane through the public `ui::terminal::pane`, the same element the app renders | agent-resolved | shell/links.rs `mod acceptance` |
| 23 | 4-milestones | Hint colours | `surface_container_highest` container, `on_surface` text, as `TermPalette::hint_container`/`hint_content` | agent-resolved | ui/terminal.rs |
| 24 | 4-milestones | The hover change needs a repaint, but `idle_requests_no_frames.rs` allows one `request_redraw` in the rendering layer | Edge-triggered `shell.invalidate_widgets()`, as `select.rs` does | agent-resolved | gate failure recorded in tdd/cycle-log.md "Gate fixes: M3" |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | B | F1 (MINOR): `link_open_finished`'s `NotFound` text ships untested | The arm exists because the match is exhaustive; its behaviour is U77, assigned to M5, whose cycle must show its own red |
| M2 | B | F3 (MINOR): U94's `< 250 ms` bound would pass a shorter debounce | `perform` is `Task::perform` over `spawn_blocking` with no timer in the code; the bound guards against a real delay, and a tighter wall-clock bound would flake on a loaded CI runner |
| M1 | B | F5 (MINOR): contract §3 rows `http://localhost:5173/`, `mailto:team@example.com,`, `file:///home/u/My%20Doc.pdf`, `team@example.com`, `src/main.rs:42`, `data:text/html,x` have no `detect` unit test of their own | Each is a line of the SC-002 corpus (`tests/fixtures/link_corpus.txt`, section "Contract link-recognition §3") checked cell by cell through `link_at`, which calls `detect`; a second table test would pass on arrival and duplicate it |

## Open escalation

None. (M2's block on the Windows install smoke was resolved by #358; see Decisions 20 and git history of this file.)

## Follow-ups not done

- Scrollback lines fetched by `apply_scrollback` do not bump the grid `seq`, so a resting pointer over just-fetched lines refreshes its hover only on the next pointer move or modifier change (M3, minor).
