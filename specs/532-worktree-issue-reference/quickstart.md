# Quickstart

## Part A: automated

1. `mise run test-core`: catalog and `initialize` instruction tests (T1, T2, T4).
2. `cargo test -p micold-daemon --test mcp_create_worktree` (via `mise run gate` for the full set):
   parity and tag test (T3), plus the unchanged literal-call tests.

## Part B: by hand (optional, no visual pass needed)

1. Run the app, connect a session, ask it to "create a worktree for issue #123".
2. Check the worktree row shows `#123` and the issue filter lists it.
3. Open `docs/user-guide/agent-tools.md` and confirm the convention and `ticket` input are described.

No UI changes, so there is no quickstart §B visual pass.
