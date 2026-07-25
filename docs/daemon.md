# The Micold session daemon

Micold runs your AI-CLI and terminal sessions in a small background service — the **daemon** — that
is separate from the app window. The window (the *client*) is a thin viewer: it draws the terminal
and sends your keystrokes, but it does not own the running process. That separation is what makes
the guarantees below possible.

You never start the daemon yourself. The first time the app needs it, it launches one automatically
and it keeps running in the background afterward.

## What survives, and what doesn't (User Story 1)

A session is a running process (your `claude` session or a shell) plus the interpreted screen it has
produced. Both live in the daemon, so:

| You do this | What happens to your sessions |
|---|---|
| **Close the app window** | Every session keeps running. Output keeps being produced and recorded. |
| **The app window crashes** | Same — the sessions are in a different process, untouched. |
| **Rebuild / reinstall the app and relaunch** | The new window reconnects to the *same* daemon and finds every session where it left off. |
| **Reopen a session after any of the above** | You get the **current** screen immediately (a snapshot, not a replay), with scrollback covering the whole time you were away. |
| **Log out / end your login session** (Linux) | This is the one thing that can stop the daemon; see [Surviving logout](#surviving-logout) (User Story 7). |

Concretely, if you start a long-running build or a `claude` session that is working through a task,
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

A couple of current limitations worth knowing:

- Deleting a worktree keeps its git **branch** (only the worktree directory is removed), so a delete
  is always recoverable.
- The scrollback limit in Settings is applied by the daemon; other Settings are still saved locally.

## Sessions are supervised even with no window open (User Story 4)

Because a session's process lives in the daemon, the daemon can watch over it whether or not a window
is attached — and it does. The behaviour is **identical** attended and unattended: closing the window
never changes how a session's exit is handled.

- **A crash restarts automatically.** If a session's process exits unexpectedly (a nonzero exit or a
  signal — a crash, an out-of-memory kill), the daemon relaunches it. For a `claude` session that
  means resuming the same conversation, so a crash mid-task is recovered on its own.

- **A normal exit just stops it.** If the process ends cleanly — you quit `claude`, or a shell
  `exit` — the session is left **stopped**, not restarted. Reopening it starts it again on demand.

- **A crash *loop* gives up, loudly.** If a session keeps crashing, the daemon retries a bounded
  number of times (three consecutive restarts) and then settles it in a **Failed** state instead of
  restarting forever. Failed is durable: it shows up in the session list the next time a window
  attaches — with the attempt count — and you can restart it manually once you've addressed the
  cause. This is the same limit whether or not a window was open while it was crashing.

- **Teardown reaps the whole process tree.** Closing or deleting a session terminates not just its
  top-level process but any helpers it spawned, so nothing is orphaned in the background.

**A recovered session heals.** A restart that *survives* — the new process is still running on the
next supervision check, rather than crashing again immediately — returns the session to a normal
running state and **resets the crash-loop counter**. So repeated crashes only add up while they are
happening back-to-back; an occasional crash that recovers cleanly never creeps toward the give-up
limit over the session's lifetime. Only a genuine tight loop (crash, restart, crash again before it
could survive a check) exhausts the budget and settles `Failed`.

## Surviving logout

On Linux, whether the daemon outlives your closing the app depends on how it was started; making it
survive a full logout is covered under User Story 7 and documented there when it lands.

---

*This document grows with the feature: attach/detach and the activity badges (User Story 2) are
appended as those land.*
