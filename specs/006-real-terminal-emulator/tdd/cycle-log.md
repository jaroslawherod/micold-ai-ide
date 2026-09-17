# Cycle Log: Real Terminal Emulator — BUG-007

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and failed
before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3085 passed, 0 failed, 6 ignored (311 test binaries)
- commit: `3e513a1f` (BUG-007 record and spec patch; no code for the fix yet)
- recorded: cycle 0, before any change, 2026-09-15
- per-cycle runs use the changed crate's tests (`-p micold-core` / `-p micold-daemon` /
  `-p micold-client`); the whole workspace is re-run before each commit that touches more than one
  crate and by `mise run gate` before the PR. The full suite takes ~6 minutes behind a shared build
  lock, so it is not re-run after every refactor move.

## Cycle 1: U8 a dark terminal defaults to on_surface over surface

- test: `crates/micold-core/tests/tokens.rs::a_dark_terminal_defaults_to_on_surface_over_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test tokens a_dark_terminal_defaults_to_on_surface_over_surface -- --exact`
  -> `panicked at crates/micold-core/src/tokens/mod.rs:356:5: not yet implemented: BUG-007 U8` (1 failed).
  The symbol was declared first as a `todo!()` stub, so the red is the not-implemented signal, not a
  compile error.
- green: fake it — `terminal_defaults` returns `DARK.on_surface` / `DARK.surface` for any scheme; U9
  forces the generalisation. Suite `scripts/build-lock.sh cargo test -p micold-core --all-targets`
  -> 1059 passed, 0 failed
- refactor: none — the fake is the cycle's intended state
- notes: the first attempt at this cycle hit `ENOSPC` (disk at 2 MB free) before compiling; space was
  reclaimed (shared `target-shared/debug/incremental`) and the cycle re-run from the start.

## Cycle 2: U9 a light terminal defaults to on_surface over surface

- test: `crates/micold-core/tests/tokens.rs::a_light_terminal_defaults_to_on_surface_over_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test tokens a_light_terminal_defaults_to_on_surface_over_surface -- --exact`
  -> `assertion left == right failed … left: TerminalDefaults { foreground: Rgb { r: 230, g: 225, b: 230 }, background: Rgb { r: 20, g: 19, b: 22 } }`
  (1 failed — the U8 fake answers dark)
- green: `crates/micold-core/src/tokens/mod.rs` `terminal_defaults` reads `roles(scheme)`. Suite
  `scripts/build-lock.sh cargo test -p micold-core --all-targets` -> 1060 passed, 0 failed
- refactor: separate structural commit — `TermPalette::from_scheme` (client) takes its `fg`/`bg` from
  `terminal_defaults` instead of naming the roles itself (T075)
  — done: `crates/micold-client/src/ui/terminal.rs` `TermPalette::from_scheme`; suite
  `scripts/build-lock.sh cargo test -p micold-client` -> 1696 passed, 0 failed (`style_snapshot`
  unchanged, no regeneration). T075 ticked (U8, U9 DONE).

## Cycle 3: U1 a dark pane answers a background query with the dark surface

- test: `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_background_query_with_the_dark_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries a_dark_pane_answers_a_background_query_with_the_dark_surface -- --exact`
  -> `left: "\u{1b}]11;rgb:e5e5/e5e5/e5e5\u{7}"  right: "\u{1b}]11;rgb:1414/1313/1616\u{7}"` (1 failed —
  the reported reason: xterm entry 7 for a dark pane). `TerminalColors` and the new
  `DaemonListener::new` parameter were added first as a stub the listener ignored, with its two
  callers (`supervisor.rs`, `tests/support/mod.rs`) passing `TerminalColors::default()`, so the red
  is the assertion and not a compile error (T074).
- green: `crates/micold-daemon/src/terminal.rs` — `TerminalColors` holds the scheme in an
  `Arc<AtomicBool>`; the `ColorRequest` arm answers `NamedColor::Background` from
  `tokens::terminal_defaults(scheme).background`, read at reply time. Suite
  `scripts/build-lock.sh cargo test -p micold-daemon` -> 331 passed, 0 failed
- refactor: none yet — the `tokens::Rgb` → `vte::Rgb` conversion is written once; extract it when a
  second role needs it

