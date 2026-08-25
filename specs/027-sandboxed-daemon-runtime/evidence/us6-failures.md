# US6 verification: the sandbox reports its own failures, and can be restarted

**Date**: 2026-08-25 · **Runtime**: Docker 29.5.1 (build 2518b52), x86_64 Linux 7.0.0-30-generic
**Image**: `micold-daemon:dev` (`sha256:2a7f0d3e…`, built from this working tree)
**Covers**: quickstart.md §B.3 (network posture) and the failure items of §B.5 (lifecycle)

Unlike the US1 pass, this one is not a hand-driven `docker` transcript. The checks live in
`crates/micold-core/tests/sandbox_real_lifecycle.rs`, behind the `sandbox-real-runtime` feature,
and they drive **`CliRuntime` against a real Docker daemon** — the same `create`/`start`/`stop`/
`remove`/`find` calls the application makes, building the argument vector the application builds.
Docker is not what can be wrong here; the argv and the state machine are. Writing the probe by hand
would have tested a `docker create` I typed, not the one the product runs.

That also makes this evidence re-runnable: the same tests are what the Linux `sandbox-runtime` CI
job executes.

## The run

```
$ cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1
```

— the CI job's own command, verbatim, so the evidence and the job select the same tests.

### One finding, from writing it this way

The first run of these tests used plain descriptive function names, and they passed. Under the CI
command they would have run **zero times**: `sandbox_real_` is a *test-name* filter, and an
integration target's file name is not part of a test's path, so `tests/sandbox_real_lifecycle.rs`
matches nothing unless its functions are named to match too. The result is not a failure — it is a
smaller number in a green summary, which is the kind of gap a green suite hides. Every test here now
carries the prefix in its own name, and the contract is written down beside the filter in
`.github/workflows/ci.yml`.

```
     Running tests/sandbox_real_handshake.rs
running 3 tests
test sandbox_real_handshake_refuses_a_wrong_token ... ok
test sandbox_real_handshake_succeeds_with_the_mounted_token ... ok
test sandbox_real_state_is_written_where_the_host_can_read_it ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.18s

     Running tests/sandbox_real_lifecycle.rs
running 7 tests
test sandbox_real_a_container_stopped_from_outside_is_reported_lost ... ok
test sandbox_real_a_file_written_inside_belongs_to_the_host_user ... ok
test sandbox_real_an_explicit_stop_leaves_no_container_behind ... ok
test sandbox_real_no_outbound_blocks_egress_while_the_control_port_still_answers ... ok
test sandbox_real_no_outbound_still_resolves_names ... ok
test sandbox_real_outbound_permits_egress ... ok
test sandbox_real_the_daemons_state_survives_container_recreation ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 40.44s
```

(The three handshake tests are US1's, re-run here because the CI command selects them too; the seven
below them are this pass. Every other target in the workspace reported `0 passed; N filtered out`,
which is the filter doing its job.)

## §B.3 — network posture

| Check | Test | Result |
|---|---|---|
| `NoOutbound`: egress fails **and** the control channel is unaffected | `sandbox_real_no_outbound_blocks_egress_while_the_control_port_still_answers` | pass |
| `NoOutbound`: name resolution still works (the documented caveat) | `sandbox_real_no_outbound_still_resolves_names` | pass |
| `Outbound`: egress succeeds | `sandbox_real_outbound_permits_egress` | pass — **not** skipped; the host had egress |

The egress probe is `timeout 5 bash -c 'exec 3<>/dev/tcp/1.1.1.1/443'`. The image carries neither
`curl` nor `wget` (`packaging/sandbox/Containerfile` installs only ca-certificates, git,
openssh-client, procps, less, bash), so the quickstart's `curl` is spelled with bash's `/dev/tcp`.
Resolution is checked with `getent hosts example.com`.

The first of those tests is the one that earns its keep. It asserts egress is blocked *and* that
`TcpStream::connect_timeout` to the published control port still succeeds within 20s. The rejected
alternative implementation — `--internal` — passes the first half and fails the second, which is
exactly the regression that would sever the client from the daemon while looking like a hardening
win. Research R4 chose `enable_ip_masquerade=false` for this reason; this test is what keeps that
choice from being quietly undone.

The DNS caveat is stated in `docs/user-guide/sandboxed-daemon.md` (§ "What the sandbox does not
block", line 156ff): "with outbound connections blocked, **DNS lookups still resolve**".

## §B.5 — lifecycle and failure

| Check | Covered by | Result |
|---|---|---|
| `docker stop` from outside → persistent failure with a reason and a remedy, no silent unsandboxed fallback | `sandbox_real_a_container_stopped_from_outside_is_reported_lost` | pass |
| Daemon state survives container recreation | `sandbox_real_the_daemons_state_survives_container_recreation` | pass |
| An explicit stop leaves nothing behind, and repeating it is harmless (C-7) | `sandbox_real_an_explicit_stop_leaves_no_container_behind` | pass |
| A file written inside belongs to the host user | `sandbox_real_a_file_written_inside_belongs_to_the_host_user` | pass |

The stop-from-outside test stops the container with `docker stop`, confirms `CliRuntime::find`
still reports the container with `running == false`, and then requires
`lifecycle::container_lost` to move the sandbox out of `Running` into `Failed` carrying
`RuntimeError::SandboxStopped`, a `reason()` that names the container, and a non-empty `remedy()`
(FR-034). It does **not** reach an unsandboxed state: only `Failed` admits `accept_fallback`, and
only with an explicit `ConsentedFallback` (`lifecycle.rs` tests
`a_failure_cannot_reach_a_working_daemon_without_consent`; client side in `features/sandbox.rs`).

The recreation test writes a file into `/var/lib/micold-ai-ide` inside the container, asserts it
appears at the bound host path, then stops **and removes** the container, brings a second container
up over the same state directory, and reads the file back from inside. That is FR-011/M-3's claim
end to end, on a container that no longer exists.

## What this pass did **not** run

Reported honestly rather than ticked:

- **New project registered while the sandbox runs → marked stale, sessions keep working, nothing
  restarts on its own.** Not exercised against real Docker; it is pure state-machine behaviour with
  no container involvement, and it is covered by
  `crates/micold-core/tests/sandbox_state.rs::{registering_a_project_marks_a_running_sandbox_stale,
  a_stale_sandbox_still_serves_the_sessions_already_in_it, a_stale_sandbox_can_also_be_lost}`.
- **Accepting the offered fallback explicitly, and the unsandboxed state staying visible.** Needs
  the GUI. Covered at the unit level by `micold-client/src/features/sandbox.rs` (consent required,
  only from `Failed`) and `micold-client/tests/banner_is_not_a_snackbar.rs` (the state stays on a
  persistent surface rather than a transient one).
- **Sessions survive a client restart while sandboxed** (FR-014). Needs a GUI session and a client
  restart; not automatable here.
- **With survive-logout enabled, the sandbox comes back after a reboot** (FR-014a/b, R6). Needs a
  reboot of this machine, which is not mine to do.

The three unrun items are the ones that require a display or a reboot. Everything in §B.3 and every
container-level item in §B.5 was exercised against a real Docker daemon.
