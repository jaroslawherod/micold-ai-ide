---
feature: 006-real-terminal-emulator
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 2 # scoped to BUG-007: FR-003a, SC-014 (the rest of 006 was closed before this list existed)
planned_at: 3e513a1f
updated_at: 3e513a1f
suite_baseline: green # 3085 passed, 0 failed, 6 ignored at 3e513a1f
---

# Test List: Real Terminal Emulator — BUG-007 (default-colour queries follow the theme)

**Scope.** Feature 006 closed before this extension was installed, and this list was written for
bugfix BUG-007 only (`bugs/BUG-007.md`, tasks T074–T080). It covers `FR-003a` and `SC-014`. The
feature's other criteria shipped test-first before the list existed and are not re-derived here.

Derived from `spec.md` (FR-003a, SC-014, the 2026-09-14 Clarifications), `plan.md`'s BUG-007 note
and 010 `contracts/protocol.md` §8 / `contracts/messages.md` (`TerminalColorScheme`). It was not
derived from the code.

## Outer loop: acceptance behaviors

**Entry point.** No end-to-end GUI runner exists. The highest level a cargo test reaches is the
daemon: a real client connection to the service's connection handler (`server.rs`), a session started through `DaemonState` and
spawned under a real PTY, whose program writes `OSC 11 ; ?` and prints the reply it reads back. The
client half — that a window reports its scheme — is pinned by U12–U15 over a real `Outbox`. The
rendered result with `claude` is the T080 visual pass.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| A1  | After a window reports Dark, a program in a session started through the service asks `OSC 11` and reads the dark `surface` | FR-003a, SC-014 | example | DONE | `crates/micold-daemon/tests/terminal_color_scheme.rs::a_session_answers_the_background_query_with_the_scheme_the_window_reported` |
| A2  | When a window then reports Light, the same running program's next `OSC 11` reads the light `surface` | FR-003a, SC-014 | example | DONE | `crates/micold-daemon/tests/terminal_color_scheme.rs::a_session_answers_the_background_query_with_the_scheme_the_window_reported` |
| A3  | `claude` on theme `auto`, started in a dark-scheme terminal, draws legible body text | SC-014 | example | DONE | visual pass, T080 — `evidence/bug007-visual-pass.md` |

## Inner loop: unit behaviors

### `crates/micold-daemon/src/terminal.rs` — `DaemonListener`, `TerminalColors`

Tests in `crates/micold-daemon/tests/vt_color_queries.rs`: a `Term` wired with a real
`DaemonListener` over a writer the test keeps a handle to.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U1  | Under Dark, `OSC 11 ; ?` is answered with the dark `surface` | FR-003a | example | DONE | `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_background_query_with_the_dark_surface` |
| U2  | Under Dark, `OSC 10 ; ?` is answered with the dark `on_surface` | FR-003a | example | DONE | `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_foreground_query_with_the_dark_on_surface` |
| U3  | Under Dark, `OSC 12 ; ?` is answered with the dark `on_surface` (the cursor block is drawn in the foreground) | FR-003a | example | DONE | `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_cursor_query_with_the_dark_on_surface` |
| U4  | Under Light, `OSC 11 ; ?` is answered with the light `surface` | FR-003a | example | DONE | `crates/micold-daemon/tests/vt_color_queries.rs::a_light_pane_answers_a_background_query_with_the_light_surface` |
| U5  | A scheme changed on a listener already built is what its next query answers | FR-003a | example | DONE | `crates/micold-daemon/tests/vt_color_queries.rs::a_scheme_changed_after_the_listener_was_built_answers_the_next_query` |
| U6  | Before any scheme is set, the answer is the light scheme's | FR-003a | example | DONE | `crates/micold-daemon/tests/vt_color_queries.rs::before_any_scheme_is_reported_the_answer_is_light` |
| U7  | `OSC 4 ; 1 ; ?` is still answered with xterm red under either scheme | FR-003a ("Palette queries (`OSC 4`) are unchanged") | example | DONE | `crates/micold-daemon/tests/vt_color_queries.rs::a_palette_query_is_still_answered_from_the_xterm_table` |

### `crates/micold-core/src/tokens/mod.rs` — `terminal_defaults`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U8  | The default foreground and background are `on_surface` and `surface` of the dark roles under Dark | FR-003, FR-003a | example | DONE | `crates/micold-core/tests/tokens.rs::a_dark_terminal_defaults_to_on_surface_over_surface` |
| U9  | …and of the light roles under Light | FR-003, FR-003a | example | DONE | `crates/micold-core/tests/tokens.rs::a_light_terminal_defaults_to_on_surface_over_surface` |

