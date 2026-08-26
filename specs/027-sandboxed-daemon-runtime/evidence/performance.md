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

> *"First-time enablement, including preparing the sandbox on a working network connection,
> completes within 5 minutes and shows continuous progress throughout, so the user never has to
> guess whether the application has stopped responding."*

### The run

```
$ cargo test -p micold-core --features sandbox-real-runtime \
      sandbox_real_first_enable -- --nocapture --test-threads=1
```

```
test sandbox_real_first_enable_is_under_five_minutes_and_never_goes_quiet ...
SC-004 enable: total 851ms — acquire 419ms, create 258ms, start 123ms, answer 50ms (archive 67.7 MiB)
SC-004 progress: 1 reports during acquisition, stages ["Importing"]
SC-004 longest silence during acquisition: 396ms
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
```

**Result: 851ms.** The budget is 300,000ms.

### What is measured

`crates/micold-core/tests/sandbox_real_enable.rs`, behind `sandbox-real-runtime`. The clock covers
the application's whole enable sequence — `acquire_image`, `create`, `start`, and then waiting until
the daemon *inside* the container writes `listening (sandboxed)` into the state directory the host
shares with it. "Enabled" is the daemon answering, not the container existing.

The cold state is built by tagging the image under a throwaway reference, `docker save`-ing it, and
deleting the tag — so a developer's own `micold-daemon:dev` is never disturbed.

### Which acquisition route this is, and why the other two are not measurable here

`ImageSourceKind` has three arms, and only one of them can be driven from this repository today:

| Route | Measured? | Why |
|---|---|---|
| `Registry` | **No** | Nothing is published, so there is no reference to pull. This gap is why SC-004a exists. |
| `LocalBuild` | Not through the app | `acquire_image` deliberately refuses it — staging a cross-compiled Linux binary beside a Containerfile is a build-system job. Timed separately below. |
| `ImportedFile` | **Yes** | SC-004a's documented no-network procedure, and the one streaming acquisition runnable here. |

### Where this evidence is weaker than the number suggests

Say this plainly, because 851ms against a five-minute budget invites the wrong conclusion:

- **The total is real; the continuity is barely exercised.** Acquisition emitted **one** progress
  report over 419ms. That is `docker load` being nearly instant because every layer is already in
  the local store — the honest reading is not "progress is continuous" but "there was nothing long
  enough to report on". A cold *machine*, or a registry pull over a real network, moves data this
  run did not.
- **The claim's own long case is the unmeasurable one.** The per-line reporting that would carry a
  four-minute pull is `run_streaming` feeding `pull_progress`, and it is covered against a fake
  runtime in `crates/micold-core/tests/sandbox_runtime.rs`
  (`assert!(reports.len() >= 2, "SC-004 gives this five minutes; silence for that long reads as a
  hang")`). Mechanism tested, duration not.
- **67.7 MiB is the transfer size.** Stated so the total can be scaled by a reader on a slower link
  rather than taken as machine-independent.

The `MAX_SILENCE` bound the test asserts (10s between reports) therefore passes on a route that
could not plausibly have violated it. It is a regression guard, not proof of the claim.

---

## SC-004b — source change to running sandboxed, without a registry

> *"…without publishing an image and without any registry interaction, and the loop is no more
> onerous than the existing build-and-run loop plus a single image build."*

Measured directly, since it is a developer loop rather than an application path:

```
$ cargo clean -p micold-daemon --release --target x86_64-unknown-linux-gnu
Removed 194 files, 37.5MiB total
$ time mise run image
   Compiling micold-daemon v0.8.0
    Finished `release` profile [optimized] target(s) in 9.09s
Built micold-daemon:dev -- select it in Settings > Daemon > Image.
SECONDS_TOTAL=9
```

**9 seconds** from a cleaned daemon crate to a rebuilt image; **10 seconds** when nothing has
changed at all. That is one release compile of one crate plus a layer-cached `docker build` — the
"plus a single image build" the criterion allows, and no registry is involved at any point.

Not measured: a genuinely cold machine, where the dependency graph compiles from scratch. That cost
belongs to the existing build-and-run loop, not to sandboxing, but it is the one case where a first
enable on this repository could exceed five minutes, and it should be measured before the criterion
is called closed for a new contributor's machine.
