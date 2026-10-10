# Quickstart: validating multiple daemons

## Part A: automated
1. `mise run test-core` : registry rules, migration (host and container fixtures), binding
   resolution, state transitions.
2. `mise run test` : client reducers (routing, removal flow, per-daemon isolation), geometry gates.
3. `mise run image && mise run test-sandbox` : `sandbox_real_multi_daemon*` against Docker/Podman
   with the scripted fake AI CLI: container session starts and echoes; host session does the same;
   with both bound, stop the container daemon and the host session still accepts input and the
   container worktrees report the daemon unreachable (SC-001, SC-002, SC-006).
4. `mise run gate` before pushing.

## Part B: visual pass (needs eyes at a display; `visual-pass` skill)
1. Open Settings -> Daemons: host row connected; add a container daemon, watch Starting ->
   Connected without reopening Settings.
2. Create two worktrees of one project, bind one to each daemon; sidebar labels each with its daemon.
3. Stop the container daemon: its worktrees show "daemon unavailable"; host worktree unchanged.
4. Remove the container daemon with bound worktrees: confirmation lists counts and "nothing on disk
   is deleted"; cancel leaves all unchanged; confirm shows "no daemon", then bind explicitly.
5. Upgrade check: start from a pre-feature settings.json (host, then container): one daemon, all worktrees
   usable, nothing asked.
