## Milestone M4 — provisioned daemons via the orchestrator
(specs/1726-app-via-orchestrator)

**Deliverable**: SSH, container and Kubernetes daemons are routed through the orchestrator after
provisioning (FR-007 in full); a refused link forward is shown in the daemon's status cell.
**Verify**: `cargo test -p micold-client --test orchestrator_provisioned`;
`cargo test -p micold-client --test main orchestrator_single_connection daemon_config_delivery_host`;
`cargo test -p micold-core provision`; `cargo test -p micold-daemon --test ssh_real ssh_real_link` (needs `sshd`)
**Tasks**: T023–T034 · **Satisfies**: US3 (provisioned placements); FR-007, FR-009, FR-013

## What changed
- Core: `argv::hold_link` and `provision::hold_link` / `LinkHolder` — after provisioning, an `ssh`
  running `remote hold-link` keeps only the orchestrator link's `-R` open (prints `ready` once ssh
  confirmed the forward). New `micold-daemon remote hold-link`.
- Client (`daemon.rs`): `route_provisioned` closes the provisioning bridge, opens the holder and
  routes to the daemon by its wire id (`routed_to`, 10 s deadline). Wired into the SSH, container
  and pod paths (`kube_attempt_routed` / `kube_reconnect_routed`; the relay outlives the holder).
  No fallback to the bridge.
- Container and pod `daemon.toml` now carry `[daemon] id` (they announced `local` before).
- FR-009: the actor keeps the holder across a lost route; after an orchestrator restart it routes
  again over the same holder instead of re-provisioning.
- `orchestrator_link()` is unconditional; SSH/container/pod options return `io::Result`.
- User guide `orchestrator-mode.md`: remote, container and pod daemons go through the
  orchestrator; a refused forward is named in the status cell. `drift-M4.md` records the drift.

## Tests
`orchestrator_provisioned` (own binary, process-wide router; real orchestrator, scripted `ssh`,
real daemon linked by wire id): routed with holder kept; orchestrator killed → re-routed over the
same holder, nothing provisioned again; holder end observed; unbound forward refused with its
cause; a daemon that never links fails at the deadline naming its id. Container `daemon.toml`
names the wire id. `ssh_real_link` gains a holder case (runs only where `sshd` exists).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvPiCoDdkee8v234nRg
