# `claude` stream-json fixtures (feature 938, FR-024)

Nine of these files are **recorded**: the stdout of a real `claude` run, byte for byte, then
scrubbed. Two are **synthetic**, written by hand: `tolerance.jsonl` and `insights-tolerance.jsonl`.

- CLI: Claude Code `2.1.302`, model `claude-sonnet-5-5`; the four marked (944) with `2.1.306`
- Captured: 2026-10-18 (the (944) ones 2026-10-21), in the dev container, with
  `scripts/capture-stream-json.sh` (for the (944) ones it sets `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`)
- Argv of every recorded run: `claude -p --input-format stream-json --output-format stream-json
  --verbose --include-partial-messages --session-id <new uuid>` (or `--resume <id>`), in a fresh
  scratch git repo holding one file, `notes.txt` (`The answer is 42.`). The prompt went in on
  stdin as one stream-json user message.
- Scrubbed by the script's `scrub_stream_json` (tested by
  `scripts/tests/capture-stream-json.test.sh`): the scratch repo path → `<CWD>`, the home
  directory → `<HOME>`, the session uuid → `<SESSION_ID>`, e-mail addresses and account /
  organisation identifiers → `<ACCOUNT>`. First, `scrub_split_deltas` does the same for the
  path and uuid where the CLI cut them across `input_json_delta` chunks: it joins each run of
  chunks, scrubs the joined text and cuts it again at the original chunk lengths, so the joined
  deltas equal the scrubbed `tool_use` input. `tool-turn` and `error-result` were recorded before
  that step existed and had it applied afterwards. Nothing else was edited.

| Fixture | Prompt | Extra flags | Shows |
|---|---|---|---|
| `text-turn.jsonl` | `Reply with exactly one short sentence: what colour is a clear daytime sky?` | — | a plain answer ending in a `success` result |
| `tool-turn.jsonl` | `Use the Read tool once to read notes.txt in the current directory, then tell me the number it states. Do not use any other tool.` | — | a `Read` tool call and its `tool_result` |
| `partial-deltas.jsonl` | `Write four short numbered lines, each naming a different planet and one fact about it.` | — | many `stream_event` text deltas |
| `error-result.jsonl` | `Use the Read tool to read notes.txt, then tell me the number it states.` | `--max-turns 1` | an `error_max_turns` result (`is_error: true`) |
| `resume-init.jsonl` | `Now reply with one word: the colour you named.` | `--resume <text-turn's session>` | a resumed session's `init`, same session id as `text-turn` |
| `subagent-turn.jsonl` (944) | `Use the Agent tool once, with subagent_type general-purpose, to have a subagent read notes.txt in the current directory with the Read tool and report the number it states. Then tell me that number. Do not use any other tool yourself.` | — | a foreground `Agent` call: the subagent's messages carry `parent_tool_use_id`, its `task_*` lines |
| `nested-subagent.jsonl` (944) | `Use the Agent tool once, with subagent_type relay, asking it to find out the number stated in notes.txt in the current directory. Then tell me that number. Do not use any other tool yourself.` | `--agents '{"relay":{"description":"Finds out what a file says by delegating the reading to another subagent.","prompt":"You never read files yourself. Use the Agent tool exactly once, with subagent_type general-purpose, asking that subagent to read the named file with the Read tool and report what it states. Then report its answer."}}'`, environment `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=2` | a subagent starting a subagent: each level's messages name their own parent call (`spawn_depth` 1 and 2) |
| `background-command.jsonl` (944) | ``Use the Bash tool exactly once, with run_in_background set to true, to run the command `sleep 5; echo finished`. Do not run any other command. When its completion notification arrives, tell me its output in one short sentence.`` | `--allowedTools Bash` | a `local_bash` background task that finishes: `task_started`, `background_tasks_changed`, `task_notification`, and a second turn after it |
| `background-subagent.jsonl` (944) | `Use the Agent tool exactly once, with subagent_type general-purpose and run_in_background set to true, to have a subagent read notes.txt in the current directory with the Read tool and report the number it states. Do not use any other tool yourself. When its completion notification arrives, tell me that number in one short sentence.` | — | a backgrounded `local_agent` task, its parented messages, and a second turn after its notification |

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

`insights-tolerance.jsonl` is **synthetic** (feature 944, SC-006): the real `init` line of
`text-turn.jsonl`, then `autocompact_state` lines with a threshold as a string, with no `value` and
disabled; main `assistant` lines whose `usage` has a figure as a string, a negative figure, no
`usage` and a `usage` that is a string, then one well-formed; `rate_limit_event` lines with a
utilization as a string, a window that is a number and `rate_limit_info` a string; `task_started`,
`task_updated`, `task_progress`, `task_notification` and `background_tasks_changed` lines with
fields of the wrong type; and `result` lines with a `modelUsage` entry holding a string and an entry
that is a number, with no `usage` and no `modelUsage`, with `modelUsage` a list, with a negative
`total_cost_usd`, and with a token count as a string.
