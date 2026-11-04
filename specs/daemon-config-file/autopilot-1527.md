# Autopilot ledger — #1527 daemon-config-file (Issue 6)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1527` (Deliver the configuration to container daemons); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526 and #1534 (see `autopilot-1524.md`, `autopilot-1526.md`); the tasks unit only cuts milestones for the "Issue 6" group (T030–T035, T090).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1527
- **Worktree branch**: feat/1527_deliver-the-configuration-to-container-daemons
- **Started**: 2026-11-03
- **Phase**: implement (M1 in progress)
- **Next step**: PR open; orchestrator waits on CI and merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1671 | Design (milestone for Issue 6) | merged | 2d674a9a494599c2a45de2afca5ab714fa2c43ed |
| #1687 | M1 | open | — |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | container daemons read the same read-only `/run/micold/config/daemon.toml` at boot and over SSH; no `MICOLD_*` env; `LANG=C.UTF-8` | T030–T035, T090 | full | #1687 |

Checked 2026-11-03: `container_env` is at `daemon.rs:1328` (not 1283); `docs/user-guide/sandboxed-daemon.md` exists; the entrypoint reads no `MICOLD_SSH_DIR` today. Fixed in tasks.md.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec? | No: design merged on main (PR #1509, `specs/daemon-config-file/spec.md`, FR-005, FR-029, FR-039; contract §3.3). Same precedent as #1524, #1525, #1526, #1534 | agent-resolved | specs/daemon-config-file/autopilot-1526.md D1 |
| D2 | implement | T030 says "the old writable sign-in mount is gone"; remove it in M1? | No: it travels with T065 (Issue 11), which delivers the sign-in in the read-only secrets folder. Removing it earlier leaves a container with no AI sign-in and no replacement. M1 mounts config and secrets read-only and adds no single-file mount | agent-resolved | tasks.md T065 names `CredentialShare::writable` and `sandbox/mod.rs:317-322` |
| D3 | implement | A container's `daemon.toml` needs `listen.tcp` + token by V3, but the container is reached over SSH and its daemon serves the unix socket the bridge dials (`remote.rs` `run_bridge`), as it did with no `MICOLD_LISTEN_ADDR` | `listen.socket` is valid for kind `container` (V3 relaxed; pods keep `tcp`); delivery renders `socket = "auto"`, `data_dir`, `home`, `image`, `ssh.dir`, `idle_stop = "off"` for the keep-running opt-in | agent-resolved | `crates/micold-daemon/src/remote.rs`, `server.rs` `tcp_listen_addr` |
| D4 | implement | Where do the config and secrets folders live? | `<state_dir>.delivery/{config,secrets}`, beside the state folder: inside it they would be writable through the state mount at `/var/lib/micold-ai-ide`, defeating the read-only mounts (FR-029) | agent-resolved | `DeliveryMount::beside` |
| D5 | implement | `LANG=C.UTF-8` "unless `session.env` sets it": `session.env` is parsed but no daemon code applies it yet | The daemon's session env hygiene sets `LANG` for a container daemon (config `data_dir` is the container's) before the session's own variables, so one place serves boot and SSH starts and a later `session.env` application overrides it | agent-resolved | `supervisor.rs` `env_hygiene_with` |
| D6 | implement | The basic image's `ENV MICOLD_DATA_DIR` | Kept: the same image is the Kubernetes pod default, and pods get no `daemon.toml` until Issue 7 (#1528); removing it would move pod state. It is image metadata, not a `-e` on `create` or the bridge; Issue 15 retires it. Containers made before this change keep their old mounts until recreated (the image reference changes with the image) | agent-resolved | review A F4/F5 |

## Review rounds

Tasks review A round 1 (`f688c7ee:038b306fe`): CHANGES, F1 MAJOR (TZ unassigned) fixed in T030/T033/T090, F2/F3 MINOR fixed in T033/T034; wording only, no new round. (Spec review skipped: no spec written; the shared spec was reviewed in PR #1509.)

M1 review A (`code-review` high, tree `ba66a1d6`): CHANGES, 9 findings. Fixed: F1/F2 (image hash test, pin `BASIC_IMAGE_CONTENT_HASH` refreshed to the tree; the `ENV MICOLD_DATA_DIR` removal reverted, D6), F3 (`LANG` kept when the daemon inherited one), F4 (`adopt` replaces a container with no `/run/micold/config` mount: `StaleReason::MadeBeforeConfigDelivery`), F5 (ENV kept for pods), F6 (atomic write through `owner_only::write`), F9 (stale comments). Declined: F7, F8 (below). Fix round re-reviewed by a fresh sonnet subagent (scoped to the fix areas; snapshot unknown after the rebase, so over the whole diff): CLEAN, no BLOCKER/MAJOR. Review B round 1: CHANGES, F1 (real-runtime results) and F2 (cycle-log) addressed; F2 fixed in `tdd/cycle-log.md`.

Real-runtime results (`MICOLD_RUNTIME=podman MICOLD_TEST_RUNTIME=podman mise run test-sandbox`, tree after rebase on origin/main): 39 `sandbox_real_*` pass, 20 fail. One failure was ours and is fixed: `sandbox_real_idle_is_off_for_the_keep_running_opt_in` asserted the old `MICOLD_IDLE_STOP` env; it now reads `idle_stop = "off"` from the container's mounted `daemon.toml`. The rest are podman-on-this-host failures that do not touch config delivery: `uid=1000` mount option rejected by podman, `localhost/` image prefix in `inspect`, cgroup memory limit not enforced, probe-echo parsing in shell-output asserts, and `/etc/passwd` (root:root 664) not writable by the keep-id uid, so the entrypoint's home fix never applies and the restart tests (`provision_then_restart`) find no daemon under `$HOME`; the entrypoint's passwd logic and the Containerfile are unchanged by this PR. The SSH-provisioning tests (`multi_daemon*`, `session_start`) passed and failed on different runs while other worktrees ran podman tests on the same fixed ports. Not baselined against origin/main (worktree creation is blocked in this session): CI runs this suite on docker, which is the proof for the boot and SSH start cases.

## Declined review findings

- M1 review A F7 (MINOR): only `errors[0]` of a failed render is reported (same as `deliver_host`), and `<state>.delivery` is not removed with the daemon. Left: the first error names the fault; cleanup on removal belongs to a later daemon-removal change.
- M1 review A F8 (MINOR): the entrypoint's awk reads `[ssh] dir` without a TOML parser. Left: the file is rendered by the application, which writes no multi-line values under `[ssh]`; a miss falls back to the default directory.

## Handover

None.
