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

## Surviving logout

On Linux, whether the daemon outlives your closing the app depends on how it was started; making it
survive a full logout is covered under User Story 7 and documented there when it lands.

---

*This document grows with the feature: attach/detach and the activity badges (User Story 2) and
project/worktree operations through the daemon (User Story 3) are appended as those land.*
