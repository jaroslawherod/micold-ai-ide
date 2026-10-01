# Tools for the AI in your sessions

Every Claude Code and GitHub Copilot session the application starts can see your project the way the
sidebar shows it: which worktrees exist, which branches are free, and which sessions are running
where. It can also create, rename and delete a worktree, create, start, stop, interrupt and delete
a session, and read and type into another session of the project. The requests that stop or remove
something wait for you to allow them first, and you decide in Settings whether sessions may read
and type into each other. The session service provides this through a small tool server named
`micold`, and each session is connected to it automatically. You do not install or configure
anything.

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
| `read_session_output` | The most recent lines of another session's terminal, as plain text: 200 lines unless the assistant asks for a different number, and never more than 2,000. See [Reading and typing into other sessions](#reading-and-typing-into-other-sessions) |

Worktrees an assistant created for itself are left out, as they are in the sidebar, unless the
assistant asks for them with `include_hidden`.

These tools make changes, exactly as the sidebar does:

| Tool | What it does |
|---|---|
| `create_worktree` | Creates a worktree for a branch, as the new-worktree dialog does: under `.claude/worktrees/`, named after the branch unless a name is given, and marked as created by the application. By default it makes a new branch; `existing_local` checks out a branch that exists and is free, and `track_remote` tracks a remote branch |
| `rename_worktree` | Gives a worktree a new name in the sidebar, as the sidebar's rename does. Its folder and branch stay as they are. The name cannot be empty, and `default` (the project folder) cannot be renamed |
| `create_session` | Creates a session in a worktree (or in `default`, the project folder) and starts it. It runs the AI CLI the assistant names, or your default AI CLI from Settings. It can also be given a first prompt to type into the new session |
| `start_session` | Starts a session that is idle, failed, or waiting to be resumed, as **Start** in the sidebar does: a session that was running when the service last stopped resumes its conversation. Every window shows it starting, then running. A session that is already starting, running or restarting is left as it is, and the result says which it is |
| `send_session_input` | Types text into another session and submits it once, as if you had typed it there and pressed Enter. See [Reading and typing into other sessions](#reading-and-typing-into-other-sessions) |

A request the dialog would refuse is refused the same way, with the dialog's own explanation: a
branch that already exists or is checked out elsewhere, a name the naming rules reject, or a branch
name git does not accept. Nothing is created when a request fails. When several requests race for
the same branch, one succeeds and the others are told the branch is taken.

A session running in the project folder itself (`default`) can create a worktree, so an assistant
there can hand work to a worktree of its own instead of doing it in your project folder. It cannot
rename or delete a worktree: work in the project folder is kept separate from the worktrees that
exist, and an assistant there is refused with that reason. It can still list everything and create,
start, stop and delete sessions.

### Requests that wait for you

These tools stop or remove something, so each request waits until you allow it:

| Tool | What it does once you allow it |
|---|---|
| `stop_session` | Stops another session: its processes end, every window shows it idle, and its conversation can be resumed later with `start_session`. A session that is already stopped is left as it is, and nobody is asked |
| `interrupt_session` | Types **Ctrl-C** into another running session, as if you pressed it in its terminal. The session keeps running. A session that is not running is refused, and nobody is asked |
| `delete_session` | Removes another session, as removing it from the sidebar does |
| `delete_worktree` | Deletes a worktree and, unless the assistant asks to keep it, its branch. A worktree with running sessions is refused, naming them, unless the assistant asks to stop them (`stop_sessions`); nobody is asked then |

When the assistant asks, **every open window** shows a dialog naming the session that asks, what it
wants to do, and to what. **Allow** does it; **Deny** refuses it, and the assistant is told you
declined. Escape, a click outside the dialog, or opening another dialog over it counts as Deny. The
first answer from any window counts, and the dialog closes in every other window. If you are in
another dialog at that moment (say, halfway through a new-worktree form), the request waits until
you close it, so nothing you typed is lost.

Nothing changes until you allow it. The request fails with *needs confirmation* if no window is open
to ask, or if nobody answers within 60 seconds. It fails with *not found* if the session or worktree,
or the session that asked, goes away while the dialog is open; the dialog then closes by itself. If
the assistant gives up waiting (its session is stopped, or it cancels the request), the dialog closes
and nothing is done.

Requests that would be refused anyway never show a dialog. A session cannot stop, interrupt or
delete itself, or delete the worktree it runs in; ask from another session, or use the sidebar.

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

## Reading and typing into other sessions

An assistant can read what another session of the same project shows, and type into it. This is
how one session hands work to another: it starts a session with a prompt, reads its terminal to see
how far it got, and sends it the next instruction.

`read_session_output` returns the last lines of the other session's AI CLI terminal, oldest first,
as the text you would read there: no colours and no control codes. It reads the AI CLI's terminal
even while you have one of the session's shell terminals open instead. The result says whether
older lines exist beyond the ones returned.

`send_session_input` types the text into the other session's AI CLI and submits it once. Text of
several lines arrives as one message. The text cannot be empty or only line breaks, and it cannot
hold control characters such as Ctrl-C or Escape: an assistant that wants to interrupt another
session uses `interrupt_session`, which asks you first.

Both need the other session to be running; otherwise they fail and name `start_session`. Neither
works on the assistant's own session. Nothing is typed into a session whose AI CLI has not yet been
told to trust the project folder, because the text's Enter would answer that question for you:
the request fails and says to trust the project in that CLI first.

**You decide whether assistants may do this.** Settings → Environment has **Let agents read and
type into other sessions**
([Settings](settings.md#let-agents-read-and-type-into-other-sessions)):

| Value | Reading | Typing |
|---|---|---|
| **Auto** (the default) | Allowed | Allowed |
| **Confirm each send** | Allowed | Each message waits for you to allow it in an app window. If you decline, or no window is open, or you do not answer within 60 seconds, nothing is typed |
| **Off** | Refused | Refused |

A change applies to the very next request, also from sessions that are already running. A refused
request leaves the other session untouched and tells the assistant that the option in Settings
refused it.

## What it can see

**Its own project, and nothing else.** A session's assistant sees the worktrees, branches and
sessions of the project it runs in. Another project's sessions are reported as "not found", exactly
as a session that does not exist is, so an assistant cannot even learn that another project has one.

Each session connects with its own key. The key is created when the session starts, stops working
when the session is deleted (directly or with its worktree), and is never reused after the service
restarts. The tool server listens only on your own computer (`127.0.0.1`) and refuses a request
without a valid key; the key file is readable only by you.

When the session service runs [in a container](sandboxed-daemon.md), the sessions run there too, and
so does the tool server: it listens inside the container, its port is not published, and nothing
on your computer outside the container can reach it. The assistant sees the same tools either way.

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
