# M5 evidence: quickstart B1 step 7 and the B2 repeat against the real CLIs (T103)

Date: 2026-10-01. Claude Code 2.1.286, GitHub Copilot CLI 1.0.89, Linux.

## How it was run

A throwaway probe test (not committed) built a `DaemonState` over a temporary git repository with
worktrees `b`, `ctl-claude`, `del-claude`, `ctl-copilot` and `del-copilot`, and sessions S1 (root,
Claude), S3 (`b`, Claude) and S2 (`b`, Copilot). It bound the real tool server and started S3 and
S2 through `DaemonState::start_session` with recording stand-ins on `PATH`, so the argv and binding
files below are what the service produces. A registered fake window received each
`DaemonMsg::ConfirmationRequested` and answered Allow through `DaemonState::answer_confirmation`
after a set delay: 0 ms for a control run, 55,000 ms for the run under test.

The probe then ran the real `claude -p` (`--model haiku`) and the real `copilot -p` with the
service's binding arguments, with the working directory in `b`, once per delay, each asking:

> Call the micold delete_worktree tool for worktree `<name>` with delete_branch false. Call it
> exactly once, do nothing else, and print the tool's raw result.

The real CLIs ran with the user's own `HOME` and `PATH` and with Claude Code's `CLAUDE_*` and
`CLAUDECODE` variables removed, since the probe ran inside a Claude Code session and the
application's service does not. Each CLI had 180 s before the probe would kill it; none needed it.

## Results

| Quickstart step | Result |
|---|---|
| §B1 step 7, Claude Code, Allow after 55 s | **PASS**: the prompt reached the window 12.9 s after the CLI started and was answered at 67.9 s; the tool result arrived and `claude` exited 0 at 69.8 s; the worktree directory is gone |
| §B2 repeat, Copilot, Allow after 55 s | **PASS**: the prompt reached the window at 5.2 s and was answered at 60.2 s; the tool result arrived and `copilot` exited 0 at 63.7 s; the worktree directory is gone. Copilot's own tool line shows the call took `55s` |
| Control, Claude Code, Allow at once | Pass: prompt at 6.8 s, exit 0 at 8.9 s, worktree gone |
| Control, Copilot, Allow at once | Pass: prompt at 9.2 s, exit 0 at 12.5 s, worktree gone |
| Audit lines | Four `INFO micold::mcp` lines, one per call, `outcome=ok` |

Neither CLI's MCP client gave up during the 55 s in which the tool server sent nothing. Both
binding files carry `"timeout": 120000` (research R1). The probe did not run a variant without
that field, so it shows that the wait is survived with the binding as written, not which field is
responsible.

No folder-trust question appeared: print mode (`-p`) does not show one in either CLI, so no
private configuration copy was needed (compare m3-real-cli.md finding 4, which concerns the
interactive session).

## Deviations from quickstart §B1 step 7 and §B2

- Driven by a probe test, not from an app window: the window is a registered fake client that
  answers through `answer_confirmation`, the call the service makes on `ClientMsg::ConfirmationAnswer`.
- The real CLIs ran in print mode with the recorded binding arguments rather than as the session's
  own interactive process, so the answer could be captured verbatim. Print mode fails a tool call
  that needs a permission prompt, so a successful call also shows the pre-approval covers
  `delete_worktree`.
- Claude ran with `--model haiku` to keep the run short; the model does not affect the binding.
- Both callers were in worktree `b` and each deleted worktrees with no sessions, with
  `stop_sessions` left at its default.

## Probe report (verbatim, bearer redacted)