## Cycle 4: U2 a dark pane answers a foreground query with the dark on_surface

- test: `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_foreground_query_with_the_dark_on_surface` (new; queries with an `ST` terminator)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries a_dark_pane_answers_a_foreground_query_with_the_dark_on_surface -- --exact`
  -> `left: "\u{1b}]10;rgb:e5e5/e5e5/e5e5\u{1b}\\"  right: "\u{1b}]10;rgb:e6e6/e1e1/e6e6\u{1b}\\"` (1 failed)
- green: the `ColorRequest` arm answers `NamedColor::Foreground` from `terminal_defaults(scheme).foreground`.
  Suite `scripts/build-lock.sh cargo test -p micold-daemon` -> 332 passed, 0 failed
- refactor: separate structural commit — the `tokens::Rgb` → `vte::Rgb` conversion, now written twice,
  becomes one function and the `if` chain a `match` on the dynamic index

## Cycle 5: U3 a dark pane answers a cursor query with the dark on_surface

- test: `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_cursor_query_with_the_dark_on_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries a_dark_pane_answers_a_cursor_query_with_the_dark_on_surface -- --exact`
  -> `left: "\u{1b}]12;rgb:e5e5/e5e5/e5e5\u{7}"  right: "\u{1b}]12;rgb:e6e6/e1e1/e6e6\u{7}"` (1 failed)
- green: `TerminalColors::dynamic` answers `NamedColor::Cursor` with the foreground. Suite
  `scripts/build-lock.sh cargo test -p micold-daemon` -> 333 passed, 0 failed
- refactor: none needed, one match guard widened

## Cycles 6–9: U4–U7 the answer follows the scheme; palette queries are unchanged

These four pin behaviour cycles 3–5 already built (the listener reads `TerminalColors` at reply
time, and `dynamic` only sees indices past the palette), so each passed on first run. A passing
first run is not red evidence, so each was proved by mutating the production code instead.

- tests (new, `crates/micold-daemon/tests/vt_color_queries.rs`):
  U4 `a_light_pane_answers_a_background_query_with_the_light_surface`,
  U5 `a_scheme_changed_after_the_listener_was_built_answers_the_next_query`,
  U6 `before_any_scheme_is_reported_the_answer_is_light`,
  U7 `a_palette_query_is_still_answered_from_the_xterm_table`
- red (mutation, U4–U6): `TerminalColors::scheme` forced to `Dark` →
  `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries` -> 3 failed, each
  `left: "\u{1b}]11;rgb:1414/1313/1616\u{7}"` (the dark surface where light was expected); U1–U3, U7 passed
- red (mutation, U7): `TerminalColors::dynamic` answers the foreground for any index instead of
  `None` → 1 failed, `left: "\u{1b}]4;1;rgb:1c1c/1b1b/1e1e\u{7}"  right: "\u{1b}]4;1;rgb:cdcd/0000/0000\u{7}"`
- green: mutations reverted. Suite `scripts/build-lock.sh cargo test -p micold-daemon` -> 337 passed, 0 failed
- refactor: none

## Cycle 10: U10 `TerminalColorScheme` round-trips through the wire codec

- test: `crates/micold-core/tests/protocol_roundtrip.rs` `sample_client_msgs` gains
  `TerminalColorScheme { Dark }` and `{ Light }`; the version pins move with it —
  `protocol_auth.rs::the_protocol_version_is_thirteen` (renamed) and `schema_hash.rs`'s
  `FEATURE_026_PROTOCOL_VERSION` = 13
- red: the variant and `ColorScheme`'s serde were added first as a stub with the version left at 12, so
  the red is the wire-change gate, not a compile error.
  `scripts/build-lock.sh cargo test -p micold-core --test protocol_auth --test schema_hash --test protocol_roundtrip`
  -> `the_protocol_version_is_thirteen … left: 12 right: 13` (1 failed)
- green: `PROTOCOL_VERSION` 12 → 13 with its doc line; `server.rs` gains an arm that ignores the
  message until T078 wires it. Suite `scripts/build-lock.sh cargo test -p micold-core --all-targets`
  -> 1088 passed, 0 failed; `cargo check -p micold-daemon -p micold-client --all-targets` clean
- refactor: none (`cargo fmt` also rewrapped cycle 4's `ColorRequest` arm)

## Cycle 11: U11, A1, A2 a reported scheme reaches every session's answers

- tests (new, `crates/micold-daemon/tests/terminal_color_scheme.rs`):
  U11 `a_reported_scheme_becomes_the_services_scheme_and_the_last_report_wins` (a real
  `serve_connection` over a duplex; `Ping`/`Pong` after the unacknowledged report as the sync point);
  A1+A2 `a_session_answers_the_background_query_with_the_scheme_the_window_reported` — a regular
  session started through `DaemonState::start_session`, whose shell runs
  `stty raw -echo; printf '\033]11;?\007'; dd bs=1 count=24 | tr '\033\007' EB` and prints the reply
  on the grid; dark first, then light on the same running session
- red: `DaemonState::terminal_colors()` and its field were added first, with `server.rs`'s arm still
  ignoring the message, so the red is the answer.
  `scripts/build-lock.sh cargo test -p micold-daemon --test terminal_color_scheme` -> 2 failed:
  U11 `left: Light right: Dark`; A1 `must read E]11;rgb:1414/1313/1616B; screen: E]11;rgb:fdfd/f8f8/fdfdB…`
  (the session answered light after the window reported dark — the reported bug, end to end)
- green: `server.rs` sets `state.terminal_colors()` from the message; `PtySession::spawn_ai_cli` /
  `spawn_shell` take `&TerminalColors`, passed from `DaemonState`'s three spawn sites, through the new
  `PtySession::spawn_answering`; `PtySession::spawn` keeps its signature (light answers) so its direct
  test callers are unchanged. Suite `scripts/build-lock.sh cargo test -p micold-daemon` -> 343 passed, 0 failed
- refactor: none

## Cycle 12: U12–U15 the window reports its scheme on connect and on change

- tests (new, `crates/micold-client/src/shell/daemon_sync.rs` tests, over a real `Outbox`, every
  message delivered through the binary's `update`):
  U12 `connecting_reports_the_resolved_scheme_before_attaching`,
  U13 `a_message_that_changes_the_scheme_reports_it_once` (`SystemThemeChanged(Light)` under
  `FollowSystem`), U14 `a_message_that_keeps_the_scheme_sends_no_report` (`SystemThemeChanged(Dark)`),
  U15 `a_reconnect_reports_again_although_the_scheme_did_not_change`
- red: `App::reported_scheme` added first as an unread stub.
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide scheme` -> 3 failed:
  U12 `sent [AiCliAvailabilityRequest { req: 0 }, Attach { … }, SetViewedSession { … }]`,
  U13 `left: [] right: [Light]`, U15 `left: [] right: [Dark]`. U14 passed vacuously (nothing is ever sent)
