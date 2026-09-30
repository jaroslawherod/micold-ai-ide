# M1 evidence: quickstart B1 and B2 against the real CLIs

Date: 2026-09-30. Claude Code 2.1.285, GitHub Copilot CLI 1.0.88, Linux.

## How it was run

A throwaway probe test (not committed) built a `DaemonState` over a temporary git repository with
worktrees `a` and `b` and sessions S1 (root, Claude), S2 (`a`, Copilot) and S3 (`b`, Claude), bound
the real tool server, and started S3 and S2 through `DaemonState::start_session` with recording
stand-ins on `PATH`, so the argv and binding files below are exactly what the service produces.
It then ran the real `claude -p` (in `b`, `--model haiku`) and `copilot -p` (in `a`) with those
binding arguments against the live tool server, asking each to call `whoami`, `list_worktrees` and
`list_sessions`. The prompt runs in print mode, which fails a tool call that needs a permission
prompt, so a successful call also shows the pre-approval works.

## Results

| Quickstart step | Result |
|---|---|
| B1.2–3 Claude calls the tools, no prompt | Pass: all three answered; `whoami` names S3 and `b`; S3 alone has `is_caller: true` |
| B1.4 user configuration unchanged | Pass for what the service writes (no `micold` server anywhere in `~/.claude.json`; `~/.claude/settings.json` identical; no `.mcp.json` created). `~/.claude.json`'s hash changes on every `claude` run, with or without the binding (a control `claude -p "reply with ok"` changed it too): that is Claude Code's own bookkeeping. The service-side guarantee is `mcp_binding_spawn.rs::bound_spawns_leave_every_user_configuration_file_byte_identical` |
| B1.5 argv and file mode | Pass: `--mcp-config <dir>/mcp/<uuid>.json --allowedTools mcp__micold` last, no `--strict-mcp-config`; file `-rw-------`, directory `drwx------` |
| B1.6 request without a credential | Pass: `401` |
| B1.7 long confirmation wait | Not in M1 (after M5) |
| B2 Copilot calls the tools | Pass: `--additional-mcp-config @<file> --allow-tool micold` with `"tools":["*"]` in the entry; all three tools answered, `whoami` names S2 and `a`. Copilot stays bound (no switch to `Unsupported`) |
| Service log | 3 lines, no credential and no request body |

## Probe report (verbatim)

## hashes before
6b17ca2389bd457f4d9daa57e2024f936e594f7dc44ed3e6ab7cda7d37db4ad8  /home/jaro/.claude.json
658e65f91a1f4d824807cc89d1580c1b95a9a95f9e0ec6456d29664d533e3cb0  /home/jaro/.claude/settings.json
(absent) /tmp/.tmpkAI2CQ/.mcp.json
(absent) /home/jaro/.copilot/mcp-config.json

## claude argv (session 00000000-0000-0000-0000-000000000003)
["--session-id", "00000000-0000-0000-0000-000000000003", "--mcp-config", "/tmp/.tmppDYTf8/mcp/00000000-0000-0000-0000-000000000003.json", "--allowedTools", "mcp__micold"]

## copilot argv (session 00000000-0000-0000-0000-000000000002)
["--session-id", "00000000-0000-0000-0000-000000000002", "--no-remote", "--additional-mcp-config", "@/tmp/.tmppDYTf8/mcp/00000000-0000-0000-0000-000000000002.json", "--allow-tool", "micold"]

## ls -la binding dir
total 8
drwx------ 2 jaro jaro  80 Sep 30 08:07 .
drwxrwxr-x 3 jaro jaro 100 Sep 30 08:07 ..
-rw------- 1 jaro jaro 269 Sep 30 08:07 00000000-0000-0000-0000-000000000002.json
-rw------- 1 jaro jaro 231 Sep 30 08:07 00000000-0000-0000-0000-000000000003.json


## POST /mcp without a credential
status 401

