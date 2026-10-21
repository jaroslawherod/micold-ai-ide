# Quickstart: validation

A (automated): `mise run test-core` (guard decisions, path forms, shell analysis), `cargo test -p micold-daemon` (settings JSON, receiver deny, opt-out, result text), then `mise run gate`.

B (manual, real `claude`): start the daemon, launch a session in worktree A, create worktree B with `create_worktree`.
1. Ask the session to edit a file in B: denied with the `create_session` message.
2. Ask it to run `git -C . worktree add ../x` and to use the enter-worktree tool: both denied.
3. Edit in A, under `target-shared/`: allowed.
4. Settings: turn the guard off, launch a new session, edit in B: allowed; the earlier session is still guarded.
5. `create_worktree` then `create_session` for B works; the result text carries the hand-off sentence.
Existing project rules in `.claude/settings.json` still apply in the session (SC-007).
