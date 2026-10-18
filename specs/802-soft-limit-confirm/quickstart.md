# Quickstart: 802

Automated (no display):
- `mise run test-core` : tool description text.
- `cargo test -p micold-client` via `mise run test` : confirmation opens for an occupied start and for a stale `Refused` reply; confirm sends one flagged create with the chosen CLI; cancel sends nothing.
- `mise run gate` before pushing.

Manual (optional, Part B): in a worktree with a session, create a second session from another window or via `create_session` with the flag, then press the start action in a window that has not redrawn: the confirmation names the existing session; Cancel changes nothing; Confirm shows two running sessions with the original still main. An agent `create_session` without the flag is refused and opens no dialog.
