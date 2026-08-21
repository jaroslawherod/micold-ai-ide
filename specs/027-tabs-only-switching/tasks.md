---

description: "Task list for feature 027 — Tabs Are the Only Switcher"
---

# Tasks: Tabs Are the Only Switcher

**Input**: [spec.md](./spec.md)

**Prerequisites**: none beyond the shipped feature 026. This feature has no `plan.md`,
`research.md` or `contracts/`: it is one arrangement and two defects in code that already exists,
which is the "bounded" classification the 2026-08-21 clarification recorded. The design decisions
that would have gone in those documents are written where they are load-bearing —
`ui/terminal.rs`'s bar comments carry the alignment mechanism and why a `Fill` cannot be used for
it, and `shell/daemon_sync.rs`'s carry both defects and why each was invisible.

**Tests**: Per Constitution Principle I (NON-NEGOTIABLE), every implementation task is preceded by a
failing test. Both defects here are reachable without a renderer — they are message routing and a
reducer — and the arrangement is reachable through the layout-record gates.

**Documentation**: Per Principle VII, the specs this contradicts are amended in this change rather
than left to be read as still true (Phase 4).

**Cross-platform**: Per Principle VI, nothing here is platform-specific. No new widget, no new
platform call; the padding that right-aligns the strip is the same primitive on all three.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1, US2 — from `spec.md`

---

## Phase 1: US1 — the tabs actually switch

**Purpose**: The two defects. They come first because the toggle cannot be deleted until the tabs
work without it, and both are invisible while it is there.

- [X] T001 [US1] Failing test in `crates/micold-client/src/shell/daemon_sync.rs`'s `mod tests`:
  `selecting_the_ai_tab_attaches_the_ai_process` — from a session displaying a shell, handling
  `Message::TerminalAiCliSelected` sends the daemon a `SessionAttachProcess` naming the session's
  primary process. Fails on `main`: the message has no handler, so nothing is sent (FR-006).
- [X] T002 [US1] Route `Message::TerminalAiCliSelected` to a new
  `shell::daemon_sync::on_terminal_ai_cli_selected` in `crates/micold-client/src/main.rs`, which
  runs the pure reducer and then attaches. The catch-all it used to fall to runs only the reducer,
  which is the whole defect.
- [X] T003 [US1] Failing test in `crates/micold-client/tests/app_state.rs`:
  `selecting_a_terminal_tab_shows_that_terminal` — from the AI pane, `ShellInstanceSelected` leaves
  the session in `TerminalMode::Regular` with that instance active. Fails on `main`: only
  `active_shell` is written (FR-005).
- [X] T004 [US1] In `crates/micold-client/src/features/session.rs`, make `shell_instance_selected`
  set the mode as well as the instance.
- [X] T005 [US1] Failing test in `crates/micold-client/src/shell/daemon_sync.rs`'s `mod tests`:
  `opening_a_terminal_from_the_ai_pane_switches_to_it` — the "+" opens an instance and displays it
  from either pane (FR-004).
- [X] T006 [US1] In `crates/micold-client/src/shell/daemon_sync.rs`, drop the Regular-mode gate from
  `on_shell_instance_open_requested`, set the mode, and run the reducer.

## Phase 2: US1 — the toggle goes

**Purpose**: Delete the control and everything that existed only to draw it. Nothing here is
reachable until Phase 1 is green.

- [X] T007 [US1] Delete the mode toggle from `pane`'s bar in `crates/micold-client/src/ui/terminal.rs`,
  along with `session_mode` (FR-001).
- [X] T008 [US1] Delete `Message::TerminalModeToggled` and its reducer arm from
  `crates/micold-client/src/app.rs`, and `features::session::mode_toggled` from
  `crates/micold-client/src/features/session.rs`. Leave the explanatory comment: a deleted variant
  with no note reads as an oversight.
- [X] T009 [P] [US1] Delete `mode_glyph` and `mode_tooltip` from
  `crates/micold-client/src/icons.rs`, and the tests that named them in
  `crates/micold-client/tests/session_terminal_mode.rs`.
- [X] T010 [P] [US1] Repoint `crates/micold-client/tests/terminal_focus.rs`'s navigation table at
  `TerminalAiCliSelected`, and replace `app_state.rs`'s two toggle tests with
  `the_two_kinds_of_tab_move_the_session_between_its_panes`.

## Phase 3: US2 — the arrangement

**Purpose**: The order and the alignment, both asserted from layout records.

- [X] T011 [US2] Failing test in `crates/micold-client/src/ui/terminal.rs`'s `mod tests`:
  `the_strip_hugs_the_trailing_edge_of_its_viewport` — `natural_strip_width` and `leading_slack` as
  pure arithmetic, including the two cases that make the alignment terminate: zero tabs, and a
  content width past the viewport (FR-003).
- [X] T012 [US2] Add `natural_strip_width`, `leading_slack` and `right_aligned_tabs` to
  `crates/micold-client/src/ui/terminal.rs`, spending the slack as the wrapper's **padding**. A
  `Space` of `Fixed(0.0)` is void and iced drops a void child, which would make the strip's index
  vary with the slack — the positional-diff defect of feature 023 FR-008a (FR-007).
