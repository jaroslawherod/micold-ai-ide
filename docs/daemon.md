# The Micold session daemon

Micold runs your AI-CLI and terminal sessions in a small background service — the **daemon** — that
is separate from the app window. The window (the *client*) is a thin viewer: it draws the terminal
and sends your keystrokes, but it does not own the running process. That separation is what makes
the guarantees below possible.

You never start the daemon yourself. The first time the app needs it, it launches one automatically
and it keeps running in the background afterward.

## What survives, and what doesn't (User Story 1)

A session is a running process (your AI CLI — `claude` or `copilot` — or a shell) plus the
interpreted screen it has produced. Both live in the daemon, so:

| You do this | What happens to your sessions |
|---|---|
| **Close the app window** | Every session keeps running. Output keeps being produced and recorded. |
| **The app window crashes** | Same — the sessions are in a different process, untouched. |
| **Rebuild / reinstall the app and relaunch** | The new window reconnects to the *same* daemon and finds every session where it left off. |
| **Reopen a session after any of the above** | You get the **current** screen immediately (a snapshot, not a replay), with scrollback covering the whole time you were away. |
| **Log out / end your login session** (Linux) | This is the one thing that can stop the daemon; see [Surviving logout](#surviving-logout) (User Story 7). |

Concretely, if you start a long-running build or an AI CLI session that is working through a task,
close the window, and come back ten minutes later, the session is still `Running`, the screen shows
the latest output, and scrolling back shows what happened while you were gone — with no gaps and no
duplicated lines.

### Scrollback is bounded

The daemon keeps a fixed amount of scrollback per session (a configurable line limit). Once a
session produces more than that, the **oldest** lines are discarded first — this happens even while
no window is open, so a chatty session left running for days cannot grow without bound. When you
scroll back, the app fetches history from the daemon on demand; you are never holding the entire
history in the window. If you scroll past the oldest retained line, the view simply stops there
rather than inventing content.

### Why reattach is instant

When a window reattaches, the daemon sends one **snapshot** of the current screen rather than
replaying every update that happened while you were away. From then on it streams only what actually
changes — typically a line or two per update, even while output is scrolling quickly. That is what
keeps reopening a busy session fast regardless of how long it ran unattended.

## What is *not* guaranteed

- **A machine reboot or power loss** stops everything, including the daemon. Sessions do not survive
  a reboot (the processes are gone); when you next launch the app, it starts a fresh daemon.
- The daemon persists the *catalog* (your projects, worktrees, and session identities) to disk, so
  those reappear after a reboot — but a session's live process and its on-screen scrollback do not.

## Attaching, driving, and the activity badges (User Story 2)

Opening a project **attaches** the window to it: the window starts drawing that project's sessions
and sending your keystrokes to them. Closing the window, or switching away, **detaches** — the
sessions keep running in the daemon (that is User Story 1); detaching only stops the *drawing*.
Because the terminal lives in the daemon, scrolling, selecting text, and resizing the pane are all
handled inside the window against the grid it already has — they cost no round trip to the daemon and
stay responsive even while a session is producing output quickly.

### The activity dot

Every session in the sidebar carries a small **activity dot** next to its name, so you can tell at a
glance what each one is doing without opening it:

| Dot | Meaning |
|---|---|
| **Filled, accent** | **Working** — the agent is actively doing something. |
| **Filled, attention** | **Awaiting input** — the agent's turn ended and it is likely waiting for you. |
| _(no dot)_ | **Unknown** — the daemon has no signal yet (see below). This is deliberate, not a bug. |
| **Hollow** | **Ended** — the session's process has finished. |

The important, and unusual, one is **Unknown shows nothing**. The daemon derives activity from what
the AI CLI itself reports at each turn boundary — the authoritative "I started a turn / I finished a
turn" signal — not from guessing based on how quiet the terminal is (which was measured and does not
work: a session can sit silent for half a minute mid-task). So if that signal isn't reaching the
daemon — you ran a bare CLI outside the app, or the CLI is configured not to emit it — the daemon
reports **Unknown** rather than inventing an "idle" or "needs you" cue it can't stand behind. **A
blank dot means "I don't know", never "nothing is happening."**

