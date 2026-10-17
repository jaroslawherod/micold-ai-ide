# TDD cycle log — 813 worktree overview

No `tdd/test-list.md` was planned for this feature (as for 905); the tasks' test-first order is the
list. Append only.

## M1 / T006–T008 → T015 (core card rules)

- **Tests**: `crates/micold-core/src/overview.rs` `tests` (15 tests: location type precedence and
  words, `card_state` exhaustive over 204 signal combinations against the table, invariants,
  `session_counts`, `order`, `attention`).
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --lib overview::` with every function
  body stubbed `todo!()` → `test result: FAILED. 0 passed; 15 failed` (`panicked at
  crates/micold-core/src/overview.rs:43:21: not yet implemented`).
- **Green**: implementation restored → `test result: ok. 15 passed; 0 failed`.
- **Notes**: deviation — the implementation was drafted before the tests and stashed for the red
  run, so the red is a stub red, not an assertion red.

## M1 / T002 → T003 (warning and success roles)

- **Tests**: `crates/micold-core/src/tokens/css.rs` `COLOR_ROLES` + 8 names;
  `crates/micold-client/src/ui/material/composition_contrast.rs`
  `warning_and_success_roles_are_legible_on_their_fills`.
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --lib tokens::` →
  `every_colour_role_is_emitted_as_a_hex_triple ... FAILED` (`light: --micold-warning is not emitted`).
- **Green**: roles + palette tones added → `test result: ok. 11 passed; 0 failed`. The client
  contrast test was not yet run (client build in progress at handover).
