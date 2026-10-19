---
feature: 972-overview-card-context-menu
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 12619e44 # short SHA audited
behaviors: 16 # rows of the table below; no tdd/test-list.md exists, the cycle log's task rows stand in
proven: 3
likely: 0
test_after: 9
no_test: 3
not_applicable: 2
high_smells: 0
high_findings: 5 # 1 weakened existing guard, 3 untested decisions, 1 test-first breach (9 behaviors)
criteria_total: 14 # FR-001..FR-014
criteria_covered: 14 # 10 reach a test through the feature's own entry points; 4 only partly (FR-004, FR-005, FR-010, FR-011)
mutation_score: unmeasured # no mutation tool in the profile; 3 deliberate mutants, 1 caught, 2 survived
mutants_survived: 2 # both untested glue, neither equivalent
suite: scoped only, 1578 passed, 0 failed (client lib 810, bin 588, 7 integration targets 180); full workspace not run, disk quota
---

# TDD Verification: Overview card context menu

**Verdict: FAIL.** An existing guard test was loosened (`overlay_registry.rs:608`), and the
keyboard and dispatch glue that carries FR-004, FR-010 and FR-011 has no test: two deliberate
mutants in it survived. Nine behaviors were written test-and-code in one pass, with the red
obtained afterwards by mutation.

Independence: this audit ran in a session whose scratchpad holds the authoring session's logs, and
an untracked earlier `verification.md` was overwritten. Every cited file was re-read here; the
earlier logs were not used as evidence. Not an independent author, but a cold read.

## Test-first evidence

