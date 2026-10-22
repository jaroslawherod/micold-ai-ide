# Quickstart: validate the pull request rows

## A. Automated

- `mise run test-core` (here: `cargo test -p micold-core --all-targets` with the cloud env flags):
  K1–K6 (K5a–K5g), Q1–Q2, P1–P7.
- `cargo test -p micold-client` overview_card / card_grid / overview tests: U1–U4; `main_tests.rs` hand-off, menu return and opener: U5–U7.
- `crates/micold-client/tests/layout_snapshot.rs`: covered state `overview-pr-rows`.
- Whole gate: `scripts/gate.sh`.

## B. Visual pass (real app, scripted pull request source)

Setup: a project with pull request status on and a fake source (`FakePullRequestSource`) answering
branch `feat-1123_x` with #12 open, checks failing; #9 merged; branch `feat-1123_y` with four pull
requests (open approved, draft pending, merged, closed); branch `feat-1123_z` with none.

1. Overview: `feat-1123_x` shows "#12 Open checks failing", "#9 Merged", counts "1 open · 1 merged"
   (spec Story 1 independent test).
2. `feat-1123_y` collapsed: two rows (open, draft) and "+2 more"; rest on it: four rows, no
   "+N more"; `feat-1123_z`: no row, no counts.
3. Click "#12": the opener is asked for #12's address; no ripple, no selection, Overview stays.
   Click the pill: the card activates.
4. Tab from `feat-1123_y`'s card: each number in order, Enter opens; Escape collapses, Tab skips the
   hidden numbers; Escape while the third number has the keyboard collapses the card and the card
   keeps the keyboard (shown focus on the card, not lost); Menu key on a number opens the card menu
   and closing it returns to that number.
5. Rate-limit then offline (fake source): rows stay. Unavailable: dash, rows gone.
6. 280, 380, 480 px; light and dark; scale 1.0 and 2.0: numbers and pills whole.

## Results

### A. Automated (T038, close unit, 2026-10-22, HEAD 2abbffff = origin/main after M3)

- `scripts/gate.sh` (fmt, clippy core and workspace with `-D warnings`, `cargo test --workspace`,
  `scripts/tests/*.test.sh`): passed, `GATE_EXIT=0`. 615 test binaries: 9556 passed, 0 failed,
  17 ignored. Every shell test passed. The gate's workspace run covers `micold-core` (K1–K6, Q1–Q2,
  P1–P7), the client tests U1–U7 and `layout_snapshot.rs` with covered state `overview-pr-rows`.
- `grep -n "url" crates/micold-core/src/pr_list.rs` shows only the field (l.24, documented "Never
  logged, never serialised") and a test fixture (l.226). `ListedPr` has a hand-written `Debug` that
  prints the number and the state only (l.38–46), and nothing in the file derives `Serialize`.

### B. Visual pass (T039)

See [visual-pass.md](visual-pass.md): §B1–§B6 PASS, with the Page keys on a number only partly
exercised.
