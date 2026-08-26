# US2 verification: the service keeps its promises inside the box

**Date**: 2026-08-19 · **Runtime**: Docker 29.5.1, x86_64 Linux · **Image**: `micold-daemon:dev`
**Covers**: quickstart.md §B.7 in full, and the parts of §B.5 that do not need a reboot

## §B.7 — the development loop (FR-024a/c/d)

`mise run image` builds the `:dev` image from the working tree, and the round trip through an
archive works:

```
$ mise run image                       # rc 0
$ docker save micold-daemon:dev -o micold-daemon-dev.tar     # 68M
$ docker rmi micold-daemon:dev         # absent locally
$ docker load -i micold-daemon-dev.tar
Loaded image: micold-daemon:dev        # and it runs identically afterwards
```

That is Principle IV's offline claim checked rather than asserted: the sandbox is reachable with no
registry involved.

`StaleDevImage` is exercised through the daemon's real accept path in
`micold-daemon/tests/sandbox_transport.rs`, in both directions — a mismatched fingerprint refuses a
local build and is accepted for a released one.

## The end-to-end handshake

`micold-core/tests/sandbox_real_handshake.rs`, behind the `sandbox-real-runtime` feature, starts a
container and connects to it the way the client does:

```
running 3 tests
test sandbox_real_handshake_refuses_a_wrong_token ... ok
test sandbox_real_handshake_succeeds_with_the_mounted_token ... ok
test sandbox_real_state_is_written_where_the_host_can_read_it ... ok
```

## Four real defects this run found

Everything above passed only after these were fixed. Each was invisible to the rest of the suite,
because the rest of the suite asserts on strings.

1. **`mise run image` built nothing.** `docker build` looks for `Dockerfile`; the file is a
   `Containerfile`, which podman finds by name and docker does not. Fixed with `-f`.
2. **The base image was too old to run the daemon.** `bookworm-slim` ships glibc 2.36 and a binary
   built on a current host needs 2.39, so the image built cleanly and failed the moment the daemon
   was executed. Base moved to `trixie-slim`, and `mise run image` now *runs* the binary inside the
   image so this cannot ship silently again.
3. **The daemon never bound a TCP listener.** The client dialled loopback and the daemon still only
   knew how to bind a Unix socket — so the sandbox could be created, started, and never connected
   to. `MICOLD_LISTEN_ADDR` is now checked before the endpoint, and binds the container's bridge
   address rather than its loopback (the runtime forwards to the former; binding the latter leaves
   the published port unreachable).
4. **The daemon could not find its own state directory.** Running as a uid with no passwd entry, it
   resolved the platform data directory under a home that does not exist and died with a bare
   `Permission denied`. The image now sets `XDG_DATA_HOME=/var/lib` so the daemon's own convention
   lands on the mounted directory, and `HOME` is passed in from the host so a shared `~/.gitconfig`
   is found where git looks for it.

The third and fourth are the reason this task existed. A feature that is configured, argv-verified
and never actually connected to is not a working feature, and only running it says so.

## Not covered here

The reboot items in §B.5 — survival with the opt-in on and off — need a reboot of this machine. The
mechanism is asserted in `sandbox_parity.rs` (`--restart unless-stopped` versus `no`, on every
platform) and in `logout_survival.rs`, but whether the host actually brings the container back is
not something a test can establish.

---

## Addendum, 2026-08-26 — parity measured rather than assumed, and a correction

### Correction to this record's §B.7 claim

The heading above says this pass covers "§B.7 in full". It did not, and the gap is instructive. What
was checked on 2026-08-19 was the *image* round-trip and the daemon-side refusal in
`sandbox_transport.rs` — both real, both still true. What was never checked was what the refusal
looks like to the developer it exists for. It reached them as a `{:?}` dump with no rebuild command,
and the image tag inside it was empty because nothing ever set `MICOLD_IMAGE_REFERENCE`. Both are
fixed and recorded in `evidence/us7-dev-loop.md`, which supersedes the §B.7 section above.

The lesson is not about §B.7. A refusal asserted at the layer that *constructs* it can be perfect
while the sentence the user reads is useless, and no amount of testing the constructor finds that.

### SC-001/FR-025 — the terminal behaves exactly as an unsandboxed one

§B.1's last box, and the one parity claim that had no test:
`crates/micold-daemon/tests/sandbox_real_parity.rs` stands up an unsandboxed daemon and a
containerised one, opens a session in each, and runs the same twelve commands in both. Each command
is one whose answer must not depend on placement — that excludes hostnames, pids and paths, and
leaves the terminal contract itself.

```
$ cargo test -p micold-daemon --features sandbox-real-runtime --test sandbox_real_parity -- --nocapture
                                          echo $TERM  ->  "xterm-256color"
                                           stty size  ->  "30 100"
                                               id -u  ->  "1000"
                                     echo $((6 * 7))  ->  "42"
                                      false; echo $?  ->  "1"
                                       true; echo $?  ->  "0"
                                        echo 'a   b'  ->  "a   b"
             for i in 1 2 3; do echo "line $i"; done  ->  "line 1\nline 2\nline 3"
                                     printf 'a\tb\n'  ->  "a\t      b"
                       printf '\033[31mRED\033[0m\n'  ->  "RED"
                                 printf 'no-newline'  ->  "no-newline"
          echo one two three | tr ' ' '\n' | tail -1  ->  "three"
test result: ok. 1 passed; 0 failed
```

Identical on both sides, command for command. Four of those are worth naming for what they would
have caught:

- `stty size` and `$TERM` are negotiated by the daemon, so a difference would be the daemon behaving
  differently inside a container rather than the shell doing so.
- The SGR line and the tab line pass through the grid encoder, so a divergence would mean the
  *client* renders sandboxed sessions differently — colours lost, columns wrong.
- `printf 'no-newline'` is the case where output shares a line with whatever follows it; it is also
  the case that broke the probe harness itself when it was first written.
- `id -u` returning 1000 on both sides is what makes `--user uid:gid` more than an argv string, and
  it is the same fact that makes a file written inside the sandbox editable on the host
  (`evidence/us1-isolation-from-a-session.md`).

The two arms use different temporary project directories, which is why nothing path-shaped is
compared. The test also guards its own comparison: twelve empty answers on both sides would satisfy
an equality loop while measuring nothing, so it requires nearly every probe to have printed
something.
