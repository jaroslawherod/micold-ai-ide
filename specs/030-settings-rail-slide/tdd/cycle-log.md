# Cycle Log: The settings rail slides when it collapses and expands

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3073 passed, 2 failed, 6 ignored
- commit: `a9f54e77`
- recorded: cycle 0, before any change
- red, local only: `micold-daemon --test exclusivity`
  `one_conversation_one_session::a_second_open_of_a_held_pi_conversation_starts_nothing`
  (`exactly one `pi` for one conversation`, left 0, right 1) and `micold-daemon --test pi_launch_wiring`
  `a_pi_session_carries_the_component_only_while_the_switch_is_on` (`launch 0 never reached `pi``).
  Both launch `pi` on this host; CI on `main` is green (latest `CI` runs `success`). Neither touches
  code this feature changes, so every red this feature records is read from its own test, not the
  suite total. Task T001 rechecks them on fresh `main`.

## Recheck on fresh main (T001)

- suite: `mise run gate` in a detached worktree at `b27ffe62` -> fmt and both clippy passes clean;
  `cargo test --workspace` stopped at `micold-daemon --test exclusivity`
  (`a_second_open_of_a_held_pi_conversation_starts_nothing`, `exclusivity.rs:435`). Re-run with
  `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3074 passed, 1 failed, 6 ignored;
  all 13 `scripts/tests/*.test.sh` pass
- the one red is one of the two recorded at the baseline; `pi_launch_wiring` now passes. No other test
  fails, so the loop continues on it (D17)

## Cycle 1: U1 the sidebar slides on the emphasized curve

- test: `crates/micold-client/src/ui/material/navigation_drawer.rs::tests::the_sidebar_slides_on_the_emphasized_curve` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::the_sidebar_slides_on_the_emphasized_curve -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:497:9: the sidebar moved linearly: 0.25 at a quarter of the slide` (1 failed)
- green: `NavigationDrawer::state` builds its `Progress` with `.easing(EMPHASIZED.x1, EMPHASIZED.y1, EMPHASIZED.x2, EMPHASIZED.y2)`
  (`navigation_drawer.rs:114`). Test -> 1 passed. Suite `cargo test --workspace --no-fail-fast`
  -> 3075 passed, 1 failed (the local-only `exclusivity` red above), 6 ignored; fmt and clippy clean
- refactor: none needed, one builder call on an existing constructor
- commit: uncommitted at the time of writing (`--no-commit`; the milestone commits after its reviews)
- notes: the single-test command is the profile's `--lib` form; the profile's `--test {file}` form
  names integration-test files and this test is in-crate (`ui::material` is `pub(crate)`)

## Correction to the two entries above (M1 review round 1)

- *Recheck on fresh main*: there are 14 `scripts/tests/*.test.sh` suites, not 13; all 14 passed. CI on
  `b27ffe62` (`main`) is `success`.
- *Cycle 1*: its commit is `bbc2414b`.

## Cycle 2: U23 a sliding drawer is never narrower than its rail

- test: `crates/micold-client/src/ui/material/navigation_drawer.rs::tests::a_sliding_drawer_is_never_narrower_than_its_rail` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_sliding_drawer_is_never_narrower_than_its_rail -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:535:13: at progress 0.05 the drawer is 21 wide, narrower than its 31 rail` (1 failed)
- green: `NavigationDrawer::layout` computes the revealed width as
  `(full.width * progress).max(rail.size().width - handle_width).clamp(0.0, full.width)`. Module tests
  -> 8 passed. Suite `cargo test --workspace --no-fail-fast` -> 3076 passed, 1 failed (the local-only
  `exclusivity` red), 6 ignored; fmt and both clippy passes clean. Shell suites: `capture-harness`
  failed once under host memory pressure and passed on three re-runs; the other 13 passed
- refactor: none; `handle_width` moved above the width it now feeds
- commit: the M1 round-1 fixes commit on top of `bbc2414b`
- notes: added mid-milestone from M1 code review round 1 (ledger D20, task T045). The behaviour
  predates 030 on a linear track, where it lasted ~33 ms; `emphasized` stretched it to ~150 ms