"Awaiting input" is a *strong hint*, not a guarantee: a turn can end and then continue on its own
(auto-continuation, or a hook that resumes it), so treat the attention dot as "probably your turn,"
not a hard stop.

### How the daemon knows (and what it never sees)

Each CLI reports differently, and the daemon takes each one's own mechanism rather than a common
guess:

- **Claude Code posts to a loopback listener.** The daemon points each `claude` session at a small
  **loopback-only** listener — bound to `127.0.0.1` on a random port, reachable only from your own
  machine — and `claude` posts a one-line notice to it at each turn boundary. Each session gets its
  own unguessable token; a request without it is refused. It is wired up through a per-session
  settings file the daemon writes, so **your own `claude` configuration is never modified**.
- **GitHub Copilot writes an event log.** `copilot` appends a line to its own session event file as
  it works, and the daemon reads the bytes appended since it last looked, woken by the operating
  system's file-change notification. Nothing is polled on a timer, and no work at all is scheduled
  for an idle session.

Either way the listener or the reader does exactly one thing — report a session's activity — and can
touch nothing else: not your projects, not session input, not the catalog. The notices are never
written to a log (they can carry file paths and prompt metadata). A session the app merely
*discovered* on disk is never watched at all; its dot stays blank until you start it.

The session **title** shown in the sidebar comes from the same terminal stream: the AI CLI
continuously sets the terminal title to the session's generated name, and the daemon reads it
directly and pushes it to every window — replacing an older approach that repeatedly re-scanned a
transcript file. A leading status glyph (the little spinner) is stripped before display; the title
text itself is treated as untrusted and length-bounded. For a session found on disk rather than
started here, there is no live terminal to read, so the title comes from the CLI's own record of the
conversation — `claude`'s transcript or `copilot`'s session state — if it has written one yet.

## Project and worktree operations run through the daemon (User Story 3)

Adding or renaming a project, creating, renaming, or deleting a worktree, and creating or deleting a
session are all performed by the **daemon**, not the window. The daemon is the single writer of your
saved catalog (`projects.json`), which removes a whole class of problems that two processes writing
the same file at once could cause. In practice this changes three things you can see:

- **Failures are specific and actionable.** When a git operation can't complete — a branch name that
  already exists, a worktree directory that collides, a worktree you asked to delete that still has a
  running session — the app tells you exactly what went wrong, with git's own message included where
  relevant, and leaves everything untouched. A failed worktree delete, for example, never strands a
  running session or half-removes a directory; the worktree simply stays as it was.

- **Changes propagate to every window.** Because the daemon owns the catalog and pushes updates, a
  second window open on the same projects sees a new worktree, a rename, or a removed session appear
  on its own — you don't refresh anything.

- **An interrupted request never leaves you guessing.** Each operation is sent to the daemon and
  applied atomically before it replies. If the connection drops after you submit but before the reply
  arrives, the app tells you the outcome is **unknown** rather than pretending it succeeded or
  failed — and when it reconnects, it reads the daemon's authoritative state so the window settles on
  what actually happened.

Every session the daemon spawns — a fresh session, one resumed after a restart, a crash respawn, or
a regular-terminal shell instance — resolves the environment-include setting (the same "source my
`~/.bashrc`" mechanism from Settings) in that session's own project/worktree directory, exactly as a
session opened from a running window would. This holds even for sessions the daemon starts entirely
on its own, with no window attached — a crash respawn sees the same environment a fresh launch would.

A couple of current limitations worth knowing:

- Deleting a worktree keeps its git **branch** (only the worktree directory is removed), so a delete
  is always recoverable.
- The scrollback limit in Settings is applied by the daemon; other Settings are still saved locally.

## Sessions are supervised even with no window open (User Story 4)

Because a session's process lives in the daemon, the daemon can watch over it whether or not a window
is attached — and it does. The behaviour is **identical** attended and unattended: closing the window
never changes how a session's exit is handled.

- **A crash restarts automatically.** If a session's process exits unexpectedly (a nonzero exit or a
  signal — a crash, an out-of-memory kill), the daemon relaunches it. For an AI CLI session that
  means resuming the same conversation — the app asks that CLI to resume the session id it owns —
  so a crash mid-task is recovered on its own.

