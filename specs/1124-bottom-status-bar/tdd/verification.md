---
feature: 1124-bottom-status-bar
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override in .specify/templates/overrides or presets
verified_at: 7eb6815 # m4-local: M1–M3 merged or open, M4 and M5 local; audit fixes c239782 and 7eb6815 included, S7/S9 in the close commit
behaviors: 36 # rows of the table below: cycle-log rows (no tdd/test-list.md), M2 rebuilt from its commit
proven: 5
likely: 23 # red shown by a deliberate mutant after green, by the milestone (M1) or by this audit (M2, M4, M5)
test_after: 0
no_test: 0
not_applicable: 8 # guards true by construction (4: B3, contrast, focus stop, M3 seven conditions), first recordings of new snapshot states (3), the M4 background-session guard (1)
high_smells: 0 # S1 fixed in 7eb6815; S2 judged a restated rule, not a weakening (see Findings)
criteria_total: 21 # FR-001..FR-015 and SC-001..SC-006
criteria_covered: 21 # SC-006 by the layout snapshot's words in every covered state and the T039 visual pass
mutation_score: 100 # 16 of 16 deliberate mutants caught. Scope: ui/status_bar.rs and the window column of ui/mod.rs, sampled; no tool (cargo-mutants absent)
mutants_survived: 0
suite: "micold-client --all-targets --no-fail-fast: 5013 passed, 1 failed (feature_registration_cost, fixed in c239782 and re-run green), 5 ignored, 246 binaries, 146 s; micold-core --all-targets: 3952 passed, 4 failed (uid 0 permission tests, not this feature), 10 ignored"
---

# TDD Verification: Bottom status bar (#1124)

**Verdict: PASS_WITH_GAPS.** Every criterion has a test through the real layout or the real model,
and all 16 deliberate mutants failed a test. The gap is test-first evidence: 23 of 36 behaviours
show their red only as a mutant run after the code was green (M1's own runs, and this audit's runs
for M2, M4 and M5). The milestones landed tests and code in the same commit.

The audit was not independent. The session that ran it is the autopilot run that wrote M4 and M5.
The smell pass ran in a fresh-context reviewer (`claude -p --agent autopilot-reviewer`, read-only
tools). Each finding it reported was opened at the cited line before it went into this report.

## Preflight

- Profile `.specify/memory/tdd-profile.md` read: conventions, exemplars, helpers. `mutation: null`
  and `coverage: null`, so Phase 4 used deliberate mutants and no coverage was taken.
- There is no `tdd/test-list.md`. The cycle-log rows served as the behaviour list, as in 943's
  audit. M2 has no cycle-log section: its behaviours were rebuilt from commit `ebcd28ca` and
  `tasks.md` T016–T020.
- Range audited: the 23 commits tagged `(1124)` between `f55628df` (design merge) and `7eb68151`.
  The range also holds rebased commits of #1126, #1260, #1299, #1120 and #943, which were not
  audited.
- `mise run gate` was not run, because the split-gate script hard-codes the main checkout. These
  were run in `/home/user/micold-m4` instead: `cargo test -p micold-client --all-targets
  --no-fail-fast` and `cargo test -p micold-core --all-targets --no-fail-fast`.
- **The suite was red at the start, because of this feature.** M4's
  `ui::status_bar::tests::no_other_body_shows_the_cli` called `features::changes::update` directly.
  The architecture gate `feature_registration_cost::only_the_root_drives_a_feature` forbids that,
  and M4's scoped checks ran only its Verify tests, so they missed it. Fixed in `c239782c`: the test
  now opens the view through `State::update_changes`. Both tests are green.
- The 4 core failures are uid-0 permission tests (`a_save_over_an_unreadable_file_is_refused…` and
  three more). The audit ran as root, which ignores file permissions. 943's audit recorded the same
  four.

## Test-first evidence

History shows tests and source landing together in each `feat(1124)` commit, with follow-up
test-only commits (`ccc14097`, `eb76bcb7`, `afef53af`, `1a387c32`, `3e610f5a`). The order inside a
commit cannot be seen. A row is `PROVEN` only where the log records a red before the change. It is
`LIKELY` where the red comes from a mutant run after green.

