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

## Results (close unit, 2026-10-11)

Part A
1. `mise run test-core`: green (core `--all-targets`, 173 result groups, 0 failed).
2. `mise run test`: client reducers, routing, removal flow, per-daemon isolation and geometry gates green (client `--no-fail-fast`, 183 result groups, 0 failed). `github_locate_desktop_launch` skipped locally with `MICOLD_SKIP_GH_LAUNCH_TEST=1` (#696).
3. `sandbox_real_multi_daemon`, `_container` and `_host` against podman: 3 passed (M4 PR #768, `MICOLD_TEST_RUNTIME=podman`); CI's sandbox job runs them on docker.
4. `mise run gate`: green on the close tree.

Part B (headless sway, `visual-pass/visual-pass-m3.md`)
1. Daemons list, details, add form: PASS. The Starting to Connected transition needs a container runtime: NOT RUN locally, covered by the M4 e2e.
2. Two worktrees bound to different daemons, sidebar labels: PASS for the label and the bind picker; two live daemons NOT RUN locally, covered by the M4 e2e.
3. Stopped daemon "daemon unavailable": sidebar state PASS (`m3-09`); live stop NOT RUN locally, covered by `sandbox_real_multi_daemon`.
4. Removal confirmation with counts, cancel, confirm to "no daemon", explicit rebind: PASS (`m3-04`, `m3-06`, `m3-07`).
5. Upgrade from a pre-feature settings.json: not driven by hand; covered by `upgrade_single_daemon.rs` (host and container fixtures).
