# Quickstart: verify #552

## A. Automated

1. `mise run test-core`, then `mise run gate`.
2. Docs: `grep -rn "Show agent worktrees\|worktree-agent-\|provenance" docs/user-guide` returns
   nothing outside a superseded note.
3. Specs: `head -5 specs/014-hide-agent-worktrees/spec.md specs/029-worktree-provenance/spec.md
   specs/582-attach-provider-worktrees-sessions/spec.md` each show a superseded note naming 552.

## B. Visual pass

Open a project fixture with `agent-<17 hex>` worktrees: every worktree is listed, the sidebar has
no reveal chip, and the attach dialog has no worktree rows.

## C. Live hook check (needs a Claude CLI)

Start a Micold Claude session, ask it to run an Explore sub-agent; the session summary (or daemon
debug log) shows one `AgentRun` going `Running` then `Stopped`; the session's activity state does
not flip to idle when the sub-agent stops; restart the daemon and the run is still listed.
