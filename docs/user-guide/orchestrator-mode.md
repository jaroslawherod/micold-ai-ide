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

## What it will not do

Asking the orchestrator to start a session is refused with `not hosted by the orchestrator`, and its
session list is empty. Sessions stay on your daemons.

Technical detail: [the daemon page](../daemon.md#orchestrator-mode-1724).
