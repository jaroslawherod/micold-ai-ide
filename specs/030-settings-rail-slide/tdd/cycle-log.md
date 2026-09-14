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

## Correction and strengthening: U23 (M1 review round 2)

- correction: cycle 2's commit is `4af105ca`. The rail is 32 px wide, not 31: `collapsed_strip` is a
  `STRIP_WIDTH - 1` surface plus a 1 px `Divider` (`divider.rs:18`). The code reads `rail.size()`, so
  only the test's constant and the records were wrong
- test strengthened, no production change: `a_sliding_drawer_is_never_narrower_than_its_rail` now
  asserts the node's exact width (32 closing at 0.05, 0.02, `2 · CLOSED` and opening at 0; 156 closing
  at 0.5) and that the handle sits at the panel's right edge. It passes on `4af105ca`
- mutant: the floor without subtracting the handle (`.max(rail.size().width)`) ->
  `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_sliding_drawer_is_never_narrower_than_its_rail -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:551:13: assertion `left == right` failed: open false at progress 0.05: the drawer's width  left: 38.0 right: 32.0`; restored

## Cycle 3: U24 a closing drawer swaps to its rail once it is no wider

- test: `crates/micold-client/src/ui/material/navigation_drawer.rs::tests::a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider` (new),
  with a stub `rail_showing: bool` on `Track` set only in `state` so the test compiles
- red: `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:611:13: assertion `left == right` failed: open false at progress 0.05 beside a 32 rail: the rail on screen  left: false right: true` (1 failed)
- green: `NavigationDrawer::layout` shows the rail when `showing_rail(progress)` or, closing,
  `full.width * progress <= rail.size().width - handle_width`, and stores that in `Track::rail_showing`;
  `update`, `draw`, `mouse_interaction` and `overlay` read the stored decision instead of recomputing it
  from progress (`overlay` had its own inline copy). The width clamp became `.min(full.width).max(0.0)`,
  equal for every valid size and unable to panic. Module tests -> 9 passed. Suite: see the M1 round-3
  gate below
- refactor: U23 re-cut (next entry); no production refactor
- commit: the M1 round-3 fixes commit on top of `bcfcb15d`
- notes: added mid-milestone from M1 review round 3, A F1 (ledger D22, task T046)

## Re-cut and strengthening: U23 (M1 review round 3)

- why: after cycle 3, U23's closing cases below the floor (0.05, 0.02, `2 · CLOSED`) lay out the rail,
  where panel and handle are both parked and its handle-edge assertion held only by float absorption
  (`-f32::MAX / 4 + 300`). Their width, 32, is the rail's own node, and U24 now holds that the rail is
  what shows there
- test changed, no production change: cases are opening at 0 and 0.05 (panel 300, revealed 26),
  closing at 0.125 and 0.5 (37.5, 150) and open at 1 with a 10 px panel (10); each asserts the node is
  revealed + handle wide and that the handle's x and the panel's right edge both equal the revealed
  width. Passes after cycle 3 (9 passed)
- mutant: the floor without the panel's cap (`.min(full.width)` removed) ->
  `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_sliding_drawer_is_never_narrower_than_its_rail -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:561:13: assertion `left == right` failed: open true at progress 1: the drawer's width  left: 32.0 right: 16.0`; restored

## M1 round-3 gate

- `cargo fmt --all --check`, clippy (core, workspace, `-D warnings`) -> clean
- `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3077 passed, 1 failed (the local-only
  `exclusivity` red, D17), 6 ignored; all 14 `scripts/tests/*.test.sh` pass
- correction: *Correction and strengthening: U23 (M1 review round 2)* was committed as `bcfcb15d`