There is no `tdd/test-list.md` (the cycle log says so: tasks.md's test tasks are the list). The
rubric lists that as `BLOCKED`; the skill allows a weaker audit against `spec.md`, which this is.
The branch was rebased once (`8a95db24`), but the feature commits keep their order.

| Behavior (cycle-log row) | Class | Evidence |
| --- | --- | --- |
| T001/T002 `Icon::Open` | PROVEN | red (E0599) logged; `245a6fea` adds tests and variant together |
| T003 `location_menu_items` extraction | NOT_APPLICABLE | behaviour-preserving refactor, existing tests unchanged |
| T004/T005 reducer and composition | PROVEN | red logged (does not compile); `245a6fea` commits the tests, `69c04c75` the code after |
| T006/T007/T008 displacement, registration, secondary press, clamp | PROVEN | compile-red logged; tests and code land together in `169e7a70` (labelled "tests red, uncompiled") |
| T012/T013 root dispatch of `CardMenuEntryChosen` (`main.rs:755`) | NO_TEST | "glue"; mutant M1 survived |
| T014/T017 targeted outline, no Enter/Space | TEST_AFTER | "written in one pass", red by later mutation (`e882fe40`) |
| T015 contrast | NOT_APPLICABLE | characterization, green first run |
| T016/T018 showcase pose | TEST_AFTER | "no separate red run" |
| T016b `is_targeted` | TEST_AFTER | red by later mutation |
| T018 empty-space dismissal | TEST_AFTER | red by later mutation, after review A found the bug |
| T020/T024 `chord` | TEST_AFTER | "in one pass", red by mutation (`12619e44`) |
| T021/T025 `on_menu_key` | TEST_AFTER | same |
| T022 `MenuOverlay::highlighted` | TEST_AFTER | same |
| T023/T026 step/wrap, `card_menu_entry`, `focus_return_target`, `keyboard_anchor`, `card_id` | TEST_AFTER | same |
| T027/T028 key listener, `card_focus_return`, `focus_landed` | NO_TEST | "glue"; visual pass only; mutant M2 survived |
| `b3887a51` no card menu while an activation waits | TEST_AFTER | test and fix in one commit, no cycle-log row, no red recorded; mutant M3 is caught by it |

The `tasks.md` header promises every test task "seen failing for the right reason before the
implementation task after it". For the nine `TEST_AFTER` rows that is false. The cycle log is
honest about it; the tasks file is not. A red obtained by mutating finished code shows the test can
fail, not that it drove the design.

Tasks against list: T001-T028 are ticked and their behaviors are in the log; no ticked task lacks
evidence beyond the rows above. T029 and T030 are unticked; `visual-pass.md` and `visual/` already
hold the pass T029 asks for, but T029 names a `visual-pass/` directory that does not exist (LOW,
finding 9).

Existing tests changed by the feature:

| File:line | Before / after | Judgement |
| --- | --- | --- |
| `tests/overlay_registry.rs:608` | `the_keyboard_subscription_names_no_surface` gains `&& !m.starts_with("Message::Overview(msg)")` | **Loosened guard, finding 1** |
| `tests/icons.rs` | curated set `65` to `66` | legitimate, new icon |
| `tests/overlay_registration.rs` | guard count `9` to `10` | legitimate, new surface |
| `tests/popover_displacement.rs` | `session_menu` and `session_start_menu` `&[]` to `["overview_card_menu"]`; every pair now starts from `base()` with the Overview open | strengthened, but the premise of all 72 pairs moved; noted, not weakened |
| `tests/showcase_completeness.rs`, `showcase_state.rs`, `test_support.rs` | pose and helper additions | additive |

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | A guard that stops `ui::subscription` naming per-surface messages now exempts every message starting `Message::Overview(msg)`. The justification (comment, lines 596-598) names only `CardMenuMoved` and `CardMenuActivated`, but the prefix exempts any Overview message the subscription ever forwards. Four earlier features added the same kind of exemption, so this is a pattern, but a weakened existing test fails the audit whatever the reason. Fix: match the two messages, or move the key mapping out of the scanned text. | `crates/micold-client/tests/overlay_registry.rs:608`, `src/ui/mod.rs:1278` |
| 2 | HIGH | The root's handling of a card-menu pick is untested. The tests send `CardMenuEntryChosen`, then send the inner message themselves ("`main.rs` sends the inner message next"), so they pass with a root that never dispatches the inner message. That is FR-004 and FR-005 (and US1.4-6) for the real path, and `CardMenuActivated` (`main.rs:744`) as well. Mutant M1 (`update_inner(app, *inner)` replaced by `Task::none()`) left 1578 tests green. | `tests/overview_card_menu.rs:213-284`, `src/main.rs:755-758` |
| 3 | HIGH | `card_menu_key` is a pure function (Down/Up/Enter/Space, no modifiers) with no test. tasks.md says "every decision those chains take is a tested function"; this one is not. Mutant M2 (Down publishes `forward: false`) survived. FR-010. | `src/ui/mod.rs:1257-1279` |
| 4 | HIGH | `card_focus_return` and `focus_landed`, added to fix the focus loss that visual pass B12 found, have no test; the regression fix has no regression test. Only the helpers beneath it (`focus_return_target`, the armed `focus_return`) are tested. FR-011, US3.3. | `src/main.rs:665-681`, `src/ui/overview.rs:158`, `cycle-log.md` last row |
| 5 | HIGH | Nine behaviors are `TEST_AFTER` (table above), against a tasks.md that makes test-first non-negotiable. Test strength for them was shown (mutants fail the tests), ordering was not. | `cycle-log.md` rows T014-T026, `b3887a51` |
| 6 | MED | `the_card_menu_is_open_then_exactly_the_row_menu` and `every_entry_is_wrapped_...` compute the expectation with `location_menu_items`, which `card_menu_items` itself calls, so a row-menu change moves both sides. The spec makes the row menu the single source, and the `seen` check keeps the fixture honest, so this is mild; the row menu's own labels are pinned elsewhere. | `tests/overview_card_menu.rs:337-397` |
| 7 | MED | Eager and rouletted tests: `the_targeted_card_draws_...` mixes an outline, a layer and two inequalities with bare numeric thresholds (`12.0`, `60.0`, pixel `(5, 80)`); `a_secondary_press_publishes_...` asserts three behaviors; `the_menu_key_...` has seven unlabelled tuple asserts. | `src/ui/material/overview_card.rs` (tests added in `e882fe40`, `12619e44`) |
| 8 | LOW | Duplicated setup: the retain/push session dance is copied in two tests, and `project_with`/`add_session` are redefined in `attention_notify.rs`, `attention_reveal.rs`, `overview.rs` and this file (`tests/support/mod.rs` offers neither). `the_menu_stays_when_its_target_changes...` and `only_the_open_menus_target_is_marked...` overlap on the reorder. | `tests/overview_card_menu.rs:459-477, 490-513` |
| 9 | LOW | T029 names `specs/972-.../visual-pass/`; the evidence is in `visual-pass.md` and `visual/`. | `tasks.md:261` |

No HIGH smell from the catalogue (assertion free, tautology, vacuous, doubled subject, conditional
logic, skipped tests): none found. The `for` loops iterate fixed tables and every iteration
asserts. Isolation and determinism are fine (no clock, network or sleep); speed is fine
(`overview_card_menu` 23 tests in under 10 ms, client lib 810 in 5 s).

## Mutation results

No mutation tool in the profile. Three deliberate mutants, one at a time, each restored with
`git checkout` and the touched suites re-run green. Sampled: dispatch, keyboard mapping, and the
activation guard. Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `main.rs:758` `update_inner(app, *inner)` to `Task::none()` | FR-004/005 dispatch | Yes | Real gap, finding 2 |
| `ui/mod.rs:1267` Down key `forward: true` to `false` | FR-010 keys | Yes | Real gap, finding 3 |
| `features/overview.rs:239` drop `CardMenuOpened` from the activation guard | `b3887a51` | No | Caught by `a_pending_activation_opens_no_card_menu` |

Mutants inside a behavior marked done: both survivors sit in rows the log calls "glue", which is
how they escaped.

## Traceability

Entry points: the feature has no automated test through the window's `update`/`view` (they live in
the binary); tests reach the reducer, `app::State::update`, builders mounted in the test renderer,
and the registry.

