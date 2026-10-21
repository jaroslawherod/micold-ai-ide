# Contract: the Claude CLI's permission prompts and mode in a structured session

What the daemon sends to and reads from the CLI. Observed with Claude Code `2.1.306` (research
*Probe log*); the recorded fixtures (research R12) are the test oracle.

## Launch (FR-001, FR-013, FR-015)

Feature 938's structured argv, plus:

```text
--allow-dangerously-skip-permissions                     # every structured launch (R6)
[--permission-mode acceptEdits|plan|bypassPermissions]   # only when the mode is not Default
[--permission-prompt-tool mcp__micold__permission_prompt] # when bound and the switch is on (R14)
--mcp-config <binding> --allowedTools mcp__micold        # last, as today
```

The structured session's binding file gives the `micold` server `"timeout": PROMPT_TOOL_TIMEOUT_MS`
(R3) instead of the terminal sessions' `TOOL_TIMEOUT_MS`; terminal sessions' binding and argv are
unchanged (FR-018).

## The prompt tool (FR-002–FR-004, FR-010)

Listed by `tools/list` only to a caller whose session is structured; a `tools/call` of it from any
other caller fails `unknown_tool`, as an unlisted tool does today.

```json
{"name": "permission_prompt",
 "description": "Asks the user of the Micold window whether a tool use may run.",
 "inputSchema": {"type": "object",
   "properties": {"tool_name": {"type": "string"}, "input": {"type": "object"},
                  "tool_use_id": {"type": "string"}},
   "required": ["tool_name", "input"]}}
```

Call arguments (P-1): `{"tool_name": "Bash", "input": {"command": "…", "description": "…"},
"tool_use_id": "toolu_…"}`; `params._meta["claudecode/toolUseId"]` repeats the id.

Reply: an ordinary JSON-RPC result, `{"content": [{"type": "text", "text": <answer JSON>}]}` with
`isError: false`, written when the request is decided or withdrawn. Answer JSON:

| Outcome | Answer |
|---|---|
| allow | `{"behavior":"allow","updatedInput":<input unchanged>}` |
| always allow | the allow answer + `"updatedPermissions":[{"type":"addRules","rules":[<rule>],"behavior":"allow","destination":"session"}]` (ignored by 2.1.306; the daemon's own rule does the work, R9) |
| deny / keep planning / decline / withdrawn / malformed | `{"behavior":"deny","message":"<text>"}` |
| question answered | `{"behavior":"allow","updatedInput":<input + "answers":{"<question text>":"<label>[, <label>…]" or "<free answer>"}>}` |
| plan approved | the allow answer, after the mode change below |

## Mode change (FR-016, FR-017, R6, R7)

stdin, one line:

```json
{"type":"control_request","request_id":"micold-<n>","request":{"subtype":"set_permission_mode","mode":"<cli_name>"}}
```

stdout:

```json
{"type":"control_response","response":{"subtype":"success","request_id":"micold-<n>","response":{"mode":"acceptEdits"}}}
{"type":"control_response","response":{"subtype":"error","request_id":"micold-<n>","error":"<text>","error_code":"bypass_not_launched"}}
```

and, on success, `{"type":"system","subtype":"status","permissionMode":"<cli_name>",…}`. The CLI
also writes a `status` line when it changes mode by itself (an approved `ExitPlanMode`, P-3), and
`init` carries `permissionMode` at every start.
