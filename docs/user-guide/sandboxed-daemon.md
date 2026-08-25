# Running the session service in a container

The session service — the process that owns your terminals and worktrees — normally runs directly on
your computer, with the same access to your files that you have. It can instead run inside a
container, where it sees **only the projects you have registered** and nothing else.

This is off by default. Turning it on changes nothing about how sessions behave; what changes is
what a session can reach when it goes looking.

## Turning it on

Settings → **Session service** → "Run the service in a container", then restart the application.

You need a container runtime installed. Docker is supported today; Podman works and is exercised by
the same test suite, so if you already run Podman you can select it once the daemon section lands.

The first start downloads the service's image, which can take a few minutes on a slow connection.
After that it starts in seconds.

## What the container can see

| Reachable | Not reachable |
|---|---|
| Every project you have registered, at its own path | Your home directory |
| The service's own data directory | Any project you have not registered |
| The credentials you explicitly share (below) | Your SSH keys, browser profile, system configuration |
| | The container runtime's own control socket |

Projects are mounted at **the same absolute path** they have on your computer. That is deliberate:
git records absolute paths inside worktree metadata, and both the application and the service run
git, so a project seen at two different paths would make the two disagree about which worktrees
exist.

Files a session creates in a project come out owned by **you**, not by root.

## Credentials

Nothing of yours is shared unless you say so. Each item is a separate opt-in:

- **Git configuration** — your commit name and email, read-only.
- **SSH agent** — the agent's socket, so a session can push. Never your key files.
- **Git credentials** — the credential helper's store.
- **AI CLI sign-in** — the AI tool's own authentication.

With none of these on, a session that tries to push to a remote fails for want of credentials, and
the application says so rather than leaving you with an unexplained authentication error.

While any of them is on, the settings view shows which — a partially shared sandbox should never
look like a fully isolated one.

## Limits

Settings → **Session service** → **Limits** caps what the sandbox may consume. A session that tries
to use every core does not make your machine unusable, and one that forks without bound does not take
your desktop with it.

| Setting | Unit | Default | Lowest you can set |
|---|---|---|---|
| Processor limit | cores, fractions allowed | 2 | 0.25 |
| Memory limit | MiB | 4096 | 512 |
| Process limit | count | 512 | 64 |
| Writable storage limit | MiB | *unset* | 1024 |

**Leave a field empty to unset it.** Empty is not zero: it hands the decision to the container
runtime, which applies whatever it does by default — usually no limit at all. That is why the
writable-storage limit ships unset while the other three ship with a number.

**The minimums are not arbitrary and the form enforces them.** Below them the service does not work
rather than working slowly: a quarter of a core is roughly what it takes to keep the control channel
answering, 512 MiB is under what the daemon plus one session needs before the kernel starts killing
things, and 64 processes is about what a shell running a build already has open. Type something
smaller and the field itself refuses it, naming the minimum — the settings are not saved with a
number that would produce a sandbox that cannot start.

**Not every limit works with every setup, and the writable-storage one often does not.** Whether it
can be enforced depends on how your container runtime stores images — Docker's `overlayfs` driver
accepts a size cap, the older `overlay2` driver rejects it unless it sits on xfs with project quotas
enabled, and podman differs again. The application asks your runtime once, when the sandbox starts,
and remembers the answer against that runtime's version.

Where a limit cannot be enforced, its field is shown **disabled with the runtime's own reason**
underneath it. It is not hidden — a value you set earlier stays visible — and it is never quietly
accepted, because a number that is ignored is worse than no number at all: you would believe a bound
exists.

If a session is stopped because it hit a limit, the application names which limit and which setting
governs it, so you know which one to raise. The processor limit is the exception: a container over
its processor share is slowed down, never stopped, so it is never named as a cause.

## Network

By default the container **cannot open outbound connections**. Package installs and anything else
that reaches the internet will fail inside a session, and — importantly — so will the AI agent's
connection to its provider. If you need either, turn network access on for the sandbox; the
application warns you at the point you turn it off, for the same reason.

One thing to know precisely: with outbound connections blocked, **DNS lookups still resolve**. Names
resolve, connections to them do not. The block works by leaving the container's bridge without a
route out to the internet, and the runtime answers DNS from your side of that boundary — so a lookup
succeeds and the connection that follows it fails. Expect errors that say "connection refused",
"network unreachable" or a timeout, rather than "host not found"; a program reporting that it
resolved an address has not reached it.

That is a small channel — something inside could signal outward by choosing what to look up — and it
is stated here rather than left for you to discover. The guarantee is "no outbound connections", not
"no outbound traffic of any kind". The control channel between this application and the service is
unaffected either way; it does not travel over the container's network.

## Working offline

The sandbox does not need a registry. If the machine cannot reach one, bring the image in by hand:

```sh
# Somewhere with a connection
docker pull ghcr.io/micold/micold-daemon:<version>
docker save ghcr.io/micold/micold-daemon:<version> -o micold-daemon.tar

# On the machine without one
docker load -i micold-daemon.tar
```

Then point the image setting at the reference you imported. Once the image is present, everything
works with the network off entirely.

## Sessions, restarts and reboots

Closing the application leaves sessions running, exactly as it does without the sandbox. Reopening
re-attaches to them.

If you have "keep sessions running after logout" enabled, the sandbox comes back after a reboot with
its sessions live — on Linux, macOS and Windows alike. (Without the sandbox, that option only works
on Linux.)

Stopping and recreating the container does not lose anything: the service's data lives in your own
data directory, mounted in, so it outlives any container.

## When it does not start

Every failure names a cause and a next step. The common ones:

| What you see | What it means |
|---|---|
| "…is not installed" | No container runtime found. Install one, or switch back to running on this computer. |
| "…is installed but not running" | Start Docker (or Podman) and retry. |
| "…is not permitted to use it" | Your user is not in the `docker` group, or rootless Podman was never initialised. |
| "The image … was not found" | Check the image reference, or import an archive. |
| "…could not be fetched" | Rate limit, sign-in, or a proxy. Import from a file instead. |
| "Port … is already in use" | Something else holds the service's port. |
| "… cannot be shared with the sandbox" | The project is on a path the runtime will not mount — a network share, for instance. |

**The application never quietly runs unsandboxed instead.** If the sandbox will not start, you are
offered the choice explicitly, for that occurrence only: the next launch tries the sandbox again
without you having to remember. While you are running without it, the application says so
persistently — a sandbox that failed once and was forgotten about is a sandbox that is off for weeks.

If you add or remove a project while the sandbox is running, it is marked as needing a restart rather
than restarting itself. What the container can see is fixed when it is created, and restarting on its
own would end sessions you are using to service a settings change.

## For maintainers

`mise run image` builds a `:dev` image from the working tree. If you rebuild the application but not
the image, the two disagree, and the connection is refused with `StaleDevImage` naming the tag —
rather than connecting and misbehaving in ways that look like bugs in the new code. Run
`mise run image` again.

See `packaging/sandbox/README.md` for building and publishing.
