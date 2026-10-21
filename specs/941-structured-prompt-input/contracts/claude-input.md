# Contract: what the daemon writes to the CLI's stdin (#941)

Builds on #938's `contracts/claude-stream-json.md` (launch arguments unchanged: no
`--replay-user-messages`, research R3) and #942's control requests. One line each, `\n`-terminated,
keys in this order. Evidence: research P-4, P-5.

## User message (changed: images)

```json
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"<text>"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"<base64>"}}]}}
```

- The text block is first and present only when the text is not blank (FR-027 AS6); images follow
  in the order attached. With no images the line is #938's line, byte for byte.
- Written only while no turn is open (research R2): the daemon never relies on the CLI's own queue.

## Interrupt

```json
{"type":"control_request","request_id":"micold-<n>","request":{"subtype":"interrupt"}}
```

Expected answer and turn end (P-4):

```text
{"type":"control_response","response":{"subtype":"success","request_id":"micold-<n>","response":{"still_queued":[]}}}
{"type":"assistant", … the partial message …}
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"[Request interrupted by user]"}]}, …}
{"type":"result","subtype":"error_during_execution","is_error":true,"terminal_reason":"aborted_streaming", …}
```

- The daemon does not wait on the `control_response`; the `result` ends the turn as any `result`
  does. No `result` within 10 s → `Overdue` (FR-015). The process is never signalled (FR-013).
- An `error` `control_response` leaves the turn open; the overdue notice covers it.

## Recorded fixtures (M1)

| Fixture | Run |
|---|---|
| `crates/micold-core/tests/fixtures/stream-json/interrupt-turn.jsonl` | a long answer interrupted after its first text delta |
| `crates/micold-core/tests/fixtures/stream-json/image-turn.jsonl` | one message with a text and a PNG |
