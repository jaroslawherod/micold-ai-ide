# The §B pass, end to end (T120)

**Date**: 2026-08-26 · **Runtime**: Docker 29.5.1 (linux/amd64), cgroup v2 with the `systemd`
driver, `overlayfs` storage, kernel 7.0.0-30-generic
**Image**: `micold-daemon:dev`, rebuilt from this working tree immediately before the run
**Profile**: `--release`, for the whole suite rather than only the test that demands it
**Base**: `28ae99f`

The other files in this directory each record one user story, captured while it was being built.
This one records the pass §B asks for: the whole thing, in one sitting, against one runtime, on
one build of the image — because a set of boxes ticked on different days against different images
is not the same claim as a suite that passes together.

## The run

Two commands, both green, 22 tests:

```
cargo test --release -p micold-core   --features sandbox-real-runtime --no-fail-fast sandbox_real_ -- --test-threads=1
cargo test --release -p micold-daemon --features sandbox-real-runtime --no-fail-fast sandbox_real_ -- --test-threads=1
```

```
Running tests/sandbox_real_enable.rs
test sandbox_real_first_enable_is_under_five_minutes_and_never_goes_quiet ... ok
Running tests/sandbox_real_handshake.rs
test sandbox_real_handshake_refuses_a_wrong_token ... ok
test sandbox_real_handshake_succeeds_with_the_mounted_token ... ok
test sandbox_real_state_is_written_where_the_host_can_read_it ... ok
Running tests/sandbox_real_lifecycle.rs
test sandbox_real_a_container_stopped_from_outside_is_reported_lost ... ok
test sandbox_real_a_file_written_inside_belongs_to_the_host_user ... ok
test sandbox_real_an_explicit_stop_leaves_no_container_behind ... ok
test sandbox_real_no_outbound_blocks_egress_while_the_control_port_still_answers ... ok
test sandbox_real_no_outbound_still_resolves_names ... ok
test sandbox_real_outbound_permits_egress ... ok
test sandbox_real_the_daemons_state_survives_container_recreation ... ok
Running tests/sandbox_real_storage.rs
test sandbox_real_storage_capability_matches_what_the_runtime_enforces ... ok

Running tests/sandbox_real_boundary.rs
test sandbox_real_boundary_holds_from_inside_a_session ... ok
Running tests/sandbox_real_fingerprint.rs
test sandbox_real_a_freshly_built_image_passes_the_strict_fingerprint_check ... ok
Running tests/sandbox_real_limits.rs
test sandbox_real_limits_change_only_by_recreating_the_container ... ok
test sandbox_real_limits_stop_the_session_not_the_daemon ... ok
Running tests/sandbox_real_parity.rs
test sandbox_real_parity_a_sandboxed_terminal_answers_exactly_as_an_unsandboxed_one ... ok
Running tests/sandbox_real_session_start.rs
test sandbox_real_session_start_is_within_two_seconds_of_the_host_placement ... ok
Running tests/sandbox_real_staleness.rs
test sandbox_real_a_project_registered_after_boot_is_outside_the_running_container ... ok
test sandbox_real_sessions_survive_a_client_restart ... ok
test sandbox_real_the_survival_opt_in_brings_the_sandbox_back_without_the_application ... ok
test sandbox_real_without_the_opt_in_nothing_brings_the_sandbox_back ... ok
```

The first attempt was not in release and stopped at `sandbox_real_session_start`, which refuses to
measure at all outside it: *"run this with --release: the containerised daemon is release-built, so
a debug host daemon would make the sandbox look faster than it is."* That refusal is the test
working. Worth knowing that it also ends the run — cargo stops at the first failing target, so a
debug pass silently skips `sandbox_real_staleness` behind it unless `--no-fail-fast` is passed.

Only the `micold-core` half of this runs in CI (`sandbox against a real runtime (linux)`); the
`micold-daemon` half is local-only, because CI builds these in debug and the timing test above
would refuse there. Anyone repeating this pass has to run both commands themselves.

## The last box that could be closed: survival (§B.5, FR-014a/b, R6)

> With survive-logout enabled, the sandbox comes back after a reboot

The claim is about the *host* restarting. Rebooting is not something a suite may do to the machine
it runs on, so the claim was taken apart and the part that is the mechanism was measured.

The opt-in selects a restart policy, and that decision now has one home —
`micold_core::sandbox::argv::restart_policy` — which both `argv::create` and the real-runtime
harness call. Before this, the harness hardcoded `"no"`; a harness that spelled the policy itself
would have been checking its own copy of the decision rather than the application's.

`sandbox_real_the_survival_opt_in_brings_the_sandbox_back_without_the_application` then starts a
sandbox with the opt-in on, asserts the runtime *received* `unless-stopped`, opens a session and
runs a command in it, kills the container, and requires that:

- the runtime restart it unasked — `State.Running` true again with a **changed** `StartedAt`;
- the daemon inside come back up and accept a client, with nothing on the host helping it;
- the catalogue still hold the session (a sandbox that returns empty has survived in name only);
- the session answer a command again.

`sandbox_real_without_the_opt_in_nothing_brings_the_sandbox_back` is the control: same death, opt-in
off, policy `no`, and the container stays exited (FR-014c — a setting turned off must stop acting).

### `docker kill` is the wrong kill, and it fails silently

The obvious spelling made the test fail, and for a reason worth recording: **an API-issued kill is
recorded as a manual stop**, and declining to restart after a manual stop is the whole difference
between `unless-stopped` and `always`. So the runtime honouring the policy and the runtime ignoring
it look identical — the container stays down either way. Measured here: 60s after `docker kill`, a
container created with `--restart unless-stopped` was still `false`, at its original `StartedAt`.

The probe therefore signals the container's main process on the host instead
(`kill -9 $(docker inspect -f '{{.State.Pid}}' …)`). That is the death the policy exists for:
nothing asked for it, so the runtime restarts. It needs no privilege — the sandbox runs as the host
user (`--user uid:gid` on Docker, `--userns=keep-id` on podman), so its PID 1 is ours to signal.

### What this does not establish

A restart policy is acted on in two situations: the container dies, and the runtime itself starts.
This measures the first. The second — dockerd restoring policy-carrying containers when it comes up
at boot — is the runtime's own documented behaviour, is why the policy is `unless-stopped` rather
than `always`, and nothing in this repository can influence it. **No reboot was performed**, and the
box is ticked on that basis, not on a reboot.

Two further limits: the probe ran under Docker only (podman enforces the same policy through
conmon, untested here), and rootless podman needs a generated systemd unit to restore containers at
boot at all — a packaging concern this feature does not carry.

## The box left open

> Idle with the view open: no repainting

Still **inconclusive**, and left unticked. `evidence/us3-settings-view.md` records why: under
lavapipe the Settings view costs no more idle than the main surface, which is the comparison that
can be made there, but the absolute claim rests on `idle_requests_no_frames.rs`. A software
rasteriser cannot settle it, and this pass did not change that.
