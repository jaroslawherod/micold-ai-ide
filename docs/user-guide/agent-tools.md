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
| `list_sessions` | The project's sessions with their label, AI CLI, state, activity and worktree. The calling session is marked. A session that another session started also names that session (`origin`) and whether it still exists (`origin_state`: `known` or `removed`); sessions you started carry neither. It can be narrowed to one worktree |
| `get_session` | One session. A session that failed to start includes the reason; a session another session started names it in `origin` and `origin_state` |
| `list_resumable_sessions` | Sessions the AI CLIs recorded for this project that the application does not list yet, newest first, with the worktree each ran in and whether it can be resumed (`resumable` or `unresumable`, with the reason). It takes `limit` (1 to 200, 50 unless given), `offset` and an optional `worktree`. It only reads (so a Default session may call it), and it touches none of the CLIs' files. `total` counts at most the 200 newest sessions, and `provider` names the AI CLI by its tool name, such as `claude_code`. Resuming one is done in the app |
| `read_session_output` | The most recent lines of another session's terminal, as plain text: 200 lines unless the assistant asks for a different number, and never more than 2,000. See [Reading and typing into other sessions](#reading-and-typing-into-other-sessions) |

Every worktree is listed, as it is in the sidebar; no name hides one.

These tools make changes, exactly as the sidebar does:

| Tool | What it does |
|---|---|
| `create_worktree` | Creates a worktree for a branch, as the new-worktree dialog does: under `.claude/worktrees/`, named after the branch unless a name is given. By default it makes a new branch; `existing_local` checks out a branch that exists and is free, and `track_remote` tracks a remote branch. Instead of a branch, it can take a `type`, a `ticket` and a `name`, as the new-worktree form does (see below); a worktree for an issue should be created this way, or with `github_issue`, so that it carries the issue reference; a `github_issue` call also reports what blocks the issue and its other relationships (see below). With `ai_cli` or `prompt` it also creates the worktree's one session in the same call (see below) |
| `rename_worktree` | Gives a worktree a new name in the sidebar, as the sidebar's rename does. Its folder and branch stay as they are. The name cannot be empty, and `default` (the project folder) cannot be renamed |
| `create_session` | Creates a session in a worktree (or in `default`, the project folder) and starts it. It runs the AI CLI the assistant names, or your default AI CLI from Settings. It can also be given a first prompt to type into the new session. A worktree holds at most one session by default: when it already has one, the call fails with a conflict that names the worktree's main session (its name and id), says that `start_session` on that id brings it back, and says to pass `allow_additional_session: true` to start another on purpose; nothing is created. With `allow_additional_session: true` (optional, default false) the call creates a second session beside the existing one and leaves that one untouched; the policy, the AI CLI check and every other check still apply |
| `start_session` | Starts a session that is idle, failed, or waiting to be resumed, as **Start** in the sidebar does: a session that was running when the service last stopped resumes its conversation. Every window shows it starting, then running. A session that is already starting, running or restarting is left as it is, and the result says which it is. Starting a session while another session of its worktree is starting or running fails with a conflict that names the running one (its name and id) and says to stop it first or to pass `allow_additional_session: true` (optional, default false) to run both on purpose; nothing is started. Policy and every other check still apply |
| `send_session_input` | Types text into another session and submits it once, as if you had typed it there and pressed Enter. See [Reading and typing into other sessions](#reading-and-typing-into-other-sessions) |
| `report_review_comment` | Reports the outcome of one comment of a review you sent the session: applied, or answered with a reply saying why not. See [Reporting on review comments](#reporting-on-review-comments) |

#### Starting the worktree's session with it

A worktree's session is defined when the worktree starts. Give `create_worktree` an `ai_cli` (one
of the AI CLIs `create_session` accepts), a `prompt`, or both, with either form of the call, and the
worktree and its one session are created together:

- `ai_cli`: the session's AI CLI. With only a `prompt` it is your default AI CLI from Settings.
- `prompt`: the session's first input, typed once the CLI is ready, as for `create_session`.

The result is the worktree's row plus `session` (its id), `lifecycle` and, when a `prompt` was
given, `prompt_delivered` (and `prompt_reason` when it was not typed), exactly as `create_session`
returns them. The session runs in the new worktree, also when the caller runs in the project folder
(`default`).

If the AI CLI is not available where the session would run, the call fails with the reason and
creates nothing: no worktree and no session. If the worktree is created and its session cannot be,
the error names the worktree, which then has no session; `create_session` starts its one. Without
`ai_cli` and `prompt` the call is as it always was: a worktree with no session, to which
`create_session` adds its one session. A worktree holds at most one session, so a `create_session`
for it after this one is refused with a conflict that names the existing session.

#### Creating a worktree from a type, ticket and name

`create_worktree` takes either a literal `branch` (with `mode`, `remote` and an optional directory
`name`) or the inputs of the new-worktree form, never both:

- `type`: one of `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `build`, `ci`, `perf`, `style`.
- `ticket`: optional; a reference such as `#123` or `ABC-123`. Blank means none. It is what puts
  the issue tag on the worktree's sidebar row and makes the issue filter find it.
- `github_issue`: optional; the number of an open issue in the project's GitHub repository (see
  below).
