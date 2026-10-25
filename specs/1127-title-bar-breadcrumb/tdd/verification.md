---
feature: 1127-title-bar-breadcrumb
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 13beef6d # short SHA audited (M4's head, on M3's head; M1 #1376 and M2 #1409 merged)
behaviors: 16 # rows of the table below; no tdd/test-list.md exists, tasks.md's test tasks stand in
proven: 0
likely: 0
test_after: 14
no_test: 0
not_applicable: 2
high_smells: 1 # finding 2, fixed in the close PR
high_findings: 2 # finding 1, the test-first breach (14 behaviors), and finding 2
criteria_total: 25 # FR-001..FR-019, SC-001..SC-006
criteria_covered: 25 # 22 by tests; FR-012 by the layout snapshot's menu state; FR-018 docs; SC-002 by existing chord tests
mutation_score: unmeasured # no mutation tool in the profile; 6 deliberate mutants, 6 caught
mutants_survived: 0
suite: full workspace through scripts/gate.sh on the close PR's code (1d992f44), green — 11137 passed, 0 failed, 18 ignored in 662 test binaries; scripts/tests green
---

# TDD Verification: One 44 px title bar with a breadcrumb and a jump-to field

**Verdict: FAIL.** No behavior has a recorded red: there is no `tdd/cycle-log.md`, and each
milestone's feature commit adds its tests and its code together, so by the rubric all fourteen
behaviors are `TEST_AFTER`. Test strength is otherwise good: all six deliberate mutants on the highest-risk behaviors were caught, and the one `HIGH`
smell (a test that built four bars and asserted nothing about them) was fixed in the close PR with
four `MEDIUM`/`LOW` ones and a test doc.

Independence: run inline by the close unit, which did not write the tests (M1–M4 were other units);
the smell pass ran in a fresh-context reviewer (`claude -p --agent autopilot-reviewer`), and every
line it cited was re-read here before it went into this report.

## Test-first evidence

There is no `tdd/test-list.md` and no `tdd/cycle-log.md` (the rubric's `BLOCKED` condition; the
skill allows a weaker audit against `spec.md`, which this is). tasks.md words each test task "write
failing tests", but the ledger keeps no red command or output. The history is rebase-merged per
milestone: one feature commit with tests and source together, then review and gate fixes.

| Behavior (tasks) | Class | Evidence |
| --- | --- | --- |
| T001/T002 `anatomy::app_bar` figures (core) | TEST_AFTER | `c5add84d` changes test and figures together |
| T003 `plan()` and its tests move to `give_way.rs` | NOT_APPLICABLE | behavior-preserving move; the three status-bar plan tests kept verbatim (`give_way.rs:698`, `:729`, `:752`) |
| T004/T005 `IconButton::selected`, `keyboard`, `target` | TEST_AFTER | `c5add84d` |
| T006/T007 `AppMark` (tooltip, no press, no Tab stop) | TEST_AFTER | `c5add84d` |
| T008/T009 `Toolbar` 43 + 1, order, give-way lane | TEST_AFTER | `c5add84d`; `f8f744cf` (lay out once) with review A |
| T010/T017 `settings_press`, bar `view()` | TEST_AFTER | `c5add84d` |
| T011/T018/T026 switcher hangs at `BOTTOM_EDGE`, keyboard open at the segment | TEST_AFTER | `c5add84d`, `d696b813`; `3b54759c`/`a5d460af` (`Anchor::Below`) fix and test together |
| T012/T039 `title_bar_keyboard` Tab order, Enter, mark inert | TEST_AFTER | `c5add84d`, `fc06310d` |
| T013/T032/T044/T049 `title_bar_fits` gate (640/1280, both schemes) | TEST_AFTER | `c5add84d`, `d696b813`, `fc06310d`, `12aa9a11` |
| T014/T015 covered states, `sibling_parity` | TEST_AFTER | `c5add84d`, `d696b813` |
| T023/T028 `Button::trailing`, `Button::tooltip` | TEST_AFTER | `d696b813` |
| T024/T029 `Breadcrumb` (separator, shortening, tooltip on cut) | TEST_AFTER | `d696b813`, `eca00243` |
| T025/T030 `model()`: project, place, unread, switcher open | TEST_AFTER | `d696b813` |
| T027/T040 contrast pairs (breadcrumb, field) | NOT_APPLICABLE | characterization of theme tokens, green on first run by nature |
| T034–T038/T041–T043 `FieldButton`, `Toolbar::centre`, `jump_press`, opens the switcher focused, closes it | TEST_AFTER | `fc06310d`, `9d18157e` (fonts loaded before measuring) |
| T046/T047 no Overview control in the bar; the chord opens and closes it | TEST_AFTER | `12aa9a11`; `f2f681b0` drives the chord through the root reducer |

