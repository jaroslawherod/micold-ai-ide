# Contract: the CLI's side (#943)

Recorded with Claude Code `2.1.307` (research P-1–P-5); fixtures in
`crates/micold-core/tests/fixtures/stream-json/command-*.jsonl`, `model-*.jsonl`,
`auth-failure.jsonl` (M1). Lines are abbreviated here; the fixtures are the oracle.

## Launch

| Kind | Added arguments |
|---|---|
| structured, terminal | `--model <name>` when the model choice is `Named` (FR-017) |
| structured, terminal, `LaunchMode::Resume` | `--resume <handle>`: the session's conversation handle (R6), else its id |

## stdin (daemon → CLI)

```json
{"type":"control_request","request_id":"<id>","request":{"subtype":"initialize"}}
{"type":"control_request","request_id":"<id>","request":{"subtype":"set_model","model":"haiku"}}
{"type":"control_request","request_id":"<id>","request":{"subtype":"set_model","model":"default"}}
```

`initialize` once per structured spawn, before any user message. Commands are ordinary user
messages (`encode_user_message`), text unchanged.

## stdout (CLI → daemon)

```json
{"type":"control_response","response":{"subtype":"success","request_id":"<id>","response":{
  "commands":[{"name":"compact","description":"…","argumentHint":"<optional …>","builtin":true},
              {"name":"config","description":"Set a setting by key","argumentHint":"key=value","aliases":["settings"],"builtin":true}, …],
  "models":[{"value":"default","resolvedModel":"claude-sonnet-5-5","displayName":"Default (recommended)","description":"…"}, …], …}}}
{"type":"control_response","response":{"subtype":"success","request_id":"<id>","response":{"model":"claude-haiku-5-5"}}}
{"type":"control_response","response":{"subtype":"error","request_id":"<id>","error":"Model 'x' not found","error_code":"not_found","model":"claude-sonnet-5-5"}}
{"type":"system","subtype":"init","session_id":"<id>","model":"…","slash_commands":[…],"skills":[…],"terminal_slash_commands":["doctor","color","focus","reload-plugins"], …}
{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"## Context Usage …"}], …}}
{"type":"conversation_reset","new_conversation_id":"<uuid>","trigger":"clear", …}
{"type":"system","subtype":"status","status":"compacting"}
{"type":"system","subtype":"status","status":null,"compact_result":"success"}
{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"manual","pre_tokens":37839,"post_tokens":6728, …}}
{"type":"user","isSynthetic":true,"message":{"role":"user","content":"This session is being continued … Summary: …"}}
{"type":"assistant","error":"authentication_failed","message":{"model":"<synthetic>","content":[{"type":"text","text":"Invalid API key · …"}]}}
```

- An `init` precedes every turn; after `/clear` its `session_id` is new and is the handle
  `--resume` takes (R5, R6). `new_conversation_id` is not used.
- Unknown keys anywhere are ignored (938 FR-012); a missing list is empty.
