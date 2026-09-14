---
feature: 006-real-terminal-emulator
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 1 # scoped to BUG-007: FR-013e / SC-014 only — see "Out of scope"
planned_at: 226d3a8b
updated_at: 226d3a8b
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
| A1  | In a real Regular Terminal, a click at human pace after a drag leaves no highlight and the paste is still the dragged text; a synthetic press+release after a drag leaves other clipboard text in place; a drag still highlights and copies | FR-013e, SC-014 | visual pass | PENDING | |

## Inner loop: unit behaviors

### `crates/micold-client/src/selection.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | A `Char` selection started and never updated contains no cell and yields empty text | FR-013e | example | PENDING | |
| U2  | A `Char` selection updated onto its own start anchor contains that cell and yields its character | FR-013e, D2 | example (mutant: empty while `current == start`) | PENDING | |
| U3  | A `Char` selection dragged into another cell and back contains the pressed cell and yields its character | FR-013e, D2 | example (mutant: empty until the moving end differs from the start) | PENDING | |
| U4  | `Word` and `Line` selections started without motion select the word and the line | FR-013b, FR-013e | example | PENDING | |

### `crates/micold-client/tests/clipboard_request.rs` (the copy decision)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U5  | A click-only selection over text produces no copy request | FR-013e, FR-013c | example | PENDING | |

### `crates/micold-client/src/ui/material/terminal_pane.rs` (`clipboard_gestures`)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U6  | After a left press, pointer motion that stays inside the pressed cell publishes no `TerminalSelectUpdate` | FR-013e, D5 | example | PENDING | |
| U7  | After a left press, motion into another cell and back onto the pressed cell publishes a `TerminalSelectUpdate` for each | FR-013a, D2 | example (mutant: the pane never publishes an update) | PENDING | |
| U8  | With a selection already held, a left press and release delivered together write nothing to the clipboard | FR-013e, D4 | example | PENDING | |
| U9  | A left release publishes `TerminalSelectionReleased`, after its press's `TerminalSelectStart` | FR-013, D4 | example | PENDING | |

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
