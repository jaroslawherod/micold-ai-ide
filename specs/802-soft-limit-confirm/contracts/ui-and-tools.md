# Contracts: 802

## Wire
Unchanged. `SessionCreate { allow_additional_session }` and `OperationError { kind: Refused, detail: Some(<existing session uuid>) }` are as in 776.

## Client behaviour contract
- Person start on an occupied location: confirmation opens, no `SessionCreate` is sent.
- Person start whose reply is `Refused` with a session-id `detail` (and `additional == false`): confirmation opens, no error notice.
- Confirm: exactly one `SessionCreate { allow_additional_session: true }` with the chosen CLI. Cancel/Escape: nothing sent, no notice.
- Automatic (startup) start refused as occupied: error notice, no dialog.
- Dialog with the occupant absent from the catalog: generic text.
- Any other `Refused`/error: the existing error notice.

## Tool descriptions (`create_session`, `start_session`)
State: a person using the app is asked to confirm a second session; an agent is refused unless it sets `allow_additional_session: true`, because nobody is present to confirm and unattended doubling of sessions in one worktree is what the limit guards against. Pinned by `crates/micold-core/tests/mcp_tools_catalog.rs`.