- **A normal exit just stops it.** If the process ends cleanly — you quit the AI CLI, or a shell
  `exit` — the session is left **stopped**, not restarted. Reopening it starts it again on demand.

- **A crash *loop* gives up, loudly.** If a session keeps crashing, the daemon retries a bounded
  number of times (three consecutive restarts) and then settles it in a **Failed** state instead of
  restarting forever. Failed carries a sentence saying how many attempts were spent and what the
  last exit was — `Gave up after 3 restart attempts — last exit: exit status 1.` The terminal pane
  shows that sentence in place of the output it has none of, beside the `restart` control that
  resolves it, and the status bar under it reads `failed after 3 attempts`. A window that attaches
  after the loop ended is shown both, so a loop that ran while nobody was watching is not reduced to
  the word `failed`. It survives until the daemon itself stops (the state is held in memory, not
  written to disk), and you can restart the session manually once you've addressed the cause. This
  is the same limit whether or not a window was open while it was crashing.

- **Teardown reaps the whole process tree.** Closing or deleting a session terminates not just its
  top-level process but any helpers it spawned, so nothing is orphaned in the background.

**A recovered session heals.** A restart that *survives* — the new process is still running on the
next supervision check, rather than crashing again immediately — returns the session to a normal
running state and **resets the crash-loop counter**. So repeated crashes only add up while they are
happening back-to-back; an occasional crash that recovers cleanly never creeps toward the give-up
limit over the session's lifetime. Only a genuine tight loop (crash, restart, crash again before it
could survive a check) exhausts the budget and settles `Failed`.

## One window per project, with deliberate takeover (User Story 5)

Because the sessions live in the daemon, more than one app window can talk to it at once. To keep two
windows from fighting over the same terminal, a project may be **attached** to only one window at a
time. This is per-project, not global: you can have several windows open, each on a different project,
all fully live and never interfering with one another.

- **A second window on the same project is refused — with an offer, not a wall.** If you open a
  window on a project another window already holds, attachment is refused with a message naming the
  current holder and how long it has held it, and offering to **take over**. Nothing happens to the
  other window until you confirm. The refused window says the project is already open elsewhere —
  not that anything was taken from it, which is the other case below and a different sentence.

- **Takeover is deliberate and non-destructive.** When you confirm, the new window becomes the holder
  and the previous one is **displaced**: it shows a banner saying another window took over, stops
  sending input, and goes read-only for that project — but it does **not** close, and its other
  projects are untouched. A "Take over" button on that banner claims the project back the same way,
  displacing whoever holds it now.

- **A holder that goes away frees the project automatically.** If the window holding a project simply
  closes — or crashes — the project becomes attachable again with no ceremony and no service restart.
  The next window to ask for it just gets it.

### Detecting a dead connection

An ordinary close sends a clean disconnect, so the daemon frees the project at once. A **half-open**
connection is trickier: if a window's machine loses power or its network drops, no disconnect ever
arrives and the socket would otherwise sit there forever, with the window still showing the last
screen as though it were live. To prevent that, the window sends a lightweight keepalive probe every
few seconds and expects a prompt reply; if the service goes silent past a short deadline, the window
declares itself **disconnected within ten seconds**, stops presenting the stale screen as live, and
shows a banner. It then reconnects on its own in the background, and on reconnect re-reads the
service's authoritative state rather than replaying whatever it missed — so the window always settles
on what is actually true, never on a guess.

### Trying it with a second window

To see the exclusivity behaviour, launch a second app instance pointed at the same project while the
first is open: the second is refused with the takeover offer. Confirm the takeover and watch the first
window drop to its read-only "taken over" banner while the second becomes live. Close the second, and
the first can take the project back from its banner. Two instances on two *different* projects, by
contrast, both stay fully live.

## A version mismatch fails loudly, and recovers (User Story 6)

