---
feature: 813-worktree-overview
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 511194cd
behaviors: 32 # test tasks of tasks.md; no tdd/test-list.md exists (cycle-log.md says so)
proven: 16
likely: 8
test_after: 8
no_test: 0 # plus one sub-behavior with no test: catalog `session_summary` gives None (M6 T063)
high_smells: 3
criteria_total: 24 # acceptance scenarios US1 x6, US2 x5, US3 x5, US4 x5, US5 x3
criteria_covered: 21 # 3 partial: US1.6, US3.2 (start-menu glue), US4.5
mutation_score: unmeasured # no tool in profile; deliberate mutants NOT run, see below
mutants_survived: 1 # recorded in cycle-log (mutation 5), not re-run by this audit
suite: not completed by this audit; profile baseline 5071 passed, 0 failed, 9 ignored (343 s) at cdc473ab
---

> Close update (2026-10-18): T072–T077 and T079–T082 cleared the three HIGH smells, the loosened gate and B1–B5 (S7 skipped). T078 (mutation re-runs) and a fresh `tdd-verify` were not done, so the FAIL verdict below is the audit as written, not re-measured.


# TDD Verification: Worktree Overview (813)

**Verdict: FAIL.** Eight behaviors are `TEST_AFTER` (the cycle log itself records "no recorded red"), an existing
gate test was loosened (`overlay_registry.rs`), and three tests are vacuous or known to survive a mutant.

Audit independence: the auditing session did not write these tests. It could not build: the shared target dir was
held by another worker's cold build (`Blocking waiting for file lock on build directory`), so the suite run and
every deliberate mutant are **unmeasured**, not passing. All findings below are from reading files and git history.

## Test-first evidence

No `tdd/test-list.md` was planned (rubric says `BLOCKED` for that; audited against `tasks.md` test tasks and
`spec.md` instead, per the skill). History is one commit per milestone with tests and source together
(`3bcd4698`, `e50368f2`, `4c20af1a`, `86bcc97e`, `d93c9dbc`, `ec762865`, `616baee6`, `cb12f89d`), so order is
only as good as the cycle log. The log and history do not contradict each other.

| Class | Behaviors (test tasks) | Evidence |
| --- | --- | --- |
| PROVEN (16) | T002, T006, T007, T008, T010, T012, T014, T024, T025, T026, T040, T041, T042, T043, T050, T051 | red output recorded (`todo!()` stub red, assertion text, `unresolved import`) and test lands in the same commit as source. T006-T008 stub red, not assertion red; the log admits the implementation was drafted first |
| LIKELY (8) | T031, T033, T034, T052, T053, T061, T062, T064 | compile-red only, "observed by the implementing workers", no output; T052/T053 "no assertion-level red" |
| TEST_AFTER (8) | T004, T009 (icons, prompt caller: red only by mutation after the fact), T011, T013 (card and contrast "written together with their implementation"), T032 (pane chord: "not observed separately"), T056 (watch targets, retry test: "no recorded red"), T057 (`issues.rs` log-line), T063 (daemon hook tests "written against the finished code") | cycle-log.md:55, 66-67, 76, 143, 179, 185 |
| NO_TEST sub-item | `catalog.rs` `session_summary` gives `None` (T063) | cycle-log.md:186 "Known gap"; another worker was adding it during this audit |

Existing tests changed by the feature (all 813 commits diffed):

| File:line | Before / after | Judgment |
| --- | --- | --- |
| `crates/micold-client/tests/overlay_registry.rs:~573` (d93c9dbc) | the named-message scan exempted `EscapePressed`, `FocusMoved`; now also `!m.starts_with("Message::Overview(")` | **Loosened invariant**; any future `Message::Overview(..)` in `ui::subscription` passes. FAIL condition per rubric whatever the reason |
| `crates/micold-client/tests/gates/sibling_parity.rs:58-90` (d93c9dbc) | `glyph_of` was "deepest, leftmost"; now "leftmost leaf, deepest on x ties" | helper semantics rewritten so the gate measures the Overview button; justified in the comment but it changes what every sibling-parity case measures |
| `crates/micold-client/tests/fixtures/layout_snapshot.txt` (a58e27f2, d93c9dbc) | 714+/459- regenerated wholesale, commit body empty | self-approving snapshot risk; no record the diff was reviewed beyond the added button |
| `schema_hash.rs` 49 to 50, `root_vocabulary_is_cross_cutting.rs` 22 to 23, `icons.rs` 55 to 65 | pinned counts raised | legitimate, each with a doc line |
| `pull_request_*` tests, gh fixtures | `Result<BTreeMap..>` to `StatusReading` shape, `oc/mc` aliases | API change, assertions kept |

