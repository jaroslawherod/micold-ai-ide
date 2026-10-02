# Cycle Log: Notify When a Session Needs Attention, and Track Unread Sessions

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 4004 passed, 1 failed, 7 ignored in the 315 binaries that ran; the run stopped at `micold-daemon --test mcp_create_session` (`a_pi_session_start_event_makes_pi_ready`, `left: ""` at line 600), so the binaries after it did not run. `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_create_session` alone -> 19 passed, 0 failed. The branch changes only `specs/` at this point, so the failure predates the feature; it is listed in the ledger's follow-ups.
- commit: `a8521731` (test list planned against it)
- recorded: cycle 0, before any change. Baseline `unknown`: the first M1 cycle re-measures on its own base before the loop starts.
