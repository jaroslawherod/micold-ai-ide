# Running the session daemon on another computer over SSH

A daemon can run on a computer you reach with SSH, a build machine or a server, and your worktrees
bound to it run their sessions there. Micold uses the `ssh` you already have, with your own keys,
agent and `~/.ssh/config`; it stores no password or key.

## Adding one

1. Open **Settings > Daemons** and choose **Add daemon**.
2. Name it, and set **Runs in** to **Remote over SSH**.
3. Enter the **Host**: a name, or a `Host` alias from your SSH configuration. **User**, **Port** and
   **Key file** are optional; what you leave blank comes from your SSH configuration.
4. Leave **Connect when the application starts** on, or start it yourself from its row.

## What happens when it starts

The row shows **Starting**. If the host has no daemon of this build yet, Micold copies its own
daemon there (`copying daemon to <host> (n%)`) into `~/.micold/remote/` of your account, then starts
it. When the handshake succeeds the row shows **Connected**, the daemon's version and
`user@host:port`. Nothing is listening on a network port on either machine: the connection is the
SSH session itself.

## Adding a repository that lives on the host

Once the row shows **Connected**, a **Repository on the host** field appears under it. Type the
absolute path of a git repository on that computer (for example `/home/dev/project`) and choose
**Add repository**. Micold asks the host whether the folder is a git repository and, when it is,
adds it to your projects. A relative path, or a daemon that is not connected, is refused with a
sentence under the field. Nothing is read from your own disk for a remote repository.

The project, its worktrees and their sessions run on that daemon alone. Wherever a remote worktree
appears its path is the path on the host, and its tooltip names the daemon (`Remote over SSH:
/home/dev/project`). Creating a worktree, starting a session and opening a terminal work as for a
local project, on the host.

## When it does not connect

The row shows **Unreachable** with the reason: the host could not be found, refused the connection,
did not answer in 15 seconds, did not accept your key, or has a host key that is unknown or has
changed (check it with `ssh <host>` once, then try again). Micold never accepts an unknown host key
on its own. Choose **Reconnect** once it is fixed.

Host key and permission problems need your action; the daemon is not retried until you start it.

## When the network drops

If a connection that was working is lost (a laptop sleeps, Wi-Fi changes), the row shows
**Reconnecting** with the reason, for example `reconnecting to dev@host: connection closed`. Micold
tries again after 1, 2, 4 and 8 seconds, then every 15 seconds, for as long as it takes; it does
not start a second daemon. The sessions keep running on the host, their worktrees show
`disconnected` meanwhile, and when the connection is back the same daemon serves them and their
output is complete.

Choose **Stop trying** to end the attempts: the row then shows `not connected to dev@host:
attempts stopped` and nothing is dialled. **Reconnect** dials again at once and starts the delays
over. A session whose daemon was stopped, or restarted without it, shows `lost to daemon <name>`.

## Your local work is not affected

A remote daemon that is starting, reconnecting, unreachable, on a different version or failed never
changes your local daemon, its worktrees, sessions or terminals. Each daemon has its own
connection, so a remote that is down only changes its own row.

When Micold starts, your local projects open at once. A remote daemon set to **Connect when the
application starts** is dialled in the background and its row shows **Starting** until it answers
or shows why it did not; the window never waits for it.

## Several Micold windows on one host

Several computers or windows can connect to the same remote daemon. Each sees the same sessions,
what you type in one goes only to the session you typed it in, and closing one leaves the others
connected.
