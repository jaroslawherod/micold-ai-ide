# Permission-prompt fixtures (#942, FR-020, SC-004)

Recorded from a real `claude` by `scripts/capture-permission-prompt.sh <case>`; nothing was edited
by hand afterwards. The core tests in `crates/micold-core/tests/structured_permission.rs` and
`structured_parse.rs` read them as the oracle: each recorded call parses and classifies as its
kind, and the answer core builds for the recorded choice equals the answer the CLI acted on.

- CLI: Claude Code `2.1.306`
- Captured: 2026-10-21, in the dev container
- Argv of every run, in a fresh scratch git repo holding `notes.txt`: `claude -p --input-format
  stream-json --output-format stream-json --verbose --include-partial-messages --session-id <new
  uuid> [case flags] --permission-prompt-tool mcp__micold__permission_prompt --mcp-config
  <binding> --allowedTools mcp__micold`. The binding names one server, `micold`, of `"type":
  "http"` at the stub `scripts/capture/prompt-stub.py` on loopback, with `"timeout": 2147483647`
  (research R3). The prompt went in on stdin as one stream-json user message; stdin stayed open
  until the turn's `result`.
- Scrubbed by `scrub_prompt_fixture` (the `scrub_stream_json` of `scripts/capture-stream-json.sh`,
  tested by `scripts/tests/capture-permission-prompt.test.sh`): the scratch repo path → `<CWD>`,
  the home directory → `<HOME>`, the session uuid → `<SESSION_ID>`, account identifiers →
  `<ACCOUNT>`. The run's stdout is saved beside the feature 938 fixtures as
  `../stream-json/prompt-<case>.jsonl`, scrubbed the same way.

Each `<case>.json` holds `call` (the `tools/call` `arguments` and `_meta` the CLI sent), `answer`
(the answer JSON the stub returned in `content[0].text`), `tool_result` (the text of the tool result
the CLI then wrote to the transcript) and `later` (the run's further prompt calls, same shape).

| Case | Case flags | Stub answers | Shows |
|---|---|---|---|
| `tool-request` | — | allow | a `Bash` request (`touch micold-probe.txt`) allowed with its input unchanged |
| `always-allow` | — | always-allow, then allow | the always-allow answer (`updatedPermissions` `addRules`, destination `session`); see below |
| `plan-approval` | `--permission-mode plan` | allow, allow | `ExitPlanMode` with `{plan, planFilePath}` approved; the CLI's `status` line moves to `default` and the `Write` that follows is asked (`later[0]`) |
| `question-multi` | — | first option(s) | `AskUserQuestion`, two questions, the second `multiSelect`; the answers map became `"Which color?"="red", "Which fruits?"="apple, pear"` |
| `mode-change` | see `runs` | — | `set_permission_mode`: launched without `--allow-dangerously-skip-permissions`, `acceptEdits` succeeds and `bypassPermissions` fails `bypass_not_launched`; launched with it, `bypassPermissions` succeeds. Stdout: `prompt-mode-change.jsonl` and `prompt-mode-change-bypass.jsonl` |

**Always allow on 2.1.306.** The `always-allow` run made the same `Bash` call twice and the stub was
asked only once: this CLI version honoured the answer's session rule and ran the second call
without asking (both tool results are in `prompt-always-allow.jsonl`). A control run answering a
plain allow twice was asked twice. This differs from research probe P-11; the daemon still keeps
its own rule (research R9), so the second call is never asked on a CLI that ignores the rule.

Re-record with `scripts/capture-permission-prompt.sh <case>` (it needs a logged-in `claude`; CI has
none and never runs it). The model chooses the input text (descriptions, the plan), so a new
recording differs in those; the tests read the values from the fixture, never from a literal.
