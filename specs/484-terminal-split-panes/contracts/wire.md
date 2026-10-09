# Contract: wire changes (PROTOCOL_VERSION 36 → 37)

| Message | Change |
|---|---|
| `GridFrame` | + `process: SessionProcess` (`#[serde(default)]` → `Primary`). |
| `ClientMsg::SessionInput` | + `process: Option<SessionProcess>` (None = currently attached, today's behaviour). Serial stays per session. |
| `ClientMsg::SessionResize` | + `process: Option<SessionProcess>` (None = today's behaviour: all live PTYs of the session / the attached one). |
| `ClientMsg::SetViewedTerminals { project, terminals: Vec<(SessionId, SessionProcess)> }` | new; ≤ 6 entries; daemon streams exactly these for this client+project, replaces the previous set, records the first as the foreground session (`remember_foreground`). Unknown terminals are ignored. |
| `SetViewedSession`, `SessionAttachProcess` | unchanged; equivalent to a one-element `SetViewedTerminals`. |

Rules: a frame for `(session, process)` is sent only to clients whose set contains it; resize with `process` resizes that PTY only and never touches the session's other processes; a failed resize of one terminal does not affect another. `tests/schema_hash.rs` pin and any version pins are moved with the bump (one bump for the whole feature).
