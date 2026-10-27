# Contract: client ↔ daemon additions (#1554)

Every new field is `#[serde(default)]`. `PROTOCOL_VERSION` is bumped by one in each milestone that
changes the wire (M1: events and `ReportedModel`; M2: the message and the summary field), each
merging to `main` on its own, `crates/micold-core/tests/schema_hash.rs` re-pinned. A concurrent
bump from another feature is a rebase conflict resolved by taking the higher number + 1.

## Events (M1)

`StructuredEvent` gains `EffortChanged { effort: EffortChoice }` and
`EffortRefused { effort: EffortChoice, message: String }`; `ReportedModel` (inside
`CommandsReported`) gains `effort: Option<EffortSupport>`. They travel in the existing
`StructuredEvents` frames and handshake replay; a window learns the supported levels only this way.

## Changed (M2)

| Message | Change |
|---|---|
| `SessionSummary` | `+ effort: EffortChoice` (FR-001, FR-010) |

## New, client → daemon (M2)

```text
SetSessionEffort { req: u64, session: SessionId, effort: EffortChoice }
```

As `SetSessionModel` (#943 contracts/wire.md), answered `OperationOk` or `OperationError { kind,
message }`:

| Session | Answer | Stored | Note in history |
|---|---|---|---|
| unknown | `OperationError` (`NotFound`) | — | — |
| provider not Claude Code | `OperationError` (`Refused`, "Reasoning effort is for Claude Code sessions") | no | — |
| terminal kind | `OperationError` (`Refused`, "Effort is changed from the structured view; the choice is used at the next launch") | yes | — |
| not running | `OperationOk` | yes | `EffortChanged` |
| running, idle | after the CLI's answer (≤ the 10 s control bound): success → `OperationOk`; refusal, no answer or the process ended → `OperationError` with the reason (FR-005) | on success only | `EffortChanged` on success |
| running, busy | `OperationOk` at once | yes (put back if the CLI refuses at the turn's end) | at the turn's end: `EffortChanged`, or `EffortRefused` with the reason |

The new choice reaches every window in the next catalog snapshot (`SessionSummary.effort`), within
1 s (SC-002).

## Changed behaviour (M2)

`SessionUserMessage` whose text `classify_draft` reads as `Effort(_)` → `OperationError` ("/effort
is handled by the app"), nothing written to the CLI, nothing queued (FR-007, FR-008, SC-003). The
window never sends it: this is the backstop for a client that does.
