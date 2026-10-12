# Quickstart: validating one session per worktree

Contracts: [wire](contracts/wire.md), [MCP tools](contracts/mcp-tools.md). Rule:
[data-model](data-model.md).

## Part A: automated

```sh
mise run test-core                       # occupancy rule, wire additions, tool schemas and text
cargo test -p micold-daemon --test one_session_per_worktree   # new daemon integration file
mise run gate                            # fmt, clippy, workspace tests, scripts
```

| Check | Spec |
|---|---|
| second `create_session` on a worktree with a session in each lifecycle state: error names id + name + hint, catalog unchanged | US1 s1-s2, SC-002 |
| 100 concurrent `create_session` on an empty worktree: exactly one `Ok` | US1 s3, SC-003 |
| `SessionCreate` RPC from a stale window refused with the same text | US1 s1 |
| `create_worktree` with `ai_cli` + `prompt`: one session, defined; without: none, `create_session` then works | US2 s2, s5 |
| missing CLI: no worktree, no session | US2 s4 |
| Default root: second session refused | FR-011 |
| stop, `start_session`, restart: same id, still one | US3 s1-s2, FR-008a |
| resume into an occupied worktree: refused without `replace`, swapped with it; old process gone | US3 s3-s4, FR-008b |
| catalog fixture with two sessions in one worktree: both listed after load, create refused, start refused while the other runs | US4, SC-005 |
| project open with two resumable provider sessions for one worktree: one catalogued, one listed resumable | FR-012 |
| review send into a worktree whose own session is stopped starts that session, adds none | R10 |
| sequence test over create/start/stop/restart/resume/delete from two connections never observes two un-archived sessions on a fresh catalog | SC-001 |

## Part B: visual pass (needs eyes at a display)

Run through the `visual-pass` skill against the client.

1. Open the create-worktree dialog: AI CLI select (default CLI preselected, "None" last) and a
   first-prompt field render within the dialog width in light and dark; confirm creates the
   worktree and the session starts (SC-004).
2. A worktree row with a session shows no start action; a worktree without one shows exactly one;
   the Default row follows the same rule.
3. Attach dialog: resuming into an occupied worktree shows the confirmation naming both
   sessions; cancel changes nothing.

Record the result in `tdd/` evidence as the other features do.
