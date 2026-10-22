# `claude` stream-json fixtures (feature 938, FR-024)

All but three of these files are **recorded** (943's eleven in their own table below): the stdout of a real `claude` run, byte for byte, then
scrubbed. Three are **synthetic**, written by hand: `tolerance.jsonl`, `insights-tolerance.jsonl` and
`markdown-subset.md` (plain markdown, not stream-json; see below).

- CLI: Claude Code `2.1.302`, model `claude-sonnet-5-5`; the four marked (944), the one marked
  (940) and the two marked (941) with `2.1.306` (the (940) one with `--model claude-sonnet-5-5` passed)
- Captured: 2026-10-18 (the (944), (940) and (941) ones 2026-10-21), in the dev container, with
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
| `markdown-turn.jsonl` (940) | `Do not use any tools. Reply with a short markdown document about brewing tea that uses each of these elements at least once: a level-1 heading and a level-2 heading; **bold**, *italic* and ~~strikethrough~~ text; `inline code`; a bullet list with a nested bullet list; a numbered list; a task list with one checked and one unchecked item; a block quote; a horizontal rule; a link to https://example.com; a table with three columns whose columns are left-, centre- and right-aligned; a code block tagged rust, a code block tagged teapot-lang, and a code block with no tag.` | `--model claude-sonnet-5-5` | one assistant text block streamed in many `text_delta`s, using every element of #940's markdown subset |
| `edit-turn.jsonl` (940) | `Make exactly three file changes in the current directory, one tool call each, and use no other tool. First, use the Write tool to create fruits.txt with these four lines: apple, banana, apple, cherry. Second, use the Edit tool once with replace_all set to true to replace every "apple" with "pear". Third, use the Edit tool once more, with old_string the three lines "banana", "pear", "cherry" and new_string the four lines "kiwi", "pear", "plum", "fig". Then reply with one short sentence.` | `--model claude-sonnet-5-5 --allowedTools Write Edit` | a `Write`, an `Edit` with `replace_all: true` and a multi-line `Edit` with a shared line, each with its `tool_result`; this CLI (2.1.306) has no `MultiEdit` tool, so MultiEdit is covered by `tests/edit_diff.rs`'s synthetic set |
| `interrupt-turn.jsonl` (941) | `Write a long story of at least 1500 words about a lighthouse keeper. Do not use any tool.` | capture flag `--interrupt-after-first-delta` | P-4: the `control_request` `interrupt` written after the first text delta; its `control_response` `success`, the partial `assistant` message, the `[Request interrupted by user]` `user` line and a `result` `error_during_execution` (`terminal_reason: aborted_streaming`) |
| `image-turn.jsonl` (941) | `Reply with one short sentence: what colour fills this picture?` | capture flag `--image red-square.png` (a 16×16 solid red PNG) | P-5: one user message with a text block and a base64 PNG block, answered and ended `success` |

### Commands, models and authentication (943)

Recorded 2026-10-22 with Claude Code `2.1.307` (model `claude-sonnet-5-5` unless the file shows
otherwise), in the cloud dev container, by `scripts/capture-stream-json.sh`. Every scratch repo
also holds the project skill `greet` (`.claude/skills/greet/SKILL.md`) and the custom command
`hello` (`.claude/commands/hello.md`) the script writes. Argv as above; a `--control` line is
written to stdin before the prompt, in the order listed. The command lists in the `init` lines and
the `initialize` answer are the container's: its user and claude.ai-synced skills appear beside
the CLI's built-ins.

| Fixture | Prompt | `--control` lines, extra flags | Shows |
|---|---|---|---|
| `command-initialize.jsonl` | `Reply with the single word ready.` | `{"type":"control_request","request_id":"micold-1","request":{"subtype":"initialize"}}` | the `initialize` answer (P-1): `commands` with `description`, `argumentHint`, `aliases`, `builtin`, and `models`, before the turn's `init` |
| `command-skill.jsonl` | `/greet Ada` | — | a project skill run with arguments; no expanded instructions on stdout |
| `command-custom.jsonl` | `/hello world` | — | a project custom command run with arguments |
| `command-compact.jsonl` | `/compact keep greetings` | user message `Reply with one short sentence greeting Ada.` | `status` `compacting`, `compact_result`, a fresh `init`, `compact_boundary` (`manual`, `pre_tokens`/`post_tokens`), the `isSynthetic` summary user line and a `<local-command-stdout>` user line |
| `command-clear.jsonl` | `Reply with the single word after.` | user messages `Reply with the single word before.`, then `/clear` | `conversation_reset` (`trigger: "clear"`), then `init` lines with a new `session_id` (left unscrubbed: `80379009-…`) |
| `command-builtin-output.jsonl` | `/context` | — | a built-in's text answer: an `assistant` line with `model: "<synthetic>"`, a `result` with `num_turns: 0` |
| `command-unknown.jsonl` | `/nosuchcmd` | — | an unknown command handed to the model as text, and its ordinary reply |
| `model-change.jsonl` | `Reply with the single word ready.` | `set_model` `haiku` (`micold-1`) | the `success` answer `{"model":"claude-haiku-5-5"}`, then a turn whose `init` and `assistant` report it |
| `model-refused.jsonl` | `Reply with the single word ready.` | `set_model` `no-such-model-xyz` (`micold-1`) | the `error` answer (`Model 'no-such-model-xyz' not found`, `error_code: "not_found"`), the model unchanged |
| `auth-failure.jsonl` | `Reply with the single word ready.` | run under `env -i` with only `PATH`, the proxy and CA variables, an empty `HOME` and `ANTHROPIC_API_KEY=sk-ant-invalid-key-for-fixture` (P-5) | a synthetic `assistant` line with top-level `error: "authentication_failed"`, then an error `result` |
| `resume-after-clear.jsonl` | `Reply with one word: the word you replied with last.` | `--resume 80379009-…` (the post-clear `session_id` of `command-clear.jsonl`, in that capture's scratch repo) | the cleared conversation resumed: the answer is `after`, not `before` |

The recorded files also carry the line types this CLI version writes that the model does not
type (`active_goal`, `autocompact_state`, `rate_limit_event`, `system` subtypes other than
`init` and `status`): they parse as `Unrecognised`, kept whole (FR-012). `system`/`status` lines
parse as `Status` since #942.

The `prompt-*.jsonl` files are the stdout of #942's permission-prompt recordings, made by
`scripts/capture-permission-prompt.sh` and described in `../permission-prompt/README.md`.

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

`markdown-subset.md` (940, synthetic) is plain markdown for the markdown model's tests
(`tests/markdown_subset.rs`, `tests/markdown_stream.rs`): what a recording may not hold — raw HTML
(block and inline), an image, `javascript:`, relative and `file:` links, a reference defined in a
later block, a CRLF paragraph and list (kept as written by its `-text` line in `.gitattributes`), an
indented code block, an unclosed fence (last), unmatched `**` and a table without its separator
row — beside every element of the subset.