The window and the service talk over a versioned contract. When you rebuild and relaunch the app but
an **older service is still running** from before the rebuild, the two no longer agree on the
contract — and rather than misbehave subtly, the window refuses to connect and tells you exactly what
happened.

- **The diagnostic names both sides.** The banner says the running service speaks one contract
  version while this app speaks another, and includes the service's build string — enough to see at a
  glance that a stale service is the problem.

- **One click fixes it.** The banner offers **Restart service**. Choosing it stops the old service
  and lets the app start a fresh one that matches, then reconnects — no command to type, no manual
  process hunting. Because a mismatched window can't even complete the handshake, the app stops the
  old service directly rather than asking it politely.

- **Your sessions survive the restart; live processes do not.** Restarting the service stops the
  processes it was hosting, and the banner says so plainly. But the sessions themselves are durable:
  after the restart they come back in the **interrupted-resumable** state below, ready to continue.

### After installing an update

Installing an updated package replaces the service binary on disk, but not the copy already running
in memory — that only happens when something actually restarts it. Most releases don't change the
version-mismatch contract above (only wire-breaking changes do), so they show a different, milder
banner instead: **"A newer session service is installed."** It names both builds and offers the same
**Restart service** action, but doesn't warn about sessions being put at risk — the two builds still
speak the same contract, so nothing is actually incompatible, only stale. Until you restart it (or log
out, or reboot), the service keeps running the version it was already running, even though the app you
just relaunched is newer.

### Interrupted-resumable sessions after any service restart

Whenever the service starts and finds sessions that were running when it last stopped — whether from
the version-mismatch restart above, a crash, or a reboot — it does **not** relaunch them. Doing so
would make an agent take action you never asked for. Instead each such session is shown in a distinct
**interrupted-resumable** state:

- It is visibly different from a *running* session and from one you *deliberately stopped* — you can
  tell at a glance which sessions were mid-flight when the service went down.
- Nothing restarts on its own. A single explicit action (opening the session) resumes it, continuing
  the prior conversation exactly where it left off.

This is the safety guarantee behind the whole restart story: **a service restart can never cause an
agent to do anything without you asking.**

## Finding the logs and recent errors

When something misbehaves, the overflow menu's **"Session service diagnostics"** asks the service two
things and shows the answers:

- **Where it logs.** Depending on how it was started, the service logs to the systemd journal, to your
  terminal, or to a size-capped rotating file under your user data directory — the diagnostic tells
  you which, and the file path when it's a file.
- **Its recent errors.** A short list of the most recent warnings and errors the service recorded, so
  you can see what went wrong without hunting through a log file.

Logs never contain terminal output or anything you typed — sessions are referenced by identity and
state only, so credentials and code in a session are never written to a log. Total log size is
hard-capped, so the log can't grow without bound even if the service runs for weeks.

## Surviving logout (User Story 7)

Closing the window always leaves your sessions running (that is the whole point of the daemon). But a
full **logout** is different: by default the system tears down everything you were running when your
login session ends, the daemon included. Making sessions survive a logout is:

- **Supported on Linux**, via one explicit, user-enabled setting (below). It is **never turned on for
  you** — not by installation, not silently.
- **Not supported on macOS or Windows** *for a service running directly on your computer*. There is
  no unprivileged equivalent, so the app does not pretend to offer one. On those platforms sessions
  survive closing the window but not logging out.