## Findings

### Test-strength findings

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| S1 | HIGH | `hook_activity_is_runtime_only` asserts only that a freshly constructed session has `hook_activity == None`. It never serialises a session carrying activity, so "never persisted" (T064) is unpinned. Should round-trip a `Session` with `Some(..)` through the store format and assert the field is gone | `crates/micold-client/tests/overview.rs:1168-1180`; field doc `crates/micold-core/src/session.rs:334` |
| S2 | HIGH | `thirty_cards_lay_out_bounded_in_a_short_window` survives removing the grid's own scroll (cycle-log mutation 5): it wraps the grid in its own `container(Fill)` and never calls `view`. A DONE behavior (review F1) with a surviving mutant | `crates/micold-client/src/ui/overview.rs:~262-303`; `cycle-log.md:104-109` ("left open") |
| S3 | HIGH | `a_card_shows_its_type_and_state_with_their_words` claims five states are told apart by icon but checks `Vec::dedup()` (adjacent only). Mapping `Done` to `CardNeedsAction` leaves `[NA,Att,Run,NA,Idle]`, length 5, so it passes. Not run (no build); derived by reading. Should collect into a set | `crates/micold-client/src/ui/material/overview_card.rs:~431-440`; mapping at `:54-62` |
| S4 | MED | `card_state_follows_the_table...` checks the code against `table()`, a second implementation of the same ordered rules in the test. Both can be wrong together; only 3 other tests pin literal rows. Add literal (signals, state, reason) rows from `data-model.md` | `crates/micold-core/src/overview.rs:619-704` |
| S5 | MED | Redundant: `features_overview.rs` repeats `toggled_opens_then_closes_and_closed_closes` and `toggled_while_settings_is_open_changes_nothing` from `tests/overview.rs` at the same level (same bug fails both) | `crates/micold-client/tests/features_overview.rs:11,24` vs `tests/overview.rs:118,137` |
| S6 | MED | Real clock: SC-007 asserted as `elapsed() < 1 s` | `crates/micold-client/tests/overview.rs:417-419` |
| S7 | MED | Disconnect test sends `PrMsg::Released` by hand and comments "what `on_disconnected` does"; the real `on_disconnected` is never run, so R14 holds only if the comment stays true | `crates/micold-client/tests/overview.rs:377-378` |
| S8 | MED | `the_overview_key_sits_between_settings_and_changes` pins the literal `u64::MAX - 3`, and its doc says "outranks Changes and Compare" but never compares against them | `crates/micold-client/src/ui/overview.rs:~246-258` |
| S9 | MED | Eager: one daemon test covers capture, isolation, broadcast count and the log check; the log negative has no positive control showing the capture would see a log line | `crates/micold-daemon/tests/hooks_receiver.rs` (`hook_activity_is_captured_per_session_broadcast_and_never_logged`) |
| S10 | MED | Self-approving snapshot: layout snapshot regenerated wholesale | `crates/micold-client/tests/fixtures/layout_snapshot.txt` (a58e27f2) |
| S11 | LOW | Magic `60.0` ring threshold and `0.01` tolerances without names | `overview_card.rs` focus-ring test |
| S12 | LOW | Same 12-field `CardModel` literal built three times (use `model()` once, shared) | `card_grid.rs:~101`, `ui/overview.rs:~262`, `overview_card.rs:~300` |
| S13 | LOW | Unclear names: `bounds_apply`, `ledger_vocabulary`, `current_precedence`, `checkbox_variants_count` | `crates/micold-core/src/hook_activity.rs`, `task_progress.rs` tests |
| S14 | LOW | macOS chord branches are `cfg`-gated; on Linux the `Cmd+Shift+O` path never runs here | `crates/micold-client/tests/keymap.rs` overview tests |

Suite properties: unit and client tests are in-memory and deterministic apart from S6; the watcher retry test uses a
real temp dir and a real `notify` watcher (acceptable). Speed and isolation not measured (no run).

### Behaviour gaps (spec vs tests)