- green: `daemon_sync::report_color_scheme` sends `TerminalColorScheme` when the resolved scheme
  differs from `App::reported_scheme`; `on_connected` clears it and reports right after
  `app.daemon = Some(outbox)`, before `ask_cli_availability` and the attach; `on_disconnected` clears
  it; `main.rs`'s `update` calls it after every `update_inner`
- red (mutation, U14): the unchanged-scheme guard disabled -> U14 `left: [Dark] right: []`, U15
  `left: [Dark, Dark] right: [Dark]` (2 failed); reverted
- suite: `scripts/build-lock.sh cargo test -p micold-client` -> 1705 passed, 0 failed
- refactor: none

---

# Cycle Log: BUG-008 — a single click in the terminal selects nothing

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: CI (`ci complete`, full workspace on Linux plus the three-platform client run) on
  `origin/main` `226d3a8b` -> `success`. The local full suite was not run before the first test;
  `mise run gate` runs it on the finished change.
- commit: `226d3a8b`
- recorded: cycle 0, before any change

## Red: T081, the regression tests (one Red commit)

Deviation, recorded up front: this repository commits a task's failing tests as one Red commit and
the fix as a separate Green commit (tasks.md Phase 15 purpose; feature 029's `test(029): … (Red)`
commits). The five behaviours that fail on `origin/main` were therefore written and observed red
together, before any implementation existed, and share one Green change (T082). The one declaration
the pane tests need to compile, `SessionMsg::TerminalSelectionReleased` (a variant with a no-op arm
and no sender), was added with the tests.

