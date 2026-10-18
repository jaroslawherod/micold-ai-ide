# Running the session daemon on another computer over SSH

A daemon can run on a computer you reach with SSH, a build machine or a server, and your worktrees
bound to it run their sessions there. Micold uses the `ssh` you already have, with your own keys,
agent and `~/.ssh/config`; it stores no password or key.

## Adding one

1. Open **Settings > Daemons** and choose **Add daemon**.
2. Name it, and set **Runs in** to **Remote over SSH**.
3. Choose the host. If `~/.ssh/config` (and the files it includes) names hosts, **Host from your SSH
   configuration** lists them: pick one and only the name needs entering, because the user, port and
   key come from your SSH configuration. For a host that is not there, type the **Host** (a name or an
   address) and, if you need them, the **User**, **Port** and **Key file**; what you leave blank still
   comes from your SSH configuration. Micold only reads the file and never changes it.
4. Leave **Connect when the application starts** on, or start it yourself from its row.

A second daemon for the same host, user and port is refused with `<host> is already used by daemon
<name>`. Two aliases that your SSH configuration points at the same computer and account count as
one. A blank or invalid field is refused with the field named: a blank host or one starting with
`-`, a user containing `@`, a port outside 1 to 65535, or a key file that does not exist.

## What you need

- **The computer you reach.** Linux or macOS, on Intel/AMD (`x86_64`) or Arm (`aarch64`), with an
  SSH server and a writable home directory.
- **Linux, macOS and Windows clients.** Each uses the `ssh` on its `PATH`. On Windows that is the
  OpenSSH client (`ssh.exe`) that ships with Windows, or the one in Git for Windows; if you use a
  key with a passphrase, the **OpenSSH Authentication Agent** service must be running (set it to
  start automatically and run `ssh-add` once).

## Which daemon is copied

When the host has the same system and CPU architecture as your computer, Micold copies its own
daemon. Otherwise it copies a build that ships with the app. Every package (the macOS app, the
Windows setup, the `.deb` and the `.rpm`) carries static Linux builds for both `x86_64` and
`aarch64`, so a Mac or a Windows PC can start a daemon on any Linux host or container. A host
running anything else is refused with `no matching daemon build for <system>-<architecture>`, for
example `linux-riscv64`.

## What happens when it starts

The row shows **Starting** and, under it, the step it is in:

1. `connecting to the SSH server`
2. `checking the daemon build`
3. `copying the daemon (n%)`, only if the host has no daemon of this build yet: Micold copies its
   own daemon into `~/.micold/remote/` of your account
4. `starting the daemon`
5. `checking the daemon version`, only if the daemon refused the handshake over its version

Each step is on its own timeout, apart from the copy, which shows its progress instead. When the handshake succeeds the row shows **Connected**, the daemon's version and
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
did not answer in time, did not accept your key (or no key was available), or has a host key that is
unknown or has changed (check it with `ssh <host>` once, then try again). "In time" is the
`ConnectTimeout` your SSH configuration sets for the host, or 15 seconds when it sets none. A failure
is never a hang and is never retried silently. Micold never accepts or records a host key on its own.
Choose **Reconnect** once it is fixed.

Open the row's details to see its recent connection errors, newest last.

The reason names the step that failed, for example `copying the daemon failed: the daemon build
could not be written in the target`, followed by the last lines of the tool's output when it printed
any. A step that runs too long reads `<step> failed: did not finish within <time>`.

Host key and permission problems need your action; the daemon is not retried until you start it.

## When the versions differ

The daemon on the host keeps running after you update Micold, so the two can end up as different
versions. When they do, the row shows **Version mismatch** and says so in one sentence that names
both versions, which side is older, and what to do, for example `daemon dev version mismatch:
client 0.5.3, daemon 0.5.2 (daemon is older)`. This is checked on every connection, in either
direction.

- **The daemon is newer than the app:** `Update this app.` Micold never offers to replace a newer
  daemon, because that would downgrade the host.
- **The daemon is older than the app:** end the daemon on the host yourself (stop the `micold-daemon`
  process there), then choose **Reconnect**; Micold copies the matching build and starts it. Micold
  never ends the daemon's running sessions on its own to resolve a mismatch.

No session starts on a mismatched daemon: trying to start one is refused with the same sentence, and
sessions already running on the host are left alone. Your local daemon and your other remote daemons
are not affected.

## Managing a remote daemon

From its row you can **Start**, **Stop**, **Reconnect**, **Edit** and **Remove** a remote daemon, and
each applies to that daemon only. Editing the target of a connected daemon asks first, saying how
many sessions it will end; a change of name alone changes nothing else. Removal asks as it does for
any daemon and does not touch the host: the daemon there and its sessions keep running, and the
worktrees bound to it show that no daemon runs them until you add it again.

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
