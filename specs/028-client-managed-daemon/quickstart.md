# Quickstart: Validating the Client-Managed Service Lifecycle

**Feature**: 028-client-managed-daemon | **Date**: 2026-08-27

Two parts. **Part A** is the automated suite and is what CI runs. **Part B** is the handful of
things no test can honestly assert — a real thirty minutes, a real suspend, a real `.deb` upgrade,
a real container runtime — recorded once per release-worthy change with its date, machine and
outcome.

Prerequisites: the repo's usual toolchain (`mise trust`, then `mise run …`), and for the sandbox
parts a working Docker daemon. Nothing else is installed and nothing is enabled.

---

## Part A — automated

```bash
mise run test-core     # the pure rule, the clock, the presence counter
mise run test          # whole workspace, matching CI
```

Expected, by area:

| what | where | asserts |
|------|-------|---------|
| The rule | `micold-daemon` unit tests | zero connections + 30 min ⇒ stop; one connection ⇒ never; a live session does **not** hold it up (the clarified rule, and the inverse of the test it replaces) |
| Presence | `micold-daemon` unit tests | connect/disconnect pairs, reconnect cancels the window, a never-connected daemon is idle from startup |
| The clock | `micold-core` unit tests | monotonic (never decreases across many reads); `saturating_sub` at and below zero |
| Shutdown order | `micold-daemon` integration test | sessions persisted as `InterruptedResumable` **before** process trees are killed, endpoint released last |
| Teardown | `micold-daemon` integration test, short window | after the stop: no child processes, endpoint path gone, lock released — then a fresh daemon binds the same endpoint with no recovery step, over 20 consecutive cycles (SC-007) |
| The race | `micold-daemon` integration test | a connect issued as the window expires ends attached, with no `DaemonConnectFailed` reaching the client's banner |
| Migration | `micold-client` unit test (Linux) | the un-enable step is attempted when a unit is enabled, is skipped when nothing is, and every failure is swallowed |
| Packaging | `micold-client` test over the manifest | `[package.metadata.deb].assets` names no path under `usr/lib/systemd` |

The integration tests use a test-only short window; the 30-minute constant itself is asserted by a
unit test on the constant, and observed for real in §B1.

```bash
# The real-runtime parts (Linux + Docker), off by default so the default suite needs nothing:
cargo test -p micold-daemon --features sandbox-real-runtime sandbox_idle
```

| what | asserts |
|------|---------|
| Sandbox idle stop | with the opt-in off, after the (short, injected) window the container reports status `exited` and `RestartCount` unchanged |
| Opt-in interaction | with the opt-in on, the created container's argv carries `--restart unless-stopped` **and** the idle rule is suppressed — the container is still running after the window |

---

## Part B — recorded by hand

### B1. A real thirty minutes on the host

1. `mise run run`, start a session, type something, quit the client.
2. `pgrep -a micold-daemon` — the daemon is there. Note the time.
3. Leave the machine alone. At +29 min: still there. At +31 min: gone.
4. `pgrep -af 'claude|bash'` shows no process descended from it; `ls $XDG_RUNTIME_DIR/micold/` shows
   no socket.
5. Reopen the client: the session list appears, the session is marked resumable, no banner, no
   error. Resume it and confirm it runs. Time this step — attached and usable in under 3 seconds
   (SC-006).

**Record**: the two timestamps, the time-to-attached, and that step 5 showed nothing red.

### B2. The window is about connections, not activity

1. Start the client, open a session, and leave the client open and untouched for 35 minutes.
2. The daemon is still running. (If it is not, the rule is counting the wrong thing.)

### B3. Suspend

1. Start the client, quit it, suspend the machine within a minute or two.
2. Resume after more than 30 minutes.
3. Within 30 seconds of resuming, the daemon is gone.

**Record**: suspend duration and how long after resume the process disappeared.

### B4. A live session does not save the service *(the clarified rule — check it deliberately)*

1. Start a long-running session (`sleep 3000` is enough to be unambiguous).
2. Quit the client. Wait out the window.
3. The daemon and the `sleep` are both gone.
4. Reopen: the session is listed as interrupted-resumable, and did **not** resume on its own.

This is the behaviour the user chose over the previous "never exit while a session is alive" rule.
If it feels wrong at this step, that is a product conversation, not a bug.

### B5. The sandbox, opt-in off

1. Settings → session service → sandboxed placement, survive-logout **off**. Start a session.
2. `docker ps` shows the container up. Quit the client.
3. After the window: `docker ps -a` shows it `exited`; `docker inspect -f '{{.RestartCount}}'` is
   unchanged from before.
4. Reopen the client: the container starts again and the catalog, sessions and history are as they
   were.

### B6. The sandbox, opt-in on *(the amendment)*

1. Same, with survive-logout **on**.
2. After the window the container is **still running** — deliberately.
3. The settings copy at the toggle says so before the choice is made.

### B7. Upgrade migration on a real package

1. On a VM or container with the *previous* release installed, enable the old opt-in:
   `systemctl --user enable --now micold-daemon.socket`. Confirm
   `systemctl --user is-enabled micold-daemon.socket` says `enabled`.
2. `mise run deb`, install the new package over it.
3. `ls /usr/lib/systemd/user/` — both unit files are gone.
4. Open the client once, then quit.
5. `systemctl --user is-enabled micold-daemon.socket` — no longer enabled; `systemctl --user
   status` logs no failed unit. `journalctl --user -b | grep micold` shows no unit-not-found.
6. Sessions and settings from before the upgrade are all present.

**Record**: the `is-enabled` output before and after, and that step 5 was clean.

### B8. Nothing is registered on a clean install

1. Fresh VM. Note `systemctl --user list-unit-files | wc -l`.
2. Install, open the app, start a session, quit.
3. The count is unchanged and `systemctl --user list-unit-files | grep micold` is empty, while the
   session survived the client restart.

---

## What a failure here means

- **B1 late, B3 slow** — the clock or the tick, not the rule. See `research.md` R3.
- **B1 step 4 finds a stray process** — the stop took the `process::exit` path, skipping `Drop`. See
  R4; the order in `contracts/lifecycle.md` §3 is the fix.
- **B5 shows the container running with `RestartCount` climbing** — the opt-in was on, or the policy
  is `unless-stopped` when it should be `no`. That is exactly the restart loop R2 measured.
- **B7 step 5 shows a failed unit** — the client's migration ran after connecting, or not at all. See
  R7's ordering hazard.
