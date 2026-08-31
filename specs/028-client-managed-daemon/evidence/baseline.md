# Baseline before feature 028

**Recorded**: 2026-08-31 · **Commit**: `4821b34e` (spec only; no code changed yet) ·
**Toolchain**: rustc 1.97.1 (8bab26f4f 2026-07-14) · **Task**: T001

This is the before-state the US1 deletions are measured against. Two facts: the suite was green, and
this machine has the very enablement packaging contract §2.6 says the application must clean up —
so the migration T025 adds can be observed here rather than only reasoned about.

## `mise run test` — green

```
248 test binaries, 2504 tests passed, 0 failed
```

No failures, no ignored-and-forgotten targets. The full log is not checked in; the counts are what a
later run is compared against. The number that matters after US1 is not "2504 again" — US1 deletes
`may_exit`'s assertions and adds four guard tests — but that nothing green went red for a reason
unrelated to a deletion this feature made on purpose.

## `systemctl --user list-unit-files | grep micold`

```
app-gnome-micold\x2dai\x2dide-26468.scope    transient  -
micold-daemon.service                        disabled   enabled
micold-daemon.socket                         enabled    enabled
```

Both units are installed by the current release:

```
-rw-r--r-- 1 root root 1104 Aug 26 02:00 /usr/lib/systemd/user/micold-daemon.service
-rw-r--r-- 1 root root  877 Aug 26 02:00 /usr/lib/systemd/user/micold-daemon.socket
```

And the socket is **enabled for this user** — the per-user symlink that no root maintainer script can
reach:

```
/home/jaro/.config/systemd/user/sockets.target.wants/micold-daemon.socket
  -> /usr/lib/systemd/user/micold-daemon.socket
```

That symlink is exactly what packaging contract §2.6 puts on the application rather than the package,
and this machine is a genuine upgrade case for quickstart B7. The `.scope` line is not ours to
remove: it is the transient scope the desktop launcher puts the *client* in, and it disappears with
the process.

## What this makes checkable later

- §1.2 — after US1 the two unit files are gone, because the package no longer ships them.
- §2.6 — after opening the app once, the `sockets.target.wants/` symlink above is gone.
- §2.7 — opening it a second time runs nothing, because there is nothing left enabled.