- `name`: the description. Without a `branch` it is the description, not the folder name.

The branch and folder are built exactly as the form builds them: type `fix`, ticket `#123` and name
`login crash` give the branch `fix/123_login-crash` in the folder `fix-123_login-crash`, with the
type and issue tags in the sidebar. Without a ticket there is no ticket part. The result lists the
worktree and also its `branch`, `directory`, `type` and (when there is one) `ticket`. A derived
branch that already exists is refused, and nothing is created: use `branch` with `mode` to reuse
it.

When a worktree is for an issue or ticket, an agent should include the issue reference in its name:
pass `ticket` (with `type` and `name`) or `github_issue`, which name the branch
`${type}/${ticket}_${name}` and the folder `${type}-${ticket}_${name}`. A literal `branch` that does
not carry the `_` boundary gets no issue tag and is missed by the issue filter. The tool's own
description and the server's `initialize` instructions tell the agent the same.

With `github_issue`, the issue fills in what you leave out, as picking it in the form does: the
ticket is the issue number, the name is its title (cut to fit, as the form cuts it) and the type
comes from the first entry of the label-to-type mapping in Settings that matches one of its labels,
ignoring case. A `type`, `ticket` or `name` you pass replaces the one from the issue. If no label
maps to a type and you pass no `type`, the call is refused with "Select a type". The lookup uses
your existing GitHub sign-in (the `gh` command) and sends only the repository and the issue number;
it happens only when `github_issue` is passed, so every other call works offline and signed out.
While **Offer GitHub issues when creating worktrees** is off in Settings (GitHub), a call with
`github_issue` is refused with `invalid_input`, naming the switch and pointing to `branch`, or `type`,
`ticket` and `name`; it makes no GitHub request and creates nothing. Calls without it are unchanged.
It is refused, and nothing is created, with the same explanation the form gives, when the issue is
closed, is a pull request or does not exist, the project has no GitHub remote, `gh` is not
installed or not signed in, the sign-in cannot see the repository, GitHub cannot be reached or its
rate limit is reached, or GitHub does not answer within 10 seconds.

#### What a `github_issue` call reports

When a `github_issue` call creates the worktree, its result also carries an `issue` object, so the
agent knows what it is starting on. It holds the issue's `number` and, for each kind the issue has,
that kind's data:

- `blocked_by` (with how many of the blockers are still `open`) and `blocking`;
- `parent` and `sub_issues` (with how many are `closed` of the `total`);
- `labels`, `assignees`, `milestone`, and `projects` with each project's non-empty field values;
- `closing_pull_requests` and `linked_pull_requests`, each with its state: open, draft, merged or
  closed.

A kind the issue does not have is left out, so an issue with none of them reports only its number.
`blocked: true` appears when at least one blocking issue is still open. Being blocked is reported,
never enforced: the worktree is created all the same.

Each list shows at most 20 items. `more` says how many are not shown, and `hidden` how many your
sign-in cannot see (they still count in the total). When the linked pull requests could not all be
counted in time, `total_is_lower_bound` says the total is at least that many.

