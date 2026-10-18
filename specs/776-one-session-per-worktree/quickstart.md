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
| sequence test over create/start/stop/restart/resume/delete (no explicit request) from two connections never observes two un-archived sessions on a fresh catalog | SC-001 |
| resume with `allow_additional_session`: both kept, nothing stopped; flag with `replace`: refused as malformed | FR-015, FR-008c |
| `create_session` with `allow_additional_session: true` on an occupied worktree: two sessions, first untouched; refusal text names the flag | US1 s4-s5, FR-015 |
| `start_session` of a stopped extra while another runs: refused; with the flag both run | US4 s3 |
| `main_session`: marked, else first un-archived in list order, else none | FR-018, FR-020 |
| main set at create, helper beside it, `SessionSetMain` moves it and it survives a restart; archive of the main promotes the first remaining; legacy catalog derives the main unrewritten | US7, SC-008 |
| row press selects the main session whatever was last viewed; expander only for a multi-session worktree, collapsed, helpers marked | US5 |
| row menu "Start another session...": confirm sends the opt-in, cancel changes nothing | US6 |

## Part B: visual pass (needs eyes at a display)

Run through the `visual-pass` skill against the client.

1. Open the create-worktree dialog: AI CLI select (default CLI preselected, "None" last) and a
   first-prompt field render within the dialog width in light and dark; confirm creates the
   worktree and the session starts (SC-004).
2. A worktree row with a session shows no start action; a worktree without one shows exactly one;
   the Default row follows the same rule.
3. Attach dialog: resuming into an occupied worktree shows the confirmation naming both
   sessions; cancel changes nothing.

4. Sidebar with one session per worktree shows only worktree rows; selecting one opens its
   session; a worktree with two shows a collapsed expander (US5).
5. Row menu "Start another session..." on an occupied worktree opens the confirmation naming the
   existing session; confirm adds and shows the second, marked as a helper (US6).
6. A helper's sub-row menu "Make main session": the row press now opens it and the former main is
   marked a helper; deleting a main that has helpers names the successor in its confirmation (US7).

Record the result in `tdd/` evidence as the other features do.

The row press and the "Start another session..." and "Make main session" menu entries are covered at
reducer level in Part A (`features_session`, `features_sidebar`); the iced tree-item messages are not
inspectable in a test, so the widget wiring stays visual-pass only (items 4 to 6).
