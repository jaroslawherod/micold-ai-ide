# Tools for the AI in your sessions

Every Claude Code and GitHub Copilot session the application starts can see your project the way the
sidebar shows it: which worktrees exist, which branches are free, and which sessions are running
where. The session service provides this through a small tool server named `micold`, and each
session is connected to it automatically. You do not install or configure anything.

Ask the assistant in a session something like *"which worktrees does this project have?"* or *"is
anyone else working on the `parser` branch?"* and it answers from the same list you see.

## What the assistant can do

These tools only read. None of them changes anything.

| Tool | What it answers |
|---|---|
| `whoami` | Which session is asking, its project, the worktree it runs in, and its AI CLI |
| `list_worktrees` | The project's worktrees as the sidebar lists them: `default` (the project folder itself) first, then each worktree with its branch, its health (`clean`, `missing` or `prunable`), whether the application created it, and how many sessions it hosts |
| `list_branches` | Every local and remote-tracking branch, the worktree it is checked out in, and, when a new worktree could not use it, the same explanation the new-worktree dialog gives |
| `list_sessions` | The project's sessions with their label, AI CLI, state, activity and worktree. The calling session is marked. It can be narrowed to one worktree |
| `get_session` | One session. A session that failed to start includes the reason |

Worktrees an assistant created for itself are left out, as they are in the sidebar, unless the
assistant asks for them with `include_hidden`.

## What it can see

**Its own project, and nothing else.** A session's assistant sees the worktrees, branches and
sessions of the project it runs in. Another project's sessions are reported as "not found", exactly
as a session that does not exist is, so an assistant cannot even learn that another project has one.

Each session connects with its own key. The key is created when the session starts, stops working
when the session is deleted (directly or with its worktree), and is never reused after the service
restarts. The tool server listens only on your own computer (`127.0.0.1`) and refuses a request
without a valid key; the key file is readable only by you.

## Which sessions are connected

| AI CLI | Connected |
|---|---|
| Claude Code | Yes |
| GitHub Copilot | Yes |
| Pi Coding Agent | No: Pi has no support for tool servers of this kind (MCP) |
| Regular terminal | No: there is no assistant to connect |

A session that is not connected works exactly as before. The session service writes one line to
its log saying why, for example `no tool server: Pi has no MCP support`.

To stop connecting sessions, turn off **Let AI sessions manage worktrees and sessions** under
Settings → Environment ([Settings](settings.md#let-ai-sessions-manage-worktrees-and-sessions)). It
applies to sessions started afterwards; running sessions keep their connection, and each new session
logs `no tool server: disabled in settings`.

## Your configuration is left alone

The application never writes to your own Claude Code or Copilot configuration. Each session is
started with a few extra arguments pointing it at a per-session file in the session service's own
data directory. Your own MCP servers keep working beside `micold`, and nothing is left in your files
when you stop using the application.

The assistant is allowed to use the `micold` tools without asking you each time. They only read,
and they read nothing outside the project the session already works in.

## If you already have a server named `micold`

If your own configuration already defines an MCP server named `micold` — in `~/.claude.json` (or
`$CLAUDE_CONFIG_DIR/.claude.json`), in the project's `.mcp.json`, or in Copilot's
`~/.copilot/mcp-config.json` — the application does not override it. The session starts without the
application's tools, and the service log names the file:

```text
no tool server: a server named "micold" is already configured in /home/you/.claude.json
```

Rename your server to connect the session again; the change applies from the session's next start.
