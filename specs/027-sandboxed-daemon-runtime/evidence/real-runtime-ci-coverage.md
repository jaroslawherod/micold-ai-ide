# The half of the real-runtime suite that ran nowhere (T147–T149)

**Date**: 2026-08-27 · **Runtime**: Docker 29.5.1 (linux/amd64), `overlayfs`, kernel
7.0.0-30-generic
**Image**: `micold-daemon:dev`, rebuilt from this working tree immediately before the run
**Profile**: `--release`, both crates
**Base**: `2f905ead` (this branch's parent on `main`)

## What was wrong

`.github/workflows/ci.yml`'s `sandbox-runtime` job had exactly one test step:

```
cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1
```

T006 asked for a job "running the `sandbox_real_*` tests". Twelve of them are in `micold-core`.
The other eleven are in `micold-daemon`, and `-p micold-core` means cargo never built that crate's
targets — so they were not skipped, not ignored, and not reported as anything. They produced no
output at all, which is why a green job looked complete.

The eleven are not leftovers. `micold-core`'s twelve test the adapter: the argv a real runtime
accepts and the isolation it produces. `micold-daemon`'s test what that isolation is for, and each
is some user story's own claim:

| Test | The claim |
|---|---|
| `sandbox_real_boundary_holds_from_inside_a_session` | US1 / §B.2 — the boundary, probed through a session the daemon spawned |
| `sandbox_real_parity_a_sandboxed_terminal_answers_exactly_as_an_unsandboxed_one` | US2 / §B.3 — twelve commands, identical answers in both placements |
| `sandbox_real_limits_stop_the_session_not_the_daemon` | US4 — a limit ends the session, the service keeps answering |
| `sandbox_real_limits_change_only_by_recreating_the_container` | US4 — limits are creation-time, and the surface must say so |
| `sandbox_real_the_image_ships_every_ai_cli_the_application_offers` | FR-023a — every `AiCli::ALL` is on `PATH` in the published image |
| `sandbox_real_a_freshly_built_image_passes_the_strict_fingerprint_check` | FR-024d — the fingerprint refusal does not refuse a matching pair |
| `sandbox_real_session_start_is_within_two_seconds_of_the_host_placement` | SC-003 — the measured latency claim |
| `sandbox_real_a_project_registered_after_boot_is_outside_the_running_container` | FR-035 — the premise staleness rests on |
| `sandbox_real_sessions_survive_a_client_restart` | FR-014 — sessions outlive the application |
| `sandbox_real_the_survival_opt_in_brings_the_sandbox_back_without_the_application` | FR-014a/b — the restart policy, exercised by killing the container's process |
| `sandbox_real_without_the_opt_in_nothing_brings_the_sandbox_back` | FR-014c — the control that makes the one above mean something |

They have been run once: by hand, in the T120 pass on 2026-08-26, whose own evidence file records
**two** commands where CI runs one. Between then and now, nothing would have noticed any of them
breaking.

Meanwhile quickstart §B.2 said, in writing, that its boxes "re-check themselves on every
`sandbox-runtime` CI run". That sentence was false the day it was written, and it named
`sandbox_real_boundary.rs` — one of the eleven — while saying it.

## The run that landed the fix

The exact step now in the workflow, run on this machine before pushing it, so the job lands green
rather than hopefully:

```
cargo test --release -p micold-daemon --features sandbox-real-runtime sandbox_real_ \
  --no-fail-fast -- --test-threads=1
```

11 named tests, 0 failures. Run standalone on purpose: `micold-daemon`'s `sandbox-real-runtime`
feature does not forward to `micold-core`'s, and the question CI asks is whether the daemon's own
feature is sufficient by itself. It is.

And the whole suite together, as `mise run test-sandbox` now spells it:

```
cargo test --release -p micold-core -p micold-daemon \
  --features micold-core/sandbox-real-runtime,micold-daemon/sandbox-real-runtime \
  sandbox_real_ --no-fail-fast -- --test-threads=1
```

23 named tests, 0 failures — 12 from `micold-core`, 11 from `micold-daemon`.

SC-003, measured twice on the day, both runs:

```
host placement: median 3ms  container placement: median 2ms  → delta 0ms
host placement: median 2ms  container placement: median 2ms  → delta 0ms
```

The allowance is 2 seconds. What that number does *not* include is deliberate and documented in the
test: image acquisition, container creation and the handshake are SC-004's subject, and the clock
starts once both daemons are up.

## What this does not settle

- **Whether the job passes on a GitHub runner**, until it has. The reasoning for expecting it to:
  the survival probe signals the container's PID 1 on the host, which needs no privilege because
  the sandbox runs as the host user (`--user uid:gid`), and the latency test is a median of seven
  rounds against a host baseline measured on the same machine, with a 2-second allowance against a
  measured 0 — a noisy runner moves both arms together. The push is the actual answer.
- **A host reboot.** Unchanged from T120: the restart policy is exercised by killing the
  container's own process, which is the abrupt end a reboot is from inside. That the runtime
  restores the container when *it* starts at boot is the runtime's documented behaviour for
  `unless-stopped`, and nothing here can influence it.
- **macOS and Windows.** Docker Desktop is not available on GitHub's runners for either, so the
  real-runtime job stays Linux-only and the three-platform matrix keeps covering the adapter
  against an injected fake. That asymmetry is in test depth, not behaviour, and is unchanged by
  this task.
