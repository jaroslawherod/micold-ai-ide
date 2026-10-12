# Quickstart: prove the remote daemon runtime

Part A is automated; Part B is the visual pass.

## Prerequisites
- Part A (`ssh_real_*`): Linux, `sshd` binary (`openssh-server`), `ssh`, `ssh-keygen`; nothing else.
  No root, no running SSH service, no container runtime.
- Part B: a display (`visual-pass` skill), and for the live check one reachable SSH host with a git
  repository (a stand-in `sshd` from Part A is enough).

## Part A: automated
1. Pure rules, every platform: `mise run test-core` (argv shape, classifier fixtures, backoff, mismatch
   text, remote identity round-trip, registry rules, state machine, status semantics).
2. Client and daemon units: `mise run test` (actor with a stand-in transport, reducers, `remote`
   subcommands against a temp endpoint).
3. Real `sshd`: `mise run test-ssh` (builds the daemon, starts the rootless `sshd`, runs `ssh_real_*`
   with `--test-threads=1`). Expected: end-to-end (add, worktree, session with the fake AI CLI, input
   and output, terminal command), drop and reconnect with output intact within 30 s, host-key
   unknown/changed, authentication refused, missing key, unreachable host within the timeout, version
   mismatch in both directions, replace offered with no sessions and withheld with sessions, two
   clients on one daemon, no new listener, local session unaffected throughout.
4. Gate: `mise run gate`.

## Part B: visual pass (quickstart for `visual-pass`)
Use the stand-in `sshd` from the test support (`cargo test -p micold-daemon --features
ssh-real-runtime ssh_real_support -- --nocapture` prints its alias) or a real host.
1. Settings -> Daemons -> Add: runtime "Remote over SSH"; pick the alias from the host list. Only the
   host (and an optional repository path) is asked; user, port and key are not.
2. Start it. Observe `Starting` with the copy progress, then `Connected`, the version and the
   `user@host:port` endpoint.
3. Add the repository; the project and its worktrees show the remote path and the daemon's name.
4. Cut the connection (stop the proxy). Observe `Reconnecting` with the reason, the session
   `disconnected`; restore it; observe `Connected` and the same session with its output.
5. Choose "stop trying" while cut: `Unreachable ... attempts stopped`; Reconnect works.
6. Start a host with a changed key: the reason names it; no prompt, no hang.
7. Start against a daemon of another version: `version mismatch` text with both versions, which side
   is older and the action available (Replace, or the instructions).
8. Check a local project meanwhile: its session keeps running and accepts input.
Record screenshots and notes in `visual-pass/`.
