# Orchestrator mode: one registry of your daemons

`micold-daemon orchestrator` runs the daemon binary as an *orchestrator*: a small process that keeps a
list of your daemons. It does not run sessions itself.

## Start it

```sh
micold-daemon orchestrator
```

It stays in the foreground and listens on a local socket (`orchestrator.sock`) beside the daemon's own,
so it can run next to a normal daemon. It starts no terminal, no AI session and no child process.
Running `micold-daemon` without the argument starts the ordinary daemon, exactly as before.

## The desktop app connects through it

The desktop app opens one connection, to the orchestrator, and reaches your host daemon through
it; it opens no connection to the host daemon's own socket. You never start the orchestrator by hand:
when none is running, the app starts `micold-daemon orchestrator` itself, as it starts the host
daemon. If one is already running (another app window, or a service you set up), the app uses it.
Two apps starting at once still end with one orchestrator.

Every daemon the app sets up gets `[orchestrator_link] enabled = true` in its `daemon.toml`, so it
dials the orchestrator. The host daemon links as `local`; a daemon the app provisions gets its own
id, such as `build-box-1a2b3c4d`, written as `[daemon] id` and kept in the app's settings.

Remote, container and Kubernetes daemons go through the orchestrator too. The app sets one up over
SSH as before, then closes that SSH session and keeps a small one open whose only job is the
link's forward to the orchestrator; from then on the app reaches the daemon by its id through the
orchestrator. If the forward is refused (your SSH configuration forbids it, or the remote cannot
bind the socket), or the daemon does not link within 10 seconds, the daemon's status cell in
Settings → Daemons says why; the app does not fall back to the SSH session. A daemon saved before
it had an id gets one the next time Settings is saved.

If the orchestrator stops, your sessions keep running on their daemons. The app shows the daemon
as reconnecting, starts the orchestrator again, and re-reads what the daemons say, so sessions that
ended meanwhile show as ended and new ones appear.

### When the orchestrator cannot start

The connection banner says why, for example
`cannot start /usr/bin/micold-daemon: No such file or directory` when the daemon binary is missing,
or a timeout naming `orchestrator.sock` when it cannot listen there. The app does not fall back to
the daemon's socket. To fix it:

- Check that `micold-daemon` is installed beside the app (or set `MICOLD_DAEMON_BIN` to it).
- Run `micold-daemon orchestrator` in a terminal: it prints why it cannot start, such as a damaged
  `orchestrator/registry.json` (see *Restarting it*) or a socket directory it may not use.
- A message `an orchestrator is already running` means another one holds the socket; the app
  connects to that one.

## What it lists

A daemon that links to the orchestrator is listed with:

| Field | Meaning |
|---|---|
| `id` | the daemon's stable id |
| `kind` | `host`, `ssh`, `container` or `kubernetes` |
| `name` | its display name (the id when it has none) |
| `protocol_version` | the protocol version it announced |
| `capabilities` | what it says it can do |
| `health` | `online` while the daemon answers; `offline` once it has been silent for 9 seconds |
| `last_seen` | unix seconds of its last answer |
| `tenant` | the tenant it announced, or `default` |

## Online and offline

The orchestrator checks each link every 3 seconds. A daemon that has not answered for 9 seconds shows
`offline`, and `last_seen` stays at its last answer. It stays listed, and shows `online` again as soon
as it answers. One silent daemon does not change the others.

## A daemon that needs an update

A daemon on a different protocol version cannot join. The orchestrator refuses it with a message that
names both versions, for example `daemon box speaks protocol version 81; this orchestrator speaks
82`, and lists it as `needs-update` with its own version and the orchestrator's. Update the daemon
and let it link again: it then shows `online`. A daemon that is already linked and answering is never
replaced by a refused one, and the other daemons keep their status.

## One link per daemon

Each daemon id is linked once. While a daemon is linked and answering, a second daemon that claims the
same id is refused (`a link for daemon <id> is already open`), and the first one keeps working. A
daemon that reconnects after its connection closed, or after it went `offline`, is accepted at once
and replaces its old link.

A daemon that has just connected holds its id for up to 9 seconds while it finishes the handshake;
another claim on that id in that time is refused. If it does not finish, the id is free again. Two
daemons that claim one id at the same moment never both get in: exactly one is accepted.

## Restarting it

The orchestrator remembers its daemons in a database: by default the SQLite file
`orchestrator/orchestrator.db` in your data directory, or a PostgreSQL server you name in the
configuration. See [Orchestrator storage](orchestrator-storage.md). After a restart every daemon it
knew is listed again, `offline` at first (a `needs-update` daemon too), and each shows `online` as soon
as it links again; its entry is updated, not added twice. Your daemons keep running their sessions
while the orchestrator is stopped.

If the database cannot be opened, the orchestrator does not start and says why. If saving fails (a
full disk or a lost database server, for example), the orchestrator says so on its error output and
carries on; the next change tries again.

## What it will not do

The orchestrator starts no session itself: a request to start one is passed to the daemon it names
and runs there. Sessions stay on your daemons, and keep running if the orchestrator stops.

It never dials a daemon: each daemon dials it, so a firewall or network setting that blocks outbound
connections from the orchestrator's host changes nothing.

Technical detail: [the daemon page](../daemon.md#orchestrator-mode-1724).
