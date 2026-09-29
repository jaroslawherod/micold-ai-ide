# Contract: client ↔ service protocol delta

**Feature**: 034-daemon-mcp-server | Base: `specs/010-daemon-session-persistence/contracts/protocol.md`,
`PROTOCOL_VERSION = 15` (`crates/micold-core/src/protocol/version.rs`)

Each milestone that changes the wire bumps `PROTOCOL_VERSION` once and documents the bump in
`version.rs`'s changelog comment, as every earlier feature did, and updates the pin in
`crates/micold-core/tests/schema_hash.rs`. M2 and M6 add settings fields; all three new messages land
in M5. If another feature takes a number
first, this feature takes the next one.

## §1 M2 — the binding toggle (15 → 16)

- `DaemonSettings.tool_server_enabled: bool`
- `ClientMsg::SettingsSet.tool_server_enabled: Option<bool>`

Applied to sessions spawned afterwards (FR-004). `SettingsChanged` carries it to every window.

## §2 M5 — confirmations (16 → 17)

```text
DaemonMsg::ConfirmationRequested {
    id: u64,
    project: PathBuf,
    caller: SessionId, caller_label: String,
    operation: ConfirmOperation,          // DeleteWorktree{stop_sessions, delete_branch} | DeleteSession
                                          // | StopSession | InterruptSession | SendInput
    target_label: String,                 // worktree display name or session label
    expires_in_ms: u32,                   // remaining of the 60 s
}
DaemonMsg::ConfirmationWithdrawn { id: u64 }
ClientMsg::ConfirmationAnswer { id: u64, allow: bool }
```

- Broadcast to every connected client; a client that completes its handshake while prompts are
  pending receives one `ConfirmationRequested` per pending prompt, with the remaining time.
- The first `ConfirmationAnswer` for a live `id` decides; every later answer, or one for an unknown
  `id`, is ignored. Resolution in any way broadcasts `ConfirmationWithdrawn`.
- `SendInput` carries no text: the prompt names the target only.

## §3 M6 — cross-session option (17 → 18)

- `DaemonSettings.cross_session_access: CrossSessionAccess` (`Auto | ConfirmEachSend | Off`)
- `ClientMsg::SettingsSet.cross_session_access: Option<CrossSessionAccess>`

Read by the tool server on every request (FR-016).
