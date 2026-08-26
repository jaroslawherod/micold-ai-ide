# Performance evidence: SC-003 and SC-004

**Date**: 2026-08-26 · **Runtime**: Docker 29.5.1, x86_64 Linux 7.0.0-30-generic
**Image**: `micold-daemon:dev` (`sha256:92adcb99e7e1…`, built from this working tree by `mise run image`)

---

## SC-003 — a sandboxed session start is no more than 2s slower

> *"With the sandbox already prepared, a session starts and shows its first prompt within the same
> order of time as an unsandboxed session — no more than 2 seconds slower."*

### The run

```
$ cargo test -p micold-daemon --release --features sandbox-real-runtime \
      sandbox_real_ -- --nocapture --test-threads=1
```

```
test sandbox_real_session_start_is_within_two_seconds_of_the_host_placement ...
host placement first screen: "$"
container placement first screen: "$"
host placement: median 2ms, min 2ms, max 3ms, all [2, 2, 3, 3, 2, 2, 3]
container placement: median 2ms, min 2ms, max 3ms, all [2, 2, 2, 2, 3, 2, 2]
SC-003 delta: 0ms (host median 2ms, container median 2ms)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.76s
```

**Result: 0ms.** The budget is 2000ms.

### What is measured

`crates/micold-daemon/tests/sandbox_real_session_start.rs`, behind `sandbox-real-runtime`. One test,
both arms, because the two numbers are only meaningful against each other — split in two, one arm
could report while the other silently failed to run.

Each round sends `SessionStart` + `SetViewedSession` and stops the clock at the first grid frame
**for that session** carrying a non-whitespace cell. Seven timed rounds per placement, each on a
fresh session (a session starts once), preceded by one untimed warm-up.

Deliberately outside the clock: image acquisition, container creation, and the handshake. The claim
is scoped to "with the sandbox already prepared"; folding those in would report a number true of a
first launch and of nothing else. They are SC-004's subject, below.

### Three ways this measurement was wrong before it was right

Worth recording, because each one *passed* while measuring nothing:

1. **The container arm never ran a shell at all.** The image sets `XDG_DATA_HOME=/var/lib`, so the
   daemon reads `/var/lib/micold-ai-ide/projects.json` — the state directory, one level *below* the
   data home. Mounting the seeded data home there put the catalogue at
   `…/micold-ai-ide/micold-ai-ide/projects.json`. The daemon logged `catalog adopted
   load_status=Missing` and `session start failed … err=no such session in the catalog`, sent
   nothing on the wire, and the test hung for 17 minutes: its deadline was checked between reads,
   and the read never returned. The timeout now wraps the read, so a daemon that answers nothing
   fails in 30 seconds and says where to look.

2. **Stopping at the first `full` frame measured the round trip, not the session.** The daemon
   answers `SetViewedSession` with a full snapshot immediately — of a screen the shell has not
   written to. That reported a 1ms median on both placements, which is not a shell starting. Hence
   the non-whitespace condition, and hence the test *prints the screen that stopped the clock*: the
   `"$"` above is what makes these numbers checkable rather than merely green.

3. **The two arms ran different shells.** `Supervisor::spawn_shell` takes the daemon's own `SHELL`,
   so the host arm got the developer's login shell *with their startup files* and the container arm
   got the image default. The run before the fix read `host placement first screen: "Command 'usage'
   not found, did you mean: …"` at a median of 850ms against `dash` at 2ms — a 848ms "sandbox
   advantage" that was entirely Ubuntu's `command-not-found` handler. Both arms now pin
   `SHELL=/bin/sh`, which exists on both sides and reads no startup file.

A fourth asymmetry was smaller but the same shape: the host daemon is spawned and connected to in
one breath, while the container daemon has been up since `wait_for_accept` first reached it, so the
host's first session paid for a cold process (one 706ms round among 2ms rounds). Both arms now open
an untimed warm-up session before the clock is started.

### What this does not establish

- **One machine, one runtime.** Docker on Linux with identity uid/gid mapping. The macOS and Windows
  placements route through a VM and a path-rewriting mount; neither is measured here.
- **`/bin/sh`, not the real workload.** Pinning the shell is what makes the arms comparable, but a
  session that starts `claude` does far more work than `dash`, and its own startup cost would
  dominate both columns. What is bounded here is the *placement's* contribution, which is the
  claim.
- **A warm page cache.** The image had been pulled and run before every timed round.

---

## SC-004 — first enable under five minutes, with continuous progress

Not yet measured. See T117.