| Criterion | Tests | Through the real entry |
| --- | --- | --- |
| FR-001, edge: window edge | `overview_card.rs` secondary press, `context_menu_anchor.rs` clamp | Partly: view wiring is visual-pass only |
| FR-002 | `the_card_menu_is_open_then_exactly_the_row_menu` (finding 6) | Yes |
| FR-003 | `open_as_the_inner_message_...`, `every_entry_is_wrapped_...` | Partly: root dispatch untested (finding 2) |
| FR-004, FR-005 | `an_in_place_entry_...`, `a_navigating_entry_...` | **No**, the test plays the root's part (finding 2) |
| FR-006 | `the_targeted_card_draws_...`, `composition_contrast`, `is_targeted` test | Yes |
| FR-007 | `overlay_registry.rs` escape test | Yes |
| FR-008 | `a_right_press_over_a_card_...`, registry tests | Yes (primary-click swallow is the registry's) |
| FR-009 | `on_menu_key` test, `CardMenuRequested` test | Partly: the root's measure-then-open chain untested |
| FR-010 | `up_and_down_step_...`, `highlighted` menu test | **No** for the key mapping (finding 3) |
| FR-011 | `focus_return_target`, dismissal/choice arming tests | **No** for the drain (finding 4) |
| FR-012 | settle tests, grid-scroll test, registry scroll test | Yes |
| FR-013 | `a_targeted_card_does_not_publish...`, `a_card_drops_its_keys...` | Yes |
| FR-014 | `popover_displacement.rs` | Yes |

Untested criteria: none outright; four only partly (FR-004, FR-005, FR-010, FR-011). Tests tracing
to nothing: none. SC-003 (100 ms) and cross-platform edge cases have no test, by design (tasks.md).

## What was not audited

- The full workspace suite was not run to completion: the disk quota (about 40 GB) filled twice
  during the build, and `target-shared/debug` had to be cleared (`mise` is not installed here, so
  `mise run sweep` was not available). The scoped run covered the client lib and bin plus the
  feature's integration targets: 1578 passed, 0 failed. Baseline for the rest is the profile's
  5071 green at `cdc473ab`.
- Whole-feature mutation: no tool; 3 deliberate mutants only. The reducer (`features/overview.rs`),
  `settle_card_menu` and `ui/material/menu.rs` highlight drawing were not mutated.
- Coverage: no tool in the profile.
- `scripts/tests/*.test.sh`, clippy and fmt (the gate): not run.
- The visual-pass evidence (`visual-pass.md`, `visual/*.png`): not re-run or judged.
- Sandbox acceptance tests: not relevant to this feature.
- The `docs/user-guide/overview.md` changes: not graded.

## Close unit follow-up

Findings 1-4 were fixed in the close unit with tests only (no production change); the mutants M1
(root dispatch) and M2 (Down key) now fail a test, and removing the `focus_landed` clear fails the
new focus-return test. Rows are in `cycle-log.md`. Finding 9 is fixed (T029 names `visual-pass.md`
and `visual/`). Accepted, not fixed: finding 5 (nine behaviours were written test-and-code in one
pass; the cycle log records the red obtained by mutation, and history cannot be redone), findings
6-8 (MED/LOW, taste: expectation shares `location_menu_items` by design, eager asserts, setup copies).
