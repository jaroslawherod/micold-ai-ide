# US5 verification: the second runtime, and what it found

**Date**: 2026-08-26 · **Runtime**: podman 5.8.4 (Fedora build), rootless, `crun`, cgroup v2,
`overlay` storage, x86_64 Linux
**Image**: `micold-daemon:dev`, built from this working tree and carried in as a `docker save` tar
**Covers**: quickstart.md §B.2 (the boundary, adversarially) and §B.4 (limits), plus the storage
capability probe, all with `MICOLD_TEST_RUNTIME=podman`
**Tests**: `crates/micold-daemon/tests/sandbox_real_boundary.rs`,
`crates/micold-daemon/tests/sandbox_real_limits.rs`,
`crates/micold-core/tests/sandbox_real_storage.rs` — all behind `sandbox-real-runtime`

FR-020 says the runtime is replaceable. Every other piece of evidence for this feature was
produced against Docker, so the claim rested on a table of per-runtime spellings that nothing had
ever executed. This pass is that table being executed.

## The harness had to stop spelling `docker` first

The real-runtime tests drove `docker` by name, in their own bodies. A harness like that cannot
demonstrate a seam: it can only demonstrate the runtime it names. So the support module now takes
its runtime from `MICOLD_TEST_RUNTIME` and asks `Dialect::for_kind` for the program name, the
identity flags, and everything else it used to hard-code — the same table the application uses.
`--user 1000:1000` under Docker and `--userns=keep-id` under podman are then a difference the
application already knew about rather than one the test knows about.

Two probes had to become runtime-aware for the same reason: the boundary check asks for the
runtime's *own* control socket (C-3), which is `/var/run/docker.sock` under Docker and
`/run/user/<uid>/podman/podman.sock` under podman. Asking for Docker's path under podman would
have passed the box by asking about a file podman never created.

## The result

```
rootless=true cgroupmgr=systemd runtime=crun version=5.8.4
=== sandbox_real_storage
test sandbox_real_storage_capability_matches_what_the_runtime_enforces ... ok
=== sandbox_real_boundary
test sandbox_real_boundary_holds_from_inside_a_session ... ok
=== sandbox_real_limits
test sandbox_real_limits_change_only_by_recreating_the_container ... ok
test sandbox_real_limits_stop_the_session_not_the_daemon ... ok
ALLEXIT=0
```

Everything Docker was asked, podman was asked, and answered the same way — including the
keep-id half of §B.2, where a file written into the project from inside the container comes out
owned by the host user. The identity mapping is the one place the two runtimes genuinely differ,
and it is the one the dialect table exists for.

## What it found

Both of these had been green against Docker for the whole feature.

**The memory probe could not fail.** §B.4 drives a session past its memory limit with a perl
one-liner, and the one-liner was written `\$x = "A" x (512 * 1024 * 1024)`. The program is inside
single quotes, where `sh` expands nothing, so the backslash was not protecting `$x` — it was
handing perl a literal backslash, and `\$x = ...` is a fatal compile error. It passed under Docker
anyway, because perl **constant-folds** `"A" x 536870912` while compiling: the 512 MiB allocation
happens before the error can be reported, the kernel kills the process, and the test sees exactly
the `Killed` it was looking for. The first run where the limit was *not* enforced is the run where
this surfaced — as `Experimental aliasing via reference not enabled at -e line 1.` A negative
control that cannot fail correctly is not a control.

**`podman rm -f` returns before the name is free.** The limits test recreates a container under
the same name, and podman answered `creating container storage: the container name
"micold-limits-change" is already in use` — which reads like a leaked container from an earlier
run rather than the race it is. `purge` now waits for the runtime to stop listing the name before
returning.

## Two defects in the failure classifier, found by asking podman for its own words

The podman fixtures under `crates/micold-core/tests/fixtures/runtime/` had been *transcribed* from
podman's documentation, never captured. This task is where they were captured, and both surprises
came from the difference.

**A subuid range that exists but is too small is not the message we knew.** Podman 5.8.4 does not
refuse: it logs `no subuid ranges found ... check rootless mode`, warns that it is falling back to
single mapping, and carries on. The failure arrives later and in different words — mid-unpack, as
`potentially insufficient UIDs or GIDs available in user namespace ... Check /etc/subuid and
/etc/subgid` — with neither "subuid" nor "rootless" in a form the classifier recognised. It was
landing in `Unknown` while the remedy stayed one `usermod` away, which is the anonymous failure
FR-034 exists to prevent. Captured as `podman_err_subuid_range_too_small.txt` from a user given a
ten-id range, and now classified `PermissionDenied` like its sibling.

**"Port 0 is already in use", for both runtimes.** Podman 5.8.4 rootless publishes through pasta,
which writes the address as `127.0.0.1/7727` rather than Docker's `127.0.0.1:7727`. Reading the
port back only worked on Docker's separator — except it did not work there either: both runtimes
end the clause with a colon, and splitting from the right then read the empty string after it. The
variant was right and the number was zero, in the one place the user needs the number. The fixture
test now asserts the port, not just the variant; before the fix it fails on the *Docker* fixture.

## How this was run, and what that costs the claim

Podman cannot run on this host: there is no `newuidmap`/`newgidmap`, and
`kernel.apparmor_restrict_unprivileged_userns=1` blocks the unprivileged user namespace it needs.
So podman ran one level in, in a privileged `quay.io/podman/stable` container with a private
cgroup namespace, booting systemd, with linger enabled for its unprivileged `podman` user:

```
docker run -d --name micold-podman-rig --privileged --cgroupns=private \
  --tmpfs /run --tmpfs /run/lock -v <rig>:/mnt/run quay.io/podman/stable /sbin/init
```

That image ships `/etc/containers/containers.conf` with `cgroups="disabled"` and
`cgroup_manager="cgroupfs"`, under which `--memory` is accepted and silently dropped — the first
run reported `memory.max = max` and the limits test failed honestly. A user-level
`containers.conf` re-enabling cgroups with the systemd manager is what makes `--memory` reach the
kernel; the log line above records `cgroupmgr=systemd` for exactly that reason.

**Do not bind-mount the host's `/sys/fs/cgroup` into a rig like this.** An earlier attempt did,
and the container's `user@1000.service` was the host user's real session. Removed, and the host's
sessions confirmed intact before continuing.

The nesting is the limit of this evidence: it is podman's own code path — rootless, `crun`,
user namespaces, keep-id, cgroup v2 limits enforced by the kernel — but reached through a
privileged outer container rather than from a login shell on a machine where podman is the
installed runtime. What it establishes is that the seam is real and the dialect table is
executable. What it does not establish is that a podman-native host has no further surprises.