A kind your sign-in cannot read (projects need the `read:project` scope) is listed under
`not_readable` with the reason, and the other kinds are still reported. When nothing could be read
at all (GitHub cannot be reached, its rate limit is reached, or it does not answer within 10
seconds), the object holds only the number and a `details_error` sentence; the worktree is still
created. The details are read beside the lookup, so the call waits for them at most 10 seconds
after the read starts, and usually not beyond the time the worktree takes to create. A call without
`github_issue` reads nothing more, and a refused call carries no `issue` object.

A request the dialog would refuse is refused the same way, with the dialog's own explanation: a
branch that already exists or is checked out elsewhere, a name the naming rules reject, or a branch
name git does not accept. Nothing is created when a request fails. When several requests race for
the same branch, one succeeds and the others are told the branch is taken.

A session running in the project folder itself (`default`) can create a worktree, so an assistant
there can hand work to a worktree of its own instead of doing it in your project folder. It cannot
rename or delete a worktree: work in the
project folder is kept separate from the worktrees that exist, and an assistant there is refused
with that reason. It can still list everything and create,
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

### Sub-agent runs

The app registers for Claude Code's `SubagentStart` and `SubagentStop` events in each Claude Code
session it starts, and records each sub-agent run on the session that owns it: the run's id, its
kind and whether it is still going. The record is kept across restarts of the service, and a run
still going at that point is marked stopped. A report that names no run, or comes for a session the
app does not know, is ignored. Sub-agent runs do not change the session's activity: a sub-agent
finishing is not the end of the session's turn. Background sessions started with `claude --bg` are
not reported this way and are not recognised.

Which sessions the app can tell are agent work depends on the AI CLI:

| AI CLI | Native signal the app uses |
|---|---|
| Claude Code | `SubagentStart` and `SubagentStop`, registered in each session the app starts |
| GitHub Copilot | None known: no sub-agent event has been confirmed in its event log, so none is recorded |
| Pi Coding Agent, Codex, OpenCode | None: they offer no such signal, so every session is shown as your own |
| Claude Code started with `claude --bg` | Not covered: not reported through these events |

The app never guesses from a worktree's name, its branch or when it appeared.

### The first prompt

Without a prompt, `create_session` (and `create_worktree`, when `ai_cli` or `prompt` makes it start
a session) types your default prompt template from Settings, filled in for the worktree: its issue
reference, branch, type and name, and the issue's title and body when GitHub answers within 5
seconds (otherwise those two stay empty). A prompt you pass always wins, and the template is then
not used. With no template set nothing is typed, and a template alone never makes `create_worktree`
start a session.

When `create_session` is given a prompt, it waits until the new session's AI CLI is ready to take
it, types it in, and submits it once. A prompt of several lines arrives as one message. How the
service knows the CLI is ready depends on the CLI:

| AI CLI | Ready when |
|---|---|
| Claude Code | Its screen has shown something and then stopped changing for 1.5 seconds |
| Pi Coding Agent | Pi reports that its session has started, through the activity reporter the application loads into it. If you turned off **Show activity for Pi sessions**, it is ready once its screen has stopped changing for 1.5 seconds |
| GitHub Copilot | Its screen has shown something and then stopped changing for 1.5 seconds |
| Codex, OpenCode | Their screen has shown something and then stopped changing for 1.5 seconds |

Claude Code, GitHub Copilot and Codex ask whether you trust a folder the first time they run in it. The
service never answers that question for you: before it waits, it reads the CLI's own record of the
folders you trust (it never changes it), and if the CLI would ask, the prompt is not typed. The
session starts and waits at the question for you. **Trust the project in that CLI first**: run
Claude Code, Copilot or Codex once in the project folder and accept its question. Worktrees inside
the project are then trusted too, and first prompts arrive. Pi and OpenCode ask no such question.

The service waits at most 60 seconds from the request. If the CLI is not ready by then, the session
failed to start, or the CLI would ask about trusting the folder, the prompt is not typed at all,
not even later. The result says `prompt_delivered: false`, and `prompt_reason` says which of these
happened. The session itself is still there; type into it yourself or ask the assistant to try
again.