## real claude -p (exit Some(0), 12.7s)
### stdout
```json
{"ai_cli":"claude_code","project":{"name":".tmpkAI2CQ","path":"/tmp/.tmpkAI2CQ"},"session":"00000000-0000-0000-0000-000000000003","worktree":"b"}
```

```json
{"worktrees":[{"app_created":false,"assistant_owned":false,"branch":null,"display_name":"Default","path":"/tmp/.tmpkAI2CQ","ref":"default","session_count":1,"status":"clean"},{"app_created":true,"assistant_owned":false,"branch":"a","display_name":"a","path":"/tmp/.tmpkAI2CQ/.claude/worktrees/a","ref":"a","session_count":1,"status":"clean"},{"app_created":true,"assistant_owned":false,"branch":"b","display_name":"b","path":"/tmp/.tmpkAI2CQ/.claude/worktrees/b","ref":"b","session_count":1,"status":"clean"}]}
```

```json
{"sessions":[{"activity":"unknown","ai_cli":"claude_code","is_caller":false,"label":"New session","lifecycle":"idle","ref":"00000000-0000-0000-0000-000000000001","worktree":"default"},{"activity":"unknown","ai_cli":"copilot","is_caller":false,"label":"New session","lifecycle":"running","ref":"00000000-0000-0000-0000-000000000002","worktree":"a"},{"activity":"unknown","ai_cli":"claude_code","is_caller":true,"label":"New session","lifecycle":"running","ref":"00000000-0000-0000-0000-000000000003","worktree":"b"}]}
```

### stderr


## real copilot -p (exit Some(0), 17.3s), binding args only: ["--additional-mcp-config", "@/tmp/.tmppDYTf8/mcp/00000000-0000-0000-0000-000000000002.json", "--allow-tool", "micold"]
### stdout
● whoami (MCP: micold)
  └ {

● list_sessions (MCP: micold) · worktree: "default"
  └ {

● list_worktrees (MCP: micold)
  └ {

{"ai_cli":"copilot","project":{"name":".tmpkAI2CQ","path":"/tmp/.tmpkAI2CQ"},"session":"00000000-0000-0000-0000-000000000002","worktree":"a"}
{"worktrees":[{"app_created":false,"assistant_owned":false,"branch":null,"display_name":"Default","path":"/tmp/.tmpkAI2CQ","ref":"default","session_count":1,"status":"clean"},{"app_created":true,"assistant_owned":false,"branch":"a","display_name":"a","path":"/tmp/.tmpkAI2CQ/.claude/worktrees/a","ref":"a","session_count":1,"status":"clean"},{"app_created":true,"assistant_owned":false,"branch":"b","display_name":"b","path":"/tmp/.tmpkAI2CQ/.claude/worktrees/b","ref":"b","session_count":1,"status":"clean"}]}
{"sessions":[{"activity":"unknown","ai_cli":"claude_code","is_caller":false,"label":"New session","lifecycle":"idle","ref":"00000000-0000-0000-0000-000000000001","worktree":"default"}]}


### stderr


Changes    +0 -0
AI Credits 0.32 (9s)
Tokens     ↑ 24.4k (11.8k cached) • ↓ 331
Resume     copilot --resume=936143dc-959c-44ad-8065-7512fbd2034f


## hashes after
4c038aee2a72b899c5588d2dfa70c6fc010ff6db8b0e32b40105beca2fac095b  /home/jaro/.claude.json
658e65f91a1f4d824807cc89d1580c1b95a9a95f9e0ec6456d29664d533e3cb0  /home/jaro/.claude/settings.json
(absent) /tmp/.tmpkAI2CQ/.mcp.json
(absent) /home/jaro/.copilot/mcp-config.json

## whoami via the test client (control)
{"ai_cli":"claude_code","project":{"name":".tmpkAI2CQ","path":"/tmp/.tmpkAI2CQ"},"session":"00000000-0000-0000-0000-000000000003","worktree":"b"}