- U16 `crates/micold-client/src/selection.rs::tests::a_click_without_a_drag_selects_nothing` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib selection::tests`
    -> `panicked at crates/micold-client/src/selection.rs:427:9: a click is not a drag: no cell may be highlighted (FR-013e)` (11 passed; 1 failed)
- U17 `selection.rs::tests::an_update_onto_the_start_anchor_selects_that_cell` (new) — passes on
  `origin/main`, as planned (`example (mutant)`); its red is taken against a mutant after T082.
- U18 `selection.rs::tests::a_drag_out_and_back_selects_the_pressed_cell` (new) — passes on
  `origin/main`, as planned; red against a mutant after T082.
- U19 covered by existing tests `selection.rs::tests::word_expansion_selects_whole_word` and
  `line_granularity_selects_whole_line` (both assert a `Word`/`Line` selection started with no
  update selects the word and the line, and both pass). No new test.
- U20 `crates/micold-client/tests/clipboard_request.rs::a_click_without_a_drag_asks_for_nothing` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --test clipboard_request`
    -> `left: Some(ClipboardWrite("w"))  right: None` (14 passed; 1 failed)
- U21 `crates/micold-client/src/ui/material/terminal_pane.rs::tests::clipboard_gestures::pointer_jitter_inside_the_pressed_cell_is_not_a_drag` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures`
    -> `left: [(6, 0), (6, 0)]  right: []` (7 passed; 3 failed)
- U22 `clipboard_gestures::motion_into_another_cell_and_back_extends_the_selection_each_time` (new)
  — passes on `origin/main`, as planned; red against a mutant after T082.
- U23 `clipboard_gestures::a_tap_over_a_held_selection_writes_nothing_to_the_clipboard` (new)
  - red: same run -> `a press and release delivered together copied ["hello world"] — the selection from before the press — over the user's clipboard (FR-013e, BUG-008)`
- U24 `clipboard_gestures::a_release_asks_for_the_copy_after_its_press_starts_the_selection` (new)
  - red: same run -> `… start at Some(0), release request at None`

## Green: T082, the fix (one Green commit)

- change: `Selection` gains `extended`, false from `start` and set by any `update`; a `Char`
  selection that was never extended contains no cell and yields empty text. The pane records the
  pressed cell and publishes no `TerminalSelectUpdate` while the pointer stays in it; once it has
  left, every cell publishes. The left release no longer writes `self.selection` to the clipboard: it
  publishes the new `SessionMsg::TerminalSelectionReleased`, and `main.rs` hands that to
  `shell::clipboard::on_selection_released`, which copies the shell's current selection through
  `selection_copy_request` and leaves the context menu alone.
- green: `scripts/build-lock.sh cargo test -p micold-client` -> 1696 passed; 0 failed (every
  target of the client crate, the five reds above included). `mise run gate` runs the workspace
  before the push.
