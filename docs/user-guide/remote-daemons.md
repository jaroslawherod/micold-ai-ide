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

## When it does not connect

The row shows **Unreachable** with the reason: the host could not be found, refused the connection,
did not answer in 15 seconds, did not accept your key, or has a host key that is unknown or has
changed (check it with `ssh <host>` once, then try again). Micold never accepts an unknown host key
on its own. Choose **Reconnect** once it is fixed.

Host key and permission problems need your action; the daemon is not retried until you start it.