Tasks against list: T001–T050 are ticked and each names tests that exist and pass; no ticked task
lacks a test. T051 and the remediation tasks are this close unit's.

Existing tests changed by the feature (every removed or rewritten assertion in the feature's commits):

| Where | Before → after | Judgment |
| --- | --- | --- |
| `micold-core/tests/tokens_anatomy.rs` | `app_bar` 64 / 16 / 48 → 43 / 12 / 40, plus `BOTTOM_EDGE`, `GAP`, `MARK`, field figures | the spec's change (FR-001, R1), not a loosening |
| `client/src/ui/material/anatomy_size.rs`, `content_placement.rs` | the 64 dp bar and centred-title tests removed | the title they pinned no longer exists (FR-002); replaced by `Toolbar` tests and `title_bar_fits` |
| `client/tests/layout_snapshot.rs` | `the_toolbar_title_anchor_measures_the_application_name` removed | same: the bar carries no title |
| `client/tests/switcher_anchor.rs` | reported `(300, 64)` → panel at `y` 64 became: reported `y` 42 or 64 → panel at `BOTTOM_EDGE` | stricter (two inputs, one fixed edge, FR-005) |
| `material/src/status_bar.rs` | three `plan()` tests | moved verbatim to `give_way.rs` (T003) |
| `client/tests/known_projects_overflow.rs` | the panel read at the trailing edge → at the segment's edge | follows R8; same assertions |
| `client/tests/session_start_press.rs` | "exactly one chevron on screen" → "exactly one below the title bar" | narrowed so the segment's own chevron is not counted; the assertion is still exact below the bar |
| `client/tests/session_area_stability.rs` (`ab2a2efb`) | press-in-flight helper now also ignores `SessionMsg::TerminalResized` | **a loosening** (finding 3): the 21 dp taller body puts the terminal on a row boundary, so the header's 0.4 dp chip-row difference re-measures it; the ledger's follow-up asks for one chips-row height |
| `client/src/ui/toolbar.rs` | the Overview-button and badge tests removed | FR-016: the button is gone; replaced by finding 2's test and `title_bar_keyboard::no_bar_control_publishes_an_overview_message` |
| `client/src/ui/app_chord_tests.rs` | the chord test drove `overview::update` → drives `State::update` | stricter: the root reducer is the real entry point |

## Findings

1. **HIGH — test-first breach, 14 behaviors `TEST_AFTER`.** No red kept anywhere; history shows
   tests and code in the same commits. History only: nothing to fix now.
2. **HIGH — assertion-free loop, fixed.** `client/src/ui/toolbar.rs`
   `the_bar_has_no_overview_button_in_any_body_branch` built `view()` for four states into `_bar`
   and asserted nothing about them; only an `include_str!` source scan asserted. The behavior is
   asserted through the built bar by `title_bar_keyboard::no_bar_control_publishes_an_overview_message`.
   Fix (T052): the dead loop is gone and the test is renamed
   `the_bars_source_builds_no_overview_control`, the check it makes.
3. **MEDIUM — existing test loosened** (`session_area_stability.rs:471`, see the table). Kept: the
   resize is real and not a press; the cause is a ledger follow-up (one chips-row height).
4. **MEDIUM — re-implemented expectation, fixed.** `ui/toolbar.rs:445` compared `jump_press` with
   a message built from the same `state.terminal_focused()` the code reads. Fix (T053): the session
   case asserts the literal `terminal_had_focus: true`.
5. **MEDIUM — vacuous loop, fixed.** `material/src/field_button.rs:379`
   `without_a_press_it_is_disabled_and_no_tab_stop` checked the disabled opacity inside a loop over
   painted text with nothing making the list non-empty. Fix (T054): asserts the field paints words.
6. **MEDIUM — silent skip, fixed.** `client/tests/gates/sibling_parity.rs:150` `continue`d past a
   breadcrumb with no project segment. Fix (T055): that is now a reported failure.
7. **LOW — vacuous when empty, fixed.** `client/tests/title_bar_keyboard.rs:224` pressed every
   Tab stop without checking there were any. Fix (T056): asserts the four stops first.
8. **LOW — wrong test doc, fixed (prose).** `material/src/breadcrumb.rs:396` said "built in both
   schemes without panicking"; it checks `host()`'s content colour, and now says so.
9. **MEDIUM — eager gate, not fixed.** `gates/title_bar_fits.rs:30` checks fit, centring, unread,
   shortening order, separator, glyphs and Overview absence in one sweep of 2 widths × 2 schemes,
   and `segment()` accepts any prefix of the name. By design (one layout pass per fullest state,
   like the repo's other `gates/`); a failure message names the check. Left as is.
10. **MEDIUM — re-derived stage, not fixed.** `material/src/toolbar.rs:347`
    `the_bar_gives_way_in_the_five_step_order` reads each width's stage back from painted text
    with its own ladder over ~750 widths. The planner's literal-vector tests (`give_way.rs`) pin the
    rule independently. Left as is.
11. **LOW — not fixed.** `tokens_anatomy.rs:57` pins `anatomy::ALL.len()` (the repo's convention
    for that list; the figures' values are asserted by name above it). Hand-built press/release
    pairs in `breadcrumb.rs` and `field_button.rs` where `Mounted::press` covers half (style).

Isolation: no shared state found. Determinism: `main_tests.rs`
`the_jump_field_opens_the_quick_switcher_with_its_input_focused` runs a real tokio runtime with a
10 s timeout (not flaky in any run here). Speed: finding 10.

## Test strength

No mutation tool (profile `mutation: null`). Deliberate mutants, one at a time, each restored with
`git checkout` and the suite re-run green afterwards:

| # | Mutant | File | Caught by |
| --- | --- | --- | --- |
| 1 | `app_bar::HEIGHT` 43 → 44 | `micold-core/src/tokens/anatomy.rs` | `tokens_anatomy::the_app_bar_is_the_small_variant` |
| 2 | drops go highest rank first (`a.0.cmp(&b.0)` → `b.0.cmp(&a.0)`) | `micold-material/src/give_way.rs` | `give_way::tests::drops_go_by_rank_before_any_shortening` |
| 3 | of two hints at one rank the leftmost drops first | `micold-material/src/give_way.rs` | `give_way::tests::of_two_hints_the_rightmost_drops_first` |
| 4 | the field's Settings guard dropped (`true.then(…)`) | `micold-client/src/ui/toolbar.rs` | `ui::toolbar::tests::the_jump_field_sends_the_chords_message_except_in_settings` |
| 5 | the place reads the full branch, not the short one | `micold-client/src/ui/toolbar.rs` | `a_session_on_a_branch_reads_its_short_branch`, `changes_reads_its_entry_and_compare_keeps_the_session` |
| 6 | the panel hangs from `app_bar::HEIGHT`, not `BOTTOM_EDGE` | `micold-client/src/ui/mod.rs` | `switcher_anchor::the_panel_opens_where_the_trigger_reported`, `opened_from_the_keyboard_the_panel_hangs_from_the_segments_edge` |

After the last mutant the tree had only the close PR's test edits, and the sampled tests ran green
(core `tokens_anatomy`, material lib 400, client `ui::toolbar` 11, `switcher_anchor` 3).

Sampled: the bar's height, the give-way order (both tie-breaks), the Settings guard on the field,
the place segment's short-branch rule, and the panel's anchor edge. Not sampled: centring
(`Toolbar::centre`), `FieldButton`'s fit choice, `AppMark`, the breadcrumb's cut tooltip.

## Traceability

| Criterion | Tests (real entry point marked ★) |
| --- | --- |
| FR-001, SC-001 | `tokens_anatomy::the_app_bar_is_the_small_variant`; `material::toolbar` tests; ★ layout snapshot `title-bar-*` states |
| FR-002, B1–B3 | `material::toolbar` order and centre tests; ★ `title_bar_fits` |
| FR-003, C1 | `material::app_mark` tests; ★ `title_bar_keyboard::the_app_mark_publishes_nothing_when_pressed` |
| FR-004 | `ui::toolbar::tests::the_project_segment_reads_the_project_or_select_project`; ★ `title-bar-no-project` |
| FR-005, SC-004 | ★ `switcher_anchor` (3 tests); ★ `gates/panel_placement` over `title-bar-switcher-open` |
| FR-006, SC-005 | `ui::toolbar::tests` (short branch, display name, Overview/Settings, Changes/Compare, removed location); core `the_branch_chip_is_short_with_the_full_branch_on_hover` |
| FR-007 | `material::breadcrumb` shortening and tooltip tests; ★ `title_bar_fits` (60-character branch) |
| FR-008 | `material::toolbar` centre tests; ★ `title_bar_fits` centring |
| FR-009 | `ui::toolbar::tests::the_jump_field_sends_the_chords_message_except_in_settings`; ★ `main_tests::the_jump_field_opens_the_quick_switcher_with_its_input_focused`; ★ `overlay_dismissal_rules::a_press_on_the_jump_field_closes_an_open_quick_switcher`; ★ `title_bar_keyboard::enter_on_the_jump_field_publishes_its_press`, `with_settings_open_tab_skips_the_jump_field` |
| FR-010, SC-003 | `give_way` planner tests; `material::toolbar::the_bar_gives_way_in_the_five_step_order`; ★ `title_bar_fits`; `layout_text_overflow` |
| FR-011 | `ui::toolbar::tests::the_settings_button_opens_settings_only_while_it_is_closed`; ★ `title_bar_keyboard::enter_on_settings_opens_settings` |
| FR-012 | ★ layout snapshot `title-bar-menu-open` (items and order); `gates/panel_placement` |
| FR-013 | ★ `title_bar_keyboard::the_bar_controls_are_tab_stops_in_bar_order` |
| FR-014, SC-002 | existing `keymap`, `app_chord_tests` (unchanged set of chords) |
| FR-015 | superseded by FR-016 once #1125's row merged (M4) |
| FR-016, SC-006 | ★ `title_bar_keyboard::no_bar_control_publishes_an_overview_message`; ★ `toolbar::the_overview_chord_*` and `app_chord_tests` through `State::update` |
| FR-017 | `composition_contrast::the_breadcrumbs_words_are_legible_on_the_title_bar`, `the_jump_fields_words_and_edges_are_legible_on_the_title_bar` |
| FR-018 | docs (user guide pages in each milestone's PR) |
| FR-019 | existing `app_bar_scroll` |

Every criterion has a test that exists and runs; none traces only to doubles. Tests tracing to
nothing: none found.

## What was not audited

- Ordering beyond the commit level: rebase-merged milestones cannot show a red that was never kept.
- Mutation beyond the six mutants above; no coverage tool exists.
- macOS (`Cmd+Shift+P` hint): Linux container only; the `keymap` tests cover the label.
- Performance of the suite as a whole: one full `scripts/gate.sh` run (green, counts in `suite:`), not timed per test.