- U16, U20, U21, U23, U24: red above, green in this run.
- U19: already covered, unchanged and green.
- Deliberate mutants for the three behaviours that passed on `origin/main`, each applied alone to
  the fixed code, run, and reverted by copying the file back (byte-compared):
  - U17, mutant `self.extended = anchor != self.start;` in `Selection::update`:
    `scripts/build-lock.sh cargo test -p micold-client --lib selection::tests::an_update_onto_the_start_anchor_selects_that_cell`
    -> `panicked at crates/micold-client/src/selection.rs:462:9` (the "a drag that ends on its start
    cell selects that cell" assertion) (0 passed; 1 failed)
  - U18, mutant `is_empty` also true while `self.current == self.start`:
    `… --lib selection::tests::a_drag_out_and_back_selects_the_pressed_cell`
    -> `panicked at crates/micold-client/src/selection.rs:475:9` (the "returning to the pressed cell
    must not turn a drag back into a click" assertion) (0 passed; 1 failed)
  - U22, mutant `if true {` in place of the pressed-cell check, so the pane never publishes an update:
    `… --lib clipboard_gestures::motion_into_another_cell_and_back_extends_the_selection_each_time`
    -> `left: []  right: [(7, 0), (6, 0)]` (0 passed; 1 failed)
- refactor: none. The copy chord still uses `selectable_content`, so nothing became dead. The
  `SessionMsg` doc comments' arm and transition counts, already stale on `origin/main`, were
  corrected alongside the new variant's handling.
- commit: the `fix(006)` Green commit that carries this entry (the SHA is in `git log`; a commit
  cannot name its own hash).

## Acceptance: A3, the visual pass (T083)

- A3 [evidence/bugfix-008-pass-2026-09-14.md](../evidence/bugfix-008-pass-2026-09-14.md). This is not
  an automated test: the profile's acceptance runner has no GUI. On `origin/main` `226d3a8b`, a
  synthetic click and a click at human pace each left the `w` highlighted, and the slow one pasted
  `w` (red). On this branch's build, neither click leaves a highlight, the dragged `hello` is pasted,
  and a synthetic click after a drag leaves another client's `OTHER-TEXT` on the clipboard (green).
- commit: the `docs(006)` T083 commit that carries this entry.

## Red: U25 and U26, found by code review A (one Red commit)

Review A (the `code-review` skill at `high`) reported two ways the pane's pressed-cell check (U21)
treats a drag as a click. First, a wheel scroll while the button is held puts other text under an
unmoved pointer. Second, a position outside the pane clamps back onto a pressed edge cell. Both
were added to the list as behaviours and written as tests before any change.

- U25 `terminal_pane.rs::tests::clipboard_gestures::motion_after_a_scroll_while_held_is_a_drag_even_in_the_pressed_screen_cell` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures`
    -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1751:13` `left: []  right: [(6, 0)]` (10 passed; 2 failed)
- U26 `clipboard_gestures::motion_past_the_panes_edge_is_a_drag_even_where_it_clamps_to_the_pressed_cell` (new)
  - red: same run -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1775:13` `left: []  right: [(0, 0)]`

## Green: U25 and U26

- change: the pane's wheel `ScrollLocally` arm clears the pressed cell. The `CursorMoved` arm treats
  a position outside the content area as having left the pressed cell.
- green: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures` -> 12 passed;
  0 failed. `mise run gate` runs the full suite before the push.
- refactor: none. With the same commit, and at Review B's request, one comment in the copy-chord arm
  now points at `selection::copy_request` instead of the pane's old release write.
- commit: the `fix(006)` commit that carries this entry.

## Red: U27 and U28, found by code reviews A and B, round 2 (one Red commit)

Round 2 of both reviews found that the U26 fix treats every position outside the character area
as having left the pressed cell. A press in the pane's focus gutter, which the pane still accepts
and `grid_at` clamps onto the edge cell, therefore became a drag on the first jitter. Review A also
found that the U25 fix clears the pressed cell on a wheel turn that cannot move the view. Both were
added to the list and written as tests before any change.

- test fixture (a stated test change, taken before the implementation): U25 now runs over
  `grid_with_history(0, 10)`, because its premise is a wheel turn that moves the view. On a grid with
  no scrollback that turn moves nothing, which is U28. `grid(mode)` delegates to the new helper
  unchanged. U25 still passes on the unchanged code.
- U27 `terminal_pane.rs::tests::clipboard_gestures::jitter_in_the_focus_gutter_beside_the_pressed_edge_cell_is_not_a_drag` (new)
  - red: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures`
    -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1812:13` `left: [(0, 0), (0, 0)]  right: []` (12 passed; 2 failed)
- U28 `clipboard_gestures::a_wheel_turn_that_cannot_scroll_leaves_jitter_a_click` (new)
  - red: same run -> `panicked at crates/micold-client/src/ui/material/terminal_pane.rs:1839:13` `left: [(6, 0)]  right: []`

## Green: U27 and U28

- change: in the pane's `CursorMoved` arm, only a position outside the pane's `bounds` (not its
  character area) leaves the pressed cell. The wheel `ScrollLocally` arm clears the pressed cell only
  when `(display_offset + lines).clamp(0, history_size)` differs from the offset, which is the clamp
  `main.rs` applies to `TerminalScrolled`.
- green: `scripts/build-lock.sh cargo test -p micold-client --lib clipboard_gestures` -> 14 passed; 0 failed.
  `mise run gate` runs the full suite before the push.
- refactor: none. With the same commit, the T081/T082 text and the plan and contract notes now name
  U25–U28 (Review B round 2, F2).
- commit: the `fix(006)` commit that carries this entry.