| # | Sev | Gap | Evidence |
| --- | --- | --- | --- |
| B1 | HIGH | FR-008 / US4.5 / SC-004: no test that a task-list change on disk reaches the card within 5 s; `overview_watch` has only its target list test, and the quickstart pass that would cover it did not run | `shell/overview_watch.rs:90`; `visual-pass-m5.md` ("NOT RUN") |
| B2 | MED | US3.2 / FR-013: the `OfferStart` chain (reveal, focus start control, bounds, `StartMenuAnchored`, `StartMenuOpened`) lives in `main.rs` and has no test; only `follow_up`'s value is asserted | `cycle-log.md:168`; `tests/overview.rs:690-717` |
| B3 | MED | Catalog `session_summary` giving `None` has no direct test (T063 names it) | `crates/micold-daemon/src/catalog.rs`; `cycle-log.md:186` |
| B4 | MED | Edge case "disconnected: cards show data may be stale" has no test (only that states stay) | spec.md edge cases; `tests/overview.rs:355` |
| B5 | MED | US1.6 / FR-008: no test drives a change (session starts, PR reading arrives) through `update` and re-reads `cards`; cards are derived per view so this is by construction, but SC-004 is asserted nowhere | `tests/overview.rs` |
| B6 | LOW | T070/T071 (quickstart B, macOS/Windows) not run; M5 visual pass blocked (no xdotool/Vulkan) so FR-016 progress bar, FR-017 empty state and ProgressBar have no visual evidence | `tasks.md:458-460`, `visual-pass-m5.md` |
| B7 | LOW | No `sandbox_real_*` acceptance test for the hook route: the HTTP hook receiver is real in `hooks_receiver.rs`, the window half is not | profile `acceptance` |

## Mutation results

Not run. No mutation tool in the profile, and the deliberate-mutant step needs a build that this audit could not get
(shared target locked by another worker's cold dependency build, no edits allowed to the shared tree). Nothing is
claimed. The only mutant evidence is the feature's own after-the-fact log (`cycle-log.md:88-120`, 8 mutants, 7 caught,
1 survivor = S2). Mutants queued for the next run, all cheap `-p micold-core` or `--lib` targets:

| Mutant | Behavior | Expect |
| --- | --- | --- |
| `overview.rs:403` `order` sort by reversed key tie, or `sort_unstable_by_key` | T008 FR-007 | caught by `order_puts_the_worst_state_first...` only if ties are reordered by the unstable sort; may survive |
| `overview.rs:419` `min()` to `max()` in `attention` | T034 | caught |
| `hook_activity.rs:14` `MAX_TODOS` 50 to 51 | T061 | caught by `bounds_apply` |
| `pull_request.rs:253` drop the `is_cross_repository` filter | T024 | caught by the forks test |
| `overview_card.rs:57` `NeedsAttention` icon to `CardNeedsAction` | T011 | caught (adjacent) |
| `overview_card.rs:59` `Done` icon to `CardNeedsAction` | T011 SC-005 | predicted to survive (S3) |

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1, 1.2, 1.3, 1.4, 1.5 | core `overview` tests, `tests/overview.rs`, `pull_request_counts.rs`, card/grid geometry | Yes (through `State::update` and `cards`) |
| US1.6 | none driving a change | Partial (B5) |
| US2.1-2.2, 2.4 | `tests/overview.rs` toggle/badge, `toolbar.rs` | Yes |
| US2.3 | `keymap.rs`, `terminal_pane.rs` chord capture | Yes (unit level on Linux; macOS gated) |
| US2.5 | doc scripts (`documentation-set`, `check-user-guide-updated`) | Yes |
| US3.1, 3.3, 3.4, 3.5 | `tests/overview.rs` activation/escape/arrows, `overview_card.rs` keyboard | Yes |
| US3.2 | `follow_up` value only | Partial (B2) |
| US4.1-4.4 | `task_progress.rs`, `tests/overview.rs` | Yes |
| US4.5 | watch targets only | Partial (B1) |
| US5.1-5.3 | `hook_activity.rs`, `hooks_receiver.rs`, `tests/overview.rs` | Yes (S1 for non-persistence) |

Untested criteria: none whole; three partial. Tests tracing to nothing: none found (`features_overview.rs` is redundant, S5).

## What was not audited

- The suite was not run and no mutant was run: build blocked behind another worker's build in the shared target dir; no
  edits were made to source or tests. Verdict stands on reading alone; a clean run would not lift it (S1-S3, TEST_AFTER, loosened gate).
- Coverage: none configured. Performance, flakiness, isolation under parallel run: not assessed.
- Daemon crates beyond the hook tests, `micold-core` tokens and `store.rs` changes belonging to other features on this branch
  (803, 946, 686, 806) were excluded; only 813 commits `3bcd4698..511194cd` were diffed.
- `sandbox_real_*` acceptance tests and the macOS/Windows jobs: not run here.
- Docs, showcase output and quickstart visuals: no evidence beyond the files.
