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