| Behaviour (cycle-log row) | Class | Evidence |
|---|---|---|
| M1 `anatomy::status_bar::HEIGHT` (T001/T002) | PROVEN | compile error `cannot find status_bar in anatomy` before T002 |
| M1 B1 height/width | LIKELY | mutant after green (bar 4 dp taller) FAILED |
| M1 B2 insets and gaps | LIKELY | mutant: leading group at x 0, FAILED |
| M1 B4 drop/shorten order | LIKELY | mutant: no give-way, 4 tests FAILED |
| M1 B5 dropped item draws nothing | LIKELY | mutant FAILED |
| M1 tones map to roles | LIKELY | mutant FAILED |
| M1 busy ring frames | LIKELY | mutant (no ring), both tests FAILED |
| M1 connection item table | LIKELY | mutant (dot always positive) FAILED |
| M1 location host/other runtime | LIKELY | mutant (host named) FAILED |
| M1 sandbox item table | LIKELY | mutant FAILED |
| M1 location (no project / default key / unbound), steady words, retry tooltip | LIKELY | not reached by M1's mutants; this audit's MU9, MU10, MU11, MU12 and MU13 each FAILED (table below) |
| M1 mount under every branch (T013) | LIKELY | mount line removed: "an ordinary window draws the status bar" FAILED |
| M1 fits at 640 dp (T009) | LIKELY | mutant FAILED (`height: 32.0`) |
| M1 body 28 dp shorter (T014, snapshot) | LIKELY | fixture differed in all 59 states before re-recording |
| M1 B3 no item publishes | NOT_APPLICABLE | a guard: no item has a press handler |
| M1 contrast of words and dots (T006) | NOT_APPLICABLE | a guard on existing roles, as T006 allows |
| M1 no focus stop (T010) | NOT_APPLICABLE | a guard: no focusable widget by construction |
| M2 no row above the body during bring-up (T016/T018) | LIKELY | no cycle-log section. MU2 (the old row restored) FAILED `sandbox_progress::the_window_column_has_no_row_above_the_body_during_bring_up` |
| M2 bring-up covered states (T017, T019) | NOT_APPLICABLE | first recording of new snapshot states |
| M3 a lost connection raises no banner | PROVEN | FAILED before the `Disconnected` arm. Rewritten through the layout in `7eb68151`, where MU1 FAILED it |
| M3 seven action conditions keep banner and button | NOT_APPLICABLE | a guard on unchanged arms (R2/D3) |
| M3 banner height only for an action condition | PROVEN | the new `strip(disconnected) == 0.0` failed on the previous `connection_banner` |
| M3 warning/lost-connection states fit | NOT_APPLICABLE | first recording of new snapshot states |
| M4 CLI item names the active session's CLI | LIKELY | MU14 (`"CLI {}"`) FAILED |
| M4 every other body omits it | LIKELY | MU3 (guard removed) FAILED at "Settings" |
| M4 closing the Overview brings it back | LIKELY | MU15 FAILED |
| M4 background session leaves the model unchanged | NOT_APPLICABLE | a guard: `model` reads `session.active` only |
| M4 hints read the platform's chords | LIKELY | MU5 (Overview hint reads the switcher chord) FAILED |
| M4 focus between panes changes the item | LIKELY | MU4 (first session instead of the focused one) FAILED |
| M4 CLI item and hints fit; hints go first | LIKELY | MU6 (hints never drop) FAILED 2 tests |
| M4 snapshot: CLI item and hints | LIKELY | the fixture differed before re-recording |
| M5 usage only for a current reading, switch on | PROVEN | FAILED (`expect("a current reading")` on `None`) |
| M5 app bar draws no usage | PROVEN | FAILED ("37%" drawn in the app bar) |
| M5 usage follows the CLI item | LIKELY | passed before the change (a guard then). MU16 (usage before the CLI) FAILED |
| M5 usage drops before any text shortens | LIKELY | M5's only recorded failure was a fixture clash, not the missing behaviour; MU7 (usage never drops) FAILED |
| M5 usage covered state | NOT_APPLICABLE | first recording |

### Tests that already existed

The 1124 commits were diffed for removed or loosened assertions, `#[ignore]`, renamed tests and
config changes.

- `sandbox_progress.rs` (`ebcd28ca`): `the_indicator_occupies_nothing_unless_the_sandbox_is_coming_up`
  was deleted along with the widget it measured, because the bring-up row is gone (FR-003). Its
  replacement asserts the stronger property: no row and the same body top edge during every
  bring-up stage. The idle-frames half lives on in
  `idle_requests_no_frames::the_status_bar_asks_for_frames_only_while_the_sandbox_comes_up`.
- `gates/composer_fits.rs` (`eb76bcb7`, decision D6): the absolute floor `transcript.height >=
  crowded` was replaced by a bound on what the crowded composer takes from the same pane's idle
  transcript, `lost <= QUEUE_MAX_HEIGHT + 2·SM + MAX_HEIGHT`, plus `> 0.0`. This is finding S2
  below.
- `overview.rs` banner-height test (M3): `strip(true) > 0.0` was inverted to `== 0.0` together with
  the behaviour change. FR-003/FR-009 call for that change, so it is not a loosening.
- `containment::CLIP_REVEALED` was renumbered with the column's new child order (M3), and
  `tooltip_clears_its_row` and `notification_kind_rows_sit_under_the_switch` now measure against
  the body's bottom edge instead of the window's (M1). Both are re-anchored, not loosened.
