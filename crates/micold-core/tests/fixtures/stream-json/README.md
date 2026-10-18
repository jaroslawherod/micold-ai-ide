# `claude` stream-json fixtures (feature 938, FR-024)

Five of these files are **recorded**: the stdout of a real `claude` run, byte for byte, then
scrubbed. One is **synthetic**: `tolerance.jsonl`, written by hand.

- CLI: Claude Code `2.1.302`, model `claude-sonnet-5-5`
- Captured: 2026-10-18, in the dev container, with `scripts/capture-stream-json.sh`
- Argv of every recorded run: `claude -p --input-format stream-json --output-format stream-json
  --verbose --include-partial-messages --session-id <new uuid>` (or `--resume <id>`), in a fresh
  scratch git repo holding one file, `notes.txt` (`The answer is 42.`). The prompt went in on
  stdin as one stream-json user message.
- Scrubbed by the script's `scrub_stream_json` (tested by
  `scripts/tests/capture-stream-json.test.sh`): the scratch repo path → `<CWD>`, the home
  directory → `<HOME>`, the session uuid → `<SESSION_ID>`, e-mail addresses and account /
  organisation identifiers → `<ACCOUNT>`. Nothing else was edited.

| Fixture | Prompt | Extra flags | Shows |
|---|---|---|---|
| `text-turn.jsonl` | `Reply with exactly one short sentence: what colour is a clear daytime sky?` | — | a plain answer ending in a `success` result |
| `tool-turn.jsonl` | `Use the Read tool once to read notes.txt in the current directory, then tell me the number it states. Do not use any other tool.` | — | a `Read` tool call and its `tool_result` |
| `partial-deltas.jsonl` | `Write four short numbered lines, each naming a different planet and one fact about it.` | — | many `stream_event` text deltas |
| `error-result.jsonl` | `Use the Read tool to read notes.txt, then tell me the number it states.` | `--max-turns 1` | an `error_max_turns` result (`is_error: true`) |
| `resume-init.jsonl` | `Now reply with one word: the colour you named.` | `--resume <text-turn's session>` | a resumed session's `init`, same session id as `text-turn` |

The recorded files also carry the line types this CLI version writes that the model does not
type (`active_goal`, `autocompact_state`, `rate_limit_event`, `system` subtypes other than
`init`): they parse as `Unrecognised`, kept whole (FR-012).

`tolerance.jsonl` is **synthetic** (SC-002): a real `init` line copied from `text-turn.jsonl`,
then an unknown `type`, an `assistant` line with unknown fields at top level and inside a block,
an `assistant` line with no `message`, a `system` line of unknown subtype, an `assistant` message
with a content block of unknown type, a line that is not JSON, a `result` line ending in `\r\n`,
and a last line with no trailing newline. `.gitattributes` keeps these files byte for byte.

Re-record with `scripts/capture-stream-json.sh <name> "<prompt>" [--resume <id>] [-- <flags>]`
(it prints the session uuid on stderr for a later `--resume`); it needs a logged-in `claude`.