### `crates/micold-core/src/protocol/messages.rs` — `ClientMsg::TerminalColorScheme`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U10 | `TerminalColorScheme` round-trips through the wire codec | FR-003a; 010 `contracts/messages.md` | example | DONE | `crates/micold-core/tests/protocol_roundtrip.rs` (`TerminalColorScheme` both schemes in `sample_client_msgs`) |

### `crates/micold-daemon/src/server.rs` + `state.rs`

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U11 | A `TerminalColorScheme` from a connected client becomes the service's scheme (last report wins) | FR-003a; 010 protocol §8 | example | DONE | `crates/micold-daemon/tests/terminal_color_scheme.rs::a_reported_scheme_becomes_the_services_scheme_and_the_last_report_wins` |

### `crates/micold-client/src/shell/daemon_sync.rs` + `main.rs` `update`

Tests in `daemon_sync.rs`'s `#[cfg(test)] mod tests` over a real `Outbox`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U12 | On connecting, the window reports its resolved scheme before its `Attach` | FR-003a | example | DONE | `crates/micold-client/src/shell/daemon_sync.rs::tests::connecting_reports_the_resolved_scheme_before_attaching` |
| U13 | A message that changes the resolved scheme sends exactly one new report | FR-003a | example | DONE | `crates/micold-client/src/shell/daemon_sync.rs::tests::a_message_that_changes_the_scheme_reports_it_once` |
| U14 | A message that leaves the resolved scheme unchanged sends no report | FR-003a | example | DONE | `crates/micold-client/src/shell/daemon_sync.rs::tests::a_message_that_keeps_the_scheme_sends_no_report` |
| U15 | A reconnect reports again although the scheme did not change | FR-003a | example | DONE | `crates/micold-client/src/shell/daemon_sync.rs::tests::a_reconnect_reports_again_although_the_scheme_did_not_change` |

## Invariants and edge cases still to place

None.

## Out of scope

- The reply's terminator (`ST` vs `BEL`) matching the query's: formatted by `alacritty_terminal`'s
  `ColorRequest` closure, which this fix does not touch. The tests query with both and would show a
  change, but no behavior on this list is about it.
- DEC private mode 2031 theme-change notifications (`CSI ? 997 ; 1|2 n`): spec Edge Cases, a
  follow-up (ledger D3).
- `OSC 4` indices 0–15 differing from the client's `STANDARD_ANSI16`: follow-up in `BUG-007.md`.
- Two windows reporting different schemes: not a state the application produces (Clarifications
  2026-09-14); last writer wins, covered by U11's wording, not a separate test.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` (detected at `cdc473ab`):

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact` (assert on the
  observed `N passed`: a filter matching nothing exits 0)
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Coverage: none (no `cargo-llvm-cov`)
- Mutation: none (no `cargo-mutants`); strength is checked by deliberate mutants by hand
- Property: none (no `proptest`)

In-source `#[cfg(test)]` modules run with `scripts/build-lock.sh cargo test -p <crate> --lib <path>`
(or `--bin micold-ai-ide` for `shell/` and `main.rs`).

---

<!-- BUG-008's list, kept separate from BUG-007's above: the two bugs were fixed on parallel
branches with their own baselines. Its own front matter: -->

- `loop: outside-in`
- `profile: .specify/memory/tdd-profile.md`
- `spec_criteria: 1 # scoped to BUG-008: FR-013e / SC-015 only — see "Out of scope"`
- `planned_at: 226d3a8b`
- `updated_at: fadb8e9f # the origin/main this branch was last rebased onto; states match the branch head`
- `suite_baseline: green # CI run on origin/main 226d3a8b (before the rebase onto fadb8e9f) concluded success; see cycle-log.md`

# Test List: BUG-008 — a single click in the terminal selects nothing

This feature shipped before the TDD extension existed and had no `tdd/` directory. This list is
written for **BUG-008 only** (Phase 15, T081–T083); the feature's earlier behaviours are not
re-planned here. Traces use `FR-`/`SC-` ids from [spec.md](../spec.md) and decision ids from
[BUG-008.autopilot.md](../bugs/BUG-008.autopilot.md).

`example (mutant)` marks a behaviour whose test passes on `origin/main` because the defect does not
break it; it guards the fix against over-reach, so its red is taken against the named deliberate
mutant after the fix exists, per the playbook.

**Acceptance runner.** The profile's acceptance runner is the sandbox real-runtime suite, which has no
GUI. A3 is therefore held by T083's recorded visual pass on Xvfb (the `visual-pass` skill), as feature
030 did; the unit behaviours below are the automated half.

