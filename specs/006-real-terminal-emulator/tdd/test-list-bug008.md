---
feature: 006-real-terminal-emulator
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 1 # scoped to BUG-007: FR-013e / SC-014 only — see "Out of scope"
planned_at: 226d3a8b
updated_at: 2c8c5ae3 # the Red commit; states updated with the Green commit
suite_baseline: green # CI run on origin/main 226d3a8b concluded success; see cycle-log.md
---

# Test List: BUG-007 — a single click in the terminal selects nothing

This feature shipped before the TDD extension existed and had no `tdd/` directory. This list is
written for **BUG-007 only** (Phase 14, T074–T076); the feature's earlier behaviours are not
re-planned here. Traces use `FR-`/`SC-` ids from [spec.md](../spec.md) and decision ids from
[BUG-007.autopilot.md](../bugs/BUG-007.autopilot.md).

`example (mutant)` marks a behaviour whose test passes on `origin/main` because the defect does not
break it; it guards the fix against over-reach, so its red is taken against the named deliberate
mutant after the fix exists, per the playbook.

**Acceptance runner.** The profile's acceptance runner is the sandbox real-runtime suite, which has no
GUI. A1 is therefore held by T076's recorded visual pass on Xvfb (the `visual-pass` skill), as feature
030 did; the unit behaviours below are the automated half.

## Outer loop: acceptance behaviors

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | In a real Regular Terminal, a click at human pace after a drag leaves no highlight and the paste is still the dragged text; a synthetic press+release after a drag leaves other clipboard text in place; a drag still highlights and copies | FR-013e, SC-014 | visual pass | DONE | [evidence/bugfix-007-pass-2026-09-14.md](../evidence/bugfix-007-pass-2026-09-14.md) |

## Inner loop: unit behaviors

### `crates/micold-client/src/selection.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | A `Char` selection started and never updated contains no cell and yields empty text | FR-013e | example | DONE | `crates/micold-client/src/selection.rs::tests::a_click_without_a_drag_selects_nothing` |
| U2  | A `Char` selection updated onto its own start anchor contains that cell and yields its character | FR-013e, D2 | example (mutant: empty while `current == start`) | DONE | `selection.rs::tests::an_update_onto_the_start_anchor_selects_that_cell` |
| U3  | A `Char` selection dragged into another cell and back contains the pressed cell and yields its character | FR-013e, D2 | example (mutant: empty until the moving end differs from the start) | DONE | `selection.rs::tests::a_drag_out_and_back_selects_the_pressed_cell` |
| U4  | `Word` and `Line` selections started without motion select the word and the line | FR-013b, FR-013e | example | DONE | `selection.rs::tests::word_expansion_selects_whole_word, line_granularity_selects_whole_line (existing)` |

### `crates/micold-client/tests/clipboard_request.rs` (the copy decision)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U5  | A click-only selection over text produces no copy request | FR-013e, FR-013c | example | DONE | `crates/micold-client/tests/clipboard_request.rs::a_click_without_a_drag_asks_for_nothing` |

### `crates/micold-client/src/ui/material/terminal_pane.rs` (`clipboard_gestures`)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U6  | After a left press, pointer motion that stays inside the pressed cell publishes no `TerminalSelectUpdate` | FR-013e, D5 | example | DONE | `terminal_pane.rs::tests::clipboard_gestures::pointer_jitter_inside_the_pressed_cell_is_not_a_drag` |
| U7  | After a left press, motion into another cell and back onto the pressed cell publishes a `TerminalSelectUpdate` for each | FR-013a, D2 | example (mutant: the pane never publishes an update) | DONE | `clipboard_gestures::motion_into_another_cell_and_back_extends_the_selection_each_time` |
| U8  | With a selection already held, a left press and release delivered together write nothing to the clipboard | FR-013e, D4 | example | DONE | `clipboard_gestures::a_tap_over_a_held_selection_writes_nothing_to_the_clipboard` |
| U9  | A left release publishes `TerminalSelectionReleased`, after its press's `TerminalSelectStart` | FR-013, D4 | example | DONE | `clipboard_gestures::a_release_asks_for_the_copy_after_its_press_starts_the_selection` |

## Invariants and edge cases still to place

None.

## Out of scope

- The shell's handling of `TerminalSelectionReleased` (`shell::clipboard::on_selection_released`):
  it calls `selection::copy_request` (U1, U5) and hands the outcome to `interpret`, whose "decides
  nothing" shape `clipboard_request.rs::the_shells_translation_decides_nothing` already holds. The
  `iced::Task` it returns cannot be inspected without a runtime; A1 exercises it end to end.
- Every behaviour of feature 006 before BUG-007: shipped and tested before this list existed.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`; `mise run gate` before pushing)
- Coverage: none (no tool)
- Mutation: none (no tool) — deliberate mutants by hand

Tests in a `#[cfg(test)] mod` inside `src/` are run through the client's lib target:
`scripts/build-lock.sh cargo test -p micold-client --lib {path} -- --exact`.
