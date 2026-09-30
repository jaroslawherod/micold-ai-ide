# Tools for the AI in your sessions

Every Claude Code and GitHub Copilot session the application starts can see your project the way the
sidebar shows it: which worktrees exist, which branches are free, and which sessions are running
where. It can also create and rename a worktree and create or start a session, as you would from
the sidebar. The session service provides this through a small tool server named `micold`, and each
session is connected to it automatically. You do not install or configure anything.

Ask the assistant in a session something like *"which worktrees does this project have?"* or *"is
anyone else working on the `parser` branch?"* and it answers from the same list you see. Ask it to
*"create a worktree for `feat-login` and start a Claude Code session there that fixes the login
form"*, and the new worktree and session appear in every open window within a couple of seconds.

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

These tools make changes, exactly as the sidebar does:

| Tool | What it does |
|---|---|
| `create_worktree` | Creates a worktree for a branch, as the new-worktree dialog does: under `.claude/worktrees/`, named after the branch unless a name is given, and marked as created by the application. By default it makes a new branch; `existing_local` checks out a branch that exists and is free, and `track_remote` tracks a remote branch |
| `rename_worktree` | Gives a worktree a new name in the sidebar, as the sidebar's rename does. Its folder and branch stay as they are. The name cannot be empty, and `default` (the project folder) cannot be renamed |
| `create_session` | Creates a session in a worktree (or in `default`, the project folder) and starts it. It runs the AI CLI the assistant names, or your default AI CLI from Settings. It can also be given a first prompt to type into the new session |
| `start_session` | Starts a session that is idle, failed, or waiting to be resumed, as **Start** in the sidebar does: a session that was running when the service last stopped resumes its conversation. Every window shows it starting, then running. A session that is already starting, running or restarting is left as it is, and the result says which it is |

A request the dialog would refuse is refused the same way, with the dialog's own explanation: a
branch that already exists or is checked out elsewhere, a name the naming rules reject, or a branch
name git does not accept. Nothing is created when a request fails. When several requests race for
the same branch, one succeeds and the others are told the branch is taken.

A session running in the project folder itself (`default`) cannot create or rename worktrees. Work
in the project folder is kept separate from worktrees, and an assistant there is refused with that
reason. It can still list everything and create and start sessions.

### The first prompt

When `create_session` is given a prompt, it waits until the new session's AI CLI is ready to take
it, types it in, and submits it once. A prompt of several lines arrives as one message. How the
service knows the CLI is ready depends on the CLI:

| AI CLI | Ready when |
|---|---|
| Claude Code | Its screen has shown something and then stopped changing for 1.5 seconds |
| Pi Coding Agent | Pi reports that its session has started, through the activity reporter the application loads into it. If you turned off **Show activity for Pi sessions**, it is ready once its screen has stopped changing for 1.5 seconds |
| GitHub Copilot | Its screen has shown something and then stopped changing for 1.5 seconds |

Claude Code and GitHub Copilot ask whether you trust a folder the first time they run in it. The
service never answers that question for you: before it waits, it reads the CLI's own record of the
folders you trust (it never changes it), and if the CLI would ask, the prompt is not typed. The
session starts and waits at the question for you. **Trust the project in that CLI first**: run
Claude Code or Copilot once in the project folder and accept its question. Worktrees inside the
project are then trusted too, and first prompts arrive. Pi asks no such question.

The service waits at most 60 seconds from the request. If the CLI is not ready by then, the session
failed to start, or the CLI would ask about trusting the folder, the prompt is not typed at all,
not even later. The result says `prompt_delivered: false`, and `prompt_reason` says which of these
happened. The session itself is still there; type into it yourself or ask the assistant to try
again.

If the AI CLI is not installed where the session would run, `create_session` fails, names the CLI,
and leaves no session behind.

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

The assistant is allowed to use the `micold` tools without asking you each time. They reach
nothing outside the project the session already works in, and they change only what you could
change from the sidebar.

## What the log records

The session service writes one line to its log for every change an assistant asks for, whether it
succeeds or not: the calling session, the tool, what it acted on (a branch, worktree or session), and
`ok` or why it failed. For example:

```text
INFO micold::mcp: tool call caller=6f1c… op=create_worktree target=feat-login outcome=ok
```

A prompt, and anything else an assistant types into a session, is never written to the log.

## If you already have a server named `micold`

If your own configuration already defines an MCP server named `micold` — in `~/.claude.json` (or
`$CLAUDE_CONFIG_DIR/.claude.json`), in the project's `.mcp.json`, or in Copilot's
`~/.copilot/mcp-config.json` — the application does not override it. The session starts without the
application's tools, and the service log names the file:

```text
no tool server: a server named "micold" is already configured in /home/you/.claude.json
```

Rename your server to connect the session again; the change applies from the session's next start.