## Outer loop: acceptance behaviors

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A3  | In a real Regular Terminal, a click at human pace after a drag leaves no highlight and the paste is still the dragged text; a synthetic press+release after a drag leaves other clipboard text in place; a drag still highlights and copies | FR-013e, SC-015 | visual pass | DONE | [evidence/bugfix-008-pass-2026-09-14.md](../evidence/bugfix-008-pass-2026-09-14.md) |

## Inner loop: unit behaviors

### `crates/micold-client/src/selection.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U16  | A `Char` selection started and never updated contains no cell and yields empty text | FR-013e | example | DONE | `crates/micold-client/src/selection.rs::tests::a_click_without_a_drag_selects_nothing` |
| U17  | A `Char` selection updated onto its own start anchor contains that cell and yields its character | FR-013e, D2 | example (mutant: empty while `current == start`) | DONE | `selection.rs::tests::an_update_onto_the_start_anchor_selects_that_cell` |
| U18  | A `Char` selection dragged into another cell and back contains the pressed cell and yields its character | FR-013e, D2 | example (mutant: empty until the moving end differs from the start) | DONE | `selection.rs::tests::a_drag_out_and_back_selects_the_pressed_cell` |
| U19  | `Word` and `Line` selections started without motion select the word and the line | FR-013b, FR-013e | example | DONE | `selection.rs::tests::word_expansion_selects_whole_word, line_granularity_selects_whole_line (existing)` |

### `crates/micold-client/tests/clipboard_request.rs` (the copy decision)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U20  | A click-only selection over text produces no copy request | FR-013e, FR-013c | example | DONE | `crates/micold-client/tests/clipboard_request.rs::a_click_without_a_drag_asks_for_nothing` |

### `crates/micold-client/src/ui/material/terminal_pane.rs` (`clipboard_gestures`)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U21  | After a left press, pointer motion that stays inside the pressed cell publishes no `TerminalSelectUpdate` | FR-013e, D5 | example | DONE | `terminal_pane.rs::tests::clipboard_gestures::pointer_jitter_inside_the_pressed_cell_is_not_a_drag` |
| U22  | After a left press, motion into another cell and back onto the pressed cell publishes a `TerminalSelectUpdate` for each | FR-013a, D2 | example (mutant: the pane never publishes an update) | DONE | `clipboard_gestures::motion_into_another_cell_and_back_extends_the_selection_each_time` |
| U23  | With a selection already held, a left press and release delivered together write nothing to the clipboard | FR-013e, D4 | example | DONE | `clipboard_gestures::a_tap_over_a_held_selection_writes_nothing_to_the_clipboard` |
| U24  | A left release publishes `TerminalSelectionReleased`, after its press's `TerminalSelectStart` | FR-013, D4 | example | DONE | `clipboard_gestures::a_release_asks_for_the_copy_after_its_press_starts_the_selection` |
| U25 | After a left press, a wheel scroll and then motion inside the pressed screen cell publish a `TerminalSelectUpdate` | FR-013e, D8 | example | DONE | `clipboard_gestures::motion_after_a_scroll_while_held_is_a_drag_even_in_the_pressed_screen_cell` |
| U26 | After a left press in an edge cell, motion outside the pane, which clamps back onto the pressed cell, publishes a `TerminalSelectUpdate` | FR-013e, D8 | example | DONE | `clipboard_gestures::motion_past_the_panes_edge_is_a_drag_even_where_it_clamps_to_the_pressed_cell` |
| U27 | A left press in the focus gutter beside an edge cell, then motion within that gutter or into that edge cell, publishes no `TerminalSelectUpdate` | FR-013e, D9 | example | DONE | `clipboard_gestures::jitter_in_the_focus_gutter_beside_the_pressed_edge_cell_is_not_a_drag` |
| U28 | After a left press, a wheel turn that cannot move the view (no scrollback) leaves motion inside the pressed cell a click | FR-013e, D9 | example | DONE | `clipboard_gestures::a_wheel_turn_that_cannot_scroll_leaves_jitter_a_click` |

## Invariants and edge cases still to place

None.

## Out of scope

- The shell's handling of `TerminalSelectionReleased` (`shell::clipboard::on_selection_released`):
  it calls `selection::copy_request` (U16, U20) and hands the outcome to `interpret`, whose "decides
  nothing" shape `clipboard_request.rs::the_shells_translation_decides_nothing` already holds. The
  `iced::Task` it returns cannot be inspected without a runtime; A3 exercises it end to end.
- Every behaviour of feature 006 before BUG-008: shipped and tested before this list existed.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`; `mise run gate` before pushing)
- Coverage: none (no tool)
- Mutation: none (no tool) — deliberate mutants by hand

Tests in a `#[cfg(test)] mod` inside `src/` are run through the client's lib target:
`scripts/build-lock.sh cargo test -p micold-client --lib {path} -- --exact`.
