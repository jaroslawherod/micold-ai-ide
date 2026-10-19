# Settings

Open **Settings** from the overflow menu (the three-dots button) in the top toolbar. It fills the
main area, with a rail down the left listing seven sections. The app bar and the connection strip
stay where they are, so the way back out is always in view.

**Collapse**, at the bottom of the rail, slides the rail in to its icons and gives the width to the
section. Collapsed, the same control is drawn as the show-sidebar icon, and pressing it slides the
rail back out. Every section stays one press away while the rail is collapsed or moving, and a
section with something to report keeps marking its row throughout: by its badge while there is room
for it, and by a tinted icon when there is not.

Editing is one form across all the sections except Daemons, which saves its own list as you go: switching sections never discards what you typed, and
**Save** applies every section at once. If a value is rejected, Settings jumps to the section
holding it and marks the field — press **Cancel**, or Esc, to leave without saving anything.

Press **Save** with a required field empty and every empty one reads "This field is required" at
once, across all the sections, and nothing is saved. Settings jumps to the section holding a value
that is filled in but wrong, which keeps its own message, else to the first section with an empty
required field. Leaving a required field empty also marks that field alone; the others wait for **Save**.

A page or form with a field marked `*` shows a `* required` line above its first field; Appearance,
which has none, shows no such line. Fields marked `*` must be filled before Settings accepts the form: **Scrollback lines** in
Terminal; **Long-task threshold**, **Warn at** and the script timeout in Environment; each **Label**
in GitHub; and in Jira the Site URL, Project key, Account email and API token, once you start
filling the connection in (the token only while none is stored). Appearance has nothing to fill. In
the Daemons form, **Name** is always marked; the **Host** appears marked when you choose SSH and
goes away when you switch back; a container daemon marks **Container name** and **Image reference**,
and **Image file** when the image comes from a file; a remote daemon's **Repository on the host** is
marked too. Port, User and Key file are optional and carry no mark.