```text
## versions
claude: 2.1.286 (Claude Code)
copilot: GitHub Copilot CLI 1.0.89.
tool server: http://127.0.0.1:46609/mcp
project: /tmp/.tmpr37D6b

## service argv, claude (session 3)
["--session-id", "00000000-0000-0000-0000-000000000003", "--mcp-config", "/tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000003.json", "--allowedTools", "mcp__micold"]

## service argv, copilot (session 2)
["--session-id", "00000000-0000-0000-0000-000000000002", "--no-remote", "--additional-mcp-config", "@/tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000002.json", "--allow-tool", "micold"]

## binding file /tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000003.json (bearer redacted)
{
  "mcpServers": {
    "micold": {
      "headers": {
        "Authorization": "Bearer <redacted>"
      },
      "timeout": 120000,
      "type": "http",
      "url": "http://127.0.0.1:46609/mcp"
    }
  }
}

## binding file /tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000002.json (bearer redacted)
{
  "mcpServers": {
    "micold": {
      "headers": {
        "Authorization": "Bearer <redacted>"
      },
      "timeout": 120000,
      "tools": [
        "*"
      ],
      "type": "http",
      "url": "http://127.0.0.1:46609/mcp"
    }
  }
}

## case claude-control: claude, answer Allow after 0 ms
argv: claude ["-p", "Call the micold delete_worktree tool for worktree `ctl-claude` with delete_branch false. Call it exactly once, do nothing else, and print the tool's raw result.", "--model", "haiku", "--mcp-config", "/tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000003.json", "--allowedTools", "mcp__micold"]
cwd: /tmp/.tmpr37D6b/.claude/worktrees/b
worktree dir exists before: true
### window events (seconds since the CLI was spawned)
6.8s prompt 1 reached the window: caller=00000000-0000-0000-0000-000000000003 DeleteWorktree { stop_sessions: false, delete_branch: false } target="ctl-claude" expires_in_ms=60000
6.8s prompt 1 answered Allow
6.8s prompt 1 withdrawn
### exit Some(0) after 8.9s
### stdout
{"branch_deleted":false,"leftovers":[],"removed":"ctl-claude"}
### stderr

worktree dir exists after: false

## case claude-55s: claude, answer Allow after 55000 ms
argv: claude ["-p", "Call the micold delete_worktree tool for worktree `del-claude` with delete_branch false. Call it exactly once, do nothing else, and print the tool's raw result.", "--model", "haiku", "--mcp-config", "/tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000003.json", "--allowedTools", "mcp__micold"]
cwd: /tmp/.tmpr37D6b/.claude/worktrees/b
worktree dir exists before: true
### window events (seconds since the CLI was spawned)
12.9s prompt 2 reached the window: caller=00000000-0000-0000-0000-000000000003 DeleteWorktree { stop_sessions: false, delete_branch: false } target="del-claude" expires_in_ms=60000
67.9s prompt 2 answered Allow
67.9s prompt 2 withdrawn
### exit Some(0) after 69.8s
### stdout
{"branch_deleted":false,"leftovers":[],"removed":"del-claude"}
### stderr

worktree dir exists after: false

## case copilot-control: copilot, answer Allow after 0 ms
argv: copilot ["-p", "Call the micold delete_worktree tool for worktree `ctl-copilot` with delete_branch false. Call it exactly once, do nothing else, and print the tool's raw result.", "--additional-mcp-config", "@/tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000002.json", "--allow-tool", "micold"]
cwd: /tmp/.tmpr37D6b/.claude/worktrees/b
worktree dir exists before: true
### window events (seconds since the CLI was spawned)
9.2s prompt 3 reached the window: caller=00000000-0000-0000-0000-000000000002 DeleteWorktree { stop_sessions: false, delete_branch: false } target="ctl-copilot" expires_in_ms=60000
9.2s prompt 3 answered Allow
9.2s prompt 3 withdrawn
### exit Some(0) after 12.5s
### stdout
● delete_worktree (MCP: micold) · worktree: "ctl-copilot", delete_branch: false
  └ {

{"branch_deleted":false,"leftovers":[],"removed":"ctl-copilot"}

### stderr

Changes    +0 -0
AI Credits 0.19 (9s)
Tokens     ↑ 28.1k (14.0k cached, 14.1k written) • ↓ 52
Resume     copilot --resume=255505a9-9d1e-4479-a270-2c0517bc6f84

worktree dir exists after: false

## case copilot-55s: copilot, answer Allow after 55000 ms
argv: copilot ["-p", "Call the micold delete_worktree tool for worktree `del-copilot` with delete_branch false. Call it exactly once, do nothing else, and print the tool's raw result.", "--additional-mcp-config", "@/tmp/.tmpmXkTbA/mcp/00000000-0000-0000-0000-000000000002.json", "--allow-tool", "micold"]
cwd: /tmp/.tmpr37D6b/.claude/worktrees/b
worktree dir exists before: true
### window events (seconds since the CLI was spawned)
5.2s prompt 4 reached the window: caller=00000000-0000-0000-0000-000000000002 DeleteWorktree { stop_sessions: false, delete_branch: false } target="del-copilot" expires_in_ms=60000
60.2s prompt 4 answered Allow
60.2s prompt 4 withdrawn
### exit Some(0) after 63.7s
### stdout
● delete_worktree (MCP: micold) · worktree: "del-copilot", delete_branch: false                  55s
  └ {

{"branch_deleted":false,"leftovers":[],"removed":"del-copilot"}

### stderr

Changes    +0 -0
AI Credits 0.19 (1m 2s)
Tokens     ↑ 28.1k (14.0k cached, 14.1k written) • ↓ 52
Resume     copilot --resume=ca2ca9f8-9189-448f-906c-2074359e9da4

worktree dir exists after: false

## git worktree list
/tmp/.tmpr37D6b                     c028f40 [main]
/tmp/.tmpr37D6b/.claude/worktrees/b c028f40 [b]

## audit lines
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000003 op=delete_worktree target=ctl-claude outcome=ok
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000003 op=delete_worktree target=del-claude outcome=ok
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000002 op=delete_worktree target=ctl-copilot outcome=ok
INFO micold::mcp: tool call caller=00000000-0000-0000-0000-000000000002 op=delete_worktree target=del-copilot outcome=ok
```

In the report above, Claude wrapped its one-line answer in a Markdown code fence; the fence lines
are left out here so the block stays well formed. The audit lines are shown without their
timestamps.
