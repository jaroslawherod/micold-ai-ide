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
| A3  | `claude` on theme `auto`, started in a dark-scheme terminal, draws legible body text | SC-014 | example | PENDING | visual pass, T080 — not a cargo test; `speckit-tdd-run` does not drive it |

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
