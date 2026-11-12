# TDD cycle log, #1724

## M1 (T001-T016)

Gap, recorded honestly: no red phase was captured. Tests and implementation were written together
and uncompiled (other worktrees held the shared build lock for over 40 minutes, so no `cargo` ran in
the writing session). They were compiled and run afterwards.

Green evidence: `cargo test -p micold-core --lib -- orchestrator daemon_config` 146 passed;
`cargo test -p micold-daemon --test orchestrator` 11 passed; clippy `-D warnings` clean.

Compile errors found when first built (the closest thing to a red): a `todo!` left in
`preamble_expired`, and untyped `.into()` on codec sends in `link.rs` and the test support.

Review A found `no_child_process_is_spawned` vacuous (it read only the main thread's children); it
now sums the children of every task. M2 onward: write each behaviour's test, see it fail, then pass.
