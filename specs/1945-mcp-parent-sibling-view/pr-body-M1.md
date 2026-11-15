Part of #1945 (M1, T001-T005). Design PR: #1947.

## What
- `micold-core::worktree_parent::neighbourhood()` (pure): parent, siblings and direct children of a worktree, a worktree with no recorded parent counting as a child of `default` for visibility only.
- MCP `Scope::new` is rebuilt on it: an unparented session sees `default`, its siblings (mutual with recorded-`default` children) and its own children; a caller whose worktree was removed sees only itself; `default` stays unrestricted.
- No parent link is recorded and listings still omit `parent` for such worktrees.
- User guide "Which worktrees a session can see" rewritten.

## Tests
- core: `scripts/build-lock.sh cargo test -p micold-core worktree_parent`
- daemon: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent` (the 1857 expectations for an unparented caller amended in place)
- `scripts/gate.sh`

Closes nothing yet; M2 and M3 follow.

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvKNynWdKUiNRXgAUic
