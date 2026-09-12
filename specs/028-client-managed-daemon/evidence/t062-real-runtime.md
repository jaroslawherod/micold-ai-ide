# The real-runtime suite, on this feature's build (T062)

**Date**: 2026-09-12 · **Runtime**: Docker 29.5.1 (linux/amd64), kernel 7.0.0-30-generic
**Image**: `micold-daemon:dev`, rebuilt from this working tree immediately before the run
(`mise run image`)
**Profile**: `--release`, both crates, one test thread — what `mise run test-sandbox` does
**Base**: `47b8481b`

These tests are off by default, so `mise run test` — and therefore the CI matrix — never runs them.
That is deliberate (a container runtime is not a build dependency), and it is also why this file
exists: the only record that the sandboxed half of the idle rule was exercised against a real
runtime is the one written by hand.

## The run

```
mise run image
mise run test-sandbox
```

`SANDBOX_EXIT=0`, 25 tests, no failures. The two this feature added:

```
Running tests/sandbox_real_idle.rs
test sandbox_real_idle_is_off_for_the_keep_running_opt_in ... ok
test sandbox_real_idle_sandbox_stops_and_stays_stopped ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.81s
```

The 27.81s is the two halves of FR-019/FR-022 measured against a real container: a sandbox with the
opt-in off stops when its window expires **and stays stopped** (no restart policy brings it back),
and a sandbox with the opt-in on is still running when the same window has passed.

The remaining 23 are feature 027's suite, re-run unchanged to show this feature did not disturb
them:

```
sandbox_real_enable         1   sandbox_real_boundary       1
sandbox_real_handshake      3   sandbox_real_fingerprint    1
sandbox_real_lifecycle      7   sandbox_real_limits         2
sandbox_real_storage        1   sandbox_real_parity         1
                                sandbox_real_session_start  1
                                sandbox_real_staleness      4
```

## The filter, and why the quickstart names `sandbox_real_idle`

`mise run test-sandbox` selects on `sandbox_real_`, which is a **test-name** prefix, not a file
name. The quickstart's single-file alternative has to obey the same rule, and originally did not:
it said `cargo test … sandbox_idle`, which matched nothing, exited 0 and printed `test result: ok`.
T063's gate now fails on any Part A filter that selects no test, and the two tests here were renamed
so the file stem `sandbox_real_idle` is also a name prefix.
