# US2 evidence — the presence count is honest

**Date**: 2026-09-03 · **Machine**: Linux 7.0.0-30-generic, x86_64 · **Commit**: `26cfdcaf`

## The problem with this story's tests

Every test written for T029–T031 **passed the moment it was written**. That is not a TDD red step,
and pretending otherwise would be the whole failure mode Principle I exists to prevent: a suite that
asserts what the code already does, drifts with it, and never fails.

The reason is structural rather than an oversight. The mechanism these tests cover — `Presence` fed
by `register`/`deregister` — landed in Phase 2 (T011/T012) because US2 could not start without it.
By the time the US2 tests existed, §2.5 and §2.6 already held: the refusal path returns before
`register`, and `deregister` already ran after `route()` on all three of its exits.

So the red step was recovered by **deliberately breaking the mechanism** and confirming each test is
the one that notices. Both probes were applied to a clean tree at `26cfdcaf` and reverted with
`git checkout` immediately after; the working tree was verified clean before continuing.

## Probe A — `deregister` stops decrementing the count

Removed the `presence.client_disconnected(now)` call from `DaemonState::deregister`.

```
test the_window_arms_when_the_last_client_leaves_and_expires_after_it ... FAILED
test a_live_session_does_not_hold_the_service_up ... FAILED
test result: FAILED. 3 passed; 2 failed          (daemon_lifecycle)

test the_reported_client_count_is_the_presence_count ... FAILED
test a_clean_close_decrements_the_count ... FAILED
test an_unclean_drop_is_counted_gone_within_sixty_seconds ... FAILED
test result: FAILED. 2 passed; 3 failed; finished in 60.00s   (presence_counting)

test a_session_outlives_the_client_that_was_connected_when_it_started ... FAILED
test result: FAILED. 2 passed; 1 failed          (session_survival)
```

Six failures across all three binaries. Two details worth keeping:

- `presence_counting` took **60.00s** rather than failing instantly — the §2.6 test really does wait
  out its ceiling and then assert against a *measured* elapsed time. A version of that test which
  merely finished would have passed under this probe.
- `the_reported_client_count_is_the_presence_count` failed too, which is what proves T032 removed a
  second counter rather than renaming the first: the client table's length still went to zero while
  the presence count did not.

## Probe B — a refused handshake registers

Inserted `state.register(client_identity)` immediately before the refusal's `return Ok(())`.

```
test a_refused_handshake_never_increments_the_count ... FAILED
test result: FAILED. 4 passed; 1 failed          (presence_counting)
```

Exactly one failure, and it is the §2.5 test. The other four are unmoved by a refusal, which is the
correct blast radius — a broader failure here would have meant the tests were coupled, not specific.

## What this does not cover

Suspend-inclusiveness (data-model G3) cannot be reached from a unit test; it needs a machine that
actually sleeps, and quickstart B3 is where it is measured (T066).
