# Quickstart: Daemons dial the orchestrator

Part A is by hand with no SSH; Part B uses a rootless `sshd`. There is no visual pass (no UI).

## A. Host daemon

1. `micold-daemon orchestrator` in one terminal (it listens on `orchestrator.sock` beside the daemon socket).
2. In `daemon.toml`: `[orchestrator_link]` / `enabled = true`; start `micold-daemon`.
3. List the registry (`{"query":"list"}` on `orchestrator.sock`, or the #1724 test helper): the daemon is
   `online`, kind `host`.
4. Stop the orchestrator: the daemon logs `orchestrator link lost` once, sessions keep running; start it:
   `online` again within 16 s with no daemon restart.
5. Remove the table: no dial, no new log line at start (FR-010).

## B. Through a forward (Linux, `ssh_real` suite)

`scripts/gate.sh` does not run it; run `cargo test -p micold-daemon --features ssh-real-runtime --test ssh_real ssh_real_link -- --test-threads=1` per the suite's
README (rootless `sshd` on loopback, real `ssh`).

1. Four daemons (host, container, ssh, pod: pod = a second loopback `sshd`): each `online` once, own kind.
2. Kill the bridge `ssh` of one: `offline` within 10 s, the others unchanged; reopen: `online` within 16 s,
   the daemon pid unchanged.
3. On the remote, `ss -xl` shows `orchestrator.sock` only; connecting from the remote to a client address
   fails (SC-006).
4. Kill the app's `ssh` with `kill -9` and reopen at once while the remote sshd session lives: `online`.
5. Start with `AllowStreamLocalForwarding no`: the open fails with ssh's text (FR-005).

## Test layers

See plan.md, *Test strategy*.
