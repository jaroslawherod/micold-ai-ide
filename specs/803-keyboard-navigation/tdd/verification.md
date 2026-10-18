# TDD verification: 803 keyboard navigation

**Verdict: PASS with recorded deviations.** Audited 2026-10-18 from the merged history (#967, #984, #987, #990) and `tdd/cycle-log.md`.

## Test-first evidence

- **M1** has red runs for most behaviours (`cycle-log.md`: stub failures for `step`, `update`, `apply_navigation`); the `row_index`/reveal behaviour (T009/T012) was test-after, shown red by a hand mutation.
- **M1–M3 red runs were not captured** for several rows (M2 `shown`/`keyboard_focus`; M3's session chords T024/T026 and `session_step` T025, whose rows sit under the M2 heading in `cycle-log.md`): the tests did not compile or were written in the same step as the code. The cycle log marks each as a deviation, with the reason compiling-failure or the arm a stub would have hit.
- **M4** is test-after (T030/T033, T031→T034): labels were seen red through the docs test only.
- Git history cannot show test-first: every milestone landed as one commit holding code and tests. The cycle log is the only evidence.

## Test strength (hand mutants; no `cargo-mutants`, `tdd-profile.md` `mutation: null`)

Mutants on `micold_core::navigation::step`, each run with `cargo test -p micold-core --lib navigation`:

| Mutant | Killed by |
|---|---|
| `(None, Next) => 0` → `len - 1` | `a_current_entry_missing_from_the_order_counts_as_none`, `no_current_entry_starts_at_the_end_the_press_points_to` |
| Previous `(i + len - 1)` → `(i + len - 2)` | `a_one_entry_list_stays_on_its_entry`, `every_entry_is_reached_within_half_the_list_in_the_shorter_direction` |
| Next wrap `% len` → `.min(len - 1)` | `stepping_past_either_end_wraps`, `every_entry_is_reached_…` |

All three killed. The client-side `location_order`, `session_step`, `project_step` and the keymap rows were not mutated: M1's mutation of `reveal_row` is the only other one recorded.

## Acceptance coverage

Each user story is covered by an automated test (`tests/keymap.rs`, `tests/navigation.rs`, `tests/navigation_apply.rs`, `tests/sidebar_state.rs`, the `keyboard_focus` and `tree_view` lib tests, `showcase_completeness`) and the live pass in `visual-pass/t038.md` (B1, B3, B4, B6, B7). Not covered live: B2, B5, B8–B12 (automated only), and macOS/Windows (§C not run).

## Remediation tasks

None blocking. Open follow-ups: a mutation run of the client-side step functions once `cargo-mutants` is available.
