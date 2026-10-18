# Research: 802 soft session limit

## R1 Where the gap closes: client, not daemon
- **Decision**: client only. The daemon keeps refusing an unflagged create (FR-007, spec Assumptions).
- **Rationale**: the flag is already the explicit request; only the window decides whether a person is present.
- **Rejected**: daemon-side "ask the window" round trip (new protocol, needs a person-present notion); auto-allow for windows (silently doubles sessions, the harm 776 guards).

## R2 Two layers: pre-check plus refusal fallback
- **Decision**: pre-check with `State::start_offered`; fallback on `OperationError { kind: Refused, detail: Some(session id) }` of a `CreateSession` op.
- **Rationale**: the pre-check alone misses a session created after the window's last catalog push (the spec's stale-row case); the fallback alone shows a needless round trip when the window knows.
- **Rejected**: fallback only (extra request on every known case); pre-check only (leaves SC-001 false in the race).

## R3 Recognising the refusal
- **Decision**: `ErrorKind::Refused` with a UUID `detail`, as `server.rs` sends for `CreateSessionError::Occupied`. A `Refused` without it (policy denial) stays an error notice.
- **Rejected**: matching `refusal_text` wording (brittle); a new `ErrorKind` (wire change).

## R4 Keep the chosen AI CLI
- **Decision**: store the provider with the pending confirmation (`additional_provider`), falling back to `provider_for_start(None)` for the row entry.
- **Rejected**: always the default CLI (silently ignores an override-list choice).

## R5 Agent-initiated create
- **Decision**: keep the 776 refusal; the flag is the opt-in; document it (spec US3). No tool-initiated `needs_confirmation`, since no person may be present.

## R6 Name in the confirmation
- **Decision**: the dialog names the main session from the catalog; when the catalog lacks it (stale window) it draws generic text ("already has a session"), never nothing.
- **Rejected**: carrying the name in the wire error (wire change); a dialog that draws nothing (invisible pending state).

## R7 Automatic starts
- **Decision**: `StartRequested` gains `automatic: bool`; only the startup auto-start sets it. An automatic start never opens the confirmation: its occupied refusal keeps the error notice (the spec covers person-initiated creates only).
- **Rejected**: treating every unflagged create alike (a dialog nobody asked for at startup).