- **Supported everywhere when the service runs in a container** — see
  [Where the service runs](#where-the-service-runs-feature-027) below. Not a second mechanism
  bolted on: it is the container runtime's own restart policy, and the runtime is a service the
  platform already keeps running across logout and reboot.

### Enabling it (Linux)

The app does it for you: open the overflow menu and choose **"Keep sessions after logout."** That
runs, in your own session, the two steps that matter:

1. `loginctl enable-linger` — lets your user manager (and anything it runs) keep going after you log
   out.
2. `systemctl --user enable --now micold-daemon.socket` — moves the session service under that
   lingering user manager, so it is no longer tied to your login session.

If you prefer to do it by hand, run those two commands yourself, in that order.

> **Order matters — it is not retroactive.** Enabling linger does **not** rescue a service that is
> *already* running inside your login session; that process stays put and still dies at logout. You
> must enable linger **first**, then (re)start the service under the user manager. The menu action
> does exactly this — it enables linger, stops the session-bound service, and restarts it under the
> lingering manager — which is why using it is simpler than hand-rolling the commands.

If enabling linger is refused (some hardened systems restrict it via policy), the app tells you rather
than silently pretending it worked; ask your administrator to enable lingering for your account.

### How it is packaged

The systemd **user** units ship with the app (in `/usr/lib/systemd/user/`) but are **inert until you
enable them** — installation touches no per-user manager. The service is the same single binary
whether the user manager socket-activates it or a window spawns it directly, so nothing behaves
differently based on how it started.

## Where the service runs (feature 027)

The daemon has a **placement**: where it runs. Until this feature there was only one, and it was
assumed rather than described.

This section is the *model*. For turning the container placement on, what it can and cannot see,
which runtimes work, and what to do when it will not start, see
[Running the session service in a container](user-guide/sandboxed-daemon.md); the switch itself
lives in Settings → Session service, described in
[Settings](user-guide/settings.md#session-service).

| Placement | What it is | Reached over |
|---|---|---|
| **On this computer** (default) | A detached host process, spawned by the app on a cold start | A Unix socket or named pipe in a `0700` directory |
| **In a container** | A container on this machine, seeing only your registered projects | Loopback TCP, authenticated by a shared secret |
| *Remote* | Reserved. Not selectable in this release | — |

The third row is why the model exists as a model. Adding the variant now costs one `match` arm per
site and forces every placement-dependent decision to be *stated*; adding it later would mean finding
every place the host process was assumed by omission.

### Why the container is not reached over a socket

A bind-mounted Unix socket does not survive Docker Desktop's file sharing on macOS or Windows — that
layer passes file *contents*, not socket semantics. Socket-only would therefore mean Linux-only. So
the sandbox listens on loopback TCP, which every platform forwards the same way.

That transport carries none of the protection a `0700` directory gives: any local process can connect
to a loopback port. What replaces it is a shared secret, generated per sandbox start, written `0600`
and bind-mounted read-only into the container. The guarantee moves from "you cannot reach it" to "you
cannot answer for it", and the filesystem permission is still what enforces it. This is why the wire
protocol grew an authenticated handshake — version 6 when the sandbox landed, and version 7 today,
after the repository-root query the container placement also needed (below).

### The lifecycle

Enabling the sandbox does not start it on the spot; the next launch does. From then on each start
runs: **probe** the runtime → **acquire** the image → **adopt or create** the container → **start**
it → connect.

The adopt step matters more than it looks. A sandbox outlives the app by design, so on almost every
start there is already a container with our name. It is reused if it is ours, started if it is ours
and stopped, and **replaced** if it was built from a different image or a different source tree —
replaced rather than accumulated beside, because a second container would leave the first holding the
control port and the state directory.

### What it does not do

It never falls back. A sandbox that will not start is an error with a cause and a remedy, and running
without it is a choice the user makes for that occurrence — never a substitution the app performs
because the alternative was easier. If the app ever silently connected to a host process after a
sandbox failed, the feature would be gone and nothing would report it.

### Who answers "is this a git repository?"

With the service in a container, the app can no longer answer that question for itself: the folder
you picked is a host path, and the app's own `git` sees a machine the sessions do not run on. So the
question goes to whoever can see the folder the way the sessions will — the service — over the wire
(`RepoRootQuery`/`RepoRoot`, the addition that moved the protocol to version 7). On the default
placement nothing changes: the app still answers locally, because there it *is* the machine that
runs the sessions.

### State

The service's data directory — `projects.json`, per-project state, logs — is mounted from your own
data directory rather than kept inside a runtime-managed volume. Two reasons: the app has to read the
registered project list *before* the sandbox exists in order to know what to mount, and your data
stays somewhere you can see and back up.

---

*This document covers the daemon feature end to end (User Stories 1–7), plus feature 027's placement
model.*
