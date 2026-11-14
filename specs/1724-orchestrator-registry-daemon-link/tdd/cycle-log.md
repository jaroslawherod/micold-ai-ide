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

## M2 (T017-T025)

Gap, recorded honestly: no red phase was captured for T017-T022. The tests and code were written
together by an earlier unit and compiled afterwards (the shared build lock was held by other
worktrees for long stretches). T022's client test `a_link_ping_is_ignored` passes without the new
arm too, because the match keeps its `_ => {}` wildcard; the explicit arm is documentation.

Review A found a real bug the first tests missed: after `Expired` a closed transport never ended
the link loop, and a read error or `Goodbye` ended it without waiting for the deadline. Fixed in
`link.rs`; `a_link_that_drops_after_going_offline_can_be_relinked` covers the first case.