If the AI CLI is one a session would not find in the target directory, `create_session` fails and
leaves no session behind. The reply gives the same reason and action a person is shown, for example
that "Source a script before each session" is off, or that the startup script was not found. See
[When a CLI isn't installed](./worktrees-and-sessions.md#when-a-cli-isnt-installed).

### One session per worktree is a soft limit

A worktree, or `default`, holds one session unless you ask for more on purpose. `create_session`
without `allow_additional_session` refuses a second session with the conflict described in the
table above, so two assistants cannot end up editing the same files by accident. An assistant that
really wants a second session passes `allow_additional_session: true`; a value that is not a
boolean is an input error.

When you start a second session yourself in an occupied worktree, the app asks you to confirm. An
assistant is never asked: nobody is present to answer, and an assistant silently doubling the
sessions in one worktree is the harm the limit guards against. So an assistant's `create_session`
or `start_session` is refused unless it sets `allow_additional_session: true`, and no confirmation
opens in any window.

The first session of a worktree is its **main session**; a session created with
`allow_additional_session: true` beside it is a helper. The conflict names the main session.
`list_sessions` rows do not mark which session is main, and the listing order does not indicate
it. Only you change which session is main, from the app.

### Sessions cannot enter or add worktrees

A Claude Code session the app launches cannot call its `EnterWorktree` tool, and it cannot run
`git worktree add`, including `git -C <dir> worktree add`. Both are denied with a message. The
check reads the command text, so a command built by a script or a shell wrapper it cannot see
through can slip past it. The route is the one above: `create_worktree`, which stays available, then
`create_session` for the new worktree.

A successful `create_worktree` result carries a `next_step` field, repeated as the last line of the
result text: do not work in the new worktree from this session, and hand the work to a dedicated
session with `create_session` for that worktree.

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

## Reporting on review comments

When you send a review from the Changes view
([Sending a review](reviewing-changes.md#sending-a-review)), each comment in the message carries a
reference such as `C-1a2b3c4d`. The session answers a comment with `report_review_comment`:

| Argument | What it is |
|---|---|
| `comment` | The comment's reference from the review message |
| `outcome` | `applied` when it made the change, `answered` when it did not and the reply says why |
| `reply` | What it did, or why not. Required for `answered`, optional for `applied`; at most 20,000 characters |

The comment's card then shows **Applied** or **Answered** and the reply, in every window open on the
project and after a restart
([Statuses and replies](reviewing-changes.md#statuses-and-replies)). A later report on the same
comment replaces the outcome and the reply.

A session reports only on its own entry's comments: a session in a worktree on that worktree's
comments, and a session in the project folder on the Default entry's. A report changes only the
comment's status and reply, so it never waits for you to allow it. It is refused, and nothing
changes, when:

- the reference is malformed, the outcome is neither value, an `answered` report has no reply, or
  the reply is over 20,000 characters;
- the comment belongs to another worktree;
- no comment has that reference (it may have been cleared or discarded);
- the comment is in a send that has not finished delivering;
- the comment is still a draft, never sent to the session.

The log records the reference of each report, never the reply.

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
| Codex | No: the app does not connect it to its tool server yet; the log line says `no per-launch tool-server binding was verified for Codex` |
| OpenCode | No: the app does not connect it to its tool server yet; the log line says `no per-launch tool-server binding was verified for OpenCode` |
| Regular terminal | No: there is no assistant to connect |

A session that is not connected works exactly as before. The session service writes one line to
its log saying why, for example `no tool server: Pi has no MCP support`.

To stop connecting sessions, turn off **Let AI sessions manage worktrees and sessions** under
Settings → Environment ([Settings](settings.md#let-ai-sessions-manage-worktrees-and-sessions)). It
applies to sessions started afterwards; running sessions keep their connection, and each new session
logs `no tool server: disabled in settings`.

## One tool server per daemon

Each session service (daemon) runs its own tool server over its own worktrees and sessions. When
you run a host daemon and container daemons together, the AI in a session sees and manages only the
worktrees and sessions of the daemon that session runs on. It cannot list, create or read sessions
on another daemon, and there is no cross-daemon tool. See [Running a host daemon and container
daemons together](sandboxed-daemon.md#running-a-host-daemon-and-container-daemons-together).

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
