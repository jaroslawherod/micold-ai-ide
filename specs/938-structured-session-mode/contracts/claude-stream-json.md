# Contract: the Claude CLI in structured mode

## Launch (FR-005, FR-006, FR-006a)

```text
claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages
       (--session-id <id> | --resume <id>)
       [--settings <hook file>]          # as for a terminal session
       [--mcp-config <binding>]          # last, as for a terminal session
```

- cwd, environment and the availability / recorded-conversation checks are the terminal session's.
- Never added: `--permission-mode`, `--allowedTools`, `--disallowedTools`,
  `--dangerously-skip-permissions`, `--permission-prompt-tool`.
- stdin, stdout, stderr are pipes. stderr goes to the daemon log only.

## Output (stdout): one JSON object per line

| `type` (`subtype`) | Model |
|---|---|
| `system` (`init`) | `Init` |
| `assistant` | `Assistant` with `message.content[]` blocks |
| `user` | `User` (tool results arrive here) |
| `result` (`success`, `error_*`) | `Result` — ends the turn |
| `stream_event` | `StreamDelta` (`event.type`, `event.index`, `event.delta`) |
| anything else, or a known type that does not fit | `Unrecognised { raw }` |
| not JSON | `Unreadable { text }` |

Line ending: `\n`; one trailing `\r` is removed. A line is never split or truncated by the reader.

## Input (stdin): one user message per turn (FR-008)

```json
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"<text>"}]}}
```

followed by `\n`. `<text>` is the user's text unchanged, escaped by `serde_json`.

## Fixtures (FR-024)

`crates/micold-core/tests/fixtures/stream-json/*.jsonl`, stdout verbatim from the CLI version named
in the directory README, scrubbed of account identifiers and home paths. `tolerance.jsonl` is
synthetic and says so.