| Section | What it holds |
| --- | --- |
| [Appearance](#appearance) | The theme |
| [Terminal](#terminal) | The embedded terminal's scrollback limit, and whether its history is saved to disk |
| [Environment](#environment) | Which AI CLI a session runs, whether Pi sessions report activity, whether AI sessions get the app's tools, and the script sourced before it starts |
| [Daemons](#daemons) | The session services you have set up: add, edit, start, stop and remove them, and each container daemon's runtime, image, credentials and resources |
| [GitHub](#github) | Whether worktrees show their pull request, which worktree type an issue's labels choose, and the GitHub access tokens that let sandboxes push |
| [Jira](#jira) | Whether Jira issues can start worktrees, and whether the open project is connected |

<!-- media: settings-view-light -->

Everything you save lands in one file on this computer — see [Where settings are
stored](#where-settings-are-stored) for the path, and for what the app does when that file can't be
read.

## Appearance

**Theme** — follow the system, or pin light or dark. Following the system switches with it while
the app is running.

The app bar's mode button cycles the same setting, for when you want it in one press.

## Terminal

**Scrollback lines** controls how many lines of earlier output each session's terminal keeps for
scrolling back through (see [Worktrees & sessions → Sizing, resize &
scrollback](./worktrees-and-sessions.md)).

- **Default**: 10,000 lines.
- **Range**: 100 – 1,000,000 lines. Values outside the range (or non-numeric input) are
  rejected with a message and not saved.
- The value is **saved on your machine** and restored the next time you open the app.
- A changed limit applies to sessions started **after** the change; already-running terminals
  keep their current buffer.
- A very large value makes each saved history larger, because a saved history holds up to this many
  lines per session.

**Save terminal history** keeps each session's scrollback on disk, so it is still there after the
background service or the computer restarts (see [Worktrees & sessions → Earlier output after a
stop and start](./worktrees-and-sessions.md#earlier-output-after-a-stop-and-start)).

- **Default**: on.
- It also keeps the event history of [structured
  sessions](./worktrees-and-sessions.md#structured-sessions) on disk, so their log is still there
  after the background service restarts.
- Turning it **off** deletes every saved history, and every structured session's saved events, at
  once, without asking, and nothing is written until you turn it back on. It takes effect when you press **Save**.
- With it off, a stop and start still shows the earlier output in the terminal; only a restart of
  the background service or the computer loses it.
- If a file cannot be deleted at that moment, the service retries until it succeeds.

## Environment

### Default AI CLI

Which AI coding CLI a new session runs when you don't choose one for it.

- **The choices** are the CLIs you actually have installed where sessions run: on this computer, or
  in the container's image when the [session service](#container-daemons) runs in one. If only one is
  installed, that is the only option — and nothing else about starting a session changes for you.
- **"Installed" means a session would find it.** A CLI is offered when it is on the `PATH` a session
  starts with, and that includes what
  [the environment a session starts in](#the-environment-a-session-starts-in) adds. So a CLI you
  installed through a version manager — `pi`, `codex`, `opencode` or `copilot` from `npm install -g` under mise or nvm,
  say — is offered as long as environment-include is on and your startup file sets that version
  manager up. This field answers for your home directory, because the default applies everywhere.
  Each sidebar row answers for its own project or worktree, so a CLI one project's script adds is
  offered on that project's rows even when it is not listed here.
- **A note under the field names any CLI a session would not find, says why, and says what to
  change.** It answers for your home directory, like the field itself. The reason is one of six,
  following the state of [the environment a session starts in](#the-environment-a-session-starts-in):

  | State | What the note says | What it tells you to do |
  |---|---|---|
  | **Source a script before each session** is off | Sessions get only the login `PATH` | Turn it on if your startup file puts the CLI on the `PATH`, or install the CLI on the login `PATH` |
  | It is on and **Script path** is empty | No script is sourced | Set **Script path**, or install the CLI on the login `PATH` |
  | No file exists at the script path | The startup script was not found for your home directory | Correct **Script path** |
  | The script exited with an error (this includes a path that exists but cannot be sourced) | The startup script exited with an error for your home directory | Fix the script |
  | The script ran past the timeout | The startup script timed out for your home directory | Fix the script, or raise **Timeout** |
  | The script was applied and the CLI is still not found | The CLI was not found on the `PATH` sessions get for your home directory: the login `PATH` plus what the script adds | Install the CLI, or make the script add its directory |

  Only the last row means the CLI is not there. In the first five the note does not say the CLI is
  not installed, because no script was applied and the app cannot know. The note is complete by
  itself: it does not depend on the outcome line in the environment-include group, which reports
  the directory resolved most recently and may be about another project. Saving a change to these
  settings asks again, so the note and the list are up to date the next time you open Settings,
  with no restart. Nothing is said when every CLI is found, or before the session service has
  answered.
- **Default**: Claude Code.
- **Changing it affects new sessions only.** A session's CLI is fixed when the session is created
  and never changes afterwards, so sessions you already have keep running the CLI they started on.
- You can override it per session without touching this setting — see
  [Worktrees & sessions → Choosing which AI CLI a session runs](./worktrees-and-sessions.md#choosing-which-ai-cli-a-session-runs).

**A CLI you installed is not offered?** It is not on the `PATH` a session gets. Either keep
**Source a script before each session** on, with a script that sets up the version manager you
installed it with, or install the CLI somewhere on the `PATH` your login session already has (for
example with a symlink in `~/.local/bin`). The app does not look anywhere else, because a session
would not either.

**If your default names a CLI a session would not find, the app keeps it rather than quietly changing
it.** That is deliberate. A CLI can be missing for a moment — a startup file that failed or timed
out, environment-include switched off, an upgrade in progress — and silently rewriting your
preference would lose a choice you made without saying so. The setting stays as you left it, and
when you start a session the app tells you what is missing and offers the CLIs that are available
instead of substituting one.

### Show activity for Pi sessions

On by default. Pi has no built-in way to tell another program whether it is working or waiting
for you, so when you start a Pi session the app loads **a small component of its own into that
Pi process** to report it. That is what drives the session's working/idle badge.

What the component does, and all of what it does:

- It writes one line to a file the app watches each time Pi starts or finishes a turn, starts or
  finishes a tool, settles, or quits. Each line is the event's name and a timestamp — nothing
  else.
- It **never reads or transmits your conversation**: no prompts, replies, file contents or tool
  arguments, and it makes no network connection.
- It is loaded only into Pi sessions this app starts. It is not installed into Pi's own extension
  folders, so running `pi` yourself outside the app does not load it.

Pi gives every extension it loads **full permissions on your system** — the same as Pi itself.
The component uses none of that beyond appending to its one file, but if you would rather Pi load
nothing from this app, turn **Show activity for Pi sessions** off. Sessions started after that run
plain `pi`, and their badge reads **unknown**. That is the expected result of turning it off, not a
fault: the session works exactly as before, the app just has no way to see whether Pi is busy.

It is one setting for the whole app — there is no per-project or per-session copy — and it applies
to Pi sessions started after you save. Claude Code and Copilot sessions are unaffected either way.

### Let AI sessions manage worktrees and sessions

On by default. Claude Code and GitHub Copilot sessions are connected to the app's own tool server,
so the assistant in a session can see the project's worktrees, branches and sessions the way the
sidebar shows them. [Tools for the AI in your sessions](agent-tools.md) lists what they can do.

Turn it off and sessions started after you save get no connection: they start exactly as they
did before this feature, and the session service writes one line to its log for each, saying
`no tool server: disabled in settings`. Sessions already running keep the connection they started
with until they stop; restart one to drop it. Turning it back on connects the next session again.

It is one setting for the whole app, kept by the session service beside the default AI CLI, so
every open window shows the same value.

### Desktop notifications

On by default. When a session you are not looking at needs you, the app shows one desktop
notification for it
([Being told when a session needs you](worktrees-and-sessions.md#being-told-when-a-session-needs-you)).
Turn **Desktop notifications** off and save, and the app shows none.

Under the master switch are four switches, one per kind of notification, in this order. Each one
applies to every AI CLI.

- **Needs permission** (on by default): the session stopped mid-turn to ask for a permission or an
  answer. The notification's title is the session's name followed by *needs permission*.
- **Session error** (on by default): the session ended because of an error: it kept crashing until
  the app gave up restarting it, or its AI CLI reported an error and stopped. The title ends in
  *stopped with an error*. One window shows it, the one you used last. Closing or stopping a
  session, a CLI that exits normally, and a crash the app restarts raise none. An error does not
  mark the session unread.
- **Long task finished** (on by default): the session finished a turn at least as long as the
  long-task threshold (60 seconds by default), counted from your prompt, any wait for you inside it
  included. The title ends in *finished a long task*.
- **Turn finished** (off by default): the session finished a shorter turn. With it off such a turn
  raises no notification, but still marks the session unread.

**Long-task threshold.** Directly under **Long task finished**, in seconds, from 10 to 3600, 60 by
default. It decides which of **Long task finished** and **Turn finished** a turn that ended counts
as, even while **Long task finished** is off: a turn at least that long is never reported as
**Turn finished**. A change applies to the next turn that ends, one already running included.
Input outside the range, or that is not a whole number, is refused when you save, with a message
under the field. A value outside the range in the settings file is clamped to the range.

The master switch **Desktop notifications** turns all four off at once. While it is off the four
switches and the threshold are greyed and cannot be changed, but they keep their positions and
their values; turn it on again and they are as you left them.

Claude Code's helper agents (subagents) finishing inside a turn are not the end of the turn: they
neither notify you nor mark the session unread.

Which AI CLIs report their sub-agents, and which do not, is listed in
[Sub-agent runs](agent-tools.md#sub-agent-runs).

- **One switch for every AI CLI.** It applies alike to Claude Code, GitHub Copilot, Pi, Codex and OpenCode sessions.
  There is no switch per CLI.
- **Every window at once.** The session service keeps the setting, so it holds for every open
  window from the moment you save, and every window's Settings shows the same value.
- **A save changes only what you changed.** Saving Settings changes only the settings you changed
  on the page. A setting changed in another window while this page was open keeps that window's
  value, for these switches and every other setting. When that window's save changes a setting
  the session service keeps (the Environment page, notifications, pull request status), an open
  Settings page shows the new values in every field you have not edited; a field you edited keeps
  your edit. Reopen Settings to see a change to the theme or the sandbox.
- **Nothing to restart.** A change to the master switch, a kind switch or the threshold applies to
  the next event, also for sessions that are already running.
- **Nothing after the fact.** When you turn a switch on again, the next event of that kind while you
  are not looking at the session notifies you. Events that happened while it was off are never
  notified later.
- **Unread marks do not depend on the switches.** With every switch off a session that needs you is still
  marked unread in the sidebar and counted in the project switcher
  ([Unread sessions](worktrees-and-sessions.md#unread-sessions)).
- **It survives a restart.** In the settings file the master switch is `desktop_notifications`,
  `true` or `false`. A file written before the switch existed, or one that cannot be read, counts as
  on. The kinds are `notification_kinds`, with the four fields `needs_permission`, `session_error`,
  `long_task_finished` and `turn_finished`, each `true` or `false`; a field the file lacks has its
  default above. The threshold is `long_task_threshold_secs`, a whole number of seconds.

**Each kind has its own icon.** The icon beside a kind's switch is the one its desktop
notifications carry: a raised hand for **Needs permission**, a circled exclamation mark for
**Session error**, a circled tick for **Long task finished** and a speech bubble for **Turn
finished**. In a notification the icon is drawn in white on a coloured rounded square, so it stands
out on a light and a dark desktop alike.

- **Linux**: the notification shows the icon in place of the app's, on desktops whose notification
  service shows images (GNOME, KDE Plasma and most others do).
- **Windows**: the toast shows the icon in place of the app's logo.
- **macOS**: the notification always shows the app's own icon; the kind's icon is attached to it
  and shows on its right side, or larger when you expand the notification.

Where a system shows no icon from the app, the notification is still shown, and its title names
the kind: *needs permission*, *stopped with an error*, *finished a long task* or *finished its
turn*. Windows and macOS need the icons as files: the app writes them to `notification-icons/`
beside the settings file ([Where settings are stored](#where-settings-are-stored)) once each time
it runs. If it cannot, it notes that once and shows the notifications without icons.

The operating system's own permission is separate. With the switch on, the system can still
withhold notifications: you refused the permission, turned notifications off for the app in the
system's settings, or have Do Not Disturb on. With the switch off, the app asks the system for
nothing, whatever the system would allow. Where each system keeps its setting is on
[Installing on macOS](install-macos.md#notifications) and
[Installing on Windows](install-windows.md#notifications).

### Claude plan usage

**Show Claude plan usage** shows how much of your Claude plan's usage limits you have used, and
when the next limit resets, in the app bar (see
[Claude plan usage](worktrees-and-sessions.md#claude-plan-usage)). It is **on** by default,
because it reads no credential and sends nothing off your machine. The values come from Claude
Code's own status-line data in the Claude sessions the app runs. From Claude Code's settings files
the app reads only the `statusLine` key, so that your own status line keeps working. No credential
is read, and nothing is sent anywhere. Turning it off applies to Claude sessions started
afterwards, and the indicator disappears at once.

**Warn at** sets when the indicator changes to its warning look: a warning sign instead of the
usage icon, with the icon and text in the error colour. It is a whole percent from 50 to 100, 80 by
default, and Save refuses anything else with "Enter a whole number from 50 to 100". The warning
appears as soon as any limit reaches it, and the indicator's tooltip names the limits that did. It
appears only in the indicator: there is no dialog and no desktop notification.

### Let agents read and type into other sessions

Whether the assistant in one session may read another session's terminal and type into it, with the
`read_session_output` and `send_session_input` tools
([Tools for the AI in your sessions](agent-tools.md#reading-and-typing-into-other-sessions)). It
only ever reaches sessions of the same project.

| Value | What it means |
|---|---|
| **Auto** (the default) | An assistant reads and types into other sessions without asking you |
| **Confirm each send** | Reading needs no approval. Each message an assistant wants to type waits for you to allow it in an app window; declined, unanswered for 60 seconds, or with no window open, it is not typed |
| **Off** | Both are refused |

A change applies to the next request, also from sessions that are already running: you do not need
to restart anything. It is a separate setting from **Let AI sessions manage worktrees and
sessions** above. With that one off, new sessions have no tools at all, so this one has nothing to
govern for them.

It is one setting for the whole app, kept by the session service, so every open window shows the
same value. In the settings file it is `cross_session_access`, with the value `auto`,
`confirm_each_send` or `off`. If the file holds anything else there, for example after a mistyped
edit, the option reads as **Off** and the rest of your settings are kept.

### The environment a session starts in

By default, every session's AI CLI process and regular-terminal process automatically pick up
your normal shell environment — PATH additions from version managers (nvm, pyenv, rbenv),
exported API keys, proxy settings, and anything else your shell's startup file sets. This is done
by actually running that startup file in a real, disposable shell process and capturing what it
changes — not by parsing its text — so conditionals, sourced sub-files, and version-manager init
blocks all resolve correctly.

- **Default**: on, sourcing `~/.bashrc` on Linux/macOS (via bash) or your PowerShell profile on
  Windows.
- Both the AI CLI process and the regular-terminal process for a session see the identical set of
  resolved variables.
- Resolution runs **per project directory**, not once for the whole app: if your startup file uses
  a version manager (mise, asdf, nvm, pyenv, rbenv, …) whose `PATH` additions depend on which
  project you're in, each project's own directory-specific additions are picked up correctly —
  the same way they would be in a regular terminal opened in that project.

This section holds three fields:

- **Source a script before each session** — turn environment-include off entirely (no script is
  sourced) or back on. Turning it off keeps the path, so re-enabling it doesn't mean typing it
  again.
- **Script path**: the file to source. Any path is accepted and never rejected at save time. Each
  time you open Settings, the path is checked (without running the script), and while environment
  include is off, a path that names no readable file is reported below the fields. A save that
  leaves such a path is saved as usual and then posts a notification naming it, whether the feature
  is on or off — see [If the script path names no file](#if-the-script-path-names-no-file).
- **Timeout (seconds)**: how long sourcing may run before being treated as hung. **Default**: 10
  seconds. **Range**: 1 – 60 seconds; out-of-range or non-numeric input is rejected with a message
  and not saved (same as the scrollback field).

A saved change takes effect on the next session or terminal launch — no app restart needed — even
while a session in the same project is still starting under the old settings.

Saving a change to any of these three fields also refreshes which AI CLIs every sidebar row offers,
with no restart. The script is what puts a project-local CLI on a directory's `PATH`, so each row
is asked again; until its new answer arrives, a row keeps offering what it offered before. A change
saved in another window refreshes the rows in this one too. Saving without changing these fields
asks nothing.

**Persistence**: only the enabled flag, script path, and timeout are ever saved to disk. The
variables the script resolves — and any diagnostic text captured while troubleshooting a failure
— are held in memory for the running app only and are never written to your settings file, since
they may include secrets (e.g. exported API keys).

### If the script path names no file

Each time you open Settings, the stored script path is checked in the background: the page opens
at once, and the result appears below the fields a moment later. The check only looks at the file;
it never runs or sources it, and it is not saved anywhere. Opening a session or restarting a
terminal does no check at all, so a bad path never slows a launch.

Whether **Source a script before each session** is on or off, the page tells you when the path
does not name a readable file, in the same words either way:

- **Script not found: `<path>`** — nothing exists at that path.
- **Not a readable file: `<path>`** — something is there, but it is a directory or a file you are
  not allowed to read.

Either line is followed by a note that says what that means in the current state. While environment
include is off: no script is sourced now, and turning it on will not source one until the path names
a readable file. While it is on: the script cannot be sourced until the path names a readable file.
Switching the feature on or off changes only this note, never the line about the path.

A few paths are reported differently, in either state:

- A path starting with `~` is taken literally: `~` is **not** expanded to your home directory, so
  `~/.bashrc` is reported as not found, with a note saying to use a full path.
- A relative path (such as `env.sh` or `scripts/env.sh`) is not checked, because whether it is
  found depends on each session's directory. The page says so instead.
- If the check has no answer within 2 seconds — for example, the path is on a network drive that
  is not responding — the page says **Couldn't check the script path** rather than waiting.
- A blank path, or a path that names a readable file, shows nothing about the path.

If you create the missing file while environment include is on, the page says **The last attempt
could not find the script**, followed by a note that the file exists now and that saving Settings or
restarting a session will source it. Opening Settings never sources the script by itself.

If you save Settings in another window while this one shows Settings, this page checks the newly
stored path too, so both windows say the same thing about it.

#### When you save

Saving always goes through: a missing script never stops the other settings from being saved.
Because Save closes Settings, every save that leaves a path naming no readable file then posts a
notification naming it — with **Source a script before each session** on or off, and even if the
save did not change the path:

- **The environment-include script was not found: `<path>`** — nothing exists at that path. For a
  path starting with `~`, it adds **(~ is not expanded; use a full path)**.
- **The environment-include script is not a readable file: `<path>`** — something is there, but it
  is a directory or a file you are not allowed to read.

A blank path, a relative path, a readable file, or a check with no answer within 2 seconds posts no
notification. Nothing else reports a missing script: opening a session does not, and neither does
the main window.

To fix it, edit **Script path** to the full path of a readable file, or clear it, and save.

### If the script fails

A missing, broken, or hanging script never blocks or fails opening a session — the session opens
normally with whatever environment is otherwise available. The most recent attempt's outcome is
shown at the bottom of this section whenever it didn't succeed — since resolution runs per project
directory, this reflects whichever directory was most recently (re-)resolved (typically your active
project, or the one you just restarted a session in), not necessarily every project you have open.
When the script fails only in some project folders, the page also lists each such folder after
that line — for example **Exited with an error in `<folder>`** — with what the script printed there:

- **Script not found: `<path>`** — the configured path doesn't exist. It is the same line the
  path check shows, and it appears once, not twice. It reads the same with the feature on or off
  (see [If the script path names no file](#if-the-script-path-names-no-file)), so switching the
  feature off does not hide it.
- **Exited with an error** — the script ran but failed; the script's own output is shown verbatim
  underneath, to help you see what went wrong.
- **Timed out** — sourcing didn't finish within the configured timeout and was abandoned.

To recover once you've fixed the script: use the existing **restart** control on the affected
session's terminal (shown whenever that session's process isn't running). Both restart controls
work this way — the AI CLI's and a Regular Terminal's: the session service re-sources the script
fresh for that session's directory before the process starts again, without needing to restart the
whole app. The AI CLI's restart also clears the failure note. Saving Settings (even without
changing any value) also triggers a fresh re-source. Selecting a session, reconnecting, or a
session being started again automatically after a crash does not: those reuse the last result.

## Daemons

A daemon is a session service: the process your sessions and terminals actually run in, either on
this computer (the *host* daemon, at most one) or in a container of its own. This section lists
every daemon you have set up. Each row shows its name, what it runs in, where the app connects to
it, the version it reported, a state badge (**Connected**, **Starting**, **Unreachable**,
**Version mismatch** or **Stopped**) and the worktrees bound to it.

| Action | What it does |
| --- | --- |
| **Start** | Connects to that daemon and no other. A stopped daemon is neither connected to nor reconnected until you start it |
| **Stop** | Disconnects from that daemon and no other |
| **Reconnect** | Tries an unreachable or mismatched daemon again now, instead of waiting for the next automatic try |
| **Edit** | Opens the form below the list, filled in |
| **Details** | Shows the recent connection errors and the log tail, and why a container runtime cannot be used when that is the reason |
| **Remove** | Asks first, see below |

**Add daemon** opens a form for a name, what it runs in (container or host), the container name,
port and image, and whether to connect when the application starts. A form that cannot be saved
says why beside the field at fault: a blank or repeated name, a port outside 1-65535, a container
already used by another daemon, a second host daemon. The form groups its settings in titled
sections; a closed section shows a one-line summary of its settings, and a `*` after its title when
a required field inside it is empty. For a Kubernetes daemon, the contexts found in the kubeconfig
are chosen from the **Context from your kubeconfig** select above **Context**, which can still be
typed.

Saving the Settings page does not save this section and cancelling the page does not undo it: a
daemon you added or removed here is kept at once.

### Removing a daemon

Removal asks first and says how many worktrees are bound to the daemon and how many sessions are
running on it. If you confirm:

- the sessions running on it are stopped;
- the daemon is removed from the list and its connection is dropped;
- its worktrees are kept and show `no daemon` until you bind them yourself, with **Bind to
  daemon...** in the worktree's right-click menu — nothing is bound for you, and no session or
  terminal starts for a worktree with no daemon;
- nothing on disk is deleted, and the container is not stopped or deleted — remove it with your
  container runtime if you no longer want it.

Choosing **Cancel**, or pressing Esc, changes nothing. Editing the runtime settings of a daemon that
is running asks the same way, stops its sessions and then reconnects it; a change of name alone
asks nothing and changes nothing else.

### With no daemons

The section explains what a daemon is and offers **Add daemon**. The new-worktree form offers no
daemon either and says where to add one.

### Container daemons

Everything about the process that actually runs your sessions is in the daemon's form, because it is
one decision: sharing your AI CLI sign-in means nothing when the service is a plain process on this
computer, and means a great deal when it is a container.

A daemon runs either directly on this computer (the default) or inside a container that sees only
your registered projects. See [Running the session service in a container](sandboxed-daemon.md) for
what changes, what it can and cannot reach, and how to work offline. Worktrees show the daemon they
run on in the sidebar, and the New worktree form offers the daemon to use (see [Which session
service a worktree runs on](worktrees-and-sessions.md#which-session-service-a-worktree-runs-on)).

You can run a host daemon and several container daemons together, each worktree bound to one. The
limits (one host daemon at most, unique names, container names and ports) and what an unavailable
daemon looks like are in [Running a host daemon and container daemons
together](sandboxed-daemon.md#running-a-host-daemon-and-container-daemons-together). A daemon can
also run on another computer: see [Running the session service on another
computer over SSH](remote-daemons.md). Choose **Remote over SSH** in the form and pick a host from
your SSH configuration, or enter host, user, port and key file; a second daemon for the same host,
user and port is refused, naming the existing one. A daemon can also run in a cluster: see
[Running the session service in a Kubernetes cluster](kubernetes-daemons.md).

Each container daemon keeps its own settings, and they are kept if you switch it to the host and
back. A change to a running daemon asks before it restarts it.

#### Container

- **Container runtime** — Docker (the default) or Podman.
- **Image source** — pull from a registry, load from a local archive (the fully offline path), or
  build from this checkout.
- **Image reference** — a digest or an exact tag. A moving tag like `:latest` can't be named in a
  bug report, so the app will tell you when you're on one. Sessions run in this image, so it has to
  provide every AI CLI you want to use. The image this app publishes, and one built from this
  checkout, ships Claude Code, GitHub Copilot, Pi Coding Agent, Codex and OpenCode. If the running image
  lacks one, a
  note under the field names it. That note describes the image the service is running now, not an
  unsaved or not-yet-started reference: a new image is used from the next time the service starts.
- **Image file** — the archive to load, when the image comes from a file.

#### Resources

Caps on what the sandbox may consume (CPU, memory, process count and writable storage); see
[Resources](sandboxed-daemon.md#limits). Blank uses the runtime's own default.

#### Credentials

The container starts with **none** of your credentials, and upgrading the app never opts you in.
Each share is separate, and only what you tick is passed in:

- **Git configuration** — `~/.gitconfig`, read-only: your commit identity.
- **AI CLI sign-in** — the AI CLI's own authentication material.

The **SSH agent** and **Git credentials** shares were removed: they handed the sandbox your whole
git identity. A daemon that had either on keeps working without it, and its entry in the daemon list
shows a one-time notice with a **Dismiss** button. Until you paste a GitHub token for its
repository (see [GitHub access tokens](#github-access-tokens)), a sandbox cannot push.

While either of these is on, the form lists exactly which ones by name, and the rail marks
**Daemons** with a *Sharing* badge — so you can tell at a glance, from any section, that
something is being shared.

#### Sessions

- **Keep the service running when I'm signed out or away** — off by default. It answers one
  question, and changes two things.

  With it **off** (the default), the service stops when you sign out, and it also stops itself after
  30 continuous minutes with nothing connected. Reopening the application starts a fresh one; a
  session that was running comes back *resumable* rather than lost. See [when the service stops
  itself](../daemon.md#it-stops-itself-when-nobody-has-used-it-for-30-minutes).

  With it **on**, and only in a container: the sandbox is created with a restart policy the runtime
  honours on Linux, macOS and Windows alike, **and** the idle stop does not apply to it — a service
  you asked to keep running is not one to stop for being unused. Because both are fixed when the
  container is created, changing this marks the running sandbox as out of date and takes effect the
  next time it starts. See [keeping the sandbox
  running](sandboxed-daemon.md#keeping-the-sandbox-running).

  A service running **directly on this computer** cannot honour it on any platform — the app is the
  only thing that starts one, and nothing it starts outlives the session it was started from. The
  control says so rather than accepting a choice it cannot keep. Your sessions are still kept and
  come back resumable; only the running processes inside them stop.

## GitHub

### Show pull request status on worktrees

Off until you turn it on. Checked, each worktree row whose branch has a pull request shows a small
indicator of that pull request (see [Pull request
status](worktrees-and-sessions.md#pull-request-status)); unchecked, no row shows one and nothing is
sent to GitHub for this. It applies to every project and every open window, and is kept in
`settings.json` with your other settings.

- **What is read:** the pull requests of the open project's own GitHub repository, with your GitHub
  CLI sign-in (`gh`). Only the repository's name and your worktrees' branch names are sent.
- **When:** when a project opens, when you turn the switch on, and every 5 minutes after that. An
  open Changes view's **Pull requests** list is refreshed on the same schedule; when GitHub cannot
  be reached the last list stays, captioned **Possibly stale** once it is older than ten minutes,
  and no error appears.
- **Needs:** `gh` installed and signed in (`gh auth login`). Without them the switch can still be
  checked, but nothing is shown and no error appears. Turning it on is your consent; having `gh`
  does not turn it on.
- **The Changes view:** the same switch controls the **Pull requests** section of a worktree's
  [Changes view](reviewing-changes.md#pull-requests), which lists every pull request of the
  worktree's branch. With the switch off that section is absent too.
- Turning it off removes every indicator, and that section, at once.

Save applies it, like every other setting here; another window's change to it shows here without
waiting for you.

### GitHub issues

When you create a worktree from a GitHub issue, the issue's labels choose its **Type** (see [The
issue's labels choose the type](worktrees-and-sessions.md#the-issues-labels-choose-the-type)). This
section is that label-to-type mapping: one list, applied to every project.
Also applies to Jira issues: an issue's type name and labels are matched against this list.

Each row is one entry: a **Label**, the **Type** it selects, and buttons to move the entry up or
down, or to delete it. Order matters: when an issue carries several mapped labels, the entry nearest
the top wins, so move an entry up to make it win.

- **Add entry** adds a row at the bottom with an empty label and the type `feat`.
- **Restore defaults** puts back the three default entries — `bug` → fix, `enhancement` → feat,
  `documentation` → docs — replacing whatever the list holds.
- With no entries, the section says so, and picking an issue leaves the type for you to choose.

Several labels may select the same type. A label may appear only once, though, and labels are
compared ignoring letter case and surrounding spaces, so `Bug` repeats `bug`. **Save** refuses a
blank label or a repeated one, jumps to this section, and marks the row; nothing is saved until you
fix or delete it.

A saved change applies to the next issue you pick, in every open project, without a restart. It is
kept in `settings.json` with your other settings.

### GitHub access tokens

A sandboxed daemon never holds your own GitHub token or SSH key. To let a sandbox push and pull its
project's repository, you give the application a **fine-grained personal access token** for that one
repository, once per repository.

1. **Create the token** on GitHub under *Settings → Developer settings → Personal access tokens →
   Fine-grained tokens*. Under *Repository access* choose *Only select repositories* and pick just
   that repository. Under *Repository permissions* give **Contents: Read and write** and **Metadata:
   Read-only**, and nothing else. Set an expiry: choose the shortest you can live with (GitHub's
   minimum is one day). A token with no expiry is refused.
2. **Paste it** in **Settings → GitHub → GitHub access tokens**: enter the repository as `owner/name`
   or a github.com URL, paste the token and choose **Add**. The application checks with GitHub
   that it is a fine-grained token, that it has an expiry, that it can push to that repository and
   that it reaches no other private repository. A token that fails is not stored; the reason is
   shown at the form. The token is kept only on this computer, in a folder readable only by you
   beside the application's data folder, outside anything a container can see, and it is never
   shown again or logged.
3. **Replace** pastes a new token for the same repository; running sandboxes receive it without a
   restart and without ending a session.

Each row shows the token's expiry date. **From 24 hours before it expires** the row, and the status
of every project that uses it, warn you; the application never renews a token (GitHub has no way
to), so paste a new one before then. At the expiry the token is taken out of running sandboxes and
pushes fail until you paste a new one.

**Remove** deletes the stored token and takes it out of running sandboxes. **It does not delete the
token on GitHub**: it stays valid there until it expires or you delete it under *Developer
settings*.

What a token can do, a sandbox can do. The application cannot narrow it, so every permission you
give it beyond the two above is held by the sandbox as well, and a token that leaks from a sandbox
stays valid until it expires or you delete it on GitHub. Where GitHub does not let the application
list the repositories a token reaches, the settings say so and you must check that yourself.

## Jira

### Enable Jira integration

The switch is **off** by default. While it is off the app shows no Jira control anywhere else, asks
for no Jira credentials and sends nothing to Jira, whether or not a connection was ever saved.

Turning it on lets Micold read the open issues of a Jira Cloud project to start worktrees from. The
page then says whether the integration is enabled and, for the open project, which Jira site and
project it is connected to, or that it has no Jira connection yet. With no project open it says to
open one. Once a project is connected, Micold contacts that connection's Jira site and sends the
project key, the search text you type and your account email with its API token.

The switch is one setting for the whole app, kept in `settings.json`, and a change made in one open
window shows in every other without a restart. A settings file written before the integration
existed reads as off.

### Connect a project

With the switch on and a project open, the page shows four fields for that project:

| Field | What to enter |
| --- | --- |
| **Site URL** | The address of your Jira Cloud site, for example `https://acme.atlassian.net` |
| **Project key** | The key of the Jira project, for example `ABC` (letters, digits and `_`, 2 to 10 long) |
| **Account email** | The email of your Atlassian account |
| **API token** | An API token created at <https://id.atlassian.com/manage-profile/security/api-tokens> |

Press **Save**. A field that is empty or cannot be used is marked with what to enter, and nothing
is stored until every field is valid. Each project has its own connection; a project without one
offers no Jira.

The token is shown masked while you type, and never again: once stored, the field says **A token
is stored**. Leave it blank to keep the stored token, or type a new one to replace it.

### Change or remove a connection

Edit any field and press **Save**. **Remove connection** deletes the site, project key, email and
token when you press **Save**. Turning the switch off while the open project has a connection and
pressing **Save** asks **Keep the Jira connection for <project folder>?** below the switch, which is
already off by then. **Keep** leaves the connection stored, so turning the switch on again finds it,
and **Remove** deletes it as above.

### Where the token is stored

The token is never written to `settings.json` or to a project file. It is kept in its own file,
readable only by you, one per project, in the `jira` folder beside the settings file:

| Platform | Folder |
| --- | --- |
| Linux | `~/.local/share/micold-ai-ide/jira/` (or under `$XDG_DATA_HOME`) |
| macOS | `~/Library/Application Support/micold-ai-ide/jira/` |
| Windows | `%APPDATA%\micold-ai-ide\data\jira\` |

Removing the connection deletes the file.

## Where settings are stored

Everything on this page is saved to a single file on your own computer — nothing is sent anywhere:

| Platform | File |
| --- | --- |
| Linux | `~/.local/share/micold-ai-ide/settings.json` (or under `$XDG_DATA_HOME`) |
| macOS | `~/Library/Application Support/micold-ai-ide/settings.json` |
| Windows | `%APPDATA%\micold-ai-ide\data\settings.json` |

The file survives restarts and package upgrades. You can read it, and you can copy it to another
machine, but you don't have to edit it by hand — Settings writes every value it holds.

### If your settings can't be read

Two things can go wrong with that file, and the app tells you about both rather than quietly
starting over. Neither one throws your settings away.

**"Your settings could not be read, so defaults are in use."** — shown when the app starts. The
file was there but the app couldn't make sense of it, so it opened with the defaults rather than
refusing to start. If the file was unreadable because its contents were damaged, the message also
names where the original was kept:

> Your settings could not be read, so defaults are in use. The unreadable file was kept as
> `…/settings.json.bak`.

That `.bak` file is the damaged original, untouched. Open it in any text editor to read the values
back out — your container image reference, your startup script path, whatever you'd rather not type
again — then set them in Settings and press **Save**. Once you save, the app writes a fresh
`settings.json` and the message stops appearing. If you don't need anything out of the `.bak`,
you can delete it.

**"Couldn't save your settings: the settings file could not be read, and saving now would replace
it with defaults."** — shown when you press **Save**. Here the file is still on disk and still
intact; the app just couldn't read it this time, usually because the file's permissions or its
folder changed underneath it. Saving would have written the defaults over a perfectly good file, so
the save is refused instead. Your change is not lost — it's still in the form, and the values on
disk are still the ones you set earlier.

To clear it: check that the file listed above exists and that your user account can read and write
it, then press **Save** again. Every settings write — successful or refused — is recorded in the
log, so a save you can't explain has a line to point at (see [Finding the logs and recent
errors](../daemon.md#finding-the-logs-and-recent-errors)).
