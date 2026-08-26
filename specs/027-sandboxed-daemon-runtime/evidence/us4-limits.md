# US4 verification: the limits, and one that was not a limit

**Date**: 2026-08-26 · **Runtime**: Docker 29.5.1, `overlayfs`, cgroup v2, x86_64 Linux
**Image**: `micold-daemon:dev`, built from this working tree
**Covers**: quickstart.md §B.4 (limits)
**Tests**: `crates/micold-daemon/tests/sandbox_real_limits.rs`,
`crates/micold-core/tests/sandbox_real_storage.rs` — both behind `sandbox-real-runtime`

`argv.rs` already asserts the right flag with the right unit for each supported limit, and no flag
for an unsupported one. None of that can tell you whether a limit *holds*, which is the only thing
FR-016 asks. So both checks below run against a real container: one drives a real session past its
memory limit, the other writes past a storage limit and compares what happened to what the
application claims about it.

## A session cannot exceed its memory, and the daemon survives it (FR-016)

Container created with the memory flags `argv::budget_args` emits, at 256 MiB. `docker inspect`
reports `HostConfig.Memory = 268435456`, so the limit reached the runtime and not just the argv.
Then, typed into a real session over the control channel:

```
$ perl -e '$x = "A" x (512 * 1024 * 1024); print "ALLOCATED\n"'
Killed
```

`ALLOCATED` never appears. The session itself keeps working (`echo still-here` answers), the
container is still `Running`, and a second client connects to the same daemon afterwards — so the
kernel stopped the runaway process and not the daemon, which is the half of FR-016 that a flag
assertion cannot reach.

The test requires the word `Killed` (or an out-of-memory message), not merely the absence of
`ALLOCATED`: an absent success line is equally consistent with a missing interpreter, a quoting
mistake, or input the daemon dropped. Without that, the probe would pass while measuring nothing.

**One deviation from production argv, deliberately.** The test also passes `--memory-swap` equal to
`--memory`. Docker's default swap allowance is twice the memory limit, so without it the 512 MiB
allocation succeeds — slowly, on disk — and the test proves nothing while looking like it passed.
The application does not pass `--memory-swap`, so on a host with swap the memory limit is softer
than this test's. That is a real gap, recorded rather than papered over.

## A changed limit takes effect only by recreating the container

A limit is fixed at creation; `docker start` on an existing container silently keeps the old value.
The test creates at 256 MiB, asserts `HostConfig.Memory`, recreates at 512 MiB, asserts it again,
and then connects — because a limit that takes effect by making the sandbox unstartable is not an
improvement.

## The storage limit was classified `Supported` and was not enforced

This is the finding. R5 recorded this measurement and read it as enforcement:

```
$ docker run --rm --storage-opt size=1G alpine:latest true ; echo $?
0                                     # Docker 29.5.1, storage driver: overlayfs
```

The flag is accepted. It is also ignored:

```
$ docker run --rm --storage-opt size=512m … sh -c 'dd if=/dev/zero of=/big bs=1M count=700'
734003200 bytes (734 MB, 700 MiB) copied, 0.302584 s, 2.4 GB/s

$ docker run --rm --storage-opt size=2g … sh -c 'df -h /'
overlay         481G  361G   96G  80% /
```

700 MiB written into a 512 MiB cap, no error, and a root filesystem reporting the host's whole disk.
So `storage_support` was returning `Supported` for `overlayfs`, the Settings view would have offered
the field as editable, and a user setting it would have believed in a bound that did not exist —
exactly the outcome C-2 and SC-009 exist to prevent, and the one a user is least able to discover,
because everything looks like it worked.

**Fixed**: both overlay drivers are now `Unsupported` with the reason (project quotas — xfs
`pquota`, ext4 `prjquota` — which `docker info` does not expose); btrfs and zfs stay supported.
`research.md` R5 and `docs/user-guide/sandboxed-daemon.md` are corrected.

**Kept honest**: `sandbox_real_storage.rs` probes the runtime the way the application does, then
writes past the cap for real, and fails if the claim and the behaviour disagree **in either
direction** — a false negative that denies a working limit is a failure too. On this machine it now
takes the `Unsupported` branch and asserts the reason is non-empty, because SC-009 renders it beside
the disabled field.

## The §B.4 box this does not close by itself

"Shown **disabled with the reason**, not hidden and not silently accepted" is half a view claim.
`micold-client/src/ui/settings/daemon.rs` renders it and `features_settings.rs` covers the draft
behaviour; what is verified here is the input that view is given — that the capability arriving at
it now says unsupported, with a reason, on this runtime. The rendering was checked by eye in
`evidence/us3-settings-view.md`.

## Runs

```
running 2 tests
test sandbox_real_limits_change_only_by_recreating_the_container ... ok
test sandbox_real_limits_stop_the_session_not_the_daemon ... the session's account of the limit being reached:
Killed
ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.01s

test sandbox_real_storage_capability_matches_what_the_runtime_enforces ... ok
```
