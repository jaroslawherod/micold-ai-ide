# Contract: wire changes (PROTOCOL_VERSION 36 → 37)

| Message | Change |
|---|---|
| `GridFrame` | + `process: SessionProcess` (`#[serde(default)]` → `Primary`). |
| `ClientMsg::SessionInput` | + `process: Option<SessionProcess>` (None = the currently attached process, today's behaviour). Serial stays per session. |
| `ClientMsg::SessionResize` | + `process: Option<SessionProcess>` (None = the attached process only; with `Some`, that PTY only). |
| `ClientMsg::SetViewedTerminals { project, terminals: Vec<TerminalRef> }` | new; ≤ 6 entries; daemon streams exactly these for this client+project, replaces the previous set, records the first as the foreground session (`remember_foreground`). Unknown terminals are ignored. |
| `ClientMsg::SetPaneLayout { project, layout: Option<String> }` | new; JSON of the contracts/pane-layout-file.md form; daemon validates (invalid → rejected, stored layout kept) and persists in the catalog. `ProjectSnapshot` gains `pane_layout: Option<String>` (`#[serde(default, skip_serializing_if)]`). |
| `SetViewedSession`, `SessionAttachProcess` | unchanged; equivalent to a one-element `SetViewedTerminals`. |

Rules: a frame for `(session, process)` is sent only to clients whose set contains it; resize with `process` resizes that PTY only and never touches the session's other processes; a failed resize of one terminal does not affect another. `tests/schema_hash.rs` pin and any version pins are moved with the bump (one bump for the whole feature).

Compatibility: the handshake already rejects any peer whose `PROTOCOL_VERSION` or schema hash differs, so there is no old-client case; the compatibility test is that an absent `process` (`None`) on `SessionInput`/`SessionResize` behaves exactly as before, and that `GridFrame` round-trips under the postcard codec with the new field.