- [X] T013 [US2] Push the "+" unconditionally and **before** the AI tab in
  `crates/micold-client/src/ui/terminal.rs`, so the AI tab is the bar's last child (FR-002).
- [X] T014 [US2] New gate `crates/micold-client/tests/gates/tabs_anchor_the_trailing_edge.rs`,
  compiled into `layout_snapshot`: `the_ai_tab_is_the_last_thing_in_the_bar` and
  `the_terminal_tabs_meet_the_trailing_controls`, both read from coordinates (FR-002, FR-003).
- [X] T015 [US2] In `crates/micold-client/tests/support/covered_states.rs`: delete the three
  `mode_toggle` anchors, add `terminal.add_instance`, move the pinned-tab anchor to the bar's new
  last index, and add a level to the strip paths for the slack container. Give the two
  strip-bearing states a `tab_strip_viewport_width` — without one the fixture renders the strip
  flush left and T014 could never see the requirement.
- [X] T016 [US2] Give `session-terminal-bottom-bar` its own two path constants. Its session offers a
  restart, which is a bar child of its own, so every index after it moves down one — and the
  anchors it borrowed before this feature were resolving to the *scrolling region* rather than to
  the AI tab, which `gates/tab_children_fit.rs` had been measuring as a tab.
- [X] T017 [US2] Repoint `gates/bar_controls_hold_their_size.rs`'s cross-state width comparison from
  the deleted toggle to the "+", and descend through the slack container in
  `gates/containment.rs`'s tab-overflow attribution.
- [X] T018 [US2] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with
  `UPDATE_LAYOUT_SNAPSHOT=1`.

## Phase 4: Cross-cutting — the specs that now say something untrue

**Purpose**: Principle VII. Four shipped specs describe the toggle as present and working; one
requires the opposite alignment. Amended in place with a superseded-by note, not rewritten.

- [X] T019 [P] Amend `specs/010-regular-terminal-mode/spec.md` — FR-002 and FR-009 (the toggle's
  existence, and its double duty as the pane's mode indicator), and the User Story 3 scenarios that
  read it.
- [X] T020 [P] Amend `specs/012-multiple-regular-terminals/spec.md` — FR-006 and FR-007 (the
  primary toggle "MUST continue to work as it does today"), and FR-001/FR-019 with the clarification
  and assumptions that scope the "+" and the Ctrl+Shift+T chord to Regular mode.
- [X] T021 [P] Amend `specs/026-ai-session-tab/spec.md` — FR-008 ("the toggle continues to work"),
  FR-002c's mention of the toggle among the bar's controls, FR-009's "the same glyph the mode toggle
  shows", the User Story 2 scenario that presses it, and the assumption that guessed "at the right
  side" was about the AI tab's place in the strip rather than the strip's own alignment — which is
  where the alignment claim actually lived, not in FR-002c as this task first said.
- [X] T022 Delete `TerminalMode::other()` from `micold-core` and its test. Its doc comment read "the
  mode a single toggle press switches *to*", and the toggle was its only caller — a tab names the
  mode it selects outright.

## Phase 5: Verification

- [X] T023 Full `mise run test` — 2044 tests across 209 binaries, 0 failed.
- [X] T024 Visual pass at a display: the bar's trailing group at zero, one, three and six
  instances, in both schemes. Run headlessly with the `visual-pass` skill and recorded in
  `visual-pass.md`; FR-001–FR-004 and SC-003 confirmed, three defects found and fixed below.

## Phase 6: What the visual pass found

**Purpose**: three defects, none of which any gate could see, and each gated before it was fixed.
The first is this feature's own; the other two are older code that FR-002's arrangement made
reachable. Recorded as tasks rather than folded into T024 so that each names the gate that now
holds it.

- [X] T025 The strip's edge fade drew at the trailing edge of a bar that did not overflow — the
  first instance opened, and a rule appeared beside it. It derived from a *measured* content width
  paired with a live viewport, two numbers from different frames. `strip_overflow` now derives it
  from the same source the layout does (the tab count), `Message::TabStripScrolled` carries two
  numbers instead of three, and `State::tab_strip_content_width` is deleted. Unit-tested in
  `ui/terminal.rs`.
- [X] T026 [FR-008] The terminal tabs sat 4dp above the "+" and the AI tab. `EdgeFade` boxes its
  content to `MIN_TOUCH_TARGET` so the fade spans the whole edge, and a container's default
  `align_y` is `Start`; the fix is one `.align_y(Center)`. Gated first by
  `gates/tabs_anchor_the_trailing_edge.rs`'s third test, which compares the strip's midline with
  each trailing control's. Regenerate the layout fixture: 150 nodes shift exactly +4 in y, no x or
  width changes.
- [X] T027 [FR-009] Pressing "+" for the sixth instance created a tab, marked it, and left it behind
  the trailing fade. Two of the four arms that move the mark never called `arm_tab_reveal` —
  `ShellInstanceOpenRequested` and `ShellInstanceCloseRequested`. The open arm is routed to a new
  `session::shell_instance_open_requested` rather than doing its work inline in `app.rs`, matching
  its three siblings. Gated by `tests/tab_reveal.rs`, which asserts the invariant over the set of
  arms rather than the one case, since the doc comment on `arm_tab_reveal` always claimed the set.
- [X] T028 Full `mise run test` after the three fixes.
