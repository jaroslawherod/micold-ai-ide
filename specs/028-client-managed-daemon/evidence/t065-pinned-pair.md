# The pinned client/daemon pair connects (T065)

**Date**: 2026-09-12 · **Base**: `47b8481b` · **Profile**: `dev`

This feature moves who starts the daemon, so the check that the client can still *start and talk to*
its own daemon is not ceremony. It is also the check most easily faked: `target-shared/` is shared
with every other worktree, and a `cp` out of it can pick one binary from this build and one from a
branch on a different protocol commit. The symptom is `refusing client: contract or build mismatch`
with **identical version numbers printed on both sides** — the value that differs, `SCHEMA_HASH`, is
not printed.

## How the pair was pinned

`touch` on both `main.rs` files, then one locked invocation that builds and copies without releasing
the lock in between:

```
./scripts/build-lock.sh bash -c "cargo build -p micold-client --bin micold-ai-ide \
  && cargo build -p micold-daemon --bins && cp target-shared/debug/{micold-ai-ide,micold-daemon} <pin>/"
```

Two `cargo build` invocations, not one: `--bin` filters targets across the whole invocation, and
`micold-daemon` has no `micold-ai-ide` bin, so a combined command silently builds only the client.
The `touch` is what forces cargo to re-uplift `target-shared/debug/<bin>` from `deps/`; a package it
considers fresh is never re-linked, so the copy would keep handing back another branch's binary.

## The cheap check first

Each copy carries a string only its own side of this branch produces:

| Binary | Marker | Count |
|---|---|---|
| `micold-ai-ide` | `Keep the service running when I'm signed out or away` (T028a settings copy) | 1 |
| `micold-daemon` | `nothing has been connected for the whole idle window` (T057 stop line) | 1 |

and zero of the other's — so neither copy came from a build that predates this work.

## The real check

Launched once under a private display and private XDG directories, so nothing here touched the
running instance on this machine:

```
env -i DISPLAY=:83 HOME=<t> XDG_RUNTIME_DIR=<t>/run XDG_DATA_HOME=<t>/data \
    XDG_CONFIG_HOME=<t>/config MICOLD_LOG=info <pin>/micold-ai-ide
```

The client spawned the daemon beside it (`spawn::daemon_binary` prefers a sibling of the current
executable), and the daemon's own log says:

```
INFO micold_daemon::server: client attached to daemon client_build=micold-ai-ide/0.12.1 client_window=833484
```

`client attached to daemon`, not `refusing client`. The client's log agrees: `attach: connected`.

**One thing worth writing down for the next person**: the first attempt put `XDG_RUNTIME_DIR` inside
the session scratchpad, whose path is long, and every attach failed with *"local socket name length
exceeds capacity of sun_path of sockaddr_un"* — a unix socket path is capped at 108 bytes. That is a
property of the harness, not of this feature, but it looks exactly like a broken endpoint. A short
`/tmp/<dir>` fixes it.