- `tokens_anatomy.rs`: 1124 added an `ALL.len() == 48` pin, which this close removed (S7).
- No `#[ignore]`, no exclusion in config, and no threshold changes by 1124 commits.

`tasks.md`: T001–T038 are ticked and each has evidence above. T039 is ticked by this close (the
visual pass, `quickstart.md` Part B results).

## Findings

| # | Severity | Finding | Evidence | Disposition |
|---|---|---|---|---|
| S1 | HIGH → fixed | `a_lost_connection_raises_no_banner` asserted only that `connection_banner`'s source contains `ConnectionStatus::Disconnected`, which still holds with the arm drawing a row | `tests/banner_is_not_a_snackbar.rs:262` (before `7eb68151`) | Fixed: the test now lays out the window for Connected, Disconnected and BuildMismatch and compares the column's rows and the body's top edge, with a premise that a banner moves the body. MU1 FAILED it |
| S2 | LOW (reported as HIGH) | `composer_fits` dropped its absolute floor, so the reviewer read it as a weakened existing test | `tests/gates/composer_fits.rs:219–233` | Judged a restated rule, not a weakening. `lost <= limits` is a floor relative to the same pane's idle transcript (`transcript >= roomy − limits`), each term is a named constant, and the 941 T055 mutant (queue cap removed) still fails it (D6: 475 → 0). Recorded in D6 and in the ledger's Follow-ups (#941 caps) |
| S3 | MED, declined | `each_of_the_seven_action_conditions_keeps_its_banner_and_button` checks that names and labels appear in the function source, not that they are paired | `tests/banner_is_not_a_snackbar.rs:233` | Declined, as in review A M3: the banners are private elements, and `overview::the_connection_banner_has_height_only_for_an_action_condition` measures a real banner. The arms are unchanged by 1124 |
| S4 | MED, declined | Tooltip expectations read the shared helpers (`connection_words`, `attempt_line`, `persistent_notice`) instead of literal sentences. The chord test reads `switcher_chord_labels()` | `src/ui/status_bar.rs:337,359,413,449,464,598` | Declined. FR-006 makes the banner's words the bar's tooltip, so the tests pin that the bar reads that one source, and the banner's wording has its own tests. The switcher key's label depends on the keyboard layout (`physical_label(KeyP)`), so a literal would be wrong off QWERTY. MU8, MU10 and MU5 show the tests still catch a wrong source |
| S5 | MED → fixed | The 640 dp usage test asserted only absences, so an undrawn bar passed. One assertion was implied by another | `tests/gates/status_bar_fits.rs:118` | Fixed in `7eb68151`: a premise that the connection item is drawn, and the implied assertion removed |
| S6 | MED, declined | `the_status_bar_adds_no_focusable_widget` counts focusables of the bar mounted alone | `tests/terminal_focus.rs:864` | Declined. The claim is about the bar's own widgets. The bar's mount point is pinned by the snapshot, and the window-wide traversal by `terminal_focus`'s existing tests |
| S7 | MED → fixed | `anatomy::ALL.len() == 48` (new in `bd682a57`) is a magic count that the source check already covers, and `toolbar.rs` hard-coded `origin.y < 56.0` for the app bar | `micold-core/tests/tokens_anatomy.rs:274`, `tests/toolbar.rs:29` | Fixed: the count test is removed (`the_listed_table_holds_every_constant` still reads the module source). The app bar's band is now `anatomy::app_bar::HEIGHT` (64; 56 was narrower than the bar) |
| S8 | LOW → fixed | The final focus move used `assert_ne!` against Claude Code | `src/main_tests.rs:13202` | Fixed: `assert_eq!` against the Copilot item |
| S9 | LOW → partly fixed | 1124 added `mod support;` mid-file in `features_window.rs` and `terminal_focus.rs`, and the painted-text collection is copied | `tests/features_window.rs:238`, `tests/terminal_focus.rs:859` | `mod support;` moved to the top of both files. The copied collection (two lines) is left as is |
| S10 | LOW → fixed | The `reading()` doc said its window never resets, but it resets at `NOON + 3600` | `src/ui/status_bar.rs:615` | Fixed: the comment states when the window resets |

## Mutation results

No tool (`cargo-mutants` is absent). Each mutant below was a deliberate single edit. The named test
was run, the file was restored by writing back its saved content, and `git status` was confirmed
clean after each batch. The suite stayed green afterwards: the client tests named above were re-run
in the full client suite at `7eb68151`.

| Mutant | Behaviour | Survived | Judgment |
|---|---|---|---|
| MU1 `connection_banner`: `Disconnected => connection_words(status)` | M3 no banner | No | `(4, 153.0)` vs `(3, 65.0)` |
| MU2 `view_with`: a 24 dp row while `attempt_line` is `Some` | M2 no row | No | "Probing added a row to the window column" |
| MU3 `cli_item`: guard `session_view_visible` disabled | M4 omit | No | failed at "Settings" |
| MU4 `cli_item`: first session instead of `session.active` | M4 focus | No | `main_tests::panes::moving_pane_focus_changes_the_status_bars_cli` |
| MU5 Overview hint built from the switcher chord | M4 hints | No | `"Ctrl+Shift+P Overview"` |
| MU6 hints `GiveWay::Keep` | M4 fits | No | 2 of 4 `status_bar_fits` failed |
| MU7 usage `GiveWay::Keep` | M5 drop order | No | "37%" drawn at 640 dp |
| MU8 warning tooltip `None` | M1 failure reason (US2 s4) | No | `standing_conditions_are_warning_words_with_the_notice` |
| MU9 no project → empty location | M1 location | No | `"Daemon connected · "` |
| MU10 retry detail dropped | M1 retry tooltip (US2 s2) | No | "two lines" |
| MU11 `Disabled` reads "Sandbox ready" | M1 steady words | No | `steady_sandbox_states_are_neutral_and_quiet` |
| MU12 default location keyed `"default"` | M1 location key | No | `"Daemon connected · no daemon"` |
| MU13 unbound location reads "local" | M1 unbound | No | `an_unbound_or_missing_daemon_says_no_daemon` |
| MU14 `"CLI {}"` | M4 CLI label | No | `Some("CLI Claude Code")` |
| MU15 guard disabled, Overview close | M4 s5 | No | `closing_the_overview_brings_the_cli_back` |
| MU16 usage drawn before the CLI item | M5 order | No | "usage comes after the CLI item" |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, SC-002 | `material::status_bar` B1; `layout_snapshot` (bar anchors in every state) | Yes |
| FR-002 | `features_window::the_install_location_screen_draws_no_status_bar` (both halves); snapshot states for session, split panes, project surface, Overview, full screen, Changes, Compare, Settings and the empty screen | Yes |
| FR-003, SC-001, SC-003 | `sandbox_progress::the_window_column_has_no_row_above_the_body_during_bring_up`; `banner_is_not_a_snackbar::a_lost_connection_raises_no_banner`; snapshot body anchors | Yes |
| FR-004 | `status_bar` connection tests (Connected, Disconnected, Displaced/ProjectBusy, mismatches) | Model; snapshot words |
| FR-005, FR-006 | `status_bar` sandbox tests, `a_retry_reason_reaches_the_tooltip_on_its_own_line`, `standing_conditions_…`; `idle_requests_no_frames` | Model; snapshot words |
| FR-007 | `the_session_view_names_…`, `no_other_body_shows_the_cli`, `closing_the_overview_…`, `moving_pane_focus_changes_the_status_bars_cli` (through `update`) | Yes |
| FR-008 | `the_hints_name_the_overview_and_switcher_chords`; snapshot | Yes |
| FR-009, SC-004 | `each_of_the_seven_action_conditions_…` (source), `overview::the_connection_banner_has_height_only_for_an_action_condition` (layout), `status-bar-warnings` state | Yes |
| FR-010 | snapshot states with banners in every branch (column order) | Yes |
| FR-011 | `usage_is_present_only_…`, `toolbar::the_app_bar_draws_no_usage_indicator`, `with_room_the_usage_follows_the_cli_item`, `status-bar-usage` state | Yes |
| FR-012, SC-005 | `material::status_bar` B4/B5, `status_bar_fits` (4), `layout_text_overflow` | Yes |
| FR-013 | `terminal_focus::the_status_bar_adds_no_focusable_widget`; the bar is the base column's last child (snapshot), with overlays in later layers | Yes |
| FR-014 | `a_press_on_any_item_publishes_nothing` | Yes |
| FR-015 | user guide (appearance-theming, sandboxed-daemon, overview, settings, worktrees-and-sessions); snapshot and overflow gate states | Docs check |
| SC-006 | snapshot words in every covered state; T039 visual pass (`visual/t039-*`) | Yes |

Untested criteria: none. Tests tracing to nothing: none found.

## What was not audited

- Mutation was sampled: 16 mutants in `ui/status_bar.rs` and the window column of `ui/mod.rs`.
  `material/status_bar.rs` (the give-way pass) relies on M1's mutant runs only.
- No coverage tool, so no branch-level view.
- The full workspace suite, `mise run gate` and clippy for `micold-daemon` were not run here. The
  orchestrator runs the full gate.
- macOS chord notation and the flush bottom edge inside macOS's rounded corners: Linux host only.
  The `cfg!(target_os = "macos")` arms never ran.
- The other features in the range (#1126, #1260, #1299, #1120, #943).
- Performance: there is no criterion. Idle frames are covered by `idle_requests_no_frames`.
